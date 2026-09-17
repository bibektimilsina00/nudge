//! The dropdown that hangs off the menu bar icon: settings, shortcuts, and the
//! companion's home when it is docked.
//!
//! A second window rather than more overlay: the overlay must never take focus
//! (it covers the screen, so focusing it swallows every click), while this needs
//! the keyboard and a real hit area. Two windows with one job each beats one
//! window with a mode.
use tauri::{AppHandle, LogicalSize, Manager, WebviewWindow};

/// The panel is always this size and always at the top of the screen. What opens
/// and closes is the *content* -- see notch.rs for why.
const SIZE: (f64, f64) = (560.0, 800.0);

pub fn window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window("panel")
}

/// Resize to whatever the content turned out to be, so the panel is never a box
/// with empty space at the bottom.
/// Grow the panel into a window, or put it back.
///
/// The panel hangs from the notch at a size chosen for glancing at. Some work is
/// not a glance -- reading what an agent did, going through nineteen connectors,
/// a settings page with five sections -- and for that it needs to be a window.
///
/// Sized against the screen rather than fixed, because the smallest Mac this
/// runs on and the largest differ by a factor of three, and a number that suits
/// one looks absurd on the other.
pub fn widen(app: &AppHandle, wide: bool) {
    let Some(win) = window(app) else { return };
    if !wide {
        let _ = win.set_size(LogicalSize::new(SIZE.0, SIZE.1));
        dock_to_notch(app);
        return;
    }
    let screen = win
        .current_monitor()
        .ok()
        .flatten()
        .map(|m| {
            let s = m.scale_factor();
            (m.size().width as f64 / s, m.size().height as f64 / s)
        })
        .unwrap_or((1440.0, 900.0));
    // Most of the screen, not all of it: a panel that covers everything has
    // stopped being a panel, and the thing it is for is usually behind it.
    let w = (screen.0 * 0.62).clamp(760.0, 1180.0);
    let h = (screen.1 * 0.68).clamp(480.0, 820.0);
    let _ = win.set_size(LogicalSize::new(w, h));
    let _ = win.set_position(tauri::LogicalPosition::new(
        (screen.0 - w) / 2.0,
        (screen.1 - h) / 2.5,
    ));
    let _ = win.set_ignore_cursor_events(false);
    let _ = win.show();
    let _ = win.set_focus();
}

pub fn fit(app: &AppHandle, height: f64) {
    let Some(win) = window(app) else { return };
    let _ = win.set_size(LogicalSize::new(420.0, height.clamp(160.0, 640.0)));
}

/// Park the panel over the notch and leave it there.
///
/// Level and collection behaviour match the overlay's: it has to sit above the
/// menu bar (the pill is *in* the menu bar) and follow the user between Spaces.
/// Click-through until something in it is worth clicking.
pub fn dock_to_notch(app: &AppHandle) {
    let Some(win) = window(app) else { return };
    let notch = crate::app::ui::notch::measure();
    let (w, h) = SIZE;

    let _ = win.set_size(LogicalSize::new(w, h));
    let _ = win.set_position(tauri::LogicalPosition::new(notch.center_x - w / 2.0, 0.0));
    let _ = win.set_ignore_cursor_events(true);
    let _ = win.show();

    #[cfg(target_os = "macos")]
    crate::app::ui::native::float_everywhere(&win);
}

/// Let clicks through, or not. Collapsed the pill is decoration over the menu bar
/// and must not eat clicks meant for it; open, it is a panel with buttons.
pub fn set_interactive(app: &AppHandle, on: bool) {
    if let Some(win) = window(app) {
        let _ = win.set_ignore_cursor_events(!on);
    }
}
