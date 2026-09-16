//! What a step can actually do, as a property rather than a list of names.
//!
//! The gate asks which [`Grant`](crate::core::reach::Grant) a step needs, and
//! that works precisely because every step is one this repository wrote. It
//! stops working at the first tool Nudge has never heard of: a server arrives
//! with thirty tools and the gate has nothing to decide on but the server's
//! name, so a `write_file` and a `read_file` from the same place are treated
//! alike. That is not hypothetical -- an MCP `files/write_file` replaced a file
//! with no gate, no diff and no record, because the only question asked was
//! whether that server was switched on.
//!
//! So risk becomes a declared property that one function reads. The same idea
//! OpenWorker arrived at after carrying hardcoded `WRITE_TOOLS` / `SHELL_TOOL`
//! name sets inline for long enough to regret it.
//!
//! ## The floor for somebody else's tool
//!
//! **A third-party tool is [`Risk::External`], always, whatever it is called.**
//! Its effects are a stranger's claim: there is no way to tell its reads from a
//! write wearing a read's name, and `list_items` may well post something. The
//! class is welded on rather than configurable, because a config value that
//! could drop one into the never-checked tier would switch off the gate, the
//! record and every future judge in a single line.
//!
//! What *is* negotiable is whether a person is interrupted -- see
//! [`Reach::server`](crate::core::reach::Reach::server), which is how a server
//! gets switched off. Waiving the card is a different decision from pretending
//! there is nothing to card.
use crate::core::provider::Step;
use crate::core::reach::Grant;
use serde::Serialize;

/// The intrinsic side-effect category of a step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Risk {
    /// No side effects. Looking at the screen, reading a file, listing a folder.
    Read,
    /// Reaches the network. The *request* carries data off this machine whether
    /// or not the response is interesting, which is why a GET is on this list
    /// and not on [`Risk::Read`].
    Egress,
    /// Changes something on this machine.
    WriteLocal,
    /// Runs a command, which is every other risk at once and unbounded.
    Exec,
    /// Side effects somewhere else, through something we did not write.
    External,
}

impl Risk {
    /// Anything but a pure read is worth the gate's attention.
    pub fn consequential(self) -> bool {
        self != Risk::Read
    }

    /// The grant that governs it, where one does.
    ///
    /// [`Risk::External`] has none, and that is the honest answer rather than a
    /// gap: the three grants name things this repository implements, and a
    /// stranger's tool is not one of them. It is governed per server instead.
    pub fn grant(self) -> Option<Grant> {
        match self {
            Risk::Read => None,
            Risk::Egress => Some(Grant::Http),
            Risk::WriteLocal => Some(Grant::Files),
            Risk::Exec => Some(Grant::Shell),
            Risk::External => None,
        }
    }

    /// For the record, so a log line says what kind of thing happened.
    pub fn name(self) -> &'static str {
        match self {
            Risk::Read => "read",
            Risk::Egress => "egress",
            Risk::WriteLocal => "write",
            Risk::Exec => "exec",
            Risk::External => "external",
        }
    }
}

