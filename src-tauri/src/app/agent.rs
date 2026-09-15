//! Running an agent: the loop, the window, and putting the cursor back.
use super::commands;
use super::state::Screen;
use crate::core::run::agent;
use crate::core::run::agent::{Agents, State, MAX_STEPS};
use crate::core::run::session::Nudge;
use crate::core::screen::click;
use tauri::{AppHandle, Emitter, Manager};

/// Between turns. Long enough that a runaway loop is not a spin, short enough
/// that the agent does not feel slower than it is -- the model call dominates
/// either way.
const BEAT: std::time::Duration = std::time::Duration::from_millis(120);

/// Extra grace when the model asks for the same thing twice -- whatever it is
/// looking at has not finished becoming the next screen.
const SETTLE_AGAIN: std::time::Duration = std::time::Duration::from_millis(1500);

/// How many times the same action may be asked for before we call it a loop.
///
/// Two is a page that had not finished loading. Three is a model that cannot see
/// the thing it keeps aiming at, and no number of further tries will change that
/// -- one run spent nineteen turns clicking the same YouTube link while the song
/// it was trying to start was already playing.
const SAME_ACTION_LIMIT: usize = 3;

/// How many actions may fail outright before the run is called off.
///
/// One failure is information, not an ending: an app that is not installed, a
/// control that moved, a page that was not ready. Asked to open Photoshop on a
/// machine without it, the agent died on the raw error instead of saying "you
/// do not have Photoshop" -- which it could only do if it were told what went
/// wrong and given another turn.
const FAILURE_LIMIT: usize = 3;

/// How many turns in a row may do nothing at all before giving up.
///
/// `unsure` and `reply` perform no action, so the screen cannot change and the
/// next turn sees exactly what this one saw. Without a limit that is a loop with
/// a model call in it: one run burned twelve turns repeating the same sentence.
const IDLE_LIMIT: usize = 3;

/// How long a finished task holds the line after offering a next step.
///
/// An offer is not a question the task depends on -- the work is already done --
/// so silence has to mean "no thanks" rather than leaving it waiting forever.
/// Long enough to hear it, think, and reach for the key.
const OFFER_WINDOW: std::time::Duration = std::time::Duration::from_secs(25);

/// Tell every window what the agents are doing. The list is a handful of small
/// structs; diffing it would be machinery bought for nothing.
pub fn publish(app: &AppHandle) {
    let agents = app.state::<Agents>();
    let list = agents.list();
    // Shown while anything is running, collapsed to a square per agent.
    //
    // Not the same as the full card it used to be: that was a progress bar for a
    // three-second task and it was rightly unwanted. A 38px tile that says "this
    // is still going, and here is how to stop it" is worth the corner -- an agent
    // owns the real cursor while it works, and that should never be invisible.
    // The card is one click away, and finished work lives in the Agents tab.
    let (visible, running) = (agents.running(), agents.running());
    app.emit("agents", &list).ok();

    // Onto the main thread, always.
    //
    // `publish` is called from inside the agent's async task, which tokio runs on
    // a worker thread, and `show_window` reaches through to `NSWindow`. AppKit
    // windows may only be touched from the main thread -- doing it anywhere else
    // is not a race that might bite, it is an assertion that fires immediately.
    // Starting an agent killed the app on the first call.
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || show_window(&handle, visible));
    // The notch shows blue while an agent is alive, and goes back to whatever the
    // foreground is doing once none are.
    if agents.waiting().is_some() {
        // Waiting on an answer outranks both: the notch has to keep saying so
        // until the user speaks, or the question looks like it was withdrawn.
        app.emit("status", "asking").ok();
    } else if running {
        app.emit("status", "agent").ok();
    } else {
        app.emit("status", "idle").ok();
    }
}

/// Park the card under the menu bar on the right, clear of the status items.
pub fn place_window(app: &AppHandle) {
    let Some(win) = app.get_webview_window("agents") else {
        return;
    };
    // A starting size only. It is replaced by [`fit`] as soon as the content has
    // measured itself, because a transparent window is still a window: at
    // 360x520 it sat over a swathe of desktop that could no longer be dragged on,
    // clicked through, or dropped into -- an invisible hole in somebody's screen
    // whose only tenant was a 46px face.
    let (w, h) = (360.0, 520.0);
    let screen = app.state::<Screen>();
    let _ = win.set_size(tauri::LogicalSize::new(w, h));
    let _ = win.set_position(tauri::LogicalPosition::new(screen.w - w - 16.0, 34.0));
    let _ = win.set_ignore_cursor_events(false);
}

