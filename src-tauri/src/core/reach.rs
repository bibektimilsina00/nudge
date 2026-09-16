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
    /// Hosts somebody has said yes to, this run.
    ///
    /// Held rather than written to the config, and that is the design: an
    /// approval given to get one task finished is not a standing decision about
    /// a domain, and treating it as one is how an allow-list fills up with hosts
    /// nobody remembers agreeing to. This empties when the process does.
    ///
    /// The unit is the host, not the URL. A person who agreed to reach
    /// example.com agreed to example.com; asking again for every path under it
    /// is the approval fatigue that makes people answer "always" to everything.
    hosts_allowed: std::sync::Mutex<std::collections::HashSet<String>>,
    /// Agreed to for this task only, and dropped the moment it ends. Separate
    /// from the session set rather than a timestamp on one set, because
    /// "forget these" is the whole operation and a set you can drop entire is
    /// the cheapest way to be sure it happened.
    hosts_run: std::sync::Mutex<std::collections::HashSet<String>>,
    /// Remembered across restarts, and the only one written to disk.
    hosts_always: std::sync::Mutex<std::collections::HashSet<String>>,
    /// Where the remembered ones live. `None` in tests, so a test run cannot
    /// grant a host to somebody's real installation.
    remembered: Option<std::path::PathBuf>,
}

/// The on-disk shape of the remembered hosts. A named field rather than a bare
/// list because TOML has no top-level array.
#[derive(serde::Serialize, serde::Deserialize, Default)]
struct Remembered {
    hosts: Vec<String>,
}

impl Reach {
    /// The starting point, from the config file. Absent means closed.
    pub fn from_config(granted: &[String]) -> Self {
        let reach = Reach {
            remembered: dirs::home_dir().map(|d| d.join(".config/nudge/allowed.toml")),
            ..Default::default()
        };
        reach.load_remembered();
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

    /// Remember that this host was agreed to, for as long as was agreed.
    pub fn allow_host(&self, host: &str, scope: Scope) {
        let host = host.trim().to_lowercase();
        match scope {
            Scope::Run => {
                self.hosts_run.lock().unwrap().insert(host);
            }
            Scope::Session => {
                self.hosts_allowed.lock().unwrap().insert(host);
            }
            Scope::Always => {
                self.hosts_always.lock().unwrap().insert(host);
                self.save_remembered();
            }
        }
    }

    /// Drop everything that was only agreed to for one task.
    ///
    /// Called when a run ends, however it ends -- finished, failed, or stopped
    /// by Escape. A grant that outlives a task it was stopped out of is the
    /// exact thing this scope exists to prevent.
    pub fn forget_run(&self) {
        self.hosts_run.lock().unwrap().clear();
    }

    fn load_remembered(&self) {
        let Some(path) = &self.remembered else { return };
        let Ok(text) = std::fs::read_to_string(path) else {
            return;
        };
        let Ok(kept) = toml::from_str::<Remembered>(&text) else {
            eprintln!("reach: {} is not readable -- ignoring it", path.display());
            return;
        };
        let mut always = self.hosts_always.lock().unwrap();
        for h in kept.hosts {
            always.insert(h.trim().to_lowercase());
        }
    }

    fn save_remembered(&self) {
        let Some(path) = &self.remembered else { return };
        let mut hosts: Vec<String> = self.hosts_always.lock().unwrap().iter().cloned().collect();
        // Sorted, so the file is a thing a person can read and diff rather than
        // a set printed in whatever order it happened to hash into.
        hosts.sort();
        let Ok(text) = toml::to_string(&Remembered { hosts }) else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Err(e) = std::fs::write(path, text) {
            eprintln!("reach: could not write {}: {e}", path.display());
        }
    }

    /// Has this host been agreed to?
    ///
    /// Exact match, not suffix. `evil-example.com` must not pass because
    /// `example.com` was allowed, and a subdomain rule that got that backwards
    /// would be worse than no rule -- so a subdomain is its own decision until
    /// somebody asks for something cleverer.
    pub fn host_allowed(&self, host: &str) -> bool {
        let host = host.trim().to_lowercase();
        self.hosts_run.lock().unwrap().contains(&host)
            || self.hosts_allowed.lock().unwrap().contains(&host)
            || self.hosts_always.lock().unwrap().contains(&host)
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
    pub fn usable(
        &self,
        tools: &[crate::core::tools::mcp::Tool],
    ) -> Vec<crate::core::tools::mcp::Tool> {
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
        assert!(
            r.server("github"),
            "a configured server should start usable"
        );
        r.set_server("github", false);
        assert!(!r.server("github"));
        assert!(
            r.server("files"),
            "switching one off must not touch another"
        );
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
        let all = [
            tool("files", "read"),
            tool("github", "issue"),
            tool("files", "write"),
        ];

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
        assert!(
            both.contains(" and "),
            "a list of grants should read as a list"
        );
    }
}

/// What a gate answers.
///
/// Three, and the third is the one the product does not have yet. `allow` and
/// `deny` can only be decided in advance, by somebody who does not know what
/// will be asked of them -- so the honest default has to be either uselessly
/// narrow or quietly wide, and this codebase chose narrow and then widened it
/// one grant at a time.
///
/// `Ask` is what lets a default be safe without being useless: the answer is
/// deferred to the moment there is something concrete to judge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    Allow,
    /// Pause and put it to the person. Nothing acts on this yet -- see the plan.
    Ask,
    Deny,
}

/// A gate's answer, and why.
///
/// The reason is not optional. Every refusal in this codebase used to invent its
/// own sentence at the call site, which is how the same denial came to be worded
/// three ways depending on which tool hit it -- and why an interface cannot show
/// a person *why* something did not happen without knowing where it failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    pub answer: Answer,
    /// Said to the person, or to the model. Empty only for `Allow`.
    pub reason: String,
    /// What decided it, for the record §2.1 will keep. `None` when nothing had
    /// to be granted.
    pub rule: Option<String>,
}

