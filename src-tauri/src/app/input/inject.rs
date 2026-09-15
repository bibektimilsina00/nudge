//! Saying something to Nudge without saying it out loud.
//!
//! Every end-to-end check of this program used to cost a person holding a key and
//! speaking a sentence, which made checking rare, made it manual, and meant the
//! thing being exercised was never the part under test -- a tool call, a refusal,
//! a routing decision -- but always the microphone in front of it.
//!
//! So a line of text can arrive instead, and everything after transcription runs
//! exactly as it does for speech: the same acknowledgement, the same routing, the
//! same agent, the same steps. What is skipped is the recording and the speech
//! model, which are the two parts that were never in doubt.
//!
//! ## Off unless asked for, and never from the config file
//!
//! **This is a way to make Nudge do things.** Anything that can write the file can
//! drive somebody's computer, and `/tmp` is writable by everyone on the machine.
//! So it is behind an environment variable rather than a setting: a config option
//! is a thing you turn on once and forget, while an environment variable lasts
//! exactly as long as the process somebody started on purpose.
//!
//! ```text
//! NUDGE_SAY=/tmp/nudge-say open -a Nudge
//! echo "what is on my shopping list" > /tmp/nudge-say
//! ```
//!
//! The file is emptied as it is read, so one line is one turn.
use tauri::AppHandle;

/// How often the file is looked at.
///
/// Polling rather than watching the filesystem: this is a development affordance
/// and 200ms is imperceptible to whoever is waiting for it, against a dependency
/// and a platform-specific API for the alternative.
const EVERY: std::time::Duration = std::time::Duration::from_millis(200);

/// The variable that turns this on, holding the path to watch.
const VAR: &str = "NUDGE_SAY";

/// Start watching, if this run was started with somewhere to watch.
pub fn start(app: &AppHandle) {
    let Some(path) = std::env::var_os(VAR) else {
        return;
    };
    let path = std::path::PathBuf::from(path);
    println!("nudge: taking typed goals from {}", path.display());
    // Cleared at startup so a line left over from a previous run does not fire
    // the moment this one begins.
    let _ = std::fs::write(&path, "");

    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(EVERY).await;
            let Ok(said) = std::fs::read_to_string(&path) else {
                continue;
            };
            let said = said.trim().to_string();
            if said.is_empty() {
                continue;
            }
            // Emptied before the turn rather than after, so a turn that takes
            // twenty seconds is not started again on every poll in between.
            let _ = std::fs::write(&path, "");

            println!("nudge: typed -- {said:?}");
            if let Err(e) = super::hotkey::ask(app.clone(), said).await {
                eprintln!("nudge: typed goal failed: {e}");
            }
        }
    });
}

#[cfg(test)]
mod tests {
    /// The safety property, and the only one worth a test: nothing happens
    /// unless this run was started with the variable set.
    ///
    /// Asserted on the environment rather than through a helper. There was a
    /// helper, it was used by nothing but this test, and a function that exists
    /// to be tested is a function testing itself -- `start` reads the variable
    /// directly, so this reads the variable directly.
    #[test]
    fn it_is_off_unless_the_environment_asks_for_it() {
        // The test process was not started with it, and neither is Nudge unless
        // somebody types it in front of the command.
        assert!(std::env::var_os(super::VAR).is_none());
    }
}
