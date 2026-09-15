//! How far Nudge is allowed to reach, and who decided.
//!
//! Three limits sit between the model and the machine: the shell runs only
//! commands from a list, files are written only inside the workspace, and the web
//! is read with GET and nothing else. **None of them is arbitrary.** The shell is
//! read-only because a model reached for `rm` to get around a refusal. Files are
//! workspace-bound because the first thing that ever wrote a file put it in this
//! repository's root. They are scars, and scars are not removed.
//!
//! What they are not is *somebody's decision*. A person who wants an assistant
//! that can actually change a file has no way to say so except by editing a
//! constant in the source, and a limit that can only be lifted by recompiling is
//! not a safety property -- it is a missing feature with a good excuse.
//!
//! So each one becomes a decision, under three rules taken from the plan:
//!
//! - **Widening is explicit.** A line in the config or a click in the menu bar.
//!   Never a longer list in this file.
//! - **The default does not move.** Everything here starts closed, and somebody
//!   who never opens the menu has exactly the assistant they had yesterday --
//!   which is the right default for something that listens all day.
//! - **What was granted is visible.** "It has full shell access" has to be
//!   something you can *see*, not something you have to remember agreeing to. It
//!   is in the menu bar with a tick beside it, and it is in the prompt, so it
//!   appears in the transcript of every turn that had it.
//!
//! Taking it back is the same click. Nothing here is permanent and nothing here
//! is written back to the config, so the widest Nudge has ever been is still one
//! restart away from the narrowest.
use std::sync::atomic::{AtomicBool, Ordering};

/// One thing a person can decide to allow.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Grant {
    /// Run any command, not only the readable ones.
    Shell,
    /// Write outside the workspace.
    Files,
    /// Make requests that could change something on the other end.
    ///
    /// Not "make requests with headers" -- an authenticated GET against somebody's
    /// API is still reading, and gating it would mean granting this to do the
    /// ordinary thing. What is gated is the methods that act.
    Http,
}

// MCP servers are not in `Grant`, because they are not a fixed list -- they come
// from whatever is in the config file. They are governed below, by name, under
// the same three rules. Configuring a server is already an explicit,
// deliberate grant; what is missing is being able to *see* what it can do and
// switch it off, and that wants the menu to be built after the servers connect
// rather than before. Recorded in the plan rather than half-done here.

impl Grant {
    pub const ALL: [Grant; 3] = [Grant::Shell, Grant::Files, Grant::Http];

    /// The id the menu bar and the config use.
    pub fn key(self) -> &'static str {
        match self {
            Grant::Shell => "shell",
            Grant::Files => "files",
            Grant::Http => "http",
        }
    }

    /// What a person reads in the menu. Written as the thing being allowed, not
    /// as the setting being changed: "Run any command" is a decision, "shell:
    /// true" is a checkbox whose meaning you have to already know.
    pub fn menu(self) -> &'static str {
        match self {
            Grant::Shell => "Run any command",
            Grant::Files => "Write files anywhere",
            Grant::Http => "Send requests, not just read",
        }
    }

    /// What the model is told, when it has it.
    pub fn told(self) -> &'static str {
        match self {
            Grant::Shell => "run any command, including ones that change things",
            Grant::Files => "write files outside the workspace, using absolute paths",
            Grant::Http => "send requests with any method, not only ones that read",
        }
    }
}

/// What has been allowed right now.
///
/// Atomics rather than a lock: this is read on every command, every write and
/// every fetch, and written only when somebody clicks a menu item.
#[derive(Debug, Default)]
pub struct Reach {
    shell: AtomicBool,
    files: AtomicBool,
    http: AtomicBool,
    /// Tool servers that have been switched off, by name.
    ///
    /// The opposite default to everything else here, and deliberately: putting a
    /// server in the config *is* the explicit grant, so one that is configured is
    /// on. What was missing was the other two rules -- being able to see what it
    /// can do, and being able to take it back without editing a file and
    /// restarting. Holding the ones that are off rather than the ones that are on
    /// means a server added to the config is usable without being listed twice.
    servers_off: std::sync::Mutex<std::collections::HashSet<String>>,
}

impl Reach {
    /// The starting point, from the config file. Absent means closed.
    pub fn from_config(granted: &[String]) -> Self {
        let reach = Reach::default();
        for name in granted {
            match Grant::ALL.iter().find(|g| g.key() == name) {
                Some(g) => reach.set(*g, true),
                // Named rather than ignored: a typo in the config that silently
                // grants nothing looks exactly like a grant that is not working.
                None => eprintln!(
                    "reach: no such grant {name:?} -- it is one of {}",
                    Grant::ALL.map(Grant::key).join(", ")
                ),
            }
        }
        reach
    }

    fn cell(&self, grant: Grant) -> &AtomicBool {
        match grant {
            Grant::Shell => &self.shell,
            Grant::Files => &self.files,
            Grant::Http => &self.http,
        }
    }

    pub fn has(&self, grant: Grant) -> bool {
        self.cell(grant).load(Ordering::Relaxed)
    }