impl Decision {
    pub fn allow() -> Self {
        Self {
            answer: Answer::Allow,
            reason: String::new(),
            rule: None,
        }
    }

    /// Allowed, and by which grant -- so the audit can say more than "it ran".
    pub fn allowed_by(grant: Grant) -> Self {
        Self {
            answer: Answer::Allow,
            reason: String::new(),
            rule: Some(grant.key().to_string()),
        }
    }

    pub fn deny(reason: impl Into<String>) -> Self {
        Self {
            answer: Answer::Deny,
            reason: reason.into(),
            rule: None,
        }
    }

    pub fn ask(reason: impl Into<String>) -> Self {
        Self {
            answer: Answer::Ask,
            reason: reason.into(),
            rule: None,
        }
    }

    /// May this go ahead without anybody being asked?
    pub fn allowed(&self) -> bool {
        self.answer == Answer::Allow
    }

    /// Does this want a person before it happens?
    pub fn needs_user(&self) -> bool {
        self.answer == Answer::Ask
    }
}

impl Grant {
    /// What to say when this grant is what was missing.
    ///
    /// One sentence per grant, in one place, so the same refusal reads the same
    /// way whichever tool ran into it -- and so it names where to change it,
    /// because a refusal that does not say how to lift it is a dead end.
    pub fn denied(self) -> String {
        format!(
            "That needs \u{201c}{}\u{201d}, which is off. It can be turned on under \
             \u{201c}Allowed to\u{201d} in settings or the menu bar.",
            self.menu()
        )
    }
}

#[cfg(test)]
mod decision_tests {
    use super::*;

    #[test]
    fn a_refusal_always_carries_a_reason() {
        for g in Grant::ALL {
            let d = Decision::deny(g.denied());
            assert!(!d.reason.is_empty());
            // And it says where to lift it, or somebody reads it as "no" full
            // stop and goes looking for a bug.
            assert!(d.reason.contains("Allowed to"), "{}", d.reason);
        }
    }

    #[test]
    fn the_three_answers_do_not_overlap() {
        assert!(Decision::allow().allowed());
        assert!(!Decision::allow().needs_user());

        assert!(!Decision::deny("no").allowed());
        assert!(!Decision::deny("no").needs_user());

        // The one that matters: an ask is not an allow. Anything treating
        // "not denied" as "go ahead" would run it.
        assert!(!Decision::ask("which folder?").allowed());
        assert!(Decision::ask("which folder?").needs_user());
    }

    #[test]
    fn an_allow_records_what_allowed_it() {
        assert_eq!(
            Decision::allowed_by(Grant::Shell).rule.as_deref(),
            Some("shell")
        );
        assert_eq!(Decision::allow().rule, None);
    }
}

/// Something waiting on a person's answer.
///
/// The approval path existed for exactly one question -- replacing a file -- and
/// was shaped like it: a `(PathBuf, String)` in a mutex, with the answering code
/// knowing it was about files. Every later question that needed a person would
/// have arrived as a second copy of the same machinery, and two approval paths
/// is two places for "yes" to mean something slightly different.
///
/// So the pending thing is a value with a question attached. What carrying it
/// out means is decided where the answer lands, because that is the layer with
/// the app in its hands; what it *is* lives here, with the rest of the
/// permission vocabulary.
/// How long a yes lasts.
///
/// One approval should cover a paginated loop without becoming a standing
/// decision about a domain. Today it cannot: a yes lasts until the process
/// quits, so an agent fetching six pages either asks six times or is told
/// "always" -- and the answer people give to being asked six times is always
/// "always". That is how an allow-list fills up with hosts nobody remembers
/// agreeing to, and the fault is the question, not the person answering it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Until this task ends. The one that was missing, and the one that makes
    /// the other two rare.
    Run,
    /// Until Nudge quits.
    Session,
    /// Written down and remembered across restarts.
    Always,
}