/// Shrink the window to whatever the content turned out to be.
///
/// Called from the interface, which is the only thing that knows: the tiles are
/// laid out by the browser and their number changes as agents come and go. The
/// window stays anchored to the top right, so it grows leftwards and downwards
/// from the corner and never moves under the pointer.
///
/// Rounded up by a pixel. A fractional logical size lands between device pixels
/// on a Retina display and clips the last column of whatever is at the edge.
pub fn fit(app: &AppHandle, w: f64, h: f64) {
    let Some(win) = app.get_webview_window("agents") else {
        return;
    };
    let (w, h) = (w.ceil().clamp(56.0, 380.0), h.ceil().clamp(56.0, 560.0));
    let screen = app.state::<Screen>();
    let _ = win.set_size(tauri::LogicalSize::new(w, h));
    let _ = win.set_position(tauri::LogicalPosition::new(screen.w - w - 16.0, 34.0));
}

/// Is this point outside the agent window?
///
/// Asked on every click, so that an open card can be dismissed by clicking away
/// from it -- which is what every panel, popover and menu on this machine does,
/// and the first thing anybody tries.
///
/// It has to be asked from outside the window because the click never arrives:
/// a click that lands somewhere else goes to whatever is there, and the window
/// hears nothing at all. The pointer loop is already watching the screen sixty
/// times a second for exactly this class of thing.
///
/// `false` when there is no window or its geometry cannot be read -- a card that
/// will not close is a nuisance, and a card that closes at random is worse.
pub fn away_from_card(app: &AppHandle, at: crate::core::screen::capture::Point) -> bool {
    let Some(win) = app.get_webview_window("agents") else {
        return false;
    };
    if !win.is_visible().unwrap_or(false) {
        return false;
    }
    let (Ok(pos), Ok(size), Ok(scale)) = (win.outer_position(), win.outer_size(), win.scale_factor())
    else {
        return false;
    };
    // Physical pixels from the window, logical points from the pointer. On a
    // Retina display those differ by two, which is the kind of mistake that only
    // shows up on the machine that does not have one.
    let (x, y) = (pos.x as f64 / scale, pos.y as f64 / scale);
    let (w, h) = (size.width as f64 / scale, size.height as f64 / scale);
    !(at.x >= x && at.x <= x + w && at.y >= y && at.y <= y + h)
}

fn show_window(app: &AppHandle, visible: bool) {
    let Some(win) = app.get_webview_window("agents") else {
        eprintln!("agents: no window to show");
        return;
    };
    if visible {
        let _ = win.show();
        #[cfg(target_os = "macos")]
        crate::app::ui::native::float_everywhere(&win);
    } else {
        let _ = win.hide();
    }
}

/// Start an agent and let it run. Returns immediately.
pub fn spawn(
    app: &AppHandle,
    goal: String,
    title: String,
    say: String,
    background: bool,
    carried: Vec<String>,
) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let Some(id) = app
            .state::<Agents>()
            .start(goal.clone(), title, say, background)
        else {
            // Already busy. Say so rather than silently dropping it -- the user
            // spoke again because they thought nothing was happening, and
            // starting a second agent is how one WhatsApp chat got two sets of
            // clicks and sent a voice note to a real person.
            let doing = app
                .state::<Agents>()
                .current()
                .map(|a| a.title)
                .unwrap_or_else(|| "that".into());
            eprintln!("agent refused: already running {doing:?}");
            commands::speak(
                &app,
                &format!("I'm still on {doing}. Press escape to stop me."),
            );
            return;
        };
        eprintln!("agent#{id} started: {goal:?}");
        publish(&app);

        // There is no longer a switch to force: Nudge always acts. A ring that
        // waits for the user to click is not what anyone asked an agent for, and
        // the toggle only ever existed to make that the default.
        let may_act = click::may_click();

        let end = run(&app, id, goal, carried).await;

        let end = if may_act {
            end
        } else {
            State::Failed {
                why: "Clicking needs Accessibility — System Settings › Privacy & Security".into(),
            }
        };
        // Nothing outlives the task that started it. A dev server still holding
        // port 3000 tomorrow is Nudge's fault, not the user's -- and a process
        // nobody is watching is the whole risk of being able to start one.
        app.state::<crate::app::state::Background>().stop_all();
        // What was recorded, not what was returned. `set_state` refuses to move
        // an agent out of `Stopped`, so a late `Done` arriving after Escape is
        // already ignored -- but the log printed the return value and cheerfully
        // said `ended: Done` about a run that was recorded as stopped. A log that
        // disagrees with the record is worse than no log.
        let recorded = app
            .state::<Agents>()
            .list()
            .into_iter()
            .find(|a| a.id == id)
            .map(|a| a.state);
        eprintln!("agent#{id} ended: {:?}", recorded.unwrap_or(end.clone()));
        app.state::<Agents>().set_state(id, end);
        app.state::<Nudge>().end();
        publish(&app);
    });
}

