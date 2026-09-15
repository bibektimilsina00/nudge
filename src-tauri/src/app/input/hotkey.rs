//! Push-to-talk. Speaking is the primary way in, so one key carries both jobs:
//! hold it to talk, tap it to advance a step. Nothing new to learn, and the tap
//! stays available when you are mid-sequence and just want the next nudge.
use crate::app::commands;
use crate::app::state::Mic;
use crate::config::Config;
use crate::core::run::session::Nudge;
use crate::core::voice;
use crate::core::voice::transcribe;
use crate::error::Result;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{Builder, ShortcutState};

/// Hold longer than this and you meant to speak; shorter and you meant "next step".
pub const TAP: std::time::Duration = std::time::Duration::from_millis(350);

/// Boxed error rather than [`crate::error::Error`]: the plugin's failures are setup
/// problems (bad shortcut string, already registered), not things Nudge can act on,
/// and inventing variants for them would only pad the enum.
/// Is this shortcut a bare modifier? Those cannot be registered -- see
/// [`crate::core::screen::click::control_alone`] -- and are polled instead.
pub fn is_bare_modifier(hotkey: &str) -> bool {
    matches!(
        hotkey.trim().to_ascii_lowercase().as_str(),
        "ctrl" | "control"
    )
}

pub fn register(
    app: &AppHandle,
    hotkey: &str,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    if is_bare_modifier(hotkey) {
        // Watched by the pointer loop; nothing to register.
        return Ok(());
    }
    let handle = app.clone();
    app.plugin(
        Builder::new()
            .with_shortcuts([hotkey])?
            .with_handler(move |_, _, event| on_key(&handle, event.state))
            .build(),
    )?;
    Ok(())
}

pub fn on_key(app: &AppHandle, state: ShortcutState) {
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
                    app.emit(
                        "error",
                        "Turn on the microphone for Nudge, then hold again.",
                    )
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
                drop(held);
                // Recording starts now, but the notch waits out the tap window.
                // On a bare modifier every ctrl+click is a press, and announcing
                // Listening for 200ms on each one made the notch blink constantly.
                // Nothing is lost by waiting -- the audio from before the threshold
                // is already captured.
                let app = app.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(TAP);
                    if app.state::<Mic>().0.lock().unwrap().is_some() {
                        app.emit("listening", ()).ok();
                        app.emit("status", "listening").ok();
                    }
                });
            }
        }
        ShortcutState::Released => {
            let Some((at, rec)) = app.state::<Mic>().0.lock().unwrap().take() else {
                return;
            };
            if at.elapsed() < TAP {
                // A tap. Bin the audio off-thread so the key feels instant.
                std::thread::spawn(move || drop(rec.finish()));
                // Every path out of "listening" has to clear it. This one did
                // not, so a tap left the notch saying Listening indefinitely.
                app.emit("status", "idle").ok();
                // A tap advances a sequence and otherwise does nothing. There is no
                // typed prompt to open -- speaking is the only way in.
                if let Some(doing) = busy(app) {
                    say_busy(app, &doing);
                } else if app.state::<Nudge>().active() {
                    app.emit("advance", ()).ok();
                }
                return;
            }
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = ask_by_voice(app.clone(), rec).await {
                    // Whatever went wrong, the notch must not be left mid-state.
                    app.emit("status", "idle").ok();
                    eprintln!("listening failed: {e}");
                    let goal = app.state::<Nudge>().goal();
                    app.emit("error", crate::error::plainly(&goal, &e)).ok();
                }
            });
        }
    }
}

/// What an agent is doing right now, if one is.
///
/// While an agent runs it owns the cursor and the keyboard. Every way in has to
/// check: the registry refuses a *second agent*, but the foreground path could
/// still act -- a tap advanced a step and a new goal wiped the agent's session
/// out from under it, and either way two things were clicking at once.
fn busy(app: &AppHandle) -> Option<String> {
    let agents = app.state::<crate::core::run::agent::Agents>();
    // A question is not busy: it is waiting for exactly this.
    if agents.waiting().is_some() {
        return None;
    }
    agents.current().map(|a| a.title)
}

/// Say what it is doing, and how to take the machine back. Silence here is what
/// makes someone repeat themselves, which is how one WhatsApp chat ended up with
/// three agents clicking in it.
fn say_busy(app: &AppHandle, doing: &str) {
    eprintln!("ignored: agent {doing:?} is running");
    app.emit("status", "agent").ok();
    commands::speak(
        app,
        &format!("I'm still on {doing}. Press escape to stop me."),
    );
}

