//! Typing, for goals that no amount of clicking can finish -- a search box, an
//! address bar, a filename.
//!
//! This is the sharpest thing Nudge can do. A click lands on a visible control the
//! user can see marked; typed text goes wherever focus happens to be, which might
//! be a terminal. So it is gated on the same "act for me" switch as clicking, the
//! text is bounded, and control characters are refused -- a model that emits an
//! escape sequence should not be able to drive a terminal through this.
use crate::core::screen::click;
use crate::error::{Error, Result};
use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation, CGKeyCode};
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
/// Between the last character and the Return that submits it. Long enough for a
/// JavaScript-driven input to register what was typed, short enough to feel like
/// one action.
const BEFORE_SUBMIT: std::time::Duration = std::time::Duration::from_millis(180);

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
        return Err(Error::Click(format!(
            "refusing to type {} characters",
            text.len()
        )));
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
        // Let the field catch up before sending.
        //
        // Return used to follow the last character by 12ms, and a web app that
        // manages its own composer -- a chat box, a search field -- had not
        // processed the input yet, so the Return arrived at what it still
        // considered an empty field and did nothing. Measured on WhatsApp Web:
        // the text was typed, `submit` was set, and the message just sat there
        // until the next turn pressed Return again.
        std::thread::sleep(BEFORE_SUBMIT);
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
    use super::chunks;
    use super::is_safe;
    use super::CHUNK;
    use super::MAX_LEN;

    #[test]
    fn accepts_ordinary_text() {
        assert!(is_safe("https://tiktok.com"));
        assert!(is_safe("hello world"));
        assert!(is_safe("line one\nline two\tcolumn"));
    }

    #[test]
    fn refuses_control_characters_and_runaway_length() {
        assert!(!is_safe(""));
        assert!(
            !is_safe("rm -rf /\u{1b}[A"),
            "escape sequences drive terminals"
        );
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

// ---------------------------------------------------------------------------

/// Press a keyboard shortcut: modifiers held, one key struck, modifiers released.
///
/// Menus are the least reliable way to drive a Mac app and shortcuts are the
/// most. macOS menus run a nested tracking loop, and a synthetic click into an
/// open menu often closes it without selecting anything -- one test run spent
/// fifteen turns clicking "New Private Window" while the menu obediently opened
/// and shut. The same command is one keystroke away, and the menu prints the
/// shortcut next to the item, so the model can simply read it off the screen.
///
/// Accepts what a menu prints (`⇧⌘N`) and what a person types (`cmd+shift+n`).
/// When Nudge last pressed Escape itself, in seconds since the process began.
///
/// A synthesised keystroke is indistinguishable from a real one -- that is the
/// point of it -- so the watcher that stops an agent on Escape also saw Nudge's
/// own Escape and stopped the agent that pressed it. One run dismissed a sign-in
/// dialog and killed itself doing it.
static OUR_ESCAPE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn now_ms() -> u64 {
    static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    START
        .get_or_init(std::time::Instant::now)
        .elapsed()
        .as_millis() as u64
}

/// Did Nudge press Escape in the last moment?
///
/// Generous enough to cover the key being held and released plus the poll that
/// notices it, short enough that a real Escape a beat later still stops things.
pub fn we_pressed_escape() -> bool {
    const WINDOW_MS: u64 = 600;
    let last = OUR_ESCAPE.load(std::sync::atomic::Ordering::Relaxed);
    last != 0 && now_ms().saturating_sub(last) < WINDOW_MS
}

pub fn shortcut(spec: &str) -> Result<()> {
    let (flags, key) = parse(spec)?;
    if !click::may_click() {
        return Err(Error::Click(
            "Pressing keys needs Accessibility. System Settings > Privacy & Security > \
             Accessibility, add Nudge, then quit and reopen it."
                .into(),
        ));
    }
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| Error::Click("couldn't create an event source".into()))?;

    let tap = |code: CGKeyCode, down: bool, flags: CGEventFlags| -> Result<()> {
        let e = CGEvent::new_keyboard_event(source.clone(), code, down)
            .map_err(|_| Error::Click("couldn't create a key event".into()))?;
        e.set_flags(flags);
        e.post(CGEventTapLocation::HID);
        Ok(())
    };

    // Press the modifiers as real keys, not just as flags on the keystroke.
    //
    // Setting flags alone is enough for some apps and silently ignored by
    // others: Safari took four identical cmd+shift+n presses without opening
    // anything. Real hardware sends a key event for Command itself and the
    // flags accumulate as it goes, so that is what this does -- down in order,
    // the key, then up in reverse, each event carrying the flags in force at
    // that moment.
    let mods: Vec<(CGEventFlags, CGKeyCode)> = [
        (CGEventFlags::CGEventFlagCommand, 55),
        (CGEventFlags::CGEventFlagShift, 56),
        (CGEventFlags::CGEventFlagAlternate, 58),
        (CGEventFlags::CGEventFlagControl, 59),
        (CGEventFlags::CGEventFlagSecondaryFn, 63),
    ]
    .into_iter()
    .filter(|(f, _)| flags.contains(*f))
    .collect();

    let mut sofar = CGEventFlags::empty();
    for (flag, code) in &mods {
        sofar |= *flag;
        tap(*code, true, sofar)?;
    }
    std::thread::sleep(MOD_SETTLE);

    // Recorded before posting, so the watcher cannot sample between the two.
    if key == 53 {
        OUR_ESCAPE.store(now_ms().max(1), std::sync::atomic::Ordering::Relaxed);
    }
    tap(key, true, flags)?;
    std::thread::sleep(PRESS);
    tap(key, false, flags)?;

    for (flag, code) in mods.iter().rev() {
        sofar &= !*flag;
        tap(*code, false, sofar)?;
    }
    Ok(())
}

