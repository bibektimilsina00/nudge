//! The frontend's entire API surface. Thin on purpose: every one of these is a
//! translation from an IPC call into a `core` call, and nothing more.
use super::state::{Auto, Docked, Screen, Settle, Voice};
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
        // Started, not awaited: Gemini TTS is a network round trip, and the ring
        // must not wait on audio. `speech` supersedes its own playback, so a fast
        // second step silences the first rather than talking over it.
        let mut cfg = app.state::<Nudge>().cfg.clone();
        // The menu bar overrides the file, so switching voice costs a click rather
        // than an edit and a restart.
        app.state::<Voice>().get().apply(&mut cfg);
        let line = step.say().to_string();
        tauri::async_runtime::spawn(async move { speech::speak(&cfg, &line).await });
        match step {
            Step::Launch { app: name, .. } => {
                launch::open_app(name)?;
                app.state::<Settle>().after(AFTER_OPEN);
            }
            Step::Open { url, .. } => {
                launch::open_url(url)?;
                app.state::<Settle>().after(AFTER_OPEN);
            }
            Step::Point { at, act, .. } if app.state::<Auto>().0.on() => {
                // No advance() call here on purpose. Moving the pointer and pressing
                // it are both things the watchers already notice, so auto mode
                // travels the same path a person does and the two cannot drift.
                let done = match act {
                    Act::Hover => click::move_to(*at),
                    Act::DoubleClick => click::click(*at, 2),
                    Act::Click => click::click(*at, 1),
                };
                if let Err(e) = done {
                    app.emit("error", e.to_string()).ok();
                }
                app.state::<Settle>().after(AFTER_CLICK);
            }
            // Only with "act for me" on. In guide mode the bubble shows the text
            // and the user types it -- text lands wherever focus is, which is a
            // worse thing to get wrong unasked than a click on a marked control.
            Step::Type { text, submit, .. } if app.state::<Auto>().0.on() => {
                if let Err(e) = keyboard::type_text(text, *submit) {
                    app.emit("error", e.to_string()).ok();
                }
                app.state::<Settle>().after(AFTER_CLICK);
            }
            // Guide mode: the user is about to click it themselves, and menus take
            // just as long to open for them.
            Step::Point { .. } | Step::Type { .. } => app.state::<Settle>().after(AFTER_CLICK),
            _ => {}
        }
    }

    // Finishing ends the session; being unsure does not -- open the right app and
    // tap the hotkey again and the same goal carries on.
    if matches!(&step, Some(Step::Done { .. } | Step::Reply { .. })) {
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
    }
    app.state::<Nudge>().end();
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
