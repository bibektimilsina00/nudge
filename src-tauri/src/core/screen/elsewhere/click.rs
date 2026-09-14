//! The pointer, on a platform that has not been taught how yet.
//!
//! Every function here is the shape a port has to fill in. `enigo` covers the
//! moving and clicking on all three platforms; the rest -- whether a modifier is
//! held, whether a button is down, whether we are allowed to post events at all
//! -- are per-platform questions with per-platform answers.
use crate::core::screen::capture::Point;
use crate::error::{Error, Result};

fn no() -> Error {
    Error::Click("this platform cannot drive the pointer yet".into())
}

pub fn cursor() -> Option<Point> {
    None
}

/// Whether a key is *currently* held, which is not the same as a key event
/// having arrived. The hotkey is a held modifier, so this is load-bearing.
pub fn escape_down() -> bool {
    false
}

pub fn control_alone() -> bool {
    false
}

pub fn pointer_hidden() -> bool {
    false
}

/// Whether the system will let us post input at all. macOS calls this
/// Accessibility; Windows has UIPI and integrity levels; X11 lets anyone, and
/// Wayland lets nobody without a portal.
pub fn may_click() -> bool {
    false
}

pub fn request_click_permission() -> bool {
    false
}

pub fn left_button_down() -> bool {
    false
}

pub fn move_to(_at: Point) -> Result<()> {
    Err(no())
}

pub fn click(_at: Point, _times: u8) -> Result<()> {
    Err(no())
}
