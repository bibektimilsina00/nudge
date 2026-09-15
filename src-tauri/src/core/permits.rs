//! What macOS has to let Nudge do, and whether it has.
//!
//! Nudge is an app that looks at your screen and moves your cursor, which is a
//! description of malware. Everything it does needs a grant, and the grants are
//! the first thing anybody meets -- before the product works at all, and while
//! they have no reason yet to trust it.
//!
//! Until now there was no such thing as a permission in this codebase. Each grant
//! was checked where it happened to be needed, at the moment of use, and reported
//! as a failed action: a click that returned "Clicking needs Accessibility" from
//! inside an error type, and a screenshot that was never checked at all and simply
//! came back empty. So the first time anybody learned a grant was missing was when
//! something they asked for did not happen, and the fix was a sentence telling
//! them to go and find a pane in System Settings themselves.
//!
//! Three things make that bad rather than merely unhelpful, and this module exists
//! for them:
//!
//! **A refusal is permanent.** macOS shows each prompt once. Dismiss it -- or miss
//! it, because it opened behind a full-screen window -- and it is recorded as a
//! no, and the API will never ask again. From then on the only route is the
//! Settings pane, so the app has to know the difference between "not asked" and
//! "denied" and offer a different thing for each. Offering "Allow" to somebody who
//! already refused is a button that does nothing.
//!
//! **They are not equal.** Without Accessibility and Screen Recording, Nudge
//! cannot do the one thing it is for. Without the microphone it is a worse but
//! working app. Presenting four rows as a checklist hides that, and people grant
//! the easy ones and stop.
//!
//! **Nobody grants a permission; they grant a capability.** "Accessibility" is the
//! name of a settings pane, not a reason. What belongs beside each one is what
//! Nudge cannot do without it.
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    /// Given.
    Granted,
    /// Refused, or dismissed, which macOS records as the same thing. The system
    /// will not ask again -- only the Settings pane can change it now.
    Denied,
    /// Never asked. The only state in which a prompt is still possible.
    Unasked,
}

