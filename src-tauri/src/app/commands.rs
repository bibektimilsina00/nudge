//! The frontend's entire API surface. Thin on purpose: every one of these is a
//! translation from an IPC call into a `core` call, and nothing more.
use super::state::{Auto, Docked, Screen, Settle, Voice};
use crate::core::agent::Agents;
use crate::core::provider::{Act, Step};
use crate::core::session::Nudge;
use crate::core::{click, keyboard, launch, speech};
use crate::error::Result;
use tauri::{AppHandle, Emitter, Manager};

#[tauri::command]
pub async fn start(goal: String, app: AppHandle) -> Result<Option<Step>> {
    app.state::<Nudge>().begin(goal);
    advance(app.clone()).await
}

/// A menu is drawn in well under this; it is the cost of not photographing the
/// screen mid-animation.
const AFTER_CLICK: std::time::Duration = std::time::Duration::from_millis(420);
/// Applications and web pages take their time, and there is nothing useful to see
/// until they are up.
const AFTER_OPEN: std::time::Duration = std::time::Duration::from_millis(2200);

#[tauri::command]
pub async fn advance(app: AppHandle) -> Result<Option<Step>> {
    app.emit("status", "thinking").ok();

    // Let whatever we just did finish happening before photographing the result.
    if let Some(wait) = app.state::<Settle>().remaining() {
        tokio::time::sleep(wait).await;
    }

    let (w, h) = {
        let s = app.state::<Screen>();
        (s.w, s.h)
    };
    let step = app.state::<Nudge>().step((w, h)).await?;

    if let Some(step) = &step {
        // Said before acting: launching an app or moving the pointer changes the
        // screen, and an explanation arriving after that is just noise.
        speak(&app, step.say());
        if let Err(e) = perform(&app, step) {
            app.emit("error", e.to_string()).ok();
        }
    }

    // Finishing ends the session; being unsure does not -- open the right app and
    // tap the hotkey again and the same goal carries on.
    if matches!(
        &step,
        Some(Step::Done { .. } | Step::Reply { .. } | Step::Agent { .. })
    ) {
        app.state::<Nudge>().end();
    }
    Ok(step)
}

/// End the session. `silence` decides whether the voice stops with it.
///
/// These look like one action and are not. Escape means "stop, I am taking over",
/// and must cut the voice off mid-word. The overlay clearing itself after a reply
/// means only that the bubble has been on screen long enough -- killing the
/// sentence there is how a long answer ends up chopped in half, which is exactly
/// what it sounded like.
#[tauri::command]
pub fn cancel(app: AppHandle, silence: bool) {
    if silence {
        speech::hush();
        // Escape is the key people already hit when a machine starts doing
        // something they did not expect. It has to stop the agent too.
        for a in app.state::<Agents>().list() {
            if !a.finished() {
                app.state::<Agents>().stop(a.id);
            }
        }
        super::agent::publish(&app);
    }
    app.state::<Nudge>().end();
}

/// Carry out one step. Shared by the foreground loop and the agent runtime, so
/// the two cannot drift into behaving differently.
pub(crate) fn perform(app: &AppHandle, step: &Step) -> Result<()> {
    match step {
        Step::Launch { app: name, .. } => {
            launch::open_app(name)?;
            app.state::<Settle>().after(AFTER_OPEN);
        }
        Step::Open { url, .. } => {
            launch::open_url(url)?;
            app.state::<Settle>().after(AFTER_OPEN);
        }
        Step::Type { text, submit, .. } if app.state::<Auto>().0.on() => {
            keyboard::type_text(text, *submit)?;
            app.state::<Settle>().after(AFTER_CLICK);
        }
        Step::Point { at, act, .. } if app.state::<Auto>().0.on() => {
            // No advance() call here on purpose. Moving the pointer and pressing
            // it are both things the watchers already notice, so auto mode
            // travels the same path a person does and the two cannot drift.
            match act {
                Act::Hover => click::move_to(*at),
                Act::DoubleClick => click::click(*at, 2),
                Act::Click => click::click(*at, 1),
            }?;
            app.state::<Settle>().after(AFTER_CLICK);
        }
        // Guide mode: the user is about to do it themselves, and menus take just
        // as long to open for them.
        Step::Point { .. } | Step::Type { .. } => app.state::<Settle>().after(AFTER_CLICK),
        Step::Agent { title, say } => {
            super::agent::spawn(app, app.state::<Nudge>().goal(), title.clone(), say.clone());
        }
        _ => {}
    }
    Ok(())
}

