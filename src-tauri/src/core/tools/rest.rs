//! Connectors that are a table rather than a program.
//!
//! Most of what an integration does is one shape: take some named arguments,
//! put them in a URL or a JSON body, send it with a token attached, and hand
//! back what comes out. OpenWorker writes that by hand for every endpoint of
//! every service -- 4,894 lines for forty connectors, which is about thirty
//! lines each of URL, header, argument list and schema. Every one of those
//! lines is a thing to maintain when a vendor moves a path.
//!
//! But look at what the thirty lines contain: a base URL, an auth header, a
//! method, a path with placeholders, and a list of arguments. None of it is
//! logic. So this runs the shape once and takes the rest as data, and adding a
//! service becomes writing a table.
//!
//! What it deliberately does not do is anything clever. There is no templating
//! language, no response transformation, no pagination helper. A service that
//! needs those is a service that should have a real server written for it; this
//! is for the large majority that do not.
//!
//! Everything here produces [`super::mcp::Tool`], so a REST connector arrives in
//! the model's tool list indistinguishable from an MCP one -- and inherits the
//! risk classification, the reviewer, the tool selection and the Keychain
//! indirection without any of them knowing this exists.
use super::mcp::Tool;
use serde_json::{json, Map, Value};

/// How the token is attached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Auth {
    /// `Authorization: Bearer <token>`. What most of them want.
    Bearer,
    /// A header of its own, named here. Notion and a few others.
    Header(&'static str),
    /// A query parameter, which is worse and occasionally the only option.
    Query(&'static str),
}

/// Where an argument goes once it has been given.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Put {
    /// Substituted into the path for `{name}`.
    Path,
    /// Appended as `?name=value`.
    Query,
    /// A field in the JSON body.
    Body,
}

#[derive(Debug, Clone, Copy)]
pub struct Arg {
    pub name: &'static str,
    pub kind: &'static str,
    pub about: &'static str,
    pub put: Put,
    pub needed: bool,
}

/// One callable thing.
#[derive(Debug, Clone, Copy)]
pub struct Op {
    pub name: &'static str,
    pub about: &'static str,
    pub method: &'static str,
    /// Relative to the service's base, with `{arg}` for path arguments.
    pub path: &'static str,
    /// Query parameters that are part of the call rather than part of the
    /// question -- YouTube's `part=snippet`, say. Not arguments: a model has no
    /// way to choose them well and no reason to be asked.
    pub fixed: &'static [(&'static str, &'static str)],
    pub args: &'static [Arg],
}

/// One service.
#[derive(Debug, Clone, Copy)]
pub struct Service {
    pub key: &'static str,
    pub base: &'static str,
    pub auth: Auth,
    /// Headers every call needs that are not the token -- an API version, say.
    pub headers: &'static [(&'static str, &'static str)],
    pub ops: &'static [Op],
}

impl Service {
    /// The tools this service offers, in the shape everything else consumes.
    pub fn tools(&self) -> Vec<Tool> {
        self.ops
            .iter()
            .map(|op| {
                let mut props = Map::new();
                let mut needed = Vec::new();
                for a in op.args {
                    props.insert(
                        a.name.to_string(),
                        json!({ "type": a.kind, "description": a.about }),
                    );
                    if a.needed {
                        needed.push(Value::String(a.name.to_string()));
                    }
                }
                Tool {
                    server: self.key.to_string(),
                    name: op.name.to_string(),
                    about: op.about.to_string(),
                    schema: json!({
                        "type": "object",
                        "properties": Value::Object(props),
                        "required": Value::Array(needed),
                    }),
                }
            })
            .collect()
    }

    fn op(&self, name: &str) -> Option<&Op> {
        self.ops.iter().find(|o| o.name == name)
    }