/// After the modifiers are down, before the key. Apps that track modifier state
/// separately from event flags need a moment to notice.
const MOD_SETTLE: std::time::Duration = std::time::Duration::from_millis(18);

/// How long the key stays down. A zero-length press is dropped by some apps.
const PRESS: std::time::Duration = std::time::Duration::from_millis(40);

/// Split a shortcut into modifier flags and a key code.
///
/// Returns an error rather than guessing. A shortcut we cannot spell is worth a
/// visible failure -- pressing the wrong key in someone's editor is not.
fn parse(spec: &str) -> Result<(CGEventFlags, CGKeyCode)> {
    let mut flags = CGEventFlags::empty();
    let mut key: Option<(CGKeyCode, Kind)> = None;

    // Symbols run together (⇧⌘N), words are separated (cmd+shift+n). Split on
    // the separator only when there is one, or "f12" becomes f, 1, 2.
    let lower = spec.trim().to_lowercase();
    let parts: Vec<String> = if lower.contains('+') || (lower.contains('-') && lower.len() > 1) {
        lower
            .split(|c| c == '+' || c == '-')
            .map(|s| s.trim().to_string())
            .collect()
    } else {
        // Run-together symbols, but a multi-character word stays whole.
        let syms = "⌘⇧⌥⌃⎋⇥⏎↑↓←→";
        if lower.chars().any(|c| syms.contains(c)) {
            lower.chars().map(|c| c.to_string()).collect()
        } else {
            vec![lower.clone()]
        }
    };

    for p in parts.iter().filter(|p| !p.is_empty()) {
        match p.as_str() {
            "cmd" | "command" | "⌘" => flags |= CGEventFlags::CGEventFlagCommand,
            "shift" | "⇧" => flags |= CGEventFlags::CGEventFlagShift,
            "alt" | "option" | "opt" | "⌥" => flags |= CGEventFlags::CGEventFlagAlternate,
            "ctrl" | "control" | "⌃" => flags |= CGEventFlags::CGEventFlagControl,
            "fn" | "function" => flags |= CGEventFlags::CGEventFlagSecondaryFn,
            other => {
                if key.is_some() {
                    return Err(Error::Click(format!("{spec:?} names two keys")));
                }
                key = Some(key_code(other).ok_or_else(|| {
                    Error::Click(format!("don't know the key {other:?} in {spec:?}"))
                })?);
            }
        }
    }

    let (code, kind) =
        key.ok_or_else(|| Error::Click(format!("{spec:?} has no key, only modifiers")))?;

    // A bare letter or digit is typing, and typing has its own guarded path --
    // bounded length, no control characters. Routing text through here would go
    // around all of it. Named keys are different: Escape closes a dialog and
    // Down moves in a menu, and neither is something you could have typed.
    if flags.is_empty() && kind == Kind::Character {
        return Err(Error::Click(format!(
            "{spec:?} has no modifier -- use type for plain text"
        )));
    }
    Ok((flags, code))
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum Kind {
    /// A letter, digit or punctuation mark. Needs a modifier to be a shortcut.
    Character,
    /// A key with no character of its own. Meaningful pressed alone.
    Named,
}

/// Every key worth naming. Characters are resolved through the keyboard layout
/// in use; keys with no character of their own are fixed by the hardware.
/// Anything outside this fails loudly rather than landing somewhere unintended.
fn key_code(k: &str) -> Option<(CGKeyCode, Kind)> {
    /// Words for marks a model might spell out rather than type.
    const WORDS: [(&str, char); 8] = [
        ("minus", '-'),
        ("equal", '='),
        ("plus", '='),
        ("semicolon", ';'),
        ("quote", '\''),
        ("comma", ','),
        ("period", '.'),
        ("slash", '/'),
    ];
    const NAMED: [(&str, CGKeyCode); 24] = [
        ("return", 36),
        ("enter", 36),
        ("⏎", 36),
        ("tab", 48),
        ("⇥", 48),
        ("space", 49),
        ("spacebar", 49),
        ("delete", 51),
        ("backspace", 51),
        ("forwarddelete", 117),
        ("escape", 53),
        ("esc", 53),
        ("⎋", 53),
        ("left", 123),
        ("←", 123),
        ("right", 124),
        ("→", 124),
        ("down", 125),
        ("↓", 125),
        ("up", 126),
        ("↑", 126),
        ("home", 115),
        ("end", 119),
        ("pageup", 116),
    ];
    const MORE_NAMED: [(&str, CGKeyCode); 21] = [
        ("pagedown", 121),
        ("f1", 122),
        ("f2", 120),
        ("f3", 99),
        ("f4", 118),
        ("f5", 96),
        ("f6", 97),
        ("f7", 98),
        ("f8", 100),
        ("f9", 101),
        ("f10", 109),
        ("f11", 103),
        ("f12", 111),
        ("f13", 105),
        ("f14", 107),
        ("f15", 113),
        ("f16", 106),
        ("f17", 64),
        ("f18", 79),
        ("f19", 80),
        ("f20", 90),
    ];

    // A named key first: "delete" must not be read as the letter d.
    if let Some((_, code)) = NAMED
        .iter()
        .chain(MORE_NAMED.iter())
        .find(|(name, _)| *name == k)
    {
        return Some((*code, Kind::Named));
    }
    if let Some((_, ch)) = WORDS.iter().find(|(name, _)| *name == k) {
        return key_code_for(*ch).map(|c| (c, Kind::Character));
    }
    let mut chars = k.chars();
    match (chars.next(), chars.next()) {
        (Some(ch), None) => key_code_for(ch).map(|c| (c, Kind::Character)),
        _ => None,
    }
}

// ---------------------------------------------------------------------------

/// Which physical key produces this character on the layout in use *right now*?
///
/// Not every keyboard is QWERTY. A hardcoded table sent key code 45 for "n",
/// which on this machine's Dvorak layout is the B key -- so Safari received
/// shift-command-B and ignored it, four times, while the model correctly kept
/// asking for the right shortcut.
///
/// So the table is built the only way that can be right: ask the current layout
/// what each key types, and look the character up in the answer. Colemak,
/// AZERTY, QWERTZ and every non-Latin layout come out correct for free.
///
/// Key codes for keys with no character -- escape, return, the arrows, the
/// function keys -- are fixed by the hardware and stay in the table above.
fn key_code_for(ch: char) -> Option<CGKeyCode> {
    static MAP: std::sync::OnceLock<std::collections::HashMap<char, CGKeyCode>> =
        std::sync::OnceLock::new();
    MAP.get_or_init(build_layout_map).get(&ch).copied()
}

fn build_layout_map() -> std::collections::HashMap<char, CGKeyCode> {
    use core_foundation::base::TCFType;
    use core_foundation::data::CFData;

    let mut map = std::collections::HashMap::new();
    let layout = unsafe {
        let source = TISCopyCurrentKeyboardLayoutInputSource();
        if source.is_null() {
            return map;
        }
        let data = TISGetInputSourceProperty(source, kTISPropertyUnicodeKeyLayoutData);
        if data.is_null() {
            return map;
        }
        CFData::wrap_under_get_rule(data as _)
    };

    // 0..128 covers every key a keyboard reports. Only the first character
    // matters; the codes we want all produce exactly one.
    for code in 0u16..128 {
        let mut dead: u32 = 0;
        let mut out = [0u16; 8];
        let mut len: usize = 0;
        let ok = unsafe {
            UCKeyTranslate(
                layout.bytes().as_ptr(),
                code,
                K_ACTION_DISPLAY,
                0, // no modifiers: we want the unshifted character
                LMGetKbdType() as u32,
                K_NO_DEAD_KEYS,
                &mut dead,
                out.len(),
                &mut len,
                out.as_mut_ptr(),
            )
        };
        if ok != 0 || len != 1 {
            continue;
        }
        if let Some(ch) = char::from_u32(out[0] as u32).filter(|c| !c.is_control()) {
            // First key wins: a character on two keys (a numpad digit) should
            // resolve to the main one, which comes first.
            map.entry(ch).or_insert(code);
        }
    }
    map
}

/// `kUCKeyActionDisplay` -- what the key shows, rather than a press in progress.
const K_ACTION_DISPLAY: u16 = 3;
/// `kUCKeyTranslateNoDeadKeysMask` -- resolve accents to a character instead of
/// leaving a dead key pending.
const K_NO_DEAD_KEYS: u32 = 1;

#[link(name = "Carbon", kind = "framework")]
extern "C" {
    fn TISCopyCurrentKeyboardLayoutInputSource() -> *mut std::ffi::c_void;
    fn TISGetInputSourceProperty(
        source: *mut std::ffi::c_void,
        key: *const std::ffi::c_void,
    ) -> *mut std::ffi::c_void;
    static kTISPropertyUnicodeKeyLayoutData: *const std::ffi::c_void;
    fn LMGetKbdType() -> u8;
    #[allow(clippy::too_many_arguments)]
    fn UCKeyTranslate(
        layout: *const u8,
        code: u16,
        action: u16,
        modifiers: u32,
        keyboard_type: u32,
        options: u32,
        dead_state: *mut u32,
        max_len: usize,
        actual_len: *mut usize,
        unicode: *mut u16,
    ) -> i32;
}

#[cfg(test)]
mod shortcut_tests {
    use super::*;

    #[test]
    fn a_menu_shortcut_and_a_typed_one_mean_the_same_thing() {
        // ⇧⌘N is what Safari's File menu actually prints next to New Private Window.
        let (f1, k1) = parse("⇧⌘N").unwrap();
        let (f2, k2) = parse("cmd+shift+n").unwrap();
        assert_eq!(k1, k2, "same key");
        assert_eq!(f1, f2, "same modifiers");
        // Not a literal code: which key types "n" depends on the layout.
        assert_eq!(k1, key_code_for('n').unwrap(), "the key that types n here");
        assert!(f1.contains(CGEventFlags::CGEventFlagCommand));
        assert!(f1.contains(CGEventFlags::CGEventFlagShift));
    }

    /// Every one of these would otherwise press *something*, in whatever happens
    /// to be focused. Failing loudly is the only safe answer.
    #[test]
    fn anything_we_cannot_spell_exactly_is_refused() {
        assert!(parse("cmd").is_err(), "modifiers with no key");
        assert!(parse("n").is_err(), "no modifier -- that is typing");
        assert!(parse("cmd+shift+florp").is_err(), "unknown key name");
        assert!(parse("cmd+n+m").is_err(), "two keys");
    }

    /// Keys with no character of their own are meaningful pressed alone --
    /// Escape closes a dialog, Down moves inside a menu -- and neither is
    /// something the user could have meant as typing.
    #[test]
    fn named_keys_work_without_a_modifier_and_letters_do_not() {
        assert_eq!(parse("escape").unwrap().1, 53);
        assert_eq!(parse("down").unwrap().1, 125);
        assert_eq!(parse("return").unwrap().1, 36);
        assert_eq!(parse("f11").unwrap().1, 103, "not f, 1, 1");
        assert_eq!(parse("f5").unwrap().1, 96);
        assert!(parse("q").is_err(), "a bare letter is still typing");
        assert!(parse("7").is_err(), "and so is a bare digit");
    }

    /// Does a synthesised shortcut actually reach an application?
    ///
    /// Parsing being right proves nothing -- the first version of `shortcut`
    /// parsed perfectly and Safari ignored four identical cmd+shift+n presses,
    /// because flags on the keystroke are not the same as the modifier keys
    /// being down. Only a real app can answer that, so this drives Safari and
    /// counts its windows.
    ///
    /// Ignored by default: it opens a window on the machine running the tests.
    ///
    ///     cargo test reaches_a_real_application -- --ignored --nocapture
    #[test]
    #[ignore]
    fn reaches_a_real_application() {
        use std::process::Command;
        let count = || -> i32 {
            let out = Command::new("osascript")
                .args(["-e", "tell application \"Safari\" to count windows"])
                .output()
                .expect("osascript");
            String::from_utf8_lossy(&out.stdout)
                .trim()
                .parse()
                .unwrap_or(-1)
        };

        assert!(
            click::may_click(),
            "this binary has no Accessibility permission"
        );
        Command::new("open")
            .args(["-a", "Safari"])
            .status()
            .unwrap();
        std::thread::sleep(std::time::Duration::from_secs(2));

        let before = count();
        shortcut("cmd+shift+n").expect("posting the shortcut");
        std::thread::sleep(std::time::Duration::from_secs(2));
        let after = count();

        eprintln!("Safari windows: {before} -> {after}");
        assert!(after > before, "cmd+shift+n did not open a private window");
    }

    /// The bug this exists for: a hardcoded ANSI table sent key code 45 for "n",
    /// which on a Dvorak layout is the B key. Safari received shift-command-B
    /// four times and ignored it, while the model kept correctly asking for
    /// shift-command-N.
    ///
    /// Asserts the round trip rather than a number, because the right number
    /// depends on the layout of whoever runs the tests.
    #[test]
    fn key_codes_come_from_the_layout_in_use_not_from_qwerty() {
        let n = key_code_for('n').expect("every layout types n somewhere");
        let b = key_code_for('b').expect("and b");
        assert_ne!(n, b, "two letters cannot share one key");

        // Whatever code we chose must be the one that types that letter here.
        assert_eq!(parse("cmd+n").unwrap().1, n);
        assert_eq!(parse("cmd+b").unwrap().1, b);

        // Keys with no character are hardware-fixed and must not go near the map.
        assert_eq!(parse("escape").unwrap().1, 53);
        assert_eq!(parse("f5").unwrap().1, 96);
        // "delete" is a key, not the letter d.
        assert_eq!(parse("delete").unwrap().1, 51);
    }

    /// Does the watcher actually see a physical Escape?
    ///
    ///     cargo test escape_is_seen -- --ignored --nocapture
    ///
    /// Then press Escape. Reports what both halves of the check say, because
    /// "Escape stopped working" has two possible causes -- the key not being
    /// detected, and the self-press guard swallowing it -- and they need
    /// different fixes.
    #[test]
    #[ignore]
    fn escape_is_seen() {
        eprintln!("press Escape within 4 seconds...");
        let began = std::time::Instant::now();
        let mut seen = false;
        while began.elapsed() < std::time::Duration::from_secs(4) {
            if click::escape_down() {
                seen = true;
                eprintln!(
                    "escape_down = true, we_pressed_escape = {}",
                    we_pressed_escape()
                );
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
        assert!(seen, "never saw Escape go down at all");
    }

    /// A model writes shortcuts however the app printed them.
    #[test]
    fn the_many_ways_a_shortcut_gets_written_all_land_on_one_key() {
        let n = |s| parse(s).unwrap();
        assert_eq!(n("cmd+,"), n("⌘,"), "comma as a mark");
        assert_eq!(n("cmd+comma"), n("⌘,"), "and spelled out");
        assert_eq!(n("CMD+SHIFT+N"), n("cmd+shift+n"), "case");
        assert_eq!(n(" cmd + n "), n("cmd+n"), "loose spacing");
        assert_eq!(n("command+option+escape"), n("⌘⌥⎋"), "symbols run together");
        // The separator split must not carve up a word that contains one.
        assert_eq!(
            parse("cmd+minus").unwrap().1,
            key_code_for('-').unwrap(),
            "minus is a key name, not a separator"
        );
    }
}
