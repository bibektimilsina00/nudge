//! Tools this project has never heard of.
//!
//! Six integrations written by hand buy six integrations. Speaking the protocol
//! buys the ones that exist now and the ones written next year, and it is the
//! single change that closes most of the gap between what Nudge can reach and
//! what a person actually wants done -- their email, their calendar, their
//! issues, their notes.
//!
//! **It adds no interface.** A server is three lines in `config.toml` and a
//! hundred new tools; there is no panel, no tab and nothing to click. That is
//! deliberate: a plugin surface is a promise about shapes, and these shapes
//! belong to other people.
//!
//! ## What this speaks
//!
//! JSON-RPC 2.0 over a child process's stdin and stdout, one message per line,
//! which is MCP's stdio transport. Not the HTTP transport, which is for servers
//! someone else is hosting and is a different set of problems -- authentication
//! among them.
//!
//! ## The simplification worth knowing about
//!
//! One mutex per server, held across the whole request and reply. Calls to a
//! single server are therefore serial, and a router matching replies to requests
//! by id -- the usual design -- is not needed, because there is only ever one
//! request outstanding.
//!
//! Different servers still run at once, which is the parallelism that matters:
//! nobody has two questions for GitHub at the same instant, and the slow tool is
//! slow because it is talking to the network rather than because it is queued.
use crate::error::{Error, Result};
use serde_json::{json, Value};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};

/// The version of the protocol we claim to speak.
///
/// Servers reply with their own and the two are expected to differ; this is the
/// oldest one still universally understood, which is the right thing to say when
/// the point is reaching servers nobody here has seen.
const PROTOCOL: &str = "2024-11-05";

/// How long any single exchange may take.
///
/// Generous, because a tool call may be doing real work over a network, and
/// finite, because a server that never answers must not take the assistant with
/// it. A hung request fails the turn; it does not hang the hotkey.
const PATIENCE: std::time::Duration = std::time::Duration::from_secs(30);

/// Starting one is slower: `npx` may go and fetch the server before running it.
const STARTUP: std::time::Duration = std::time::Duration::from_secs(90);

/// One tool on one server, as the server described itself.
#[derive(Clone, Debug)]
pub struct Tool {
    pub server: String,
    pub name: String,
    pub about: String,
    /// The server's own JSON Schema for the arguments. Passed to the model
    /// verbatim rather than translated: the server knows what it wants, and a
    /// translation is a second thing that can be wrong.
    pub schema: Value,
}

impl Tool {
    /// How this tool is named to the model and in a goal: `github/create_issue`.
    pub fn id(&self) -> String {
        format!("{}/{}", self.server, self.name)
    }

