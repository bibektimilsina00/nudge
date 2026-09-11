//! Push-to-talk. Speaking is the primary way in, so one key carries both jobs:
//! hold it to talk, tap it to advance a step. Nothing new to learn, and the tap
//! stays available when you are mid-sequence and just want the next nudge.
use super::commands;
use super::state::Mic;
use crate::config::Config;
use crate::core::session::Nudge;
use crate::core::{transcribe, voice};
use crate::error::Result;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{Builder, ShortcutState};

/// Hold longer than this and you meant to speak; shorter and you meant "next step".
pub const TAP: std::time::Duration = std::time::Duration::from_millis(350);

/// Boxed error rather than [`crate::error::Error`]: the plugin's failures are setup
/// problems (bad shortcut string, already registered), not things Nudge can act on,
/// and inventing variants for them would only pad the enum.
pub fn register(
    app: &AppHandle,
    hotkey: &str,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let handle = app.clone();
    app.plugin(
        Builder::new()
            .with_shortcuts([hotkey])?
            .with_handler(move |_, _, event| on_key(&handle, event.state))
            .build(),
    )?;
    Ok(())
}

fn on_key(app: &AppHandle, state: ShortcutState) {
    match state {
        ShortcutState::Pressed => {
            // Neither of these can be fixed by recording anyway, and both have a
            // real remedy the OS can show. The prompt does not block, so a
            // first-ever hold would otherwise record silence behind the dialog and
            // report it as a failure.
            match voice::access() {
                voice::Access::Unasked => {
                    voice::request_access();
                    app.emit("error", "Allow microphone access, then hold the key again.")
                        .ok();
                    return;
                }
                voice::Access::Denied => {
                    // macOS never re-prompts after a refusal; the pane is the only
                    // way back.
                    voice::open_privacy_settings();
                    app.emit("error", "Turn on the microphone for Nudge, then hold again.")
                        .ok();
                    return;
                }
                voice::Access::Granted => {}
            }
            let mic = app.state::<Mic>();
            let mut held = mic.0.lock().unwrap();
            if held.is_none() {
                // Key repeat fires Pressed over and over; only the first one counts.
                *held = Some((std::time::Instant::now(), voice::start()));
                app.emit("listening", ()).ok();
            }
        }
        ShortcutState::Released => {
            let Some((at, rec)) = app.state::<Mic>().0.lock().unwrap().take() else {
                return;
            };
            if at.elapsed() < TAP {
                // A tap. Bin the audio off-thread so the key feels instant.
                std::thread::spawn(move || drop(rec.finish()));
                let next = if app.state::<Nudge>().active() { "advance" } else { "ask" };
                app.emit(next, ()).ok();
                return;
            }
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = ask_by_voice(app.clone(), rec).await {
                    app.emit("error", e.to_string()).ok();
                }
            });
        }
    }
}

/// The transcript is shown before anything acts on it -- a voice UI that silently
/// mishears is worse than no voice UI.
async fn ask_by_voice(app: AppHandle, rec: voice::Recording) -> Result<()> {
    // finish() blocks a few ms draining the audio callbacks. Not worth a
    // spawn_blocking hop.
    let wav = rec.finish()?;
    let cfg: Config = app.state::<Nudge>().cfg.clone();

    let heard = transcribe::speech_to_text(&cfg, &wav).await?;
    app.emit("heard", &heard).ok();

    app.state::<Nudge>().begin(heard);
    let step = commands::advance(app.clone()).await?;
    app.emit("step", step).ok();
    Ok(())
}