    pub fn set(&self, grant: Grant, on: bool) {
        let was = self.cell(grant).swap(on, Ordering::Relaxed);
        // Said out loud, both ways. A widening nobody announced is the thing this
        // module exists to prevent, and a narrowing nobody announced means the
        // next refusal looks like a bug.
        if was != on {
            eprintln!(
                "reach: {} -- may now {}",
                match on {
                    true => "GRANTED",
                    false => "taken back",
                },
                grant.told()
            );
        }
    }

    /// Whether a tool server may be used right now.
    pub fn server(&self, name: &str) -> bool {
        !self.servers_off.lock().unwrap().contains(name)
    }

    pub fn set_server(&self, name: &str, on: bool) {
        let mut off = self.servers_off.lock().unwrap();
        let changed = match on {
            true => off.remove(name),
            false => off.insert(name.to_string()),
        };
        if changed {
            eprintln!(
                "reach: tool server {name:?} {}",
                match on {
                    true => "back on",
                    false => "switched off",
                }
            );
        }
    }

    /// The tools the model should be told about: everything from servers that
    /// have not been switched off.
    ///
    /// A function rather than a filter written at the call site, so that what is
    /// tested is what runs.
    pub fn usable(&self, tools: &[crate::core::tools::mcp::Tool]) -> Vec<crate::core::tools::mcp::Tool> {
        tools
            .iter()
            .filter(|t| self.server(&t.server))
            .cloned()
            .collect()
    }

    /// Everything currently allowed, for the prompt and for anything that wants
    /// to show it.
    pub fn granted(&self) -> Vec<Grant> {
        Grant::ALL.into_iter().filter(|g| self.has(*g)).collect()
    }

    /// What the model is told about its own reach.
    ///
    /// Empty when nothing is granted, so the ordinary case -- which is every case
    /// until somebody decides otherwise -- spends no tokens saying that nothing
    /// unusual is true.
    pub fn prompt(&self) -> String {
        let granted = self.granted();
        if granted.is_empty() {
            return String::new();
        }
        format!(
            "## What you have been allowed\n\n\
             Someone has widened what you may do, on purpose, and can take it \
             back at any moment. You may {}.\n\n\
             This is not an invitation to use it. It is permission for the step \
             that needs it, and the narrow way is still the better way when it \
             works -- a command that only reads cannot go wrong twice.\n\n",
            match granted.len() {
                1 => granted[0].told().to_string(),
                _ => {
                    let said: Vec<&str> = granted.iter().map(|g| g.told()).collect();
                    format!(
                        "{} and {}",
                        said[..said.len() - 1].join(", "),
                        said[said.len() - 1]
                    )
                }
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn everything_starts_closed() {
        let r = Reach::default();
        assert!(Grant::ALL.iter().all(|g| !r.has(*g)));
        assert!(r.granted().is_empty());
        // And the prompt says nothing at all, rather than saying "you may not".
        assert_eq!(r.prompt(), "");
    }

    #[test]
    fn a_grant_can_be_given_and_taken_back() {
        let r = Reach::default();
        r.set(Grant::Shell, true);
        assert!(r.has(Grant::Shell));
        assert!(!r.has(Grant::Files), "one grant must not imply another");
        r.set(Grant::Shell, false);
        assert!(!r.has(Grant::Shell));
    }

    #[test]
    fn the_config_names_what_it_grants() {
        let r = Reach::from_config(&["shell".into(), "http".into()]);
        assert!(r.has(Grant::Shell) && r.has(Grant::Http) && !r.has(Grant::Files));
    }

    /// A typo must not read as a grant, and must not read as nothing either.
    #[test]
    fn an_unknown_grant_is_ignored_rather_than_guessed() {
        let r = Reach::from_config(&["shel".into()]);
        assert!(r.granted().is_empty());
    }

    /// The opposite default to the grants, and the reason is in the doc comment:
    /// configuring a server is already the explicit decision.
    #[test]
    fn a_configured_server_is_on_until_it_is_switched_off() {
        let r = Reach::default();
        assert!(r.server("github"), "a configured server should start usable");
        r.set_server("github", false);
        assert!(!r.server("github"));
        assert!(r.server("files"), "switching one off must not touch another");
        r.set_server("github", true);
        assert!(r.server("github"));
    }

    /// Switched off has to mean gone from the prompt, not merely refused later.
    /// A tool the model is still told it has is one it will keep reaching for.
    #[test]
    fn switching_a_server_off_hides_its_tools() {
        let tool = |server: &str, name: &str| crate::core::tools::mcp::Tool {
            server: server.into(),
            name: name.into(),
            about: String::new(),
            schema: serde_json::json!({}),
        };
        let all = [tool("files", "read"), tool("github", "issue"), tool("files", "write")];

        let r = Reach::default();
        assert_eq!(r.usable(&all).len(), 3);
        r.set_server("files", false);
        let left = r.usable(&all);
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].server, "github");
        r.set_server("files", true);
        assert_eq!(r.usable(&all).len(), 3);
    }

    #[test]
    fn the_model_is_told_what_it_may_do() {
        let r = Reach::default();
        r.set(Grant::Shell, true);
        assert!(r.prompt().contains("run any command"));
        r.set(Grant::Files, true);
        let both = r.prompt();
        assert!(both.contains("run any command") && both.contains("outside the workspace"));
        assert!(both.contains(" and "), "a list of grants should read as a list");
    }
}
