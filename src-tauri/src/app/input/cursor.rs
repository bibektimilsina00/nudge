//! Pointer watching: where the cursor is, and when a click actually happens.
use crate::app::state::{Mic, Screen};
use crate::core::screen::click;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::ShortcutState;

/// A click-through window never receives mousemove -- the OS hands those straight to
/// the app underneath -- so the companion's position has to be polled from this side.
///
/// ponytail: 60Hz poll, emitting only on movement. If this ever shows up in a battery
/// profile, drop to 30Hz or only run the loop while a session is active.
/// How long the panel stays open after the pointer leaves, in poll ticks.
///
/// Sixteen milliseconds each, so about a fifth of a second -- long enough to
/// cover the overshoot of reaching for something in a corner, short enough that
/// deliberately moving away still feels like it closed when you left.
const LINGER: u32 = 12;

pub fn follow(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let mut last = (f64::MIN, f64::MIN);
        let mut was_down = false;
        let mut shown = true;
        let notch = crate::app::ui::notch::measure();
        let mut at_notch = false;
        // Ticks the pointer has been off the panel while it is open. Leaving is
        // not an event, it is a sustained absence -- see `LINGER`.
        let mut away: u32 = 0;
        // The hotkey is re-read every tick rather than captured here. Asked once
        // at thread start, the answer outlived every change to it: switching away
        // from a bare modifier left this loop still watching for the old one, and
        // switching to one left nothing watching at all.
        let mut ctrl_was = false;
        let mut click_was = false;
        let mut escape_was = false;
        let mut near = false;
        let mut tick: u32 = 0;
        loop {
            std::thread::sleep(std::time::Duration::from_millis(16));
            tick = tick.wrapping_add(1);

            // Ask whether Mission Control is up, ten times a second.
            //
            // This used to also put the overlay back when the window server
            // dropped it from a Space, re-applying the level and the collection
            // behaviour on every tick. That is gone: Clicky does none of it and
            // does not lose its overlay, because the behaviour set once at
            // creation is enough, and a poll that re-applies it is the only
            // thing here that could ever fight the compositor.
            //
            // ponytail: polling rather than observing
            // NSWorkspaceActiveSpaceDidChangeNotification, which would need an
            // Objective-C observer object to carry a Rust callback.
            if tick % 6 == 0 {
                // Mission Control has to be asked about from the main thread --
                // `MainThreadMarker::new()` answers `None` anywhere else and the
                // check would quietly report "not up" forever -- so the answer
                // is cached here for the loop below, which is not on it.
                let handle = app.clone();
                let _ = app.run_on_main_thread(move || {
                    crate::app::ui::panel::watch_overview(&handle);
                });
            } else if crate::app::ui::panel::overview() {
                // While the overview is up, watch for it ending on every tick
                // rather than every sixth.
                //
                // Coming back was visibly late, and all of the lateness was
                // here: at 10Hz it can take a tenth of a second to *sample* the
                // end, plus another for the second opinion, so the windows were
                // still away a third of a second after the desktop returned.
                // The poll rate was chosen for a check that runs forever; this
                // one only runs while Mission Control is open, which is a second
                // or two at a time, so it can afford sixty looks a second.
                //
                // Only the ending is hurried. Noticing the overview *start* a
                // few frames late costs nothing -- it is hidden behind the
                // opening animation either way.
                let handle = app.clone();
                let _ = app.run_on_main_thread(move || {
                    crate::app::ui::panel::watch_overview(&handle);
                });
            }

            // Microphone level, at half the poll rate. 30Hz is plenty for a
            // waveform and halves the IPC traffic while the key is held.
            if tick % 2 == 0 {
                if let Some((_, rec)) = app.state::<Mic>().0.lock().unwrap().as_ref() {
                    app.emit("level", rec.level()).ok();
                }
            }

            // A click anywhere else closes an open agent card.
            //
            // Edge-triggered: the press, not the holding of it, or a drag across
            // the desktop would fire this sixty times. The interface decides what
            // to do with it -- it is the only thing that knows whether a card is
            // open, and a window being clicked away from means nothing when all it
            // is showing is tiles.
            //
            // And not our own clicks. An agent spends its whole life clicking
            // things outside the agent card, so without this its own work closed
            // its own card while somebody was reading it -- the same failure as
            // Nudge pressing Escape and stopping itself, which is guarded one file
            // over for the same reason.
            let down = click::left_button_down();
            if down && !click_was && !click::we_clicked() {
                if let Some(at) = click::cursor() {
                    if crate::app::agent::away_from_card(&app, at) {
                        app.emit("away", ()).ok();
                    }
                }
            }
            click_was = down;

            // Escape stops whatever Nudge is doing to your machine.
            //
            // It is the key people already hit when a computer starts acting on
            // its own, so it must work from anywhere -- not only when some window
            // of ours happens to have focus, which is never.
            // Edge-triggered: the press, not the holding of it.
            //
            // Level-triggered, a key that reads as held stops every agent that
            // ever starts, at sixty times a second, and the log says "stopped by
            // Escape" each time with nothing to say it was the same press. A
            // stuck Escape is not hypothetical -- a synthetic key-down whose
            // key-up never landed did exactly this on the machine this was
            // written on, and every agent for the next hour died on arrival.
            //
            // The click watcher a few lines up is edge-triggered for the same
            // reason. One press should stop what is running, not everything that
            // starts afterwards.
            let escape = click::escape_down();
            let pressed = escape && !escape_was;
            escape_was = escape;

            if pressed {
                // Logged unconditionally. Escape is rare, one line costs
                // nothing, and "Escape did not cancel it" has two causes that
                // need different fixes -- the key not being seen, and the
                // self-press guard swallowing it. Without this line the two are
                // indistinguishable from outside.
                eprintln!(
                    "escape: pressed (ours={})",
                    crate::core::screen::keyboard::we_pressed_escape()
                );
            }
            if pressed && !crate::core::screen::keyboard::we_pressed_escape() {
                let agents = app.state::<crate::core::run::agent::Agents>();
                for a in agents.list() {
                    if !a.finished() {
                        eprintln!("agent#{} stopped by Escape", a.id);
                        agents.stop(a.id);
                    }
                }
                // And the foreground turn, wherever it has got to.
                //
                // Escape reached the agents and the overlay and stopped short of
                // the one thing most likely to be happening when somebody
                // presses it: a model call in flight. It could not be seen to do
                // anything, because "thinking" carried on and then spoke.
                //
                // Through the same command the overlay uses, so there is one
                // definition of what stopping means.
                eprintln!("escape: cancelling the foreground turn");
                crate::app::commands::cancel(app.clone(), true);
                app.emit("dismiss", ()).ok();
                app.emit("status", "idle").ok();
            }

            // Push-to-talk on a bare modifier, or several. Edge-triggered, so
            // the handler sees one press and one release exactly as the plugin
            // would deliver them -- hold-to-talk and tap-to-advance both fall
            // out unchanged.
            //
            // Exactly the named set, which is what keeps the gesture out of the
            // way of real shortcuts: holding Control and Option fires, and
            // holding either of them with anything else does not.
            if let Some(want) = crate::app::input::hotkey::bare_modifiers(
                &app.state::<crate::app::state::Hotkey>().get(),
            ) {
                let ctrl = click::modifiers_held() == want;
                // Drawing, while the key is held.
                //
                // Only while it is held: the pointer is somewhere for the whole
                // day and almost none of that is a gesture. Held, it is, and the
                // overlay draws the same points for the person to see.
                if ctrl {
                    let p = app.cursor_position().ok();
                    if let Some(p) = p {
                        crate::core::screen::ink::add(crate::core::screen::capture::Point {
                            x: p.x / app.state::<Screen>().scale,
                            y: p.y / app.state::<Screen>().scale,
                        });
                    }
                }
                if std::env::var_os("NUDGE_DEBUG_HOTKEY").is_some() && tick % 30 == 0 {
                    eprintln!(
                        "hotkey: want {want:?} held {:?} match {ctrl}",
                        click::modifiers_held()
                    );
                }
                if ctrl != ctrl_was {
                    ctrl_was = ctrl;
                    // Said out loud, because the overlay cannot work it out.
                    //
                    // It draws the ink, and it was deciding when to draw from
                    // the phase it happens to be in -- which says "listening"
                    // from the moment the key goes down until the *answer*
                    // arrives, because nothing tells it otherwise. So the pen
                    // stayed down after the key came up, and kept drawing while
                    // the model was thinking. This is the edge itself, which is
                    // the only thing that actually knows.
                    app.emit("drawing", ctrl).ok();

                    let state = if ctrl {
                        // A fresh gesture. Whatever was drawn for the last turn
                        // is not part of this one.
                        crate::core::screen::ink::clear();
                        ShortcutState::Pressed
                    } else {
                        ShortcutState::Released
                    };
                    crate::app::input::hotkey::on_key(&app, state);
                }
            }

            let screen = app.state::<Screen>();
            let Ok(p) = app.cursor_position() else {
                continue;
            };
            // Global points into the overlay window's own space. Identical on a
            // single display, where the union of screens starts at the origin.
            let local = screen.to_overlay(crate::core::screen::capture::Point {
                x: p.x / screen.scale,
                y: p.y / screen.scale,
            });
            let (x, y) = (local.x, local.y);

            // Release, not press: a click is only finished when the button comes
            // back up, and reporting the press would fire mid-drag too.
            let down = click::left_button_down();
            if was_down && !down {
                app.emit("click", [x, y]).ok();
            }
            was_down = down;

            // Pointing at the notch opens the dock. Polling rather than a tracking
            // area, because the window is click-through when closed and therefore
            // never sees a mouse event of its own.
            // Arriving is instant; leaving waits a moment.
            //
            // A pointer on its way to the gear in the corner clips the edge of the
            // panel, and with the close on the very next tick that shut the sheet
            // out from under the button being aimed at -- reported as "sometimes I
            // can't click those things", which is exactly what an intermittent
            // one-frame excursion feels like.
            //
            // A wider region alone does not fix it: wherever the boundary is, the
            // pointer can cross it, and the cost of crossing must not be losing
            // the window. So closing needs the pointer to be away and *stay* away.
            // Opening keeps no such delay -- a dock that hesitates before opening
            // feels broken, while one that hesitates before closing feels patient.
            // The hint, on a wider ring than the dock. Emitted separately and
            // only on change -- this runs sixty times a second, and an event per
            // tick would be sixty React renders a second to say nothing new.
            // Asked once per tick and used by both tests below.
            let overview = crate::app::ui::panel::overview();

            // Nothing is near the notch while the overview is up.
            //
            // This is the flicker, and it was never the window stack: Mission
            // Control puts its Spaces Bar along the top of the screen, which is
            // the same band the hint watches, so every drift of the pointer
            // across those thumbnails crossed the boundary and the strip blinked
            // "Hold control to ask" on and off. A recording of it showed 172
            // changes in fifteen seconds, at irregular gaps of 8ms to 300ms --
            // pointer-shaped, not poll-shaped, which is what ruled out every
            // theory about polls fighting the compositor.
            //
            // The offer is wrong there regardless of how it looks. Holding the
            // key during the overview does nothing, so advertising it is an
            // invitation to press something that cannot work.
            let close = notch.is_near(x, y) && !overview;
            if close != near {
                near = close;
                app.emit("near", close).ok();
            }

            let over = notch.is_hovered(x, y, at_notch);
            if over {
                away = 0;
            } else if at_notch {
                away += 1;
            }
            // Mission Control counts as the pointer being elsewhere.
            //
            // The strip stays: collapsed it is the notch, and the notch belongs
            // on screen during an overview of the screen. What does not belong
            // is a 540-point panel hanging off the bottom of it, over a view
            // that is about every window except ours.
            //
            // Closed this way rather than by hiding the window, so that nothing
            // is taken down and nothing has to be put back: coming out of the
            // overview, the strip is already there, and if the pointer is still
            // at the notch the next tick opens it again by itself. Routing it
            // through `hovering` is also what keeps `at_notch` honest -- closing
            // the panel behind the loop's back would leave it believing the
            // panel was open, and the next real hover would then change nothing.
            let hovering = (over || (at_notch && away < LINGER)) && !overview;

            if hovering != at_notch {
                at_notch = hovering;
                crate::app::ui::panel::set_interactive(&app, hovering);
                app.emit("notch", hovering).ok();
                if hovering {
                    // Arriving only. A tick on the way out would make leaving feel
                    // like an action, and leaving is just moving on.
                    //
                    // The burst is spaced from a throwaway thread rather than by
                    // sleeping between ticks on the main thread, which would stall
                    // the UI for the length of the thunk.
                    let handle = app.clone();
                    std::thread::spawn(move || {
                        for i in 0..crate::core::screen::haptics::BURST {
                            let _ = handle.run_on_main_thread(crate::core::screen::haptics::tick);
                            if i + 1 < crate::core::screen::haptics::BURST {
                                std::thread::sleep(crate::core::screen::haptics::GAP);
                            }
                        }
                    });
                }
            }

            let moved = (x - last.0).abs() >= 0.5 || (y - last.1).abs() >= 0.5;

            // The companion belongs to the pointer, so it goes when the pointer
            // goes -- but *only* then. An earlier version faded it after a few
            // seconds of stillness, which caught the typing case by accident and
            // also hid it whenever anyone paused to read.
            let visible = !click::pointer_hidden();
            if visible != shown {
                shown = visible;
                app.emit("cursor-visible", visible).ok();
            }

            if !moved {
                continue;
            }
            last = (x, y);
            app.emit("cursor", [x, y]).ok();
        }
    });
}
