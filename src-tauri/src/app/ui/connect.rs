//! Asking whether to connect something, once.
//!
//! A bar that hangs from the notch, names a service, and shows a row of things
//! it would let somebody ask for.
//!
//! **The examples are the design.** *"Connect GitHub?"* is a permission request,
//! and people refuse those by reflex because there is nothing in them to weigh --
//! access to your issues, in the abstract, is only a risk. *"Summarise the open
//! PRs"* is an offer, and an offer can be judged. The row is not decoration; it
//! is the entire difference between a prompt that gets read and one that gets
//! dismissed.
//!
//! **Three answers, and the middle one matters most.** Without *Not now*, a
//! person who means "not while I am in the middle of something" has to choose
//! between yes and never -- so they pick never, and the thing they would have
//! wanted is gone. Escape is *Not now* for the same reason: the key people press
//! to make something go away must not be the answer that makes it never return.
//!
//! *When* one of these appears is not decided yet, and deliberately so -- the
//! shape is worth getting right before the timing, because the timing is easy to
//! change and the shape is what people react to.
pub use crate::core::offers::{catalogue, Offer};
use tauri::{AppHandle, Emitter, Manager};

/// Park it under the notch, the width of the notch's screen.
///
/// Wider than the panel, because the row of examples is the point: a prompt that
/// shows four things it would be for needs room for four things.
pub fn place(app: &AppHandle) {
    let Some(win) = app.get_webview_window("connect") else {
        return;
    };
    let screen = app.state::<crate::app::state::Screen>();
    // Bounded on both sides: wide enough for the examples, and never edge to edge
    // on a large display, where a bar the width of a desk reads as a system alert
    // rather than as something this program is asking.
    let w = (screen.w * 0.62).clamp(620.0, 900.0);
    // Taller than the content needs. The bar animates in from behind its own top
    // edge, so the window has to be big enough to hold it while it is still
    // partly above where it will settle -- a window cropped to the resting size
    // clips the entrance.
    let h = 210.0;
    let _ = win.set_size(tauri::LogicalSize::new(w, h));
    // Hard against the top. It hangs off the edge rather than floating below it,
    // which is what lets the top corners be square and the bottom ones round.
    let _ = win.set_position(tauri::LogicalPosition::new((screen.w - w) / 2.0, 0.0));
}

/// Put an offer on screen.
///
/// Onto the main thread, always, and the hop is inside here rather than left to
/// callers. Showing a window reaches `NSWindow`, AppKit windows may only be
/// touched from the main thread, and doing it anywhere else is not a race that
/// might bite -- it is an assertion that fires immediately. Called once from a
/// tokio worker and the process died in `_reallyDoOrderWindowAboveOrBelow`, which
/// is the same way starting an agent killed the app the first time it was tried.
pub fn ask(app: &AppHandle, offer: &Offer) {
    // Shown first, then told. The other order looked right and lost the message:
    // this window exists from startup and is only hidden, so its subscription is
    // set up long before -- but a subscription is a promise, and the gap between
    // asking to listen and listening is real. Showing it makes the document
    // visible, which the component watches for and re-asks on, so the emit is the
    // fast path and being shown is the one that cannot be missed.
    //
    // Emitted to everything rather than addressed to the window.
    //
    // `emit_to("connect", ...)` looked right and delivered nothing: the component
    // mounted at startup, asked for the pending offer, correctly got none, and
    // then never heard the event that followed. Every other cross-window message
    // here is a plain `emit` -- the agent list is broadcast the same way -- and
    // the windows that do not care simply do not listen. One channel that works
    // beats a targeted one that half does.
    let handle = app.clone();
    let told = offer.clone();
    let _ = app.run_on_main_thread(move || {
        let Some(win) = handle.get_webview_window("connect") else {
            return;
        };
        place(&handle);
        let _ = win.show();
        #[cfg(target_os = "macos")]
        crate::app::ui::native::float_everywhere(&win);
        handle.emit("offer", &told).ok();
    });
}

pub fn hide(app: &AppHandle) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(win) = handle.get_webview_window("connect") {
            let _ = win.hide();
        }
    });
}