impl Scope {
    /// Read from what somebody said, because the answer arrives as a sentence
    /// whether it was typed, clicked or spoken -- the buttons send these words.
    ///
    /// Defaults to the tightest, like [`crate::core::tools::files::is_yes`]
    /// defaults to no: the cost of reading "yes" as narrower than meant is being
    /// asked again, and the cost of the other mistake is a standing grant
    /// nobody chose.
    pub fn of(answer: &str) -> Scope {
        let a = answer.trim().to_lowercase();
        const ALWAYS: &[&str] = &[
            "always",
            "every time",
            "forever",
            "ask again",
            "from now on",
        ];
        const SESSION: &[&str] = &["session", "rest of the", "until you quit", "while you"];
        if ALWAYS.iter().any(|w| a.contains(w)) {
            return Scope::Always;
        }
        if SESSION.iter().any(|w| a.contains(w)) {
            return Scope::Session;
        }
        Scope::Run
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pending {
    /// Replace a file that already exists, with these exact bytes.
    ///
    /// The bytes are held rather than regenerated. Telling the model to write it
    /// again costs a call and produces a different file -- one run made a
    /// careful dark-themed page, waited for permission, then wrote a plainer
    /// one. The person agreed to the first and would have got the second.
    Replace {
        path: std::path::PathBuf,
        content: String,
    },
    /// Reach a host this run has not reached before.
    Reach { host: String, url: String },
}

impl Pending {
    /// The answers worth offering as one tap, in order of how long they last.
    ///
    /// Shortest first, and the shortest is what a bare "yes" means, so the
    /// tightest answer is both the default and the nearest button. Free text
    /// still works -- these are the words [`Scope::of`] reads, so clicking and
    /// saying it out loud go down the same path.
    ///
    /// A file replacement is not on this list. Its grants live in a different
    /// store keyed by path, and "always replace this file" is a sentence that
    /// should be hard to say.
    pub fn choices(&self) -> Vec<String> {
        match self {
            Pending::Replace { .. } => vec!["Yes".into(), "No".into()],
            Pending::Reach { .. } => vec![
                "Just now".into(),
                "This session".into(),
                "Always".into(),
                "No".into(),
            ],
        }
    }
}

impl Pending {
    /// What to put to the person, in their words rather than the tool's.
    pub fn question(&self) -> String {
        match self {
            Pending::Replace { path, .. } => {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.display().to_string());
                format!("{name} already exists. Shall I replace it?")
            }
            // The host, not the URL. A question is only useful if it can be
            // answered, and nobody can weigh a two-hundred-character URL read
            // aloud -- while "shall I fetch from example.com" is a decision.
            Pending::Reach { host, .. } => format!("Shall I fetch something from {host}?"),
        }
    }

    /// A line for the record, once it has been answered.
    pub fn recorded(&self, agreed: bool) -> String {
        let verb = if agreed { "agreed to" } else { "refused" };
        match self {
            Pending::Replace { path, .. } => format!("{verb} replacing {}", path.display()),
            Pending::Reach { host, .. } => format!("{verb} reaching {host}"),
        }
    }
}

#[cfg(test)]
mod pending_tests {
    use super::*;

    #[test]
    fn a_question_is_asked_about_the_thing_not_the_path() {
        let p = Pending::Replace {
            path: "/Users/someone/Nudge/notes/report.html".into(),
            content: "x".into(),
        };
        // The file name, because the question is read out loud and a full path
        // is unlistenable.
        assert_eq!(
            p.question(),
            "report.html already exists. Shall I replace it?"
        );
    }

    #[test]
    fn reaching_asks_about_the_host() {
        let p = Pending::Reach {
            host: "example.com".into(),
            url: "https://example.com/a/very/long/path?with=params&more=stuff".into(),
        };
        let q = p.question();
        assert!(q.contains("example.com"), "{q}");
        // Nobody can answer a question containing a URL they cannot hold in
        // their head.
        assert!(!q.contains("?with="), "{q}");
    }

    #[test]
    fn both_outcomes_are_recordable() {
        let p = Pending::Reach {
            host: "example.com".into(),
            url: "https://example.com/".into(),
        };
        assert!(p.recorded(true).contains("agreed"));
        assert!(p.recorded(false).contains("refused"));
    }
}

#[cfg(test)]
mod host_tests {
    use super::*;