/// Start the voice without blocking on it, and clear the indicator when the
/// audio actually stops rather than after a guessed duration.
fn speak(app: &AppHandle, line: &str) {
    app.emit("status", "speaking").ok();
    let mut cfg = app.state::<Nudge>().cfg.clone();
    app.state::<Voice>().get().apply(&mut cfg);
    let line = line.to_string();
    let done = app.clone();
    tauri::async_runtime::spawn(async move {
        speech::speak(&cfg, &line).await;
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
            if !speech::is_playing() {
                break;
            }
        }
        done.emit("status", "idle").ok();
    });
}

/// Everything the card needs.
#[tauri::command]
pub fn agents(app: AppHandle) -> Vec<crate::core::agent::Agent> {
    app.state::<Agents>().list()
}

/// Answer the question an agent is blocked on, and let it carry on.
#[tauri::command]
pub fn answer_agent(app: AppHandle, id: u64, text: String) {
    app.state::<Agents>().answer(id, text.clone());
    // The running session is what the next turn reads, so the answer has to land
    // there as well as in the agent's own record.
    app.state::<Nudge>().note(format!("The user answered: {text}"));
    super::agent::publish(&app);
}

#[tauri::command]
pub fn stop_agent(app: AppHandle, id: u64) {
    app.state::<Agents>().stop(id);
    super::agent::publish(&app);
}

#[tauri::command]
pub fn dismiss_agent(app: AppHandle, id: u64) {
    app.state::<Agents>().dismiss(id);
    super::agent::publish(&app);
}

/// Is the companion parked in the panel?
#[tauri::command]
pub fn docked(app: AppHandle) -> bool {
    app.state::<Docked>().0.on()
}

/// Park the companion in the panel, or release it onto the screen.
#[tauri::command]
pub fn set_docked(app: AppHandle, docked: bool) {
    app.state::<Docked>().0.set(docked);
    // Both windows draw the companion, so both need to know which of them owns it.
    app.emit("docked", docked).ok();
    if !docked {
        // Releasing it is the point of pressing the button; the panel has done its
        // job and standing in front of the screen is the opposite of helping.
        if let Some(panel) = super::panel::window(&app) {
            let _ = panel.hide();
        }
    }
}

/// Turn "click for me" on or off from the panel.
///
/// Refuses to report itself on without the permission that makes it work: a switch
/// that looks enabled while silently doing nothing is worse than one that will not
/// move.
#[tauri::command]
pub fn set_auto(app: AppHandle, on: bool) -> bool {
    let allowed = !on || click::may_click() || click::request_click_permission();
    app.state::<Auto>().0.set(on && allowed);
    if on && !allowed {
        app.emit(
            "error",
            "Clicking needs Accessibility: System Settings > Privacy & Security > \
             Accessibility, then reopen Nudge.",
        )
        .ok();
    }
    on && allowed
}

/// Is "click for me" on?
#[tauri::command]
pub fn auto(app: AppHandle) -> bool {
    app.state::<Auto>().0.on()
}

/// Which voice is in use: "off", "system" or "gemini".
#[tauri::command]
pub fn voice_mode(app: AppHandle) -> &'static str {
    match app.state::<Voice>().get() {
        super::state::VoiceMode::Off => "off",
        super::state::VoiceMode::System => "system",
        super::state::VoiceMode::Gemini => "gemini",
    }
}

/// Change it. Unknown names are ignored rather than guessed at.
#[tauri::command]
pub fn set_voice_mode(app: AppHandle, mode: String) {
    let picked = match mode.as_str() {
        "off" => super::state::VoiceMode::Off,
        "system" => super::state::VoiceMode::System,
        "gemini" => super::state::VoiceMode::Gemini,
        _ => return,
    };
    app.state::<Voice>().set(picked);
    // The menu bar shows the same setting and has to follow.
    app.emit("voice-mode", mode).ok();
}

/// The microphone recording will use.
#[tauri::command]
pub fn microphone() -> String {
    crate::core::voice::input_name().unwrap_or_else(|| "No input device".into())
}

/// Version, for the footer.
#[tauri::command]
pub fn version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}

#[tauri::command]
pub fn quit(app: AppHandle) {
    app.exit(0);
}

/// Let the panel size itself to its content.
#[tauri::command]
pub fn fit_panel(app: AppHandle, height: f64) {
    super::panel::fit(&app, height);
}

/// Asking needs the keyboard; pointing must not steal a single click.
#[tauri::command]
pub fn set_interactive(app: AppHandle, on: bool) -> tauri::Result<()> {
    super::overlay::set_prompt_mode(&app, on)
}