/// What this step can do.
///
/// Exhaustive on purpose -- no `_ =>` arm. A new step should not be able to
/// arrive classified as a read because nobody remembered this file; the compiler
/// asks instead.
pub fn of(step: &Step) -> Risk {
    match step {
        // Nothing leaves the machine and nothing on it changes.
        Step::Point { .. }
        | Step::Done { .. }
        | Step::Unsure { .. }
        | Step::Reply { .. }
        | Step::Question { .. }
        | Step::Plan { .. }
        | Step::Show { .. }
        | Step::Read { .. }
        | Step::Output { .. }
        | Step::Search { .. }
        | Step::Skill { .. }
        | Step::Workspace { .. } => Risk::Read,

        // Remembering writes a file, but only ever Nudge's own note about an
        // application, in Nudge's own folder. Treated as a read because the gate
        // has nothing useful to decide: refusing it protects nothing.
        Step::Remember { .. } => Risk::Read,

        // Typing and pressing keys act on whatever is in front of them, which
        // may be a terminal. The screen is not a sandbox.
        Step::Type { .. } | Step::Press { .. } => Risk::Exec,

        // Starting something is running something. Stopping one is how you undo
        // that, so it is not held to the same bar.
        Step::Run { .. } | Step::Start { .. } | Step::Launch { .. } => Risk::Exec,
        Step::Kill { .. } => Risk::Read,

        Step::Write { .. } | Step::Edit { .. } => Risk::WriteLocal,

        // Opening a URL hands it to a browser, which fetches it. The machine
        // this runs on is not the one that reads the page, but the request still
        // happens and still carries whatever is in the URL.
        Step::Fetch { .. } | Step::Open { .. } => Risk::Egress,

        // A request with a method and a body, which is most of what "talking to
        // an API" means. Egress whatever the method: a GET is the exfiltration
        // channel, and the gate distinguishes the acting methods separately.
        Step::Request { .. } => Risk::Egress,

        // A task handed to another agent is every risk its steps are, and those
        // are gated as they happen. Classified as a read here so the same step
        // is not gated twice for the same act.
        Step::Agent { .. } | Step::Task { .. } => Risk::Read,

        // Handing work to a coding agent in a terminal. It is a process this
        // machine starts and it does whatever it likes afterwards, so it is
        // exec -- and the honest classification is the one that says so rather
        // than the one that notices Nudge is not typing the commands itself.
        Step::Delegate { .. } => Risk::Exec,

        // Somebody else's tool. See the module header: the name is not evidence.
        Step::Mcp { .. } => Risk::External,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mcp(tool: &str) -> Step {
        Step::Mcp {
            tool: tool.into(),
            args: serde_json::Value::Null,
            say: String::new(),
        }
    }

    /// The rule the module exists for: a stranger's tool is never a read,
    /// however it is spelled.
    #[test]
    fn somebody_elses_tool_is_never_taken_at_its_word() {
        for name in [
            "files/read_file",
            "files/list_directory",
            "notes/search",
            "anything/get_status",
            // The one that matters: a write wearing a read's name.
            "files/write_file",
        ] {
            assert_eq!(
                of(&mcp(name)),
                Risk::External,
                "{name} was taken at its word"
            );
            assert!(of(&mcp(name)).consequential());
        }
    }

    #[test]
    fn a_read_is_a_read() {
        assert_eq!(
            of(&Step::Read {
                path: "a.txt".into(),
                from: 0,
                lines: 50,
                say: String::new(),
            }),
            Risk::Read
        );
        assert!(!of(&Step::Point {
            at: crate::core::screen::capture::Point { x: 0.0, y: 0.0 },
            say: String::new(),
            act: crate::core::provider::Act::Click,
            control: None,
        })
        .consequential());
    }

    /// Typing is not a read. Whatever is in front of it may be a terminal.
    #[test]
    fn typing_into_whatever_is_there_counts_as_running_something() {
        assert_eq!(
            of(&Step::Type {
                text: "rm -rf /".into(),
                submit: true,
                say: String::new(),
            }),
            Risk::Exec
        );
    }

    /// Found by the compiler, not by me: the exhaustive match is the point.
    #[test]
    fn handing_work_to_another_agent_is_running_something() {
        assert_eq!(
            of(&Step::Delegate {
                task: "fix the build".into(),
                named: None,
                say: String::new(),
            }),
            Risk::Exec
        );
    }

    /// A GET is the exfiltration channel; the response being uninteresting does
    /// not make the request harmless.
    #[test]
    fn reaching_the_network_is_never_a_read() {
        assert_eq!(
            of(&Step::Fetch {
                url: "https://example.com/?k=secret".into(),
                say: String::new(),
            }),
            Risk::Egress
        );
        assert_eq!(
            of(&Step::Open {
                url: "https://example.com".into(),
                say: String::new(),
            }),
            Risk::Egress
        );
        assert_eq!(
            of(&Step::Request {
                method: "GET".into(),
                url: "https://example.com".into(),
                headers: Vec::new(),
                body: None,
                say: String::new(),
            }),
            Risk::Egress
        );
    }

    /// Every consequential class names the grant that governs it, except the one
    /// this repository does not implement.
    #[test]
    fn each_risk_points_at_whatever_governs_it() {
        assert_eq!(Risk::Exec.grant(), Some(Grant::Shell));
        assert_eq!(Risk::WriteLocal.grant(), Some(Grant::Files));
        assert_eq!(Risk::Egress.grant(), Some(Grant::Http));
        assert_eq!(Risk::Read.grant(), None);
        // Not a gap: a stranger's tool is governed per server, not by a grant
        // naming something we wrote.
        assert_eq!(Risk::External.grant(), None);
    }
}
