//! Running an agent: the loop, the window, and putting the cursor back.
use super::commands;
use super::state::{Auto, Screen};
use crate::core::agent::{Agents, State, MAX_STEPS};
use crate::core::session::Nudge;
use crate::core::{agent, click};
use tauri::{AppHandle, Emitter, Manager};

/// Between turns. Long enough that a runaway loop is not a spin, short enough
/// that the agent does not feel slower than it is -- the model call dominates
/// either way.
const BEAT: std::time::Duration = std::time::Duration::from_millis(120);

/// Tell every window what the agents are doing. The list is a handful of small
/// structs; diffing it would be machinery bought for nothing.
pub fn publish(app: &AppHandle) {
    let agents = app.state::<Agents>();
    let list = agents.list();
    app.emit("agents", &list).ok();
    show_window(app, !list.is_empty());
    // The notch shows blue while an agent is alive, and goes back to whatever the
    // foreground is doing once none are.
    if agents.running() {
        app.emit("status", "agent").ok();
    } else {
        app.emit("status", "idle").ok();
    }
}

/// Park the card under the menu bar on the right, clear of the status items.
pub fn place_window(app: &AppHandle) {
    let Some(win) = app.get_webview_window("agents") else { return };
    let (w, h) = (360.0, 220.0);
    let screen = app.state::<Screen>();
    let _ = win.set_size(tauri::LogicalSize::new(w, h));
    let _ = win.set_position(tauri::LogicalPosition::new(screen.w - w - 16.0, 34.0));
    let _ = win.set_ignore_cursor_events(false);
}

fn show_window(app: &AppHandle, visible: bool) {
    let Some(win) = app.get_webview_window("agents") else { return };
    if visible {
        let _ = win.show();
        #[cfg(target_os = "macos")]
        super::native::float_everywhere(&win);
    } else {
        let _ = win.hide();
    }
}

/// Start an agent and let it run. Returns immediately.
pub fn spawn(app: &AppHandle, goal: String, title: String, say: String) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let id = app.state::<Agents>().start(goal.clone(), title, say);
        publish(&app);

        // An agent that only points is not an agent. The switch is forced on for
        // the duration and put back afterwards, so a user who deliberately left
        // it off does not find it silently changed.
        let was_auto = app.state::<Auto>().0.on();
        let may_act = click::may_click();
        if may_act {
            app.state::<Auto>().0.set(true);
        }

        let end = run(&app, id, goal).await;
        app.state::<Auto>().0.set(was_auto);

        let end = if may_act {
            end
        } else {
            State::Failed {
                why: "Clicking needs Accessibility — System Settings › Privacy & Security".into(),
            }
        };
        app.state::<Agents>().set_state(id, end);
        app.state::<Nudge>().end();
        publish(&app);
    });
}

/// The loop. Each turn: stop? blocked? settle, step, act, report.
async fn run(app: &AppHandle, id: u64, goal: String) -> State {
    app.state::<Nudge>().begin(goal);

    loop {
        // Checked every turn, not once at the top: a stop that waits for the
        // current model call to return is not a stop.
        if app.state::<Agents>().stopping() {
            return State::Stopped;
        }

        // Blocked on the user. Nothing to do but wait for the card.
        let blocked = app
            .state::<Agents>()
            .list()
            .iter()
            .any(|a| a.id == id && matches!(a.state, State::Waiting { .. }));
        if blocked {
            tokio::time::sleep(BEAT).await;
            continue;
        }

        if app.state::<Agents>().list().iter().any(|a| a.id == id && a.step >= MAX_STEPS) {
            return State::Failed {
                why: format!("Gave up after {MAX_STEPS} steps — this isn't converging."),
            };
        }

        let (w, h) = {
            let s = app.state::<Screen>();
            (s.w, s.h)
        };
        let step = match app.state::<Nudge>().step((w, h)).await {
            Ok(Some(step)) => step,
            // The session ended under us -- cancelled, or the privacy guard shut
            // it down mid-task.
            Ok(None) => return State::Done,
            Err(e) => return State::Failed { why: e.to_string() },
        };

        app.state::<Agents>().advanced(id, step.say().to_string());
        if let Some(state) = agent::outcome(&step) {
            app.state::<Agents>().set_state(id, state.clone());
            publish(app);
            // A question pauses; anything else here is the end of the road.
            if matches!(state, State::Waiting { .. }) {
                continue;
            }
            return state;
        }

        if let Err(e) = commands::perform(app, &step) {
            return State::Failed { why: e.to_string() };
        }
        publish(app);
        tokio::time::sleep(BEAT).await;
    }
}
