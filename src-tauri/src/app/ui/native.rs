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

/// Is Mission Control on screen?
///
/// A four-finger swipe up takes the whole screen for a system overview, and an
/// overlay pinned above it is both wrong and unwinnable: Mission Control keeps
/// claiming the top, the poll in `keep_everywhere` kept claiming it back, and the
/// compositor showed every exchange. Recorded at 120fps the notch pill changed
/// state a hundred times in ten seconds -- the poll interval, almost exactly.
///
/// So this is the signal to get out of the way rather than push harder.
///
/// Measured rather than guessed, in and out of the overview: the Dock owns exactly
/// one extra on-screen window while it is up, at layer 20, and nothing of the sort
/// while it is not. Size is the other half of the test, because the Dock's own
/// strip is also its window and also layer 20 -- but only loosely, at half the
/// screen each way. A four-finger drag *scrubs* the overview rather than toggling
/// it, so its backdrop spends the whole gesture mid-animation, and a test for the
/// full size answers "no" for most of the flicker it is meant to prevent.
#[cfg(target_os = "macos")]
pub fn mission_control() -> bool {
    use core_foundation::base::{CFType, TCFType};
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::number::CFNumber;
    use core_foundation::string::CFString;
    use core_graphics::window::{
        copy_window_info, kCGWindowBounds, kCGWindowListOptionOnScreenOnly, kCGWindowOwnerName,
    };

    let Some(list) = copy_window_info(kCGWindowListOptionOnScreenOnly, 0) else {
        return false;
    };
    let Some(mtm) = MainThreadMarker::new() else {
        return false;
    };
    // The full screen it would have to cover, in the same points the window list
    // reports. Compared loosely: the overview's window has matched the display
    // exactly in every reading, but a few points of slack costs nothing and a
    // strict equality that drifts one point costs the whole feature.
    let screen = objc2_app_kit::NSScreen::mainScreen(mtm).map(|s| s.frame().size);
    let Some(screen) = screen else { return false };

    let owner_key = unsafe { CFString::wrap_under_get_rule(kCGWindowOwnerName) };
    let bounds_key = unsafe { CFString::wrap_under_get_rule(kCGWindowBounds) };
    let w_key = CFString::from_static_string("Width");
    let h_key = CFString::from_static_string("Height");

    for i in 0..list.len() {
        let win = unsafe {
            CFDictionary::<CFString, CFType>::wrap_under_get_rule(
                *list.get(i).unwrap() as core_foundation::dictionary::CFDictionaryRef
            )
        };
        let Some(owner) = win.find(&owner_key).and_then(|v| v.downcast::<CFString>()) else {
            continue;
        };
        if owner != "Dock" {
            continue;
        }
        let Some(bounds) = win.find(&bounds_key) else {
            continue;
        };
        // Untyped on the way out: `CFDictionary<CFString, CFType>` is not a
        // concrete CF type as far as the crate is concerned, so the bounds come
        // back through the raw form and are read key by key.
        let bounds = unsafe {
            CFDictionary::<CFString, CFType>::wrap_under_get_rule(
                bounds.as_CFTypeRef() as core_foundation::dictionary::CFDictionaryRef
            )
        };
        let num = |k: &CFString| {
            bounds
                .find(k)
                .and_then(|v| v.downcast::<CFNumber>())
                .and_then(|n| n.to_f64())
                .unwrap_or(0.0)
        };
        if num(&w_key) > screen.width / 2.0 && num(&h_key) > screen.height / 2.0 {
            return true;
        }
    }
    false
}

/// Does the window server still have this window on screen?
///
/// AppKit is not a witness worth calling here. `isOnActiveSpace` answers `true`
/// unconditionally once `CanJoinAllSpaces` is set -- even while the server has the
/// window out of the Space entirely -- and that eviction is the whole reason the
/// poll in `keep_everywhere` exists. The server's own on-screen list is what showed
/// the eviction in the first place, so it is what gets asked about it.
///
/// Ids only, not descriptions: `CGWindowListCreate` hands back a flat array of
/// numbers, which is no allocation per window. The dictionary form of the same
/// query builds one CFDictionary per window on screen to answer a yes-or-no
/// question.
#[cfg(target_os = "macos")]
pub fn on_screen(number: isize) -> bool {
    use core_foundation::array::{CFArrayGetCount, CFArrayGetValueAtIndex};
    use core_foundation::base::TCFType;
    use core_graphics::window::{create_window_list, kCGWindowListOptionOnScreenOnly};

    let Some(list) = create_window_list(kCGWindowListOptionOnScreenOnly, 0) else {
        // No answer is not the same as "gone". Treating a failed query as an
        // eviction would put the repair back on every tick, which is the flicker.
        return true;
    };
    let want = number as u32;
    let raw = list.as_concrete_TypeRef();
    // The array holds ids cast to pointers rather than CFTypes, so it is read as
    // raw values; the typed iterator would dereference them as if they were.
    (0..unsafe { CFArrayGetCount(raw) })
        .any(|i| unsafe { CFArrayGetValueAtIndex(raw, i) } as u32 == want)
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
        let already = ns.parentWindow().is_some();
        if !already {
            unsafe { anchor.addChildWindow_ordered(ns, NSWindowOrderingMode::Above) };
        }
    }
}
