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

/// Longest a single note may be.
const LONGEST: usize = 200;

/// Notes about applications, keyed by the name the system reports.
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

    pub fn load() -> Memory {
        let path = Memory::path();
        let notes = path
            .as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|text| toml::from_str::<BTreeMap<String, Vec<String>>>(&text).ok())
            .unwrap_or_default();
        if !notes.is_empty() {
            eprintln!(
                "memory: {} notes about {} applications",
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
    pub fn learn(&self, app: &str, note: &str) -> String {
        let app = app.trim();
        let note = note.trim();
        if app.is_empty() || note.is_empty() {
            return "There was nothing to remember.".into();
        }
        let note: String = note.chars().take(LONGEST).collect();

        let mut notes = self.notes.lock().unwrap();
        let for_app = notes.entry(app.to_string()).or_default();

        // Said twice is said once. Models restate things, and a note repeated in
        // slightly different words is the same note taking two of eight slots.
        if for_app
            .iter()
            .any(|n| n.eq_ignore_ascii_case(&note) || n.contains(note.as_str()))
        {
            return format!("Already noted about {app}.");
        }
        for_app.push(note.clone());
        while for_app.len() > PER_APP {
            for_app.remove(0);
        }
        let count = for_app.len();
        drop(notes);

        self.save();
        eprintln!("memory: about {app} -- {note}");
        format!("Noted about {app} ({count} kept).")
    }

    /// Forget one application entirely.
    pub fn forget(&self, app: &str) {
        self.notes.lock().unwrap().remove(app);
        self.save();
        eprintln!("memory: forgot everything about {app}");
    }

    /// Every application there is anything about, and how much.
    pub fn everything(&self) -> Vec<(String, usize)> {
        self.notes
            .lock()
            .unwrap()
            .iter()
            .map(|(app, notes)| (app.clone(), notes.len()))
            .collect()
    }

    /// What goes in the prompt. Empty unless this application has something.
    pub fn prompt(&self, app: Option<&str>) -> String {
        let Some(app) = app else {
            return String::new();
        };
        let notes = self.about(app);
        if notes.is_empty() {
            return String::new();
        }
        format!(
            "## What {app} turned out to be like\n\n\
             Learned here, by getting it wrong once. Treat it as true unless the \
             screen says otherwise -- the screen is what is happening now and this \
             is only what happened before.\n{}\n\n",
            notes
                .iter()
                .map(|n| format!("- {n}"))
                .collect::<Vec<_>>()
                .join("\n")
        )
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
        m.learn("CapCut", "The timeline view means a project is open.");
        assert_eq!(m.about("CapCut").len(), 1);
        assert!(m.about("Safari").is_empty());
        // And the prompt is silent about applications it knows nothing about.
        assert_eq!(m.prompt(Some("Safari")), "");
        assert!(m.prompt(Some("CapCut")).contains("timeline view"));
        assert_eq!(m.prompt(None), "");
    }

    /// Eight slots are worth having only if the same thing cannot take three.
    #[test]
    fn the_same_thing_said_twice_is_kept_once() {
        let m = blank();
        m.learn("Chrome", "Menus close if you click into them.");
        m.learn("Chrome", "menus close if you click into them.");
        m.learn(
            "Chrome",
            "Menus close if you click into them. Use the shortcut.",
        );
        assert_eq!(m.about("Chrome").len(), 2, "{:?}", m.about("Chrome"));
    }

    #[test]
    fn it_does_not_grow_without_limit() {
        let m = blank();
        for i in 0..20 {
            m.learn("Xcode", &format!("note number {i}"));
        }
        assert_eq!(m.about("Xcode").len(), PER_APP);
        // The oldest went; the newest stayed.
        assert!(m.about("Xcode").iter().any(|n| n == "note number 19"));
        assert!(!m.about("Xcode").iter().any(|n| n == "note number 0"));
    }

    #[test]
    fn forgetting_is_complete() {
        let m = blank();
        m.learn("Mail", "a thing");
        m.forget("Mail");
        assert!(m.about("Mail").is_empty());
        assert!(m.everything().is_empty());
    }

    #[test]
    fn nothing_is_learned_from_nothing() {
        let m = blank();
        m.learn("", "a thing");
        m.learn("Mail", "   ");
        assert!(m.everything().is_empty());
    }
}
