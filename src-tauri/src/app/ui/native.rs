//! What AppKit has to be asked directly.
//!
//! Two things: whether Mission Control is on screen, and how to give a Tauri
//! window the level and Space behaviour it needs to sit above the menu bar
//! everywhere.
//!
//! There used to be a third. A 1x1 near-invisible `NSWindow` was built here and
//! both real windows were made its children, because a child follows its parent
//! between Spaces and tao's own windows were measured being evicted the moment
//! a full-screen Space activated. It worked for that and it cost something else:
//! one parent with two children is why the strip and the companion blinked in
//! perfect step through the overview -- whatever the compositor did to the
//! anchor, it did to both of them at once, and re-adding a child re-orders it.
//!
//! Clicky keeps no parent-child relationship, sets each window up once and never
//! touches it again, and does not lose its overlay. This now does the same. If
//! the companion ever disappears inside a full-screen app, this paragraph is
//! where to start.
use objc2_app_kit::{
    NSScreenSaverWindowLevel, NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask,
};
use objc2_foundation::MainThreadMarker;

/// Is Mission Control on screen?
///
/// A four-finger swipe up takes the whole screen for a system overview, and an
/// overlay pinned above it is both wrong and unwinnable: Mission Control keeps
/// claiming the top, a poll here kept claiming it back, and the
/// compositor showed every exchange. Recorded at 120fps the notch pill changed
/// state a hundred times in ten seconds -- the poll interval, almost exactly.
///
/// So this is the signal to get out of the way rather than push harder.
///
/// Two tests, and the first one is the one to believe. WindowManager draws a
/// strip of desktop thumbnails called `Spaces Bar` for as long as the overview
/// is up, which is a name rather than a shape and so does not go vague while
/// the thing is still animating.
///
/// The second is kept underneath it because window names need Screen Recording
/// to be readable at all, and an app that has had that permission pulled should
/// degrade to the old guess rather than to nothing.
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
        copy_window_info, kCGWindowBounds, kCGWindowListOptionOnScreenOnly, kCGWindowName,
        kCGWindowOwnerName,
    };

    let Some(list) = copy_window_info(kCGWindowListOptionOnScreenOnly, 0) else {
        return false;
    };
    let Some(mtm) = MainThreadMarker::new() else {
        return false;
    };
    let Some(screen) = objc2_app_kit::NSScreen::mainScreen(mtm).map(|s| s.frame().size) else {
        return false;
    };

    let owner_key = unsafe { CFString::wrap_under_get_rule(kCGWindowOwnerName) };
    let name_key = unsafe { CFString::wrap_under_get_rule(kCGWindowName) };
    let bounds_key = unsafe { CFString::wrap_under_get_rule(kCGWindowBounds) };
    let w_key = CFString::from_static_string("Width");
    let h_key = CFString::from_static_string("Height");

    // Gathered in one pass and decided afterwards, because the two tests are not
    // equals and the window list is in z-order. Deciding inside the loop let
    // whichever window happened to come first answer, and the Dock sits above
    // WindowManager, so the guess kept beating the fact to the return.
    let mut spaces_bar = false;
    let mut any_name = false;
    let mut dock_covers = false;

    for i in 0..list.len() {
        let win = unsafe {
            CFDictionary::<CFString, CFType>::wrap_under_get_rule(
                *list.get(i).unwrap() as core_foundation::dictionary::CFDictionaryRef
            )
        };
        let Some(owner) = win.find(&owner_key).and_then(|v| v.downcast::<CFString>()) else {
            continue;
        };
        let name = win.find(&name_key).and_then(|v| v.downcast::<CFString>());
        if name.is_some() {
            any_name = true;
        }

        if owner == "WindowManager" && name.map(|n| n == "Spaces Bar").unwrap_or(false) {
            spaces_bar = true;
        }

        if owner == "Dock" {
            let Some(bounds) = win.find(&bounds_key) else {
                continue;
            };
            // Untyped on the way out: `CFDictionary<CFString, CFType>` is not a
            // concrete CF type as far as the crate is concerned, so the bounds
            // come back through the raw form and are read key by key.
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
                dock_covers = true;
            }
        }
    }

    if spaces_bar {
        return true;
    }
    // Only when names were unreadable at all, which means Screen Recording is
    // gone. A readable list that simply has no Spaces Bar in it is an answer,
    // not a gap, and falling through to the guess there is what made an
    // ordinary swipe between desktops look like the overview.
    !any_name && dock_covers
}

/// Give any Tauri window the overlay's Space behaviour and level.
///
/// The notch panel needs the same treatment as the companion: above the menu bar,
/// present on every Space.
///
/// Set once and left alone. There used to be a 1x1 window behind both of these
/// that they were made children of, on the reasoning that a child follows its
/// parent between Spaces. One parent and two children is also why they blinked
/// in perfect step: whatever the compositor did to the anchor during Mission
/// Control, it did to both of them at once. Clicky keeps no such relationship
/// and does not lose its overlay.
pub fn float_everywhere(win: &tauri::WebviewWindow) {
    let Ok(ptr) = win.ns_window() else { return };
    if ptr.is_null() {
        return;
    }
    let ns: &NSWindow = unsafe { &*(ptr as *const NSWindow) };

    // Borderless, like the companion.
    //
    // tao builds its windows as FullSizeContentView|Miniaturizable, and a
    // miniaturizable window is one the system treats as an ordinary window
    // belonging to an ordinary Space -- so a full-screen Space takes the screen
    // and this goes with the Space it was created on. Measured: with everything
    // else already right (canBecomeKey false, screen-saver level, joining all
    // Spaces) the strip still disappeared inside a full-screen app, and this
    // mask was the only thing left that differed from the companion.
    //
    // It costs nothing to set here, even though the panel takes the keyboard.
    // tao overrides `canBecomeKeyWindow` to answer from its own `focusable`
    // ivar rather than from the mask, so being borderless does not make the
    // window unfocusable -- `set_interactive` still decides that, and still
    // gets to say yes while the panel is open.
    ns.setStyleMask(NSWindowStyleMask::Borderless);

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

    // Behind the same switch the companion's dump uses. This is the state that
    // decides whether the strip survives a full-screen Space, and reading it
    // took a round trip through a println that startup had not yet redirected.
    if std::env::var("NUDGE_DEBUG_WINDOW").is_ok() {
        eprintln!(
            "nudge: panel style={:?} canBecomeKey={} level={} behavior={:?} onActiveSpace={}",
            ns.styleMask(),
            ns.canBecomeKeyWindow(),
            ns.level(),
            ns.collectionBehavior(),
            ns.isOnActiveSpace(),
        );
    }
}
