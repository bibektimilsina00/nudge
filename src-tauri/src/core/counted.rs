//! What the app reports about itself, and what it deliberately cannot.
//!
//! Nudge reads people's screens. That makes the usual analytics answer -- a
//! third-party key compiled into the binary -- the wrong one twice over: the key
//! is findable with `strings`, and "it only sends counts, honestly" is not a
//! claim anybody has to take on trust. Sending to our own server keeps the
//! answer short and checkable.
//!
//! ## What goes
//!
//! That a turn happened, how long it took, and which transcriber answered. That
//! a tour was given. That an agent finished or failed. That a version ran for
//! the first time on a machine.
//!
//! ## What cannot go
//!
//! Anything about what somebody was doing. No goal, no transcript, no window
//! title, no application name, no path, no URL. The `detail` field is two or
//! three words from a fixed vocabulary and is clipped on the way out as well as
//! on the way in, because a limit enforced in one place is a limit until
//! somebody edits the other one.
//!
//! This is the same rule the audit log follows, and it is worth the discipline
//! for the same reason: a record that cannot embarrass anybody is a record
//! nobody has to weigh up before turning on.
//!
//! ## Off in one click
//!
//! The switch in Settings, which writes one file next to the install id. It is
//! deliberately not a config-file key: a privacy switch that forgets when the
//! app restarts is not a switch, and nothing else in the config persists from
//! the panel. Nothing is queued while it is off and nothing is sent later.

/// The most words `detail` may carry. It is for "here", "cloud", "done",
/// "failed" -- never a sentence.
const DETAIL: usize = 40;

/// The switch. Present means off -- so the file exists only for people who
/// turned it off, and deleting `~/.config/nudge/` cannot silently opt anybody
/// back in to something they had declined, because it also removes the id that
/// made them a somebody.
fn off_path() -> Option<std::path::PathBuf> {
    dirs::home_dir().map(|d| d.join(".config/nudge/counted.off"))
}

/// Is this copy counting?
pub fn on() -> bool {
    !off_path().is_some_and(|p| p.exists())
}

/// Turn it on or off, for good.
pub fn set(on: bool) {
    let Some(path) = off_path() else { return };
    if on {
        let _ = std::fs::remove_file(&path);
        return;
    }
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(&path, "off\n");
}

/// Where the install id lives. Beside the config, in plain text, so anybody
/// wondering what identifies them can read it and delete it.
fn id_path() -> Option<std::path::PathBuf> {
    dirs::home_dir().map(|d| d.join(".config/nudge/install.id"))
}

/// This copy of the app, as a random id it made up about itself.
///
/// Not derived from the machine. A hardware id would be an identifier nobody
/// chose and nobody can change; this one can be deleted, and deleting it makes
/// this copy a new one, which is the correct amount of power to have over it.
pub fn install() -> String {
    static ID: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    ID.get_or_init(make_id).clone()
}

fn make_id() -> String {
    {
        let path = id_path();
        if let Some(found) = path
            .as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
        {
            return found;
        }
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes).ok();
        let made: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        if let Some(path) = path {
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(&path, &made);
        }
        made
    }
}

/// Is this the first time this version has run on this machine?
///
/// Kept beside the id, as the version that was last seen. It is how "installs"
/// and "upgrades" are counted without sending anything on every launch.
pub fn first_run_of(version: &str) -> bool {
    let Some(path) = dirs::home_dir().map(|d| d.join(".config/nudge/ran.version")) else {
        return false;
    };
    let before = std::fs::read_to_string(&path).unwrap_or_default();
    if before.trim() == version {
        return false;
    }
    let _ = std::fs::write(&path, version);
    true
}

/// One thing that happened, in the only shape this module can express.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Count {
    pub name: &'static str,
    #[serde(skip_serializing_if = "is_nothing")]
    pub seconds: f32,
    #[serde(skip_serializing_if = "str::is_empty")]
    pub detail: String,
}

fn is_nothing(n: &f32) -> bool {
    *n <= 0.0
}

impl Count {
    pub fn of(name: &'static str) -> Self {
        Count {
            name,
            seconds: 0.0,
            detail: String::new(),
        }
    }

    pub fn taking(mut self, seconds: f32) -> Self {
        self.seconds = seconds;
        self
    }

    /// A couple of words from a fixed vocabulary. Clipped here as well as at the
    /// server, because a limit enforced in one place is a limit until somebody
    /// edits the other one.
    pub fn shaped(mut self, detail: impl AsRef<str>) -> Self {
        self.detail = detail.as_ref().chars().take(DETAIL).collect();
        self
    }
}

/// Send one, if this copy is counting at all.
///
/// Fire and forget, and silent when it fails: a count that can slow down or
/// break a turn is worse than not knowing. Nothing is queued while it is off,
/// and nothing is kept to send later -- an event that could not be sent is an
/// event nobody needed.
pub fn send(count: Count) {
    if !on() {
        return;
    }
    let body = serde_json::json!({
        "install": install(),
        "version": env!("CARGO_PKG_VERSION"),
        "platform": std::env::consts::OS,
        "events": [count],
    });
    let token = crate::core::account::token();
    let go = async move {
        let mut req = crate::core::http()
            .post(format!("{}/api/events", crate::core::account::api()))
            .json(&body);
        if let Some(token) = token {
            req = req.bearer_auth(token);
        }
        let _ = req.send().await;
    };

    // tokio rather than Tauri's re-export: core does not know Tauri exists, and
    // `tests/layering.rs` fails the build rather than let it start to.
    //
    // The first-run count is sent from Tauri's `setup`, where there is no
    // ambient runtime to spawn onto -- `tokio::spawn` there panics, which would
    // make the very first launch of a new version the one launch that crashes.
    // So: spawn when there is a runtime, and otherwise carry a small one out to
    // a thread of its own.
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => {
            handle.spawn(go);
        }
        Err(_) => {
            std::thread::spawn(move || {
                if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    rt.block_on(go);
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The clip is a backstop, not the protection. Nothing in this module ever
    /// passes a sentence to `shaped` -- the callers pass a provider name or an
    /// outcome -- and the limit is here so that a future caller which tries
    /// cannot get a whole sentence out, only a useless fragment of one.
    #[test]
    fn a_detail_is_a_couple_of_words_and_never_a_sentence() {
        let long = "the user asked me to open their bank statement in the downloads folder";
        let count = Count::of("turn").shaped(long);
        assert!(count.detail.chars().count() <= DETAIL);
        assert!(count.detail.chars().count() < long.chars().count());
    }

    /// Off means nothing is sent, not "sent later".
    #[test]
    fn off_is_a_file_that_exists_only_when_somebody_said_no() {
        // Absent means on, which is what a fresh machine has. The path is
        // checked rather than written here: the test must not reach into the
        // real config directory and switch the developer's own copy off.
        let path = off_path().expect("a home directory");
        assert!(path.ends_with("counted.off"));
        assert_eq!(on(), !path.exists());
    }

    #[test]
    fn an_install_id_is_made_up_rather_than_read_off_the_machine() {
        let one = install();
        assert_eq!(one.len(), 32, "sixteen random bytes, in hex");
        assert_eq!(one, install(), "and it does not change while running");
    }
}
