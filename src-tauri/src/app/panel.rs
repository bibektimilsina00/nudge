//! The dropdown that hangs off the menu bar icon: settings, shortcuts, and the
//! companion's home when it is docked.
//!
//! A second window rather than more overlay: the overlay must never take focus
//! (it covers the screen, so focusing it swallows every click), while this needs
//! the keyboard and a real hit area. Two windows with one job each beats one
//! window with a mode.
use tauri::{AppHandle, LogicalSize, Manager, PhysicalPosition, WebviewWindow};

/// Gap between the menu bar and the panel, in points.
const DROP: f64 = 6.0;

pub fn window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window("panel")
}

/// Show the panel under the menu bar icon, or hide it if it is already up.
///
/// `at` is the tray icon's rectangle in physical pixels, as the tray event
/// reports it.
pub fn toggle(app: &AppHandle, at: Option<(f64, f64, f64)>) {
    let Some(win) = window(app) else {
        eprintln!("nudge: no panel window -- is it declared in tauri.conf.json?");
        return;
    };
    eprintln!("nudge: panel toggle, visible={:?} at={at:?}", win.is_visible());
    if win.is_visible().unwrap_or(false) {
        let _ = win.hide();
        return;
    }

    if let Some((icon_x, icon_bottom, icon_width)) = at {
        let scale = win.scale_factor().unwrap_or(2.0);
        let size = win.inner_size().map(|s| s.width as f64).unwrap_or(420.0 * scale);
        // Centre on the icon, then keep it fully on screen -- a menu bar item near
        // the right edge would otherwise push the panel off it.
        let mut x = icon_x + icon_width / 2.0 - size / 2.0;
        if let Ok(Some(mon)) = win.primary_monitor() {
            let right = mon.size().width as f64 - size - 8.0 * scale;
            x = x.clamp(8.0 * scale, right.max(8.0 * scale));
        }
        let _ = win.set_position(PhysicalPosition::new(x, icon_bottom + DROP * scale));
    }

    let _ = win.show();
    let _ = win.set_focus();
    eprintln!("nudge: panel shown at {:?} size {:?}", win.outer_position(), win.inner_size());
}

/// Resize to whatever the content turned out to be, so the panel is never a box
/// with empty space at the bottom.
pub fn fit(app: &AppHandle, height: f64) {
    let Some(win) = window(app) else { return };
    let _ = win.set_size(LogicalSize::new(420.0, height.clamp(160.0, 640.0)));
}
