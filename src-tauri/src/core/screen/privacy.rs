//! Refusing to look.
//!
//! Every step uploads a picture of the screen to somebody else's computer. Nothing
//! stopped that when a password manager or a `.env` was in front -- and during this
//! project's own development, a screenshot containing an open `.env` tab was sent
//! to a model provider. Once is a lesson; twice would be a defect.
//!
//! The check runs before the capture, not after, because there is no taking it back
//! once the bytes exist.
use crate::config::Config;

/// Applications whose entire purpose is holding secrets.
const APPS: [&str; 11] = [
    "1Password",
    "Bitwarden",
    "Keychain Access",
    "LastPass",
    "Dashlane",
    "Proton Pass",
    "Enpass",
    "KeePassXC",
    "NordPass",
    "Strongbox",
    "Authy",
];

/// Window titles that name a secret. Editors and browsers put the filename or page
/// title in the window title, which is what makes this work at all.
const TITLES: [&str; 12] = [
    ".env",
    "id_rsa",
    ".pem",
    "secret",
    "credential",
    "password",
    "passphrase",
    "private key",
    "seed phrase",
    "mnemonic",
    "api key",
    "2fa",
];

/// Why we are not looking, or `None` if there is no reason not to.
///
/// Substring and case-insensitive, and deliberately over-eager: refusing to look at
/// a web page that merely discusses passwords costs one retry, while the opposite
/// mistake cannot be undone. `privacy_guard = false` turns it off wholesale.
pub fn blocked_by(cfg: &Config, app: &str, title: &str) -> Option<String> {
    if !cfg.privacy_guard {
        return None;
    }
    let (app_l, title_l) = (app.to_lowercase(), title.to_lowercase());

    let hit_app = APPS
        .iter()
        .map(|s| s.to_string())
        .chain(cfg.blocked_apps.iter().cloned())
        .find(|needle| !needle.is_empty() && app_l.contains(&needle.to_lowercase()));
    if let Some(name) = hit_app {
        return Some(format!("not looking at {name}"));
    }

    let hit_title = TITLES
        .iter()
        .map(|s| s.to_string())
        .chain(cfg.blocked_titles.iter().cloned())
        .find(|needle| !needle.is_empty() && title_l.contains(&needle.to_lowercase()));
    hit_title.map(|needle| format!("that window looks like it holds secrets ({needle})"))
}

/// The frontmost ordinary window, as (application, title).
pub fn frontmost() -> Option<(String, String)> {
    frontmost_window().map(|(_, app, title)| (app, title))
}

/// The same window, with the process that owns it.
///
/// The pid is what the accessibility tree is asked for, and it has to be the pid
/// of *this* window -- the one the privacy check just approved and the capture is
/// about to photograph. Asking anything else which controls are on screen would
/// answer about a different screen.
///
/// Layer 0 only: menus, the Dock and our own overlay live above it, and the
/// window list is already in front-to-back order.
#[cfg(all(target_os = "macos", not(feature = "portable")))]
pub fn frontmost_window() -> Option<(i32, String, String)> {
    use core_foundation::base::{CFType, TCFType};
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::number::CFNumber;
    use core_foundation::string::CFString;
    use core_graphics::window::{
        copy_window_info, kCGNullWindowID, kCGWindowListExcludeDesktopElements,
        kCGWindowListOptionOnScreenOnly,
    };

    let windows = copy_window_info(
        kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements,
        kCGNullWindowID,
    )?;

    let get = |d: &CFDictionary<CFType, CFType>, key: &str| -> Option<CFType> {
        d.find(CFString::new(key).as_CFType()).map(|v| v.clone())
    };

    for item in windows.iter() {
        let dict = unsafe { CFDictionary::<CFType, CFType>::wrap_under_get_rule(*item as _) };

        let layer = get(&dict, "kCGWindowLayer")
            .and_then(|v| v.downcast::<CFNumber>().and_then(|n| n.to_i64()))
            .unwrap_or(-1);
        if layer != 0 {
            continue;
        }
        let owner = get(&dict, "kCGWindowOwnerName")
            .and_then(|v| v.downcast::<CFString>())
            .map(|s| s.to_string())
            .unwrap_or_default();
        // Our own overlay is not what the user is working in.
        if owner == "Nudge" {
            continue;
        }
        let title = get(&dict, "kCGWindowName")
            .and_then(|v| v.downcast::<CFString>())
            .map(|s| s.to_string())
            .unwrap_or_default();
        let pid = get(&dict, "kCGWindowOwnerPID")
            .and_then(|v| v.downcast::<CFNumber>())
            .and_then(|n| n.to_i64())
            .unwrap_or(-1) as i32;
        return Some((pid, owner, title));
    }
    None
}

/// The frontmost window, anywhere.
///
/// Not a nicety. [`blocked_by`] is given what this returns, so a platform where
/// this answers `None` is a platform where the privacy guard never fires and
/// every password manager gets photographed. It is the first thing a port must
/// make work, before anything that takes a picture.
#[cfg(any(not(target_os = "macos"), feature = "portable"))]
pub fn frontmost_window() -> Option<(i32, String, String)> {
    let w = active_win_pos_rs::get_active_window().ok()?;
    Some((w.process_id as i32, w.app_name, w.title))
}

#[cfg(test)]
mod tests {
    use super::blocked_by;
    use crate::config::Config;

    fn cfg() -> Config {
        Config::default()
    }

    #[test]
    fn blocks_password_managers_whatever_the_window_says() {
        assert!(blocked_by(&cfg(), "1Password", "").is_some());
        assert!(blocked_by(&cfg(), "Bitwarden", "Vault").is_some());
        assert!(blocked_by(&cfg(), "keychain access", "Login").is_some());
    }

    #[test]
    fn blocks_windows_that_name_a_secret() {
        // The exact case this project already got wrong, in the editor it used.
        assert!(blocked_by(&cfg(), "Code", ".env — nudge").is_some());
        assert!(blocked_by(&cfg(), "Terminal", "id_rsa").is_some());
        assert!(blocked_by(&cfg(), "Safari", "Reset your password").is_some());
    }

    #[test]
    fn allows_ordinary_work() {
        for (app, title) in [
            ("Blender", "untitled.blend"),
            ("Code", "main.rs — nudge"),
            ("Safari", "Rust documentation"),
        ] {
            assert!(blocked_by(&cfg(), app, title).is_none(), "{app}/{title}");
        }
    }

    #[test]
    fn respects_the_off_switch_and_the_extra_lists() {
        let mut off = cfg();
        off.privacy_guard = false;
        assert!(blocked_by(&off, "1Password", "").is_none());

        let mut extra = cfg();
        extra.blocked_apps.push("Banking".into());
        extra.blocked_titles.push("payslip".into());
        assert!(blocked_by(&extra, "My Banking App", "").is_some());
        assert!(blocked_by(&extra, "Preview", "payslip-2026.pdf").is_some());
    }
}
