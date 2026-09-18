//! Our own overlay window, built from scratch rather than configured.
//!
//! A window that floats over everything, never takes focus and survives a
//! full-screen Space is not a window with unusual settings -- it is a different
//! window. tao decides `styleMask` and `canBecomeKey` when it creates its own,
//! and those two are exactly what have to be different, so no amount of
//! configuring afterwards gets there.
//!
//! There is a third thing here, and it is a compromise rather than a design.
//!
//! A 1x1 near-invisible `NSWindow` is built below and both real windows are
//! made its children, because a child follows its parent between Spaces. It was
//! taken out once, on the theory that a borderless mask plus `canBecomeKey`
//! false would do the same job, and the window server said otherwise: with all
//! of that set, and the flags re-applied, and the windows ordered out and back
//! in to force a re-evaluation, `kCGWindowListOptionOnScreenOnly` still had
//! nothing of ours in it while an app was full screen.
//!
//! The cost is that one parent with two children is why both windows blink in
//! step during Mission Control, which is why they are taken off screen for its
//! duration instead. Full-screen presence is worth more than being visible over
//! the overview; that is the whole of the trade.
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

/// Anchor tao's overlay window to one of ours.
///
/// **This does not do what the rest of this comment used to say it does.** It was
/// added to keep the overlay in a full-screen Space, on the reasoning that a child
/// window follows its parent between Spaces. Measured since, with the anchor and
/// without it: the children are evicted either way, reporting `visible = true`
/// and `isOnActiveSpace = false`, while the anchor itself stays. What actually
/// keeps them there is being an `NSPanel` -- see [`become_panel`].
///
/// It is kept for the moment because the overview work was built on top of it and
/// that is a separate measurement. It is a deletion waiting to happen, and the
/// comment in `dock_to_notch` about a parent being why both windows blink in step
/// through Mission Control is the reason to make it.
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
    // Honestly transparent, rather than opaque-and-then-faded.
    //
    // This used to claim `isOpaque` while carrying an alpha of 0.004, which is
    // a contradiction the compositor has to resolve on every frame it draws --
    // and Mission Control animates for the whole time it is up. Both real
    // windows are children of this one, so anything it churns on, they churn on
    // together, which is exactly the shape of the flicker: two windows blinking
    // in perfect step.
    //
    // It still has to draw. A window that never draws gets no backing store and
    // the window server stops tracking it, and an untracked anchor is no anchor
    // -- the children go back to being evicted by full-screen Spaces. So the
    // near-invisible black stays, it is just in the colour where it belongs
    // instead of in a whole-window alpha that fights the opacity flag.
    anchor.setOpaque(false);
    // Not `clearColor`. Measured: with nothing to draw the window server stops
    // tracking the anchor entirely -- it drops out of the on-screen list, and an
    // untracked anchor is no anchor, so the children go back to being evicted by
    // full-screen Spaces. The faint black is what keeps it real.
    let faint = NSColor::colorWithCalibratedWhite_alpha(0.0, 0.004);
    anchor.setBackgroundColor(Some(&faint));
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

/// Ask the windows themselves, once a second, what the window server thinks of
/// them. Behind `NUDGE_DEBUG_SPACES`, and meant to be taken out again.
///
/// The on-screen list says *that* a window was evicted from a full-screen Space.
/// It cannot say whether the parent link that was supposed to prevent it is
/// still there, and every guess about that so far has been wrong.
pub fn watch_spaces(app: &AppHandle) {
    if std::env::var("NUDGE_DEBUG_SPACES").is_err() {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
        let here = app.clone();
        let _ = app.run_on_main_thread(move || {
            for label in ["overlay", "panel"] {
                let Some(win) = here.get_webview_window(label) else {
                    continue;
                };
                let Ok(ptr) = win.ns_window() else { continue };
                if ptr.is_null() {
                    continue;
                }
                let ns: &NSWindow = unsafe { &*(ptr as *const NSWindow) };
                println!(
                    "spaces: {label:<7} visible={} onActiveSpace={} parent={} level={} behavior={:?}",
                    ns.isVisible(),
                    ns.isOnActiveSpace(),
                    ns.parentWindow().is_some(),
                    ns.level(),
                    ns.collectionBehavior(),
                );
            }
            if let Some(anchor) = overlay() {
                println!(
                    "spaces: anchor  visible={} onActiveSpace={} children={}",
                    anchor.isVisible(),
                    anchor.isOnActiveSpace(),
                    anchor.childWindows().map(|c| c.len()).unwrap_or(0),
                );
            }
        });
    });
}

