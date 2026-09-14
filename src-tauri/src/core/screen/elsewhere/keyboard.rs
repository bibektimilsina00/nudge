//! Typing, anywhere.
//!
//! `enigo::text` is the important call and it is what makes this portable at
//! all: it types a *string*, leaving the layout problem to the platform. The
//! macOS version has to solve that itself, through `UCKeyTranslate`, and the
//! reason it does is a real bug -- a table of US key codes typed "abc;" where
//! someone meant "abcz", because the keyboard was Dvorak. Any port tempted to
//! map characters to key codes by hand should read that first.
use crate::error::{Error, Result};
use enigo::{Direction, Enigo, Key, Keyboard, Settings};

fn enigo() -> Result<Enigo> {
    Enigo::new(&Settings::default()).map_err(|e| Error::Click(format!("no input device: {e}")))
}

/// A synthesised keystroke is indistinguishable from a real one -- that is the
/// point of it -- so the watcher that stops an agent on Escape also sees Nudge's
/// own Escape and stops the agent that pressed it. One run dismissed a sign-in
/// dialog and killed itself doing it.
static OUR_ESCAPE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn now_ms() -> u64 {
    static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    START
        .get_or_init(std::time::Instant::now)
        .elapsed()
        .as_millis() as u64
}

pub fn we_pressed_escape() -> bool {
    const WINDOW_MS: u64 = 600;
    let last = OUR_ESCAPE.load(std::sync::atomic::Ordering::Relaxed);
    last != 0 && now_ms().saturating_sub(last) < WINDOW_MS
}

pub fn type_text(text: &str, submit: bool) -> Result<()> {
    let mut enigo = enigo()?;
    enigo
        .text(text)
        .map_err(|e| Error::Click(format!("could not type: {e}")))?;
    if submit {
        enigo
            .key(Key::Return, Direction::Click)
            .map_err(|e| Error::Click(format!("could not press return: {e}")))?;
    }
    Ok(())
}

/// A name from a shortcut spec, as a key.
///
/// `cmd` maps to Meta, which is Command on macOS and the Windows key elsewhere.
/// That is right for a spec written by a model thinking in Mac terms and wrong
/// for one thinking in Windows terms, and there is no way to tell which from
/// here -- so a port should expect to translate the *spec*, not this table.
fn named(word: &str) -> Option<Key> {
    Some(match word {
        "cmd" | "command" | "super" | "win" | "meta" => Key::Meta,
        "ctrl" | "control" => Key::Control,
        "shift" => Key::Shift,
        "alt" | "option" | "opt" => Key::Alt,
        "enter" | "return" => Key::Return,
        "tab" => Key::Tab,
        "esc" | "escape" => Key::Escape,
        "space" => Key::Space,
        "delete" | "backspace" => Key::Backspace,
        "up" => Key::UpArrow,
        "down" => Key::DownArrow,
        "left" => Key::LeftArrow,
        "right" => Key::RightArrow,
        "home" => Key::Home,
        "end" => Key::End,
        _ => return None,
    })
}

pub fn shortcut(spec: &str) -> Result<()> {
    let parts: Vec<&str> = spec
        .split('+')
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect();
    let Some((last, modifiers)) = parts.split_last() else {
        return Err(Error::Click(format!("empty shortcut {spec:?}")));
    };

    let held: Vec<Key> = modifiers
        .iter()
        .map(|m| {
            named(&m.to_lowercase())
                .ok_or_else(|| Error::Click(format!("unknown modifier {m:?} in {spec:?}")))
        })
        .collect::<Result<_>>()?;

    let lowered = last.to_lowercase();
    let key = match named(&lowered) {
        Some(k) => k,
        None => {
            let mut chars = lowered.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) => Key::Unicode(c),
                _ => return Err(Error::Click(format!("unknown key {last:?} in {spec:?}"))),
            }
        }
    };

    let mut enigo = enigo()?;
    // Held as real keys around the keystroke, not attached to it as flags. Menus
    // watch for the modifier going down, and one that never went down leaves
    // them unmoved.
    for m in &held {
        enigo
            .key(*m, Direction::Press)
            .map_err(|e| Error::Click(format!("could not hold {m:?}: {e}")))?;
    }
    let pressed = enigo.key(key, Direction::Click);
    for m in held.iter().rev() {
        let _ = enigo.key(*m, Direction::Release);
    }
    pressed.map_err(|e| Error::Click(format!("could not press {last:?}: {e}")))?;

    if matches!(key, Key::Escape) {
        OUR_ESCAPE.store(now_ms(), std::sync::atomic::Ordering::Relaxed);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parsing, without posting anything. The failure this catches is a spec
    /// that names a modifier we do not know, which would otherwise be typed as
    /// a literal character into whatever was focused.
    #[test]
    fn a_shortcut_spec_is_understood_before_anything_is_pressed() {
        assert!(named("cmd").is_some() && named("ctrl").is_some() && named("shift").is_some());
        assert!(named("option").is_some(), "the Mac name for Alt");
        assert!(named("banana").is_none());

        // The shapes the model actually emits.
        for spec in ["cmd+shift+n", "ctrl+c", "cmd+w", "alt+tab", "escape"] {
            let parts: Vec<&str> = spec.split('+').collect();
            let (last, mods) = parts.split_last().unwrap();
            assert!(
                mods.iter().all(|m| named(m).is_some()),
                "{spec}: a modifier was not recognised"
            );
            assert!(
                named(last).is_some() || last.chars().count() == 1,
                "{spec}: the key was neither a name nor a single character"
            );
        }
    }
}