/// The loop. Each turn: stop? blocked? settle, step, act, report.
async fn run(app: &AppHandle, id: u64, goal: String, carried: Vec<String>) -> State {
    app.state::<Nudge>().begin_agent(goal, carried);
    let mut last: Option<crate::core::provider::Step> = None;
    let mut repeats = 0usize;
    let mut failures = 0usize;
    let mut idle = 0usize;
    // Set when the wait is a post-completion offer rather than a real blocker.
    let mut offered: Option<std::time::Instant> = None;

    loop {
        // Checked every turn, not once at the top: a stop that waits for the
        // current model call to return is not a stop.
        if app.state::<Agents>().stopping() {
            return State::Stopped;
        }

        // Blocked on the user. Nothing to do but wait for the card.
        let mine = app
            .state::<Agents>()
            .list()
            .into_iter()
            .find(|a| a.id == id);
        // If this agent is not in the registry, every write to it -- its step
        // count, its state -- goes nowhere, and a question can never block. That
        // is not a state to guess about from the outside: one run asked the same
        // question four times in a row because of it.
        let Some(mine) = mine else {
            eprintln!("agent#{id} is not in the registry; stopping rather than looping");
            return State::Failed {
                why: "lost track of this task".into(),
            };
        };
        let blocked = matches!(mine.state, State::Waiting { .. });
        if blocked {
            // An offer lapses; a genuine question does not. Being asked "shall I
            // also do X?" and walking away has to end the task, not hang it.
            if let Some(since) = offered {
                if since.elapsed() > OFFER_WINDOW {
                    eprintln!("agent#{id}: offer went unanswered -- finishing");
                    return State::Done;
                }
            }
            tokio::time::sleep(BEAT).await;
            continue;
        }
        // Answered, so we are past the offer.
        offered = None;

        if app
            .state::<Agents>()
            .list()
            .iter()
            .any(|a| a.id == id && a.step >= MAX_STEPS)
        {
            return State::Failed {
                why: format!("Gave up after {MAX_STEPS} steps — this isn't converging."),
            };
        }

        // Let whatever we just did finish happening before photographing it.
        //
        // The foreground path has always done this and the agent never did, so
        // every agent screenshot was taken 120ms after the click that was meant
        // to change it -- a blank page, a menu mid-animation, a video that had
        // not started. It then reasoned about that stale picture and acted on it.
        // This is the single largest source of the flailing in the test runs.
        app.state::<Nudge>().clock_in();
        if let Some(wait) = app.state::<super::state::Settle>().remaining() {
            tokio::time::sleep(wait).await;
        }
        app.state::<Nudge>().mark("settle");

        // A trace, not logging machinery. An agent that fails silently is an agent
        // you cannot debug, and every turn is one model call -- a line each is
        // nothing next to that. Run the app from a terminal to watch it.
        let turn = app
            .state::<Agents>()
            .list()
            .iter()
            .find(|a| a.id == id)
            .map_or(0, |a| a.step);
        let began = std::time::Instant::now();

        let step = match app.state::<Nudge>().step().await {
            Ok(Some(step)) => {
                eprintln!(
                    "agent#{id} turn {turn} ({:.1}s): {step:?}",
                    began.elapsed().as_secs_f32()
                );
                step
            }
            // The session ended under us -- cancelled, or the privacy guard shut
            // it down mid-task.
            //
            // It said `Done` for every one of those, which is how `ended: Done`
            // came to sit one line under `stopped by Escape` in a real log.
            //
            // But it is not `Stopped` for all of them either -- that was the next
            // thing tried, and it marked a run that had genuinely finished as
            // stopped, because the ordinary reason a session ends under an agent
            // is the foreground turn completing the work first.
            //
            // The signal that separates them is whether anybody asked it to stop.
            Ok(None) if app.state::<Agents>().stopping() => {
                eprintln!("agent#{id} turn {turn}: stopped, and the session went with it");
                return State::Stopped;
            }
            Ok(None) => {
                eprintln!("agent#{id} turn {turn}: session ended -- nothing left to do");
                return State::Done;
            }
            Err(e) => {
                eprintln!("agent#{id} turn {turn}: FAILED {e}");
                return State::Failed { why: e.to_string() };
            }
        };

        app.state::<Agents>().advanced(id, step.say().to_string());

        // Out loud, every turn -- the foreground path has always done this and
        // the agent never did. The final sentence is the one that matters: for
        // a question like "what's the weather", the answer only exists here, and
        // writing it to a log the user cannot see is not answering them.
        //
        // Said before acting, because acting changes the screen and an
        // Before it says anything at all.
        //
        // The two checks below guard *acting*, and a terminal step never reaches
        // them -- it speaks and returns first. So a stop pressed during the model
        // call let the answer through: "I have submitted the task" for work that
        // had just been cancelled. Silence is the right response to being stopped.
        if app.state::<Agents>().stopping() {
            return State::Stopped;
        }

        // explanation arriving after that is just noise.
        if !matches!(agent::outcome(&step), Some(State::Waiting { .. })) {
            commands::speak(app, step.say());
        }
        if let Some(state) = agent::outcome(&step) {
            app.state::<Agents>().set_state(id, state.clone());
            publish(app);
            // A question nobody hears is a hang. There is no card for foreground
            // work and no typed prompt anywhere, so the voice is the whole
            // interface: Nudge asks out loud and listens for the answer.
            if let State::Waiting { question } = &state {
                // A finished task that offered a next step says both halves: what
                // it did, then what it could do. Two sentences, one breath.
                let line = match &step {
                    crate::core::provider::Step::Done { say, .. } => {
                        offered = Some(std::time::Instant::now());
                        format!("{say} {question}")
                    }
                    _ => question.clone(),
                };
                eprintln!("agent#{id} turn {turn}: asking -- {line}");
                commands::speak(app, &line);
                app.emit("status", "asking").ok();
            }
            // A question pauses; anything else here is the end of the road.
            if matches!(state, State::Waiting { .. }) {
                continue;
            }
            return state;
        }

        // A turn that performs nothing leaves the next turn looking at the same
        // screen, so it will decide the same thing. Two is thinking; three is a
        // loop.
        if matches!(step, crate::core::provider::Step::Unsure { .. }) {
            idle += 1;
            if idle >= IDLE_LIMIT {
                return State::Failed {
                    why: format!(
                        "Went {idle} turns without being able to do anything. {}",
                        step.say()
                    ),
                };
            }
        } else {
            idle = 0;
        }

        // The same action twice means the screen has not caught up -- a page still
        // loading, a window still opening. Re-issuing does not help and can undo
        // the first one; wait and look again instead.
        //
        // Compared by action, not by sentence. The model rewords every turn, so
        // the first version of this guard caught almost nothing: "Heading
        // straight to YouTube" and "Let's teleport straight to YouTube" were the
        // same Open, one after the other, and both fired.
        if last.as_ref().is_some_and(|l| l.same_action(&step)) {
            repeats += 1;
            eprintln!("agent#{id} turn {turn}: same action again (x{repeats}) -- waiting");
            if repeats >= SAME_ACTION_LIMIT {
                return State::Failed {
                    why: format!(
                        "Tried the same thing {repeats} times and the screen never \
                         changed. It may already be done, or I cannot see the \
                         right control."
                    ),
                };
            }
            tokio::time::sleep(SETTLE_AGAIN).await;
            continue;
        }
        repeats = 0;
        last = Some(step.clone());

        // The prompt tells it not to, but a model that answers `agent` from inside
        // an agent must not be allowed to fork one: burn the turn and look again.
        if matches!(step, crate::core::provider::Step::Agent { .. }) {
            eprintln!("agent#{id} turn {turn}: answered `agent` from inside an agent -- ignoring");
            tokio::time::sleep(BEAT).await;
            continue;
        }

        // Checked again, immediately before acting.
        //
        // The check at the top of the turn is not enough: the model call between
        // them takes two seconds, so a stop pressed during it still let one more
        // click through. From the user's side that is "Escape gave me the cursor
        // back and then it grabbed it again and carried on typing".
        if app.state::<Agents>().stopping() {
            return State::Stopped;
        }

        let outcome = match commands::perform(app, &step) {
            Ok(()) => commands::perform_async(app, &step).await,
            Err(e) => Err(e),
        };
        if let Err(e) = outcome {
            failures += 1;
            eprintln!("agent#{id} turn {turn}: could not perform it -- {e} (x{failures})");
            if failures >= FAILURE_LIMIT {
                return State::Failed { why: e.to_string() };
            }
            // Hand the failure back as something that happened, so the next turn
            // can route around it or explain it. The model cannot react to an
            // error it is never shown.
            app.state::<Nudge>().note(format!(
                "That did not work: {e}. Try another way, or say so."
            ));
            last = None;
            tokio::time::sleep(BEAT).await;
            continue;
        }
        publish(app);
        tokio::time::sleep(BEAT).await;
    }
}
