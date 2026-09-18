//! The same three jobs, on a platform with no AppKit.
//!
//! Every one of them is a macOS problem with a macOS answer -- a window that
//! survives a full-screen Space, a window server asked what it is showing. Off
//! macOS the toolkit is asked politely instead and that is the whole of it:
//! `set_visible_on_all_workspaces` is the one lever a compositor exposes, and
//! there is no portable way to ask whether the desktop is mid-overview.
//!
//! Nothing here pretends to be the real thing. The alternative was `cfg` blocks
//! threaded through six call sites, and a stub that says plainly what it cannot
//! do is easier to replace when somebody ports this properly.

/// No parent window to anchor to, and nothing that evicts us from a Space.
pub fn anchor_overlay(_app: &tauri::AppHandle) -> bool {
    false
}

/// GNOME's overview and KDE's desktop grid are the equivalents, and neither is
/// readable without talking to that specific shell. Answering "no" means the
/// panel behaves during them exactly as it does the rest of the time.
// Nothing calls it here: the watcher that does is itself macOS-only, because
// there is nothing to watch. Kept so the two modules answer the same questions.
#[allow(dead_code)]
pub fn mission_control() -> bool {
    false
}

/// Every desktop, at least. Level and focus behaviour are the compositor's call
/// on Wayland, and `alwaysOnTop` in the window config is as far as it goes.
pub fn float_everywhere(win: &tauri::WebviewWindow) {
    let _ = win.set_visible_on_all_workspaces(true);
}