/// Make a Tauri window an `NSPanel`, non-activating.
///
/// **This is the one thing that gets a window into a full-screen Space.** Four
/// other explanations were measured and are wrong, and they are written down
/// because each one reads as obviously right:
///
///   * the collection behaviour -- `CanJoinAllSpaces | FullScreenAuxiliary`, with
///     and without `Stationary` and `IgnoresCycle`, evicted every time;
///   * the parent window -- with the anchor and without it, evicted every time,
///     while the anchor itself stayed;
///   * ordering the window out and back in so the window server assigns its
///     Space after the flags are set -- evicted every time;
///   * the window level, which was already screen-saver and above everything.
///
/// The window reports `visible = true` and `isOnActiveSpace = false` throughout:
/// it is not hidden, it is somewhere else. An `NSWindow` belongs to the Space it
/// was born in and a full-screen Space is a Space it was not born in; an `NSPanel`
/// is a utility window and is allowed alongside one. That is the whole difference,
/// and it is a difference of *class*, which is why nothing settable on the window
/// could reach it.
///
/// `object_setClass` on a live window is the same trick `tauri-nspanel` exists to
/// perform, and it has one sharp edge: tao's subclass goes with it, **and so
/// does the `focusable` ivar declared on that class**. `WebviewWindow::set_focusable`
/// writes straight into that ivar, so calling it afterwards is not a no-op, it
/// is `ivar "focusable" not found on class NSPanel` and the process is gone.
/// Moving the pointer to the notch did it every time, because that is what
/// toggles the panel between click-through and clickable.
///
/// So focus is decided here instead, once, by class rather than per hover.
/// `keyable` is false for a window that must never take the keyboard -- the
/// companion covers the whole desk -- and true for one somebody types into.
pub fn become_panel(ns: &NSWindow, keyable: bool) {
    use objc2::runtime::{AnyClass, NSObjectProtocol};

    let Some(class) = AnyClass::get(c"NSPanel") else {
        return;
    };
    // Already converted. Doing it twice is harmless, but this runs on every dock
    // and undock and the check is cheaper than the call.
    if ns.isKindOfClass(class) {
        return;
    }
    let obj: *mut objc2::runtime::AnyObject = ns as *const NSWindow as *mut _;
    unsafe { objc2::ffi::object_setClass(obj.cast(), (class as *const AnyClass).cast()) };

    // Non-activating: clicking the panel must not bring Nudge to the front. It has
    // no Dock icon and no main window to come forward to, and stealing activation
    // from whatever somebody is working in is the whole thing this app exists not
    // to do.
    ns.setStyleMask(NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel);

    let panel: &objc2_app_kit::NSPanel = unsafe { &*(ns as *const NSWindow as *const _) };
    // A window nobody types into never becomes key at all. The ones somebody
    // does are left alone: `becomesKeyOnlyIfNeeded` asks the clicked view
    // whether it needs the panel to be key, and a WKWebView is one view as far
    // as AppKit is concerned -- it would answer for the whole page, and a sign-in
    // form that cannot be typed into is a worse failure than a panel that takes
    // the keyboard a moment early.
    //
    // It costs nothing either way while the panel is a strip: it is
    // click-through then, so there is no click to make it key.
    panel.setBecomesKeyOnlyIfNeeded(!keyable);
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
/// claiming the top, a poll here kept claiming it back, and the
/// compositor showed every exchange. Recorded at 120fps the notch pill changed
/// state a hundred times in ten seconds -- the poll interval, almost exactly.
///
/// So this is the signal to get out of the way rather than push harder.
///
/// Two tests, and the first one is the one to believe. WindowManager draws a
/// window across the display for as long as the overview is up -- either its
/// shield or its strip of thumbnails -- and a name does not go vague while the
/// thing is still animating, the way a size does.
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
    let mut wm_seen = 0usize;

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

        // Either name counts.
        //
        // `Spaces Bar` alone was wrong: the strip of desktop thumbnails is
        // collapsed until the pointer goes to the top of the screen, so for most
        // of the time the overview is open there is no window called that, the
        // answer came back "no", and the panel opened over Mission Control
        // exactly as it had before any of this.
        //
        // `ExposeShieldWindow` is up for the whole duration, and also for an
        // ordinary slide between Spaces -- which is why it was dropped. That no
        // longer costs anything: this answer only closes the panel and quiets
        // the hint now, and a panel that shuts while you change Space is right
        // rather than merely harmless. It stopped being about hiding windows.
        let overview_window = name
            .as_ref()
            .map(|n| *n == "Spaces Bar" || *n == "ExposeShieldWindow")
            .unwrap_or(false);
        if owner == "WindowManager" {
            wm_seen += 1;
        }
        if owner == "WindowManager" && overview_window {
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

    // The overview brings a crowd, and counting it does not need names.
    //
    // Measured on this machine: WindowManager owns two on-screen windows at
    // rest and thirteen while Mission Control is up -- one per Space thumbnail,
    // plus the shield behind them. The names are the part that cannot be relied
    // on; `ExposeShieldWindow` and `Spaces Bar` came back named once and come
    // back nameless now, which is why two rounds of looking for them by name
    // failed while the overview was plainly on screen.
    //
    // Four is well clear of two and well under thirteen. A false positive costs
    // a closed panel for a moment, which is the right thing to do during a
    // Space change anyway; a false negative is the panel sitting on top of
    // Mission Control, which is the bug being fixed.
    if wm_seen >= 4 {
        return true;
    }
    // Only when names were unreadable at all, which means Screen Recording is
    // gone. A readable list with neither window in it is an answer,
    // not a gap, and falling through to the guess there is what made an
    // ordinary swipe between desktops look like the overview.
    !any_name && dock_covers
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

    become_panel(ns, true);

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
