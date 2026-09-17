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

/// Get out of the way while Mission Control is up.
///
/// The panel sits above the menu bar and follows you between Spaces, which is
/// exactly the combination that makes it outstay its welcome: a four-finger
/// swipe hands the whole screen to a system overview and the pill stays pinned
/// over the top of it, belonging to nothing on screen.
///
/// Hidden rather than lowered. Dropping the window level would put it under
/// Mission Control and also under everything else for as long as the overview
/// lasts, and getting the level back afterwards is the exchange the overlay
/// already learned shows up as a flash.
///
/// Edge-triggered: `hide` and `show` are cheap but not free, and this is asked
/// ten times a second. Only a change does anything.
///
/// It deliberately does not touch the open or closed state of the contents.
/// That belongs to the pointer loop, which owns `at_notch`; collapsing the
/// panel from here would leave the loop believing it is still open, and the
/// next hover would then be a no-op because nothing appeared to change.
#[cfg(target_os = "macos")]
pub fn yield_to_overview(app: &AppHandle) {
    use std::sync::atomic::{AtomicBool, Ordering};
    static HIDDEN: AtomicBool = AtomicBool::new(false);

    let overview = crate::app::ui::native::mission_control();
    if overview == HIDDEN.load(Ordering::Relaxed) {
        return;
    }
    HIDDEN.store(overview, Ordering::Relaxed);

    let Some(win) = window(app) else { return };
    if overview {
        let _ = win.hide();
        return;
    }
    let _ = win.show();
    // Coming back is not just becoming visible again. A window that has been
    // out has to be told once more that it belongs on every Space and above the
    // menu bar, for the same reason `keep_everywhere` exists.
    crate::app::ui::native::float_everywhere(&win);
}

#[cfg(not(target_os = "macos"))]
pub fn yield_to_overview(_app: &AppHandle) {}