impl State {
    pub fn granted(self) -> bool {
        self == State::Granted
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Permit {
    /// Stable id, used by the commands and the settings deep link.
    pub key: &'static str,
    /// What macOS calls it, so it can be found in the pane it lives in.
    pub name: &'static str,
    /// What Nudge cannot do without it. Written as the loss, not the grant: the
    /// question in somebody's head is "what happens if I say no", and the answer
    /// is the only thing that makes the risk worth weighing.
    pub without: &'static str,
    pub state: State,
    /// Nudge cannot do its job at all without this one.
    pub essential: bool,
}

/// Every grant, essentials first.
///
/// Ordered rather than sorted at the call site, because the order *is* the
/// message: the two that make Nudge work are above the two that make it nicer,
/// and anybody who stops reading half way has still granted the ones that matter.
pub fn all() -> Vec<Permit> {
    vec![
        Permit {
            key: "accessibility",
            name: "Accessibility",
            without: "Nudge can point at things but never click or type them.",
            state: accessibility(),
            essential: true,
        },
        Permit {
            key: "screen",
            name: "Screen Recording",
            without: "Nudge cannot see your screen, so it cannot find anything on it.",
            state: screen(),
            essential: true,
        },
        Permit {
            key: "microphone",
            name: "Microphone",
            without: "You can't talk to it. Everything else still works.",
            state: microphone(),
            essential: false,
        },
        Permit {
            key: "speech",
            name: "Speech Recognition",
            without: "Speech goes to the network instead of staying on your Mac.",
            state: speech(),
            essential: false,
        },
    ]
}

/// The essentials that are still missing. Empty means Nudge can work.
pub fn missing() -> Vec<Permit> {
    all()
        .into_iter()
        .filter(|p| p.essential && !p.state.granted())
        .collect()
}

#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;
}

/// Posting events, which is clicking and typing.
///
/// `CGPreflightPostEventAccess` rather than `AXIsProcessTrusted`: they are
/// different grants that happen to live in the same pane, and this is the one that
/// decides whether a click lands. It also answers live -- tick the box and this
/// turns true without a restart -- which is what lets the settings page notice.
#[cfg(target_os = "macos")]
fn accessibility() -> State {
    if crate::core::screen::click::may_click() {
        State::Granted
    } else {
        // CoreGraphics offers no way to tell "never asked" from "refused" here.
        // Treated as refused, which is the safe way round: it offers the Settings
        // pane, which always works, rather than a prompt that may never appear.
        State::Denied
    }
}

#[cfg(target_os = "macos")]
fn screen() -> State {
    if unsafe { CGPreflightScreenCaptureAccess() } {
        State::Granted
    } else {
        State::Denied
    }
}

#[cfg(target_os = "macos")]
fn microphone() -> State {
    use crate::core::voice::Access;
    match crate::core::voice::access() {
        Access::Granted => State::Granted,
        Access::Unasked => State::Unasked,
        _ => State::Denied,
    }
}

#[cfg(target_os = "macos")]
fn speech() -> State {
    match crate::core::voice::ear::status() {
        "granted" => State::Granted,
        "not asked yet" => State::Unasked,
        _ => State::Denied,
    }
}

#[cfg(not(target_os = "macos"))]
fn accessibility() -> State {
    State::Granted
}
#[cfg(not(target_os = "macos"))]
fn screen() -> State {
    State::Granted
}
#[cfg(not(target_os = "macos"))]
fn microphone() -> State {
    State::Granted
}
#[cfg(not(target_os = "macos"))]
fn speech() -> State {
    State::Granted
}

/// Ask the system, if asking is still possible.
///
/// Returns whether a prompt could appear at all. False means the answer is already
/// recorded and only [`open_settings`] can change it -- the caller shows a
/// different button rather than one that does nothing.
pub fn ask(key: &str) -> bool {
    #[cfg(target_os = "macos")]
    match key {
        // Both of these prompt on first call and return the standing answer
        // afterwards, so calling them a second time is harmless and silent.
        "accessibility" => return crate::core::screen::click::request_click_permission(),
        "screen" => return unsafe { CGRequestScreenCaptureAccess() },
        "microphone" => {
            if microphone() == State::Unasked {
                crate::core::voice::request_access();
                return true;
            }
            return false;
        }
        "speech" => {
            if speech() == State::Unasked {
                crate::core::voice::ear::request_access();
                return true;
            }
            return false;
        }
        _ => {}
    }
    let _ = key;
    false
}

/// Open the exact pane, rather than System Settings and good luck.
///
/// This is the only route once something has been refused, and the difference
/// between a dead end and a fix: the panes are four clicks deep and named after
/// the API rather than after anything somebody was trying to do.
pub fn open_settings(key: &str) {
    let pane = match key {
        "accessibility" => "Privacy_Accessibility",
        "screen" => "Privacy_ScreenCapture",
        "microphone" => "Privacy_Microphone",
        "speech" => "Privacy_SpeechRecognition",
        _ => "Privacy",
    };
    let _ = std::process::Command::new("open")
        .arg(format!(
            "x-apple.systempreferences:com.apple.preference.security?{pane}"
        ))
        .spawn();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_essentials_come_first() {
        // The order is the message: somebody who stops reading half way has still
        // granted the two that make Nudge work at all.
        let all = all();
        let last_essential = all.iter().rposition(|p| p.essential).unwrap();
        let first_optional = all.iter().position(|p| !p.essential).unwrap();
        assert!(last_essential < first_optional, "essentials must lead");
    }

    #[test]
    fn every_permit_has_a_pane_and_a_reason() {
        for p in all() {
            assert!(!p.without.is_empty(), "{} needs a reason", p.key);
            // A key with no pane of its own would silently open the top of
            // Privacy & Security, which is the dead end this exists to avoid.
            assert_ne!(pane_for(p.key), "Privacy", "{} has no pane", p.key);
        }
    }

    #[test]
    fn keys_are_unique() {
        let mut keys: Vec<_> = all().iter().map(|p| p.key).collect();
        keys.sort_unstable();
        let before = keys.len();
        keys.dedup();
        assert_eq!(before, keys.len());
    }

    #[test]
    fn asking_for_something_unknown_does_nothing() {
        assert!(!ask("not-a-permit"));
    }

    /// Mirrors `open_settings`, so the test above can check the mapping without
    /// launching System Settings.
    fn pane_for(key: &str) -> &'static str {
        match key {
            "accessibility" => "Privacy_Accessibility",
            "screen" => "Privacy_ScreenCapture",
            "microphone" => "Privacy_Microphone",
            "speech" => "Privacy_SpeechRecognition",
            _ => "Privacy",
        }
    }
}
