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
    // Not focusable while it is just a strip: a pill over the menu bar has
    // nothing to type into.
    //
    // It no longer decides anything about Spaces, and it used to say it did.
    // `native::become_panel` replaces tao's class, and with it the override that
    // answered `canBecomeKeyWindow` from this flag -- `becomesKeyOnlyIfNeeded`
    // does that job now. This is left because `set_interactive` still turns it
    // back on, and the two should agree.
    let _ = win.set_focusable(false);
    let _ = win.show();

    crate::app::ui::native::float_everywhere(&win);
}

/// Let clicks through, or not. Collapsed the pill is decoration over the menu bar
/// and must not eat clicks meant for it; open, it is a panel with buttons.
pub fn set_interactive(app: &AppHandle, on: bool) {
    if let Some(win) = window(app) {
        let _ = win.set_ignore_cursor_events(!on);
        // Focusable only while it is open, because open is the only time there
        // is anything to type into. Closed, it has to stay unfocusable or it
        // stops being present in full-screen Spaces -- see `dock_to_notch`.
        //
        // The pointer already drives this exact boundary, so there is no second
        // piece of state to keep in step with the first.
        let _ = win.set_focusable(on);
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
///
/// Quick to believe it is up, slow to believe it is over. A four-finger swipe
/// does not toggle the overview, it *scrubs* it: the gesture holds the thing
/// mid-animation for as long as the fingers are down, and any test for it can
/// come back unsure somewhere in the middle. Taken at face value ten times a
/// second, one unsure reading reopens the panel and the next closes it again,
/// which is the flicker rather than a cure for it.
///
/// So a single "up" counts, and "over" has to be said four times running --
/// four polls, a little under half a second. Nothing is lost by the delay:
/// closing promptly is what matters here, and reopening half a second after
/// somebody has already put the overview away is not something anybody waits
/// for.
///
/// The counter is plain load-then-store rather than a real atomic dance
/// because this only ever runs on the main thread, from one poll.
#[cfg(target_os = "macos")]
pub fn watch_overview(app: &AppHandle) {
    use std::sync::atomic::{AtomicU8, Ordering::Relaxed};
    /// Consecutive polls that have said the overview is gone.
    ///
    /// Two, and they are cheap now: the caller steps up to every tick while the
    /// overview is up, so a second opinion costs a sixtieth of a second rather
    /// than a fifth. The debounce is kept rather than dropped because a single
    /// wrong "over" would show both windows for a frame and hide them again,
    /// which is the flicker this whole thing exists to avoid.
    static CLEAR: AtomicU8 = AtomicU8::new(0);
    const ENOUGH: u8 = 2;

    if crate::app::ui::native::mission_control() {
        CLEAR.store(0, Relaxed);
        if !OVERVIEW.swap(true, Relaxed) {
            told(app, true);
        }
        return;
    }

    let seen = CLEAR.load(Relaxed).saturating_add(1);
    CLEAR.store(seen.min(ENOUGH), Relaxed);
    if seen >= ENOUGH && OVERVIEW.swap(false, Relaxed) {
        told(app, false);
    }
}

/// Tell the windows the overview came or went.
///
/// They draw the companion with a live WebGL canvas that animates on its own,
/// and Mission Control is already compositing every window on the machine. Two
/// more surfaces redrawing sixty times a second on top of that is work nobody
/// asked for at the worst possible moment, so they stop while it is up.
#[cfg(target_os = "macos")]
fn told(app: &AppHandle, up: bool) {
    use tauri::Emitter as _;
    app.emit("overview", up).ok();
}

#[cfg(not(target_os = "macos"))]
pub fn watch_overview(_app: &AppHandle) {}
