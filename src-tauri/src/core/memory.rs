//! What this Mac's applications turned out to be like.
//!
//! Every application lies about itself in its own particular way. CapCut's
//! timeline view means a project is open. A Chromium menu closes if you click
//! into it instead of pressing the shortcut. Some window keeps a Continue button
//! on first launch and never again. None of that is in a prompt, none of it is
//! guessable, and every run rediscovers it by getting it wrong once.
//!
//! So a note gets kept, and is put back in front of the model **only when that
//! application is in front**. The scoping is the whole design: a general pile of
//! advice would be paid for on every turn and be about the wrong program almost
//! always, where thirty words about CapCut cost nothing on the other three
//! hundred and sixty-four days.
//!
//! ## Written from failure, not from success
//!
//! A note is worth keeping when something did not work and a way round it was
//! found. "It worked" teaches nothing -- the next run would have done that
//! anyway. This is also why the plan put it here rather than earlier: **memory
//! that records a broken loop's habits is worse than none**, because it turns one
//! bad turn into a permanent one.
//!
//! ## Three things worth keeping, not one
//!
//! An application is one scope and it was the only one. There was nowhere to put
//! a fact about the *project* -- this repository uses `pnpm`, the tests live
//! here -- and nowhere at all for a fact about the *person*, so the same
//! correction got made every week and nothing could tell a habit from an
//! accident.
//!
//! They are kept in one file, under keys that say which is which, and offered on
//! different turns: an application's notes when it is in front, a project's when
//! the work is happening inside it, and a person's always -- which is why that
//! one is capped hardest.
//!
//! ## Visible, and forgettable
//!
//! It changes what Nudge does, so the same rule as everything else that does
//! that: it is in the menu bar, with a count, and one click forgets an
//! application entirely. Something that silently learns is something you cannot
//! reason about when it starts behaving oddly.
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

/// How many notes one application may accumulate.
///
/// A budget, not a limit anyone should reach. Eight short lines is a paragraph of
/// prompt on the turns it applies to; beyond that it stops being a hint and
/// becomes a second set of instructions competing with the real ones. The oldest
/// goes when a ninth arrives, on the grounds that anything still true will be
/// rediscovered and anything not is better gone.
const PER_APP: usize = 8;

/// How many notes about the person.
///
/// Fewer, and for a different reason: these are read on *every* turn rather than
/// on the turns one application is in front, so a note here is paid for
/// constantly. Four is enough for how somebody likes to be answered; past that
/// it stops being a preference and becomes a second set of instructions.
const PER_PERSON: usize = 4;

/// Longest a single note may be.
const LONGEST: usize = 200;

/// What a note is about.
///
/// The key in the file carries the scope, so one file holds all three and an
/// older file -- which had bare application names -- still reads correctly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// The application in front. `Safari`, `DaVinci Resolve`.
    App,
    /// The folder being worked in.
    Project,
    /// The person. Read on every turn.
    Person,
}

impl Scope {
    pub fn of(said: Option<&str>) -> Scope {
        match said.map(str::trim).map(str::to_ascii_lowercase).as_deref() {
            Some("project") => Scope::Project,
            Some("me") | Some("person") | Some("user") => Scope::Person,
            _ => Scope::App,
        }
    }

    /// How it is written in the file.
    ///
    /// An application keeps its bare name, so a `memory.toml` written before
    /// scopes existed still reads as what it was.
    fn key(self, about: &str) -> String {
        match self {
            Scope::App => about.to_string(),
            Scope::Project => format!("project:{about}"),
            Scope::Person => "me".to_string(),
        }
    }

    fn cap(self) -> usize {
        match self {
            Scope::Person => PER_PERSON,
            _ => PER_APP,
        }
    }
}