/// Something to say while the model thinks.
///
/// Rotated rather than random: the same phrase every time is a recording, and a
/// random one is a slot machine. Cycling four is enough that it does not land on
/// a pattern anyone notices.
fn acknowledgement() -> &'static str {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    const LINES: [&str; 4] = [
        "Sure, one sec.",
        "On it.",
        "Let me take a look.",
        "Right, one moment.",
    ];
    LINES[N.fetch_add(1, Ordering::Relaxed) % LINES.len()]
}

/// The transcript is shown before anything acts on it -- a voice UI that silently
/// mishears is worse than no voice UI.
async fn ask_by_voice(app: AppHandle, rec: voice::Recording) -> Result<()> {
    // The turn starts here: the key is up and the clock is the user's from this
    // moment, whatever we spend it on.
    app.state::<Nudge>().clock_in();

    // finish() blocks a few ms draining the audio callbacks. Not worth a
    // spawn_blocking hop.
    let Some(wav) = rec.finish()? else {
        // Nothing was said. Put the notch back and let it go -- an error for
        // staying quiet would fire constantly and mean nothing.
        app.emit("status", "idle").ok();
        return Ok(());
    };
    app.state::<Nudge>().mark("wav");
    let cfg: Config = app.state::<Nudge>().cfg.clone();

    app.emit("status", "thinking").ok();

    // The picture does not depend on the words, so it need not wait for them.
    // Started here and collected below, it is taken *during* the transcription
    // instead of after it -- about half a second that now costs nothing.
    //
    // It is also taken before we say anything, which matters for a second
    // reason: this is the only moment on this path where the output device is
    // quiet, so it is the only moment the audio fact is about the world rather
    // than about us.
    let early = tokio::task::spawn_blocking({
        let cfg = cfg.clone();
        move || crate::core::screen::look(&cfg)
    });

    let Some(heard) = transcribe::speech_to_text(&cfg, &wav).await? else {
        // Room noise, or a cough. Nothing was said, so say nothing back.
        app.emit("status", "idle").ok();
        return Ok(());
    };
    app.state::<Nudge>().mark("heard");

    // A refusal or a failed capture is dropped here rather than reported.
    // `step` looks again for itself when there is nothing stashed, and that is
    // the place that can refuse properly -- it has a session to end and a
    // sentence to say. Reporting it twice would mean saying no twice.
    if let Ok(Ok(look)) = early.await {
        app.state::<Nudge>().stash(look);
    }
    ask(app, heard).await
}

/// Everything a turn does once the words exist.
///
/// Split from the listening above it so that words can arrive another way. Speech
/// is how a person uses this; it is a poor way to *test* it, because every check
/// costs somebody saying a sentence out loud and the thing being checked is never
/// the microphone. See [`super::inject`].
pub(crate) async fn ask(app: AppHandle, heard: String) -> Result<()> {
    app.emit("heard", &heard).ok();

    // Answer before thinking.
    //
    // The first model call takes two and a half seconds, and until now that was
    // two and a half seconds of silence after someone had just spoken. A person
    // says "one sec" in that gap; so does every assistant that feels quick. This
    // costs nothing and changes how the whole thing feels, because the wait did
    // not get shorter -- it stopped being a wait for a reply and became a wait
    // for the answer.
    //
    // Two words, not a sentence: it has to be finished well before the real one
    // starts, or it is a thing that gets interrupted.
    commands::speak(&app, acknowledgement());

    // An agent asked something and is holding. What you just said is the answer,
    // not a new goal -- starting a fresh session here would abandon the task
    // mid-way and leave the question unanswered forever.
    if let Some((id, question)) = app.state::<crate::core::run::agent::Agents>().waiting() {
        eprintln!("agent#{id} answered {heard:?} to {question:?}");
        commands::answer_agent(app.clone(), id, heard);
        return Ok(());
    }

    // A new goal while an agent is working is not a new goal -- it is someone
    // wondering why nothing is happening. Starting one here would wipe the
    // running agent's session and act on the same cursor.
    if let Some(doing) = busy(&app) {
        say_busy(&app, &doing);
        return Ok(());
    }

    // Counted before the work, so a session that goes wrong still counts as use.
    app.state::<crate::core::offers::Offers>().a_turn_happened();
    app.state::<Nudge>().begin(heard);
    let step = commands::advance(app.clone()).await?;
    app.emit("step", step).ok();
    Ok(())
}
