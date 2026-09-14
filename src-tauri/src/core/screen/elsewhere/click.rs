//! The pointer, anywhere.
//!
//! `enigo` posts the events and `device_query` reads the state of the keys and
//! buttons, which are two different problems: one is "make this happen", the
//! other is "is this being held right now". The hotkey is a held modifier, so
//! the second one is load-bearing rather than a nicety.
//!
//! Compiled on macOS too, behind the `portable` feature, because code that is
//! never compiled is code that does not work. The macOS build uses the Core
//! Graphics version next door -- it can answer things this cannot, like whether
//! the pointer is hidden.
use crate::core::screen::capture::Point;
use crate::error::{Error, Result};
use device_query::{DeviceQuery, DeviceState, Keycode};
use enigo::{Button, Coordinate, Direction, Enigo, Mouse, Settings};

fn enigo() -> Result<Enigo> {
    Enigo::new(&Settings::default()).map_err(|e| Error::Click(format!("no input device: {e}")))
}

fn keys() -> Vec<Keycode> {
    DeviceState::new().get_keys()
}

/// Nothing to undo: this platform never hides the pointer, because it has no
/// way to put it back that is worth the risk of not.
pub fn show_the_pointer() {}

pub fn cursor() -> Option<Point> {
    let (x, y) = enigo().ok()?.location().ok()?;
    Some(Point {
        x: x as f64,
        y: y as f64,
    })
}

pub fn escape_down() -> bool {
    keys().contains(&Keycode::Escape)
}

/// Control held, and nothing else.
///
/// "Alone" matters: Control is half of a hundred shortcuts, and treating
/// Control-C as a request to start listening would make the app unusable rather
/// than merely annoying.
pub fn control_alone() -> bool {
    let held = keys();
    let control = held
        .iter()
        .any(|k| matches!(k, Keycode::LControl | Keycode::RControl));
    let others = held.iter().any(|k| {
        !matches!(
            k,
            Keycode::LControl | Keycode::RControl | Keycode::LShift | Keycode::RShift
        )
    });
    control && !others
}

/// Whether the pointer is hidden -- during a full-screen video, say.
///
/// No cross-platform answer exists. `false` means "assume it is visible", which
/// is the safe way to be wrong: the cost is drawing a ring nobody sees, where
/// `true` would suppress a ring somebody needed.
pub fn pointer_hidden() -> bool {
    false
}

/// Whether the system will let us post input at all.
///
/// macOS gates this behind Accessibility and answers honestly. Windows and X11
/// generally allow it, and Wayland generally does not without a portal -- so
/// this optimistically says yes and lets the attempt fail loudly instead of
/// refusing in advance for a reason that may not apply.
pub fn may_click() -> bool {
    true
}

pub fn request_click_permission() -> bool {
    true
}

pub fn left_button_down() -> bool {
    DeviceState::new().get_mouse().button_pressed.get(1).copied().unwrap_or(false)
}

pub fn move_to(at: Point) -> Result<()> {
    enigo()?
        .move_mouse(at.x as i32, at.y as i32, Coordinate::Abs)
        .map_err(|e| Error::Click(format!("could not move the pointer: {e}")))
}

pub fn click(at: Point, times: u8) -> Result<()> {
    let mut enigo = enigo()?;
    enigo
        .move_mouse(at.x as i32, at.y as i32, Coordinate::Abs)
        .map_err(|e| Error::Click(format!("could not move the pointer: {e}")))?;
    for _ in 0..times.max(1) {
        enigo
            .button(Button::Left, Direction::Click)
            .map_err(|e| Error::Click(format!("could not click: {e}")))?;
    }
    Ok(())
}
