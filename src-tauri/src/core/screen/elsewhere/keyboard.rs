//! Typing, on a platform that has not been taught how yet.
//!
//! Harder than it looks, and the macOS version is the evidence: typing a string
//! means finding the key that produces each character *on the layout in use*,
//! which is why the real one goes through `UCKeyTranslate` rather than a table.
//! A port that hardcodes a US keyboard will type "abc;" where someone meant
//! "abcz", which is exactly what happened here on a Dvorak layout.
use crate::error::{Error, Result};

fn no() -> Error {
    Error::Click("this platform cannot type yet".into())
}

pub fn type_text(_text: &str, _submit: bool) -> Result<()> {
    Err(no())
}

/// Did *we* post that Escape, or did the user press it? Without this, our own
/// keystroke cancels the thing it was sent to do.
pub fn we_pressed_escape() -> bool {
    false
}

pub fn shortcut(_spec: &str) -> Result<()> {
    Err(no())
}