    /// The whole point of the scope: it is gone when the task is.
    #[test]
    fn a_host_agreed_to_for_one_task_is_not_agreed_to_for_the_next() {
        let r = Reach::default();
        r.allow_host("example.com", Scope::Run);
        assert!(r.host_allowed("example.com"), "the task it was granted for");

        r.forget_run();
        assert!(
            !r.host_allowed("example.com"),
            "a grant that outlives its task is the allow-list filling up by itself"
        );
    }

    /// And the wider ones are not swept up with it.
    #[test]
    fn forgetting_a_task_leaves_the_session_alone() {
        let r = Reach::default();
        r.allow_host("session.example", Scope::Session);
        r.allow_host("run.example", Scope::Run);
        r.forget_run();
        assert!(r.host_allowed("session.example"));
        assert!(!r.host_allowed("run.example"));
    }

    /// A bare yes is the narrowest, because the cost of being too narrow is
    /// being asked again and the cost of the other mistake is a standing grant.
    #[test]
    fn a_plain_yes_lasts_only_for_the_task() {
        assert_eq!(Scope::of("yes"), Scope::Run);
        assert_eq!(Scope::of("go ahead"), Scope::Run);
        assert_eq!(Scope::of("Just now"), Scope::Run);
        assert_eq!(Scope::of("This session"), Scope::Session);
        assert_eq!(
            Scope::of("yes, for the rest of the session"),
            Scope::Session
        );
        assert_eq!(Scope::of("Always"), Scope::Always);
        assert_eq!(Scope::of("always, don't ask again"), Scope::Always);
    }

    /// Nothing is written unless somebody said the word that writes it.
    #[test]
    fn only_always_reaches_the_disk() {
        let path = std::env::temp_dir().join(format!("nudge-allowed-{}.toml", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let r = Reach {
            remembered: Some(path.clone()),
            ..Default::default()
        };

        r.allow_host("run.example", Scope::Run);
        r.allow_host("session.example", Scope::Session);
        assert!(!path.exists(), "a narrow yes must not be written down");

        r.allow_host("always.example", Scope::Always);
        let on_disk = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(on_disk.contains("always.example"), "{on_disk}");
        assert!(!on_disk.contains("run.example"), "{on_disk}");
        assert!(!on_disk.contains("session.example"), "{on_disk}");

        // And it comes back.
        let next = Reach {
            remembered: Some(path.clone()),
            ..Default::default()
        };
        next.load_remembered();
        assert!(next.host_allowed("always.example"));
        // Even across a task boundary, which is what "always" means.
        next.forget_run();
        assert!(next.host_allowed("always.example"));

        let _ = std::fs::remove_file(&path);
    }

    /// A gate that knows what it is asking offers the answers; the model's own
    /// questions are open ones and get a field.
    #[test]
    fn reaching_offers_how_long_and_replacing_does_not() {
        let reach = Pending::Reach {
            host: "example.com".into(),
            url: "https://example.com/a".into(),
        };
        let choices = reach.choices();
        assert_eq!(choices.first().map(String::as_str), Some("Just now"));
        // Every offered answer has to read back as the scope it names, or the
        // button says one thing and grants another.
        assert_eq!(Scope::of(&choices[0]), Scope::Run);
        assert_eq!(Scope::of(&choices[1]), Scope::Session);
        assert_eq!(Scope::of(&choices[2]), Scope::Always);
        assert!(!crate::core::tools::files::is_yes(&choices[3]));

        let replace = Pending::Replace {
            path: "/tmp/x".into(),
            content: String::new(),
        };
        assert_eq!(replace.choices(), vec!["Yes", "No"]);
    }

    #[test]
    fn a_host_agreed_to_is_remembered_and_others_are_not() {
        let r = Reach::default();
        assert!(!r.host_allowed("example.com"));

        r.allow_host("Example.COM", Scope::Session);
        // Case and spacing are how a host arrives, not what it is.
        assert!(r.host_allowed("example.com"));
        assert!(r.host_allowed(" EXAMPLE.com "));
    }

    /// The rule worth protecting. A suffix match here would let
    /// `evil-example.com` through on the strength of `example.com`, and a
    /// subdomain rule that got it backwards would be worse than no rule at all.
    #[test]
    fn a_lookalike_host_is_not_the_host() {
        let r = Reach::default();
        r.allow_host("example.com", Scope::Session);

        for other in [
            "evil-example.com",
            "example.com.evil.test",
            "notexample.com",
            "sub.example.com",
        ] {
            assert!(!r.host_allowed(other), "{other} passed as example.com");
        }
    }
}
