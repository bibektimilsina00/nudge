//! Typing, for goals that no amount of clicking can finish -- a search box, an
//! address bar, a filename.
//!
//! This is the sharpest thing Nudge can do. A click lands on a visible control the
//! user can see marked; typed text goes wherever focus happens to be, which might
//! be a terminal. So it is gated on the same "act for me" switch as clicking, the
//! text is bounded, and control characters are refused -- a model that emits an
//! escape sequence should not be able to drive a terminal through this.
use crate::core::click;
use crate::error::{Error, Result};
use core_graphics::event::{CGEvent, CGEventTapLocation, CGKeyCode};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

/// Long enough for a URL or a sentence, short enough that a runaway model cannot
/// paste a novel into whatever is focused.
const MAX_LEN: usize = 500;
/// `CGEventKeyboardSetUnicodeString` is unreliable past a short run, so text goes
/// in small chunks rather than one call.
const CHUNK: usize = 16;
/// Between chunks: fast enough to feel like a paste, slow enough that apps with
/// their own input handling keep up.
const PACE: std::time::Duration = std::time::Duration::from_millis(12);

const KEY_RETURN: CGKeyCode = 36;

/// Newline and tab are real keys someone might mean. Everything else in the
/// control range is an escape sequence or a terminal command in disguise.
fn is_safe(text: &str) -> bool {
    !text.is_empty()
        && text.chars().count() <= MAX_LEN
        && !text
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t')
}

fn source() -> Result<CGEventSource> {
    if !click::may_click() {
        return Err(Error::Click(
            "Typing needs Accessibility. System Settings > Privacy & Security > \
             Accessibility, add Nudge, then quit and reopen it."
                .into(),
        ));
    }
    CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| Error::Click("couldn't create an event source".into()))
}

/// Type text wherever focus currently is, optionally pressing Return after.
pub fn type_text(text: &str, submit: bool) -> Result<()> {
    if !is_safe(text) {
        return Err(Error::Click(format!("refusing to type {} characters", text.len())));
    }
    let source = source()?;

    // Unicode strings rather than key codes: key codes are keyboard-layout
    // specific, so a Dvorak or French layout would silently type something else.
    for chunk in chunks(text) {
        for down in [true, false] {
            let event = CGEvent::new_keyboard_event(source.clone(), 0, down)
                .map_err(|_| Error::Click("couldn't build a key event".into()))?;
            event.set_string(&chunk);
            event.post(CGEventTapLocation::HID);
        }
        std::thread::sleep(PACE);
    }

    if submit {
        press(&source, KEY_RETURN)?;
    }
    Ok(())
}

fn press(source: &CGEventSource, key: CGKeyCode) -> Result<()> {
    for down in [true, false] {
        let event = CGEvent::new_keyboard_event(source.clone(), key, down)
            .map_err(|_| Error::Click("couldn't build a key event".into()))?;
        event.post(CGEventTapLocation::HID);
    }
    Ok(())
}

/// Split on character boundaries, never bytes -- chunking mid-codepoint would
/// type mojibake for anyone whose language is not ASCII.
fn chunks(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    chars.chunks(CHUNK).map(|c| c.iter().collect()).collect()
}

#[cfg(test)]
mod tests {
    use super::{chunks, is_safe, CHUNK, MAX_LEN};

    #[test]
    fn accepts_ordinary_text() {
        assert!(is_safe("https://tiktok.com"));
        assert!(is_safe("hello world"));
        assert!(is_safe("line one\nline two\tcolumn"));
    }

    #[test]
    fn refuses_control_characters_and_runaway_length() {
        assert!(!is_safe(""));
        assert!(!is_safe("rm -rf /\u{1b}[A"), "escape sequences drive terminals");
        assert!(!is_safe("bell\u{7}"));
        assert!(!is_safe(&"a".repeat(MAX_LEN + 1)));
        assert!(is_safe(&"a".repeat(MAX_LEN)));
    }

    #[test]
    fn chunks_on_characters_not_bytes() {
        // Each of these is multi-byte; splitting by byte length would corrupt them.
        let text = "नमस्ते".repeat(8);
        let parts = chunks(&text);
        assert_eq!(parts.concat(), text);
        assert!(parts.iter().all(|p| p.chars().count() <= CHUNK));
    }
}