    /// Run one, and give back whatever text came out.
    pub async fn call(&self, token: &str, tool: &str, args: &Value) -> Result<String, String> {
        let op = self
            .op(tool)
            .ok_or_else(|| format!("no tool called {tool:?} on {:?}", self.key))?;

        let mut url = format!("{}{}", self.base, op.path);
        let mut query: Vec<(String, String)> =
            op.fixed.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        let mut body = Map::new();

        for a in op.args {
            let Some(v) = args.get(a.name) else {
                if a.needed {
                    return Err(format!("{} needs {}", op.name, a.name));
                }
                continue;
            };
            // Strings arrive quoted from JSON and must not be; everything else
            // is rendered as it would be written.
            let text = match v {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            match a.put {
                // Encoded, because an id with a slash in it would otherwise
                // become a different path and reach a different thing.
                Put::Path => url = url.replace(&format!("{{{}}}", a.name), &encode(&text)),
                Put::Query => query.push((a.name.to_string(), text)),
                Put::Body => {
                    body.insert(a.name.to_string(), v.clone());
                }
            }
        }

        if url.contains('{') {
            return Err(format!("{} was not given every part of its path", op.name));
        }

        let mut req = crate::core::http().request(
            op.method
                .parse()
                .map_err(|_| format!("{} has no method", op.name))?,
            &url,
        );
        req = match self.auth {
            Auth::Bearer => req.bearer_auth(token),
            Auth::Header(name) => req.header(name, token),
            Auth::Query(name) => {
                query.push((name.to_string(), token.to_string()));
                req
            }
        };
        for (k, v) in self.headers {
            req = req.header(*k, *v);
        }
        if !query.is_empty() {
            req = req.query(&query);
        }
        if !body.is_empty() {
            req = req.json(&Value::Object(body));
        }

        let res = req
            .send()
            .await
            .map_err(|e| format!("{} could not be reached: {e}", self.key))?;
        let status = res.status();
        let text = res.text().await.unwrap_or_default();

        // A failure comes back as an error rather than as an answer, which is
        // what the rest of Nudge does with a failed step -- and stops a 403 body
        // being read by a model as though it were data.
        match status.is_success() {
            true => Ok(text),
            false => Err(format!("{} said {}: {}", self.key, status.as_u16(), clip(&text))),
        }
    }
}

/// Percent-encode one path segment.
///
/// Hand-rolled because the alternative is a dependency for fourteen lines, and
/// the set of characters that may appear unescaped in a path segment has not
/// changed since 2005.
fn encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Enough of a failure to act on, without pasting somebody's whole error page
/// into a prompt.
fn clip(s: &str) -> String {
    let s = s.trim();
    match s.char_indices().nth(300) {
        Some((i, _)) => format!("{}…", &s[..i]),
        None => s.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ARGS: &[Arg] = &[
        Arg { name: "id", kind: "string", about: "which one", put: Put::Path, needed: true },
        Arg { name: "q", kind: "string", about: "search", put: Put::Query, needed: false },
        Arg { name: "title", kind: "string", about: "new title", put: Put::Body, needed: false },
    ];
    const OPS: &[Op] = &[Op {
        name: "thing_get",
        about: "Get a thing.",
        method: "GET",
        path: "/things/{id}",
        fixed: &[("view", "full")],
        args: ARGS,
    }];
    const S: Service = Service {
        key: "demo",
        base: "https://api.example.com",
        auth: Auth::Bearer,
        headers: &[],
        ops: OPS,
    };

    #[test]
    fn fixed_parameters_are_not_offered_as_arguments() {
        // They are part of the call, not part of the question. Offering
        // `part=snippet` to a model is offering it a way to get the call wrong.
        let t = &S.tools()[0];
        assert!(t.schema["properties"].get("view").is_none(), "{:?}", t.schema);
    }

    #[test]
    fn a_table_becomes_tools_the_model_can_read() {
        let tools = S.tools();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "thing_get");
        assert_eq!(tools[0].server, "demo");
        // Only the required one is required; the optional ones are offered.
        assert_eq!(tools[0].schema["required"], json!(["id"]));
        assert_eq!(tools[0].schema["properties"]["q"]["type"], "string");
    }

    #[tokio::test]
    async fn a_tool_nobody_declared_is_refused_rather_than_guessed_at() {
        let e = S.call("t", "thing_delete", &json!({})).await.unwrap_err();
        assert!(e.contains("no tool called"), "{e}");
    }

    #[tokio::test]
    async fn a_missing_required_argument_is_said_rather_than_sent() {
        // Without this the request goes out with a literal `{id}` in the path
        // and comes back as somebody else's confusing 404.
        let e = S.call("t", "thing_get", &json!({"q": "x"})).await.unwrap_err();
        assert!(e.contains("needs id"), "{e}");
    }

    #[test]
    fn a_path_argument_cannot_become_a_different_path() {
        // The whole reason path arguments are encoded: an id containing a slash
        // would otherwise address a resource nobody asked for.
        assert_eq!(encode("a/b"), "a%2Fb");
        assert_eq!(encode("../admin"), "..%2Fadmin");
        assert_eq!(encode("plain-id_1.2~3"), "plain-id_1.2~3");
    }

    #[test]
    fn a_long_failure_is_clipped_rather_than_pasted_into_a_prompt() {
        let long = "x".repeat(900);
        assert!(clip(&long).len() < 320);
        assert!(clip("short").ends_with("short"));
    }
}
