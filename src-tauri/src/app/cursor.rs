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
        let mut tick: u32 = 0;
        loop {
            std::thread::sleep(std::time::Duration::from_millis(16));
            tick = tick.wrapping_add(1);

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

            if (x - last.0).abs() < 0.5 && (y - last.1).abs() < 0.5 {
                continue;
            }
            last = (x, y);
            app.emit("cursor", [x, y]).ok();
        }
    });
}