/// Notes, keyed by what they are about. A bare name is an application; anything
/// else says which scope it belongs to.
/// A key as somebody would say it.
fn readable(key: &str) -> String {
    match key {
        "me" => "you".into(),
        _ => match key.strip_prefix("project:") {
            Some(path) => std::path::Path::new(path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(path)
                .to_string(),
            None => key.to_string(),
        },
    }
}

#[derive(Default)]
pub struct Memory {
    path: Option<PathBuf>,
    notes: Mutex<BTreeMap<String, Vec<String>>>,
}

impl Memory {
    /// Beside the config, which is where somebody would look for it.
    pub fn path() -> Option<PathBuf> {
        dirs::home_dir().map(|d| d.join(".config/nudge/memory.toml"))
    }

    /// Read once, at startup.
    ///
    /// Which is right for one copy of the app and wrong for two: a note written
    /// by one instance is invisible to another until it restarts. Left alone
    /// because a second instance is a thing that happens while developing and
    /// never afterwards -- and because the fix is a file watch, which is a
    /// moving part to maintain for a case nobody is in.
    pub fn load() -> Memory {
        let path = Memory::path();
        let notes = path
            .as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|text| toml::from_str::<BTreeMap<String, Vec<String>>>(&text).ok())
            .unwrap_or_default();
        if !notes.is_empty() {
            // "applications" was true when that was the only scope there was.
            eprintln!(
                "memory: {} notes about {} things",
                notes.values().map(Vec::len).sum::<usize>(),
                notes.len()
            );
        }
        Memory {
            path,
            notes: Mutex::new(notes),
        }
    }

    /// What is known about the application in front, if anything.
    pub fn about(&self, app: &str) -> Vec<String> {
        self.notes
            .lock()
            .unwrap()
            .get(app)
            .cloned()
            .unwrap_or_default()
    }

    /// Keep a note. Returns what to tell the model about what just happened.
    pub fn learn(&self, scope: Scope, about: &str, note: &str) -> String {
        let about = about.trim();
        let note = note.trim();
        if note.is_empty() || (about.is_empty() && scope != Scope::Person) {
            return "There was nothing to remember.".into();
        }
        let note: String = note.chars().take(LONGEST).collect();
        let key = scope.key(about);
        let named = match scope {
            Scope::Person => "you".to_string(),
            _ => about.to_string(),
        };

        let mut notes = self.notes.lock().unwrap();
        let for_app = notes.entry(key.clone()).or_default();

        // Said twice is said once. Models restate things, and a note repeated in
        // slightly different words is the same note taking two of eight slots.
        if for_app
            .iter()
            .any(|n| n.eq_ignore_ascii_case(&note) || n.contains(note.as_str()))
        {
            return format!("Already noted about {named}.");
        }
        for_app.push(note.clone());
        while for_app.len() > scope.cap() {
            for_app.remove(0);
        }
        let count = for_app.len();
        drop(notes);

        self.save();
        eprintln!("memory: about {key} -- {note}");
        format!("Noted about {named} ({count} kept).")
    }

    /// Forget one application entirely.
    pub fn forget(&self, app: &str) {
        self.notes.lock().unwrap().remove(app);
        self.save();
        eprintln!("memory: forgot everything about {app}");
    }

    /// Everything there is anything about, and how much.
    ///
    /// The key as stored -- which is the name for an application -- and a
    /// readable name beside it, because "project:/Users/sam/projects/nudge" is a
    /// key and not something to put in a menu.
    pub fn everything(&self) -> Vec<(String, String, usize)> {
        self.notes
            .lock()
            .unwrap()
            .iter()
            .map(|(key, notes)| (key.clone(), readable(key), notes.len()))
            .collect()
    }

    /// What goes in the prompt: the application in front, the project being
    /// worked in, and the person -- whichever of them has anything.
    pub fn prompt(&self, app: Option<&str>, project: Option<&str>) -> String {
        let mut out = String::new();
        let mut block = |title: String, notes: Vec<String>| {
            if notes.is_empty() {
                return;
            }
            out.push_str(&format!(
                "## {title}\n\n\
                 Learned here, by getting it wrong once. Treat it as true unless \
                 the screen says otherwise -- the screen is what is happening now \
                 and this is only what happened before.\n{}\n\n",
                notes
                    .iter()
                    .map(|n| format!("- {n}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        };

        if let Some(app) = app {
            block(format!("What {app} turned out to be like"), self.about(app));
        }
        if let Some(project) = project {
            let name = std::path::Path::new(project)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(project);
            block(
                format!("What working in {name} is like"),
                self.about(&Scope::Project.key(project)),
            );
        }
        block("How they like to be helped".into(), self.about("me"));
        out
    }

    fn save(&self) {
        let Some(path) = &self.path else {
            return;
        };
        let notes = self.notes.lock().unwrap();
        let Ok(text) = toml::to_string(&*notes) else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        // Best effort. A note that could not be written is a note not kept, which
        // is the state this started in and is survivable.
        if let Err(e) = std::fs::write(path, text) {
            eprintln!("memory: could not write {}: {e}", path.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Nothing here touches the file, so nothing here is about somebody's real
    /// memory file.
    fn blank() -> Memory {
        Memory::default()
    }

    #[test]
    fn a_note_comes_back_for_its_own_application_and_no_other() {
        let m = blank();
        m.learn(
            Scope::App,
            "CapCut",
            "The timeline view means a project is open.",
        );
        assert_eq!(m.about("CapCut").len(), 1);
        assert!(m.about("Safari").is_empty());
        // And the prompt is silent about applications it knows nothing about.
        assert_eq!(m.prompt(Some("Safari"), None), "");
        assert!(m.prompt(Some("CapCut"), None).contains("timeline view"));
        assert_eq!(m.prompt(None, None), "");
    }

    /// Eight slots are worth having only if the same thing cannot take three.
    #[test]
    fn the_same_thing_said_twice_is_kept_once() {
        let m = blank();
        m.learn(Scope::App, "Chrome", "Menus close if you click into them.");
        m.learn(Scope::App, "Chrome", "menus close if you click into them.");
        m.learn(
            Scope::App,
            "Chrome",
            "Menus close if you click into them. Use the shortcut.",
        );
        assert_eq!(m.about("Chrome").len(), 2, "{:?}", m.about("Chrome"));
    }

    /// The three scopes are three different files' worth of fact in one file,
    /// and each is offered on its own turns.
    #[test]
    fn a_project_note_and_a_person_note_go_to_different_places() {
        let m = blank();
        let here = "/Users/sam/projects/nudge";
        m.learn(
            Scope::App,
            "Safari",
            "The reader button hides in the URL bar.",
        );
        m.learn(Scope::Project, here, "This one uses pnpm, never npm.");
        m.learn(
            Scope::Person,
            "",
            "They asked to be told before anything is deleted.",
        );

        // The application's notes only when it is in front.
        let elsewhere = m.prompt(Some("Mail"), Some(here));
        assert!(!elsewhere.contains("reader button"), "{elsewhere}");
        // The project's whenever the work is here.
        assert!(elsewhere.contains("uses pnpm"), "{elsewhere}");
        // The person's on every turn, whatever else is true.
        assert!(
            elsewhere.contains("before anything is deleted"),
            "{elsewhere}"
        );
        assert!(m.prompt(None, None).contains("before anything is deleted"));
        // Named by the folder rather than by the whole path.
        assert!(elsewhere.contains("working in nudge"), "{elsewhere}");
    }

    /// Read on every turn, so the budget is smaller and the oldest goes first.
    #[test]
    fn what_is_known_about_the_person_is_kept_shortest() {
        let m = blank();
        for i in 0..10 {
            m.learn(Scope::Person, "", &format!("preference number {i}"));
        }
        assert_eq!(m.about("me").len(), PER_PERSON);
        assert!(m.about("me").iter().any(|n| n.contains("number 9")));
        assert!(!m.about("me").iter().any(|n| n.contains("number 0")));
    }

    /// A file written before scopes existed holds bare application names, and
    /// has to keep reading as what it was.
    #[test]
    fn an_application_is_still_stored_under_its_own_name() {
        assert_eq!(Scope::App.key("Safari"), "Safari");
        assert_eq!(Scope::Person.key("anything"), "me");
        assert_eq!(Scope::of(None), Scope::App);
        assert_eq!(Scope::of(Some("me")), Scope::Person);
        assert_eq!(Scope::of(Some("PROJECT")), Scope::Project);
    }

    #[test]
    fn it_does_not_grow_without_limit() {
        let m = blank();
        for i in 0..20 {
            m.learn(Scope::App, "Xcode", &format!("note number {i}"));
        }
        assert_eq!(m.about("Xcode").len(), PER_APP);
        // The oldest went; the newest stayed.
        assert!(m.about("Xcode").iter().any(|n| n == "note number 19"));
        assert!(!m.about("Xcode").iter().any(|n| n == "note number 0"));
    }

    #[test]
    fn forgetting_is_complete() {
        let m = blank();
        m.learn(Scope::App, "Mail", "a thing");
        m.forget("Mail");
        assert!(m.about("Mail").is_empty());
        assert!(m.everything().is_empty());
    }

    #[test]
    fn nothing_is_learned_from_nothing() {
        let m = blank();
        m.learn(Scope::App, "", "a thing");
        m.learn(Scope::App, "Mail", "   ");
        assert!(m.everything().is_empty());
    }
}
