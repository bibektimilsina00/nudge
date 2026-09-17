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

/// Whether the system overview is up, as of the last look.
///
/// Cached because the question can only be asked from the main thread and the
/// pointer loop that needs the answer is not on it.
static OVERVIEW: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Is Mission Control on screen?
pub fn overview() -> bool {
    OVERVIEW.load(std::sync::atomic::Ordering::Relaxed)
}

/// Look, and remember. Main thread only, which is where the poll calls it.
#[cfg(target_os = "macos")]
pub fn watch_overview() {
    OVERVIEW.store(
        crate::app::ui::native::mission_control(),
        std::sync::atomic::Ordering::Relaxed,
    );
}

#[cfg(not(target_os = "macos"))]
pub fn watch_overview() {}
