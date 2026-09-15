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
}

// There is no `Http` grant yet, deliberately. The GET-only fetch is the third
// boundary named in the plan, but the thing it would permit -- a request with a
// method, headers and a body -- does not exist until 2.3. A menu item that can
// be ticked and changes nothing is worse than a missing one: it teaches people
// that the ticks do not mean anything.
//
// Nor is there one for MCP tools. Configuring a server is already an explicit,
// deliberate grant; what is missing is being able to *see* what it can do and
// switch it off, and that wants the menu to be built after the servers connect
// rather than before. Recorded in the plan rather than half-done here.

impl Grant {
    pub const ALL: [Grant; 2] = [Grant::Shell, Grant::Files];

    /// The id the menu bar and the config use.
    pub fn key(self) -> &'static str {
        match self {
            Grant::Shell => "shell",
            Grant::Files => "files",
        }
    }

    /// What a person reads in the menu. Written as the thing being allowed, not
    /// as the setting being changed: "Run any command" is a decision, "shell:
    /// true" is a checkbox whose meaning you have to already know.
    pub fn menu(self) -> &'static str {
        match self {
            Grant::Shell => "Run any command",
            Grant::Files => "Write files anywhere",
        }
    }

    /// What the model is told, when it has it.
    pub fn told(self) -> &'static str {
        match self {
            Grant::Shell => "run any command, including ones that change things",
            Grant::Files => "write files outside the workspace, using absolute paths",
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
        let r = Reach::from_config(&["shell".into()]);
        assert!(r.has(Grant::Shell) && !r.has(Grant::Files));
    }

    /// A typo must not read as a grant, and must not read as nothing either.
    #[test]
    fn an_unknown_grant_is_ignored_rather_than_guessed() {
        let r = Reach::from_config(&["shel".into()]);
        assert!(r.granted().is_empty());
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