    /// One line, for the prompt.
    ///
    /// Names and argument names, not schemas. A server with thirty tools would
    /// put several thousand tokens of JSON Schema into **every** turn, paid for
    /// on every hotkey press whether or not anything reaches for a tool -- which
    /// is more than the screenshot costs. A wrong argument type comes back from
    /// the server as an error the model can read and correct, and that costs one
    /// round trip on the rare turn it happens rather than tokens on all of them.
    ///
    /// Required arguments are starred.
    pub fn line(&self) -> String {
        let props = self.schema.get("properties").and_then(Value::as_object);
        let required: Vec<&str> = self
            .schema
            .get("required")
            .and_then(Value::as_array)
            .map(|r| r.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        let args: Vec<String> = props
            .map(|p| {
                p.keys()
                    .map(|k| match required.contains(&k.as_str()) {
                        true => format!("{k}*"),
                        false => k.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        // First sentence only. Server descriptions run to paragraphs, and the
        // rest of the paragraph is for a reader deciding whether to install it.
        let about = self.about.split(['.', '\n']).next().unwrap_or("").trim();
        let about = match about.chars().count() > 90 {
            true => format!("{}…", about.chars().take(90).collect::<String>()),
            false => about.to_string(),
        };
        match about.is_empty() {
            true => format!("{}({})", self.id(), args.join(", ")),
            false => format!("{}({}) -- {about}", self.id(), args.join(", ")),
        }
    }
}

/// What the config file says about one server.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Spec {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    /// Extra environment for the child -- which is where tokens go, so that a
    /// server's credentials sit beside the server rather than in Nudge.
    ///
    /// A value of `keychain:name` is looked up in the Keychain rather than used,
    /// so the config file can be copied, opened and screenshotted without
    /// carrying somebody's token with it. See [`super::secret`].
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
}

struct Wire {
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    id: u64,
}

struct Server {
    spec: Spec,
    wire: tokio::sync::Mutex<Wire>,
    /// Kept so the child is killed when this is dropped rather than outliving
    /// the application. `kill_on_drop` is set when it is spawned.
    _child: Child,
}

impl Server {
    async fn start(spec: Spec) -> Result<(Self, Vec<Tool>)> {
        // Resolved before the child exists, so a missing Keychain item stops the
        // server here with a sentence about the Keychain -- rather than starting
        // it with a blank token and failing later, further away, in the server's
        // own words.
        let mut env = std::collections::HashMap::new();
        for (key, value) in &spec.env {
            env.insert(key.clone(), super::secret::resolve(value)?);
        }

        let mut command = tokio::process::Command::new(&spec.command);
        command
            .args(&spec.args)
            .envs(&env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            // Inherited rather than piped: servers narrate on stderr, and a pipe
            // nobody reads fills up and stops the server mid-sentence.
            .stderr(Stdio::inherit())
            .kill_on_drop(true);

        let mut child = command.spawn().map_err(|e| {
            Error::Config(format!(
                "mcp server {:?}: could not run {:?}: {e}",
                spec.name, spec.command
            ))
        })?;
        let stdin = child.stdin.take().ok_or_else(|| no_pipe(&spec.name))?;
        let stdout = child.stdout.take().ok_or_else(|| no_pipe(&spec.name))?;

        let server = Server {
            spec,
            wire: tokio::sync::Mutex::new(Wire {
                stdin,
                stdout: BufReader::new(stdout),
                id: 0,
            }),
            _child: child,
        };

        let hello = server
            .request(
                "initialize",
                json!({
                    "protocolVersion": PROTOCOL,
                    "capabilities": {},
                    "clientInfo": {"name": "nudge", "version": env!("CARGO_PKG_VERSION")},
                }),
                STARTUP,
            )
            .await?;
        // Required by the protocol, and easy to leave out: several servers answer
        // `initialize` happily and then refuse everything afterwards without it.
        server.notify("notifications/initialized").await?;

        let named = hello["serverInfo"]["name"].as_str().unwrap_or("?");
        let spoke = hello["protocolVersion"].as_str().unwrap_or("?");
        eprintln!("mcp: {} is {named}, speaking {spoke}", server.spec.name);

        let listed = server.request("tools/list", json!({}), PATIENCE).await?;
        let tools = listed["tools"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .filter_map(|t| {
                Some(Tool {
                    server: server.spec.name.clone(),
                    name: t["name"].as_str()?.to_string(),
                    about: t["description"].as_str().unwrap_or_default().to_string(),
                    schema: t["inputSchema"].clone(),
                })
            })
            .collect();
        Ok((server, tools))
    }

    /// Send a request and wait for the reply with the same id.
    async fn request(
        &self,
        method: &str,
        params: Value,
        patience: std::time::Duration,
    ) -> Result<Value> {
        let mut wire = self.wire.lock().await;
        wire.id += 1;
        let id = wire.id;
        let line = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});

        wire.stdin.write_all(format!("{line}\n").as_bytes()).await?;
        wire.stdin.flush().await?;

        let read = async {
            let mut buf = String::new();
            loop {
                buf.clear();
                let n = wire.stdout.read_line(&mut buf).await?;
                if n == 0 {
                    return Err(Error::Config(format!(
                        "mcp server {:?} stopped without answering {method:?}",
                        self.spec.name
                    )));
                }
                let Ok(msg) = serde_json::from_str::<Value>(buf.trim()) else {
                    // Servers print all sorts of things. Anything that is not a
                    // JSON-RPC message is somebody else's log line.
                    continue;
                };
                // Notifications and replies to somebody else are skipped rather
                // than treated as an answer -- the mutex means there is no
                // somebody else, but a server is free to talk unprompted.
                if msg["id"].as_u64() != Some(id) {
                    continue;
                }
                if let Some(e) = msg.get("error").filter(|e| !e.is_null()) {
                    let said = e["message"].as_str().unwrap_or("no reason given");
                    return Err(Error::Config(format!(
                        "mcp {}: {method} failed: {said}",
                        self.spec.name
                    )));
                }
                return Ok(msg["result"].clone());
            }
        };

        match tokio::time::timeout(patience, read).await {
            Ok(result) => result,
            Err(_) => Err(Error::Config(format!(
                "mcp server {:?} did not answer {method:?} within {}s",
                self.spec.name,
                patience.as_secs()
            ))),
        }
    }

    /// A message with no id, which expects no reply and must not be waited for.
    async fn notify(&self, method: &str) -> Result<()> {
        let mut wire = self.wire.lock().await;
        let line = json!({"jsonrpc": "2.0", "method": method, "params": {}});
        wire.stdin.write_all(format!("{line}\n").as_bytes()).await?;
        wire.stdin.flush().await?;
        Ok(())
    }
}

fn no_pipe(name: &str) -> Error {
    Error::Config(format!("mcp server {name:?} gave no pipe to talk over"))
}

/// Every configured server, and everything they can do.
#[derive(Default)]
pub struct Servers {
    running: Vec<Server>,
    tools: Vec<Tool>,
    /// Which ones did not start, and why.
    ///
    /// Kept rather than only logged. A server that fails silently is missing from
    /// the menu and missing from the prompt, which reads as *not configured* --
    /// and the most likely reason it failed is a credential, which is exactly the
    /// thing somebody needs telling about.
    failed: Vec<(String, String)>,
}

impl Servers {
    /// Start them all, and carry on without the ones that will not start.
    ///
    /// A broken entry in a config file must not stop the assistant from
    /// launching. It is reported on stderr and its tools are simply absent,
    /// which is also what the model is told -- there is no tool to call, so
    /// there is nothing to explain.
    pub async fn start(specs: &[Spec]) -> Servers {
        let mut servers = Servers::default();
        for spec in specs {
            match Server::start(spec.clone()).await {
                Ok((server, tools)) => {
                    eprintln!("mcp: {} offers {} tools", spec.name, tools.len());
                    servers.tools.extend(tools);
                    servers.running.push(server);
                }
                Err(e) => {
                    eprintln!("mcp: {e}");
                    servers.failed.push((spec.name.clone(), e.to_string()));
                }
            }
        }
        servers
    }

    pub fn tools(&self) -> &[Tool] {
        &self.tools
    }

    /// Why a server is not here, for anything that shows what is.
    pub fn failed(&self, name: &str) -> Option<&str> {
        self.failed
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, why)| why.as_str())
    }

    /// Run one, and give back whatever text it produced.
    pub async fn call(&self, server: &str, tool: &str, args: Value) -> Result<String> {
        let Some(found) = self.running.iter().find(|s| s.spec.name == server) else {
            return Err(Error::Config(format!(
                "no mcp server called {server:?} -- there is {}",
                match self.running.is_empty() {
                    true => "none configured".to_string(),
                    false => self
                        .running
                        .iter()
                        .map(|s| s.spec.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                }
            )));
        };
        let result = found
            .request(
                "tools/call",
                json!({"name": tool, "arguments": args}),
                PATIENCE,
            )
            .await?;

        // A tool that failed says so in the result rather than in a JSON-RPC
        // error, which is the protocol's way of letting a model read the failure
        // and try something else. Handed back as an error anyway, because
        // everywhere else in Nudge a failed step is an error the model is shown.
        let text = read_content(&result);
        match result["isError"].as_bool().unwrap_or(false) {
            true => Err(Error::Config(format!("{server}/{tool} failed: {text}"))),
            false => Ok(text),
        }
    }
}

/// Flatten MCP's content blocks into the text a model can read.
///
/// Only text is kept. Images come back as base64 in the same list, and a
/// screenshot pasted into a history that already carries one is a lot of tokens
/// for a second opinion nobody asked for.
fn read_content(result: &Value) -> String {
    let blocks = result["content"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default();
    let text: Vec<String> = blocks
        .iter()
        .filter_map(|b| match b["type"].as_str() {
            Some("text") => b["text"].as_str().map(str::to_string),
            Some(other) => Some(format!("({other} omitted)")),
            None => None,
        })
        .collect();
    match text.is_empty() {
        // Not an error and not nothing: plenty of tools do a thing and say
        // nothing about it, and silence reads as failure if it is left blank.
        true => "Done, with nothing to report.".into(),
        false => text.join("\n"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tool_is_named_by_its_server_and_itself() {
        let t = Tool {
            server: "github".into(),
            name: "create_issue".into(),
            about: String::new(),
            schema: json!({}),
        };
        assert_eq!(t.id(), "github/create_issue");
    }

    #[test]
    fn a_tool_reads_as_one_line_with_required_arguments_starred() {
        let t = Tool {
            server: "files".into(),
            name: "read_text_file".into(),
            about: "Read a file from disk. Supports head and tail. Only within allowed dirs."
                .into(),
            schema: json!({
                "properties": {"path": {"type": "string"}, "tail": {"type": "number"}},
                "required": ["path"],
            }),
        };
        assert_eq!(
            t.line(),
            "files/read_text_file(path*, tail) -- Read a file from disk"
        );
    }

    /// A server that describes nothing still has to be callable.
    #[test]
    fn a_tool_with_no_description_or_arguments_still_reads() {
        let t = Tool {
            server: "x".into(),
            name: "ping".into(),
            about: String::new(),
            schema: json!({}),
        };
        assert_eq!(t.line(), "x/ping()");
    }

    #[test]
    fn text_blocks_are_joined_and_others_are_named() {
        let r = json!({"content": [
            {"type": "text", "text": "first"},
            {"type": "image", "data": "...."},
            {"type": "text", "text": "second"},
        ]});
        assert_eq!(read_content(&r), "first\n(image omitted)\nsecond");
    }

    /// Against a real server, which is the only way to find out.
    ///
    /// Ignored by default: it needs `npx`, a network on first run, and about a
    /// minute. Run it on purpose --
    ///
    ///     cargo test --lib mcp -- --ignored --nocapture
    #[tokio::test]
    #[ignore = "needs npx and the network"]
    async fn a_real_server_can_be_started_listed_and_called() {
        let dir = std::env::temp_dir().join("nudge-mcp-test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("hello.txt"), "a line from a file\n").unwrap();

        let spec = Spec {
            name: "files".into(),
            command: "npx".into(),
            args: vec![
                "-y".into(),
                "@modelcontextprotocol/server-filesystem".into(),
                dir.display().to_string(),
            ],
            env: Default::default(),
        };
        let servers = Servers::start(std::slice::from_ref(&spec)).await;
        let tools = servers.tools();
        assert!(!tools.is_empty(), "the server offered no tools");
        for t in tools {
            println!("  {}", t.line());
        }

        let read = tools
            .iter()
            .find(|t| t.name.contains("read") && t.name.contains("file"))
            .expect("a filesystem server with no way to read a file");
        let said = servers
            .call("files", &read.name, json!({"path": dir.join("hello.txt")}))
            .await
            .expect("the call failed");
        println!("  {} -> {said:?}", read.id());
        assert!(said.contains("a line from a file"));

        // And a failure has to come back as one, rather than as an answer.
        let refused = servers
            .call("files", &read.name, json!({"path": "/etc/passwd"}))
            .await;
        println!("  outside the allowed directory -> {refused:?}");
        assert!(
            refused.is_err(),
            "reading outside its own directory was allowed"
        );
    }

    /// A tool that did its job and said nothing must not read as one that failed.
    #[test]
    fn silence_is_reported_as_success() {
        assert_eq!(
            read_content(&json!({"content": []})),
            "Done, with nothing to report."
        );
        assert_eq!(read_content(&json!({})), "Done, with nothing to report.");
    }
}
