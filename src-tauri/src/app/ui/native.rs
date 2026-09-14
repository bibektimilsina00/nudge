//! Our own overlay window, built from scratch rather than configured.
//!
//! A window that floats over everything, never takes focus and survives a
//! full-screen Space is not a window with unusual settings -- it is a different
//! window. tao decides `styleMask` and `canBecomeKey` when it creates its own,
//! and those two are exactly what have to be different, so no amount of
//! configuring afterwards gets there.
//!
//! So this builds an `NSWindow` directly: borderless, screen-saver level,
//! non-activating, joining every Space, with `canBecomeKey` false. What was
//! measured to get here is in the comments below -- the window server reported
//! our overlay ABSENT from the on-screen list the moment a full-screen Space
//! activated, and a window the system considers focusable is one it considers
//! part of a Space.
use objc2::rc::Retained;
use objc2::MainThreadOnly;
use objc2_app_kit::{
    NSBackingStoreType, NSColor, NSScreenSaverWindowLevel, NSWindow, NSWindowCollectionBehavior,
    NSWindowOrderingMode, NSWindowStyleMask,
};
use objc2_foundation::MainThreadMarker;
use objc2_foundation::{NSPoint, NSRect, NSSize};
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

/// Keeps our window alive. An `NSWindow` we drop is an `NSWindow` that disappears.
static OVERLAY: Mutex<Option<usize>> = Mutex::new(None);

/// Anchor tao's overlay window to one of ours, so it inherits its Spaces.
///
/// Measured, both ways round: a window we build ourselves stays in the window
/// server's list continuously, including inside a full-screen Space, while tao's
/// is evicted the moment one activates. Same process, same flags, same moment --
/// the window is the only difference.
///
/// Moving the webview into our window was the obvious fix and it does not work:
/// the transplanted WKWebView never redraws, the window never earns a backing
/// store, and it stops registering anywhere at all.
///
/// So the webview stays exactly where it is and our window becomes its *parent*.
/// A child window follows its parent between Spaces, which is the one thing tao's
/// window could not do on its own.
pub fn anchor_overlay(app: &AppHandle) -> bool {
    let Some(mtm) = MainThreadMarker::new() else {
        return false;
    };
    let Some(tauri_win) = app.get_webview_window("overlay") else {
        return false;
    };
    let Ok(ptr) = tauri_win.ns_window() else {
        return false;
    };
    if ptr.is_null() {
        return false;
    }
    let child: &NSWindow = unsafe { &*(ptr as *const NSWindow) };

    // One pixel, in the corner, almost transparent. It has to *draw* something --
    // a window that never draws gets no backing store and the window server does
    // not track it, which would defeat the whole purpose -- but nobody needs to
    // see it.
    let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1.0, 1.0));
    let anchor = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            NSWindow::alloc(mtm),
            frame,
            NSWindowStyleMask::Borderless,
            NSBackingStoreType::Buffered,
            false,
        )
    };
    anchor.setOpaque(true);
    anchor.setBackgroundColor(Some(&NSColor::blackColor()));
    anchor.setAlphaValue(0.004);
    anchor.setLevel(NSScreenSaverWindowLevel);
    anchor.setIgnoresMouseEvents(true);
    anchor.setCollectionBehavior(
        NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::Stationary
            | NSWindowCollectionBehavior::FullScreenAuxiliary,
    );
    anchor.setHidesOnDeactivate(false);
    anchor.setHasShadow(false);
    unsafe { anchor.setReleasedWhenClosed(false) };
    anchor.orderFrontRegardless();

    // Ordered above, so the overlay is never hidden behind its own anchor.
    unsafe { anchor.addChildWindow_ordered(child, NSWindowOrderingMode::Above) };

    let raw = Retained::into_raw(anchor) as usize;
    *OVERLAY.lock().unwrap() = Some(raw);
    true
}

/// Our window, if we made one.
fn overlay() -> Option<&'static NSWindow> {
    let raw = (*OVERLAY.lock().unwrap())?;
    Some(unsafe { &*(raw as *const NSWindow) })
}

/// Put it back in front, for whatever reason it fell behind.
pub fn keep_front() {
    if let Some(win) = overlay() {
        win.orderFrontRegardless();
    }
}

/// Give any Tauri window the overlay's Space behaviour and level.
///
/// The notch panel needs the same treatment as the companion: above the menu bar,
/// present on every Space. It is anchored the same way too -- see `anchor_overlay`
/// for why a parent window is what makes that stick.
pub fn float_everywhere(win: &tauri::WebviewWindow) {
    let Ok(ptr) = win.ns_window() else { return };
    if ptr.is_null() {
        return;
    }
    let ns: &NSWindow = unsafe { &*(ptr as *const NSWindow) };
    ns.setLevel(NSScreenSaverWindowLevel);
    ns.setCollectionBehavior(
        NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::Stationary
            | NSWindowCollectionBehavior::FullScreenAuxiliary
            // Matching the overlay. Without it this is a cmd-tab target, which
            // means the system believes it is a window someone might switch to --
            // and a window someone might switch to is one it will animate during a
            // Space change rather than leave where it is.
            | NSWindowCollectionBehavior::IgnoresCycle,
    );
    ns.setHidesOnDeactivate(false);
    ns.setHasShadow(false);

    if let Some(anchor) = overlay() {
        // Same trick as the companion: a child window follows its parent between
        // Spaces, which is the only thing that survives going full screen.
        //
        // Only once. This runs again on every dock and undock, and re-adding a
        // window that is already a child re-orders it -- which the compositor
        // shows as a flash, and a Space transition is exactly when it is most
        // likely to be noticed.
        let already = unsafe { ns.parentWindow() }.is_some();
        if !already {
            unsafe { anchor.addChildWindow_ordered(ns, NSWindowOrderingMode::Above) };
        }
    }
}
