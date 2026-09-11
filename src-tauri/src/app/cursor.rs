//! Pointer watching: where the cursor is, and when a click actually happens.
use super::state::{Mic, Screen};
use crate::core::click;
use tauri::{AppHandle, Emitter, Manager};

/// A click-through window never receives mousemove -- the OS hands those straight to
/// the app underneath -- so the companion's position has to be polled from this side.
///
/// ponytail: 60Hz poll, emitting only on movement. If this ever shows up in a battery
/// profile, drop to 30Hz or only run the loop while a session is active.
pub fn follow(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let mut last = (f64::MIN, f64::MIN);
        let mut was_down = false;
        let mut shown = true;
        let notch = super::notch::measure();
        let mut at_notch = false;
        let mut tick: u32 = 0;
        loop {
            std::thread::sleep(std::time::Duration::from_millis(16));
            tick = tick.wrapping_add(1);

            // Check the overlay's Space membership ten times a second.
            //
            // Measured, not guessed: a poll of the window server showed Nudge's
            // window ABSENT from the on-screen list the moment a full-screen Space
            // activated -- not hidden behind the app, removed from the Space. The
            // collection behaviour set at startup does not survive, and no window
            // event we can subscribe to fires on a Space change, so there is nothing
            // to hook.
            //
            // ponytail: polling instead of observing
            // NSWorkspaceActiveSpaceDidChangeNotification, which needs an
            // Objective-C observer object to carry a Rust callback. At 1Hz the
            // overlay visibly blinked out on each Space change; at 10Hz the gap is
            // under a frame or two, and the check is one getter that usually says
            // "already fine".
            if tick % 6 == 0 {
                let handle = app.clone();
                let _ = app.run_on_main_thread(move || {
                    super::overlay::keep_everywhere(&handle);
                });
            }

            // Microphone level, at half the poll rate. 30Hz is plenty for a
            // waveform and halves the IPC traffic while the key is held.
            if tick % 2 == 0 {
                if let Some((_, rec)) = app.state::<Mic>().0.lock().unwrap().as_ref() {
                    app.emit("level", rec.level()).ok();
                }
            }

            let scale = app.state::<Screen>().scale;
            let Ok(p) = app.cursor_position() else { continue };
            let (x, y) = (p.x / scale, p.y / scale);

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
            let hovering = notch.is_hovered(x, y, at_notch);
            if hovering != at_notch {
                at_notch = hovering;
                super::panel::set_interactive(&app, hovering);
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
                        for i in 0..crate::core::haptics::BURST {
                            let _ = handle.run_on_main_thread(crate::core::haptics::tick);
                            if i + 1 < crate::core::haptics::BURST {
                                std::thread::sleep(crate::core::haptics::GAP);
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
