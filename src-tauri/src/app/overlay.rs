//! The one window: full-screen, transparent, always on top, click-through.
use super::state::Screen;
use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, PhysicalPosition, WebviewWindow,
    WindowEvent,
};

/// The prompt box, in points. Only has to fit one line of text.
const PROMPT: (f64, f64) = (620.0, 92.0);
/// Distance from the bottom of the screen, matching the bubble's resting place.
const PROMPT_BOTTOM_GAP: f64 = 56.0;

pub fn window(app: &AppHandle) -> WebviewWindow {
    app.get_webview_window("overlay").expect("overlay window")
}

/// Stretches the overlay across the primary monitor and returns its geometry.
pub fn fit(app: &AppHandle) -> tauri::Result<Screen> {
    let win = window(app);
    let screen = match win.primary_monitor()? {
        Some(mon) => {
            let scale = mon.scale_factor();
            let size = mon.size().to_logical::<f64>(scale);
            win.set_position(PhysicalPosition::new(0, 0))?;
            win.set_size(*mon.size())?;
            Screen { w: size.width, h: size.height, scale }
        }
        // Only reachable with no monitor attached; a sane guess beats a panic.
        None => Screen { w: 1512.0, h: 982.0, scale: 2.0 },
    };
    win.set_ignore_cursor_events(true)?;
    // Never focusable at rest -- see set_prompt_mode.
    let _ = win.set_focusable(false);
    follow_everywhere(&win);

    // Safety net. While asking, this window owns the keyboard; if it somehow ends
    // up in that state with nothing to dismiss it -- the user clicks the menu bar,
    // another app steals focus -- the machine is unusable until Nudge is killed.
    // Losing focus is proof the prompt is no longer wanted.
    let handle = app.clone();
    win.on_window_event(move |event| {
        match event {
            WindowEvent::Focused(false) => {
                handle.emit("dismiss", ()).ok();
            }
            WindowEvent::Focused(true) | WindowEvent::Moved(_) | WindowEvent::Resized(_) => {}
            _ => return,
        }
        // Focus, moves and resizes are all moments AppKit reasserts a window's
        // configured level and collection behaviour. Without re-applying, the
        // overlay works until the first thing that touches the window, then
        // silently drops behind menus -- or out of a full-screen Space entirely.
        follow_everywhere(&window(&handle));
    });
    Ok(screen)
}

/// Switch between pointing and asking.
///
/// A full-screen window cannot be both click-through and typable: the moment it
/// accepts the keyboard it also swallows every click on screen, and the app
/// underneath goes dead. So while it is asking, the window shrinks to the size of
/// the prompt -- the rest of the screen is simply not covered any more, rather
/// than covered by something that politely forwards clicks.
pub fn set_prompt_mode(app: &AppHandle, asking: bool) -> tauri::Result<()> {
    let win = window(app);
    let screen = app.state::<Screen>();
    // Resizing and focusing go through AppKit, which resets the level to whatever
    // the window was configured with. Re-assert it every time, or the overlay
    // quietly sinks back under the menus after the first question.
    follow_everywhere(&win);

    // The overlay is focusable only while it is asking something.
    //
    // This is what actually kept the companion out of full-screen Spaces. A window
    // the system considers focusable is one it considers part of a Space, and it is
    // evicted when a full-screen Space takes over -- the window server reported
    // ours ABSENT the moment VS Code went full screen. Clicky's overlay hard-codes
    // canBecomeKey to false and needs no Space handling at all; tao backs the same
    // method with a `focusable` ivar, which `set_focusable` writes.
    let _ = win.set_focusable(asking);

    if asking {
        let (w, h) = PROMPT;
        win.set_size(LogicalSize::new(w, h))?;
        win.set_position(LogicalPosition::new(
            (screen.w - w) / 2.0,
            screen.h - h - PROMPT_BOTTOM_GAP,
        ))?;
        win.set_ignore_cursor_events(false)?;
        win.set_focus()?;
    } else {
        win.set_ignore_cursor_events(true)?;
        win.set_position(PhysicalPosition::new(0, 0))?;
        if let Some(mon) = win.primary_monitor()? {
            win.set_size(*mon.size())?;
        }
    }
    Ok(())
}

/// Put the overlay back if the window server has dropped it from the current
/// Space. Cheap enough to run ten times a second.
///
/// Measured: entering a full-screen Space removes our window from the on-screen
/// list outright, and the collection behaviour set at startup does not survive it.
/// No window event fires on a Space change, so there is nothing to subscribe to --
/// this polls a getter and only does work when the answer is wrong.
#[cfg(target_os = "macos")]
pub fn keep_everywhere(app: &AppHandle) {
    use objc2_app_kit::NSWindow;

    let win = window(app);
    let Ok(ptr) = win.ns_window() else { return };
    if ptr.is_null() {
        return;
    }
    let ns: &NSWindow = unsafe { &*(ptr as *const NSWindow) };

    // Only the cheap, idempotent setters belong on a 10Hz timer.
    //
    // `setStyleMask` does not: it rebuilds the window frame, and calling it ten
    // times a second took the overlay out of the window list entirely -- the poll
    // meant to keep it alive was what killed it. Style, focusability and
    // deactivation behaviour are set once, in `fit`.
    //
    // No early-out on isOnActiveSpace() either: with CanJoinAllSpaces set, AppKit
    // answers `true` unconditionally, even while the window server has the window
    // out of the current Space.
    ns.setCollectionBehavior(behavior());
    ns.setLevel(objc2_app_kit::NSScreenSaverWindowLevel);
    // Rejoining the Space is not enough on its own; it also has to be put back in
    // front of it.
    ns.orderFrontRegardless();
    // If the webview was adopted into a window of ours, that is the one that has
    // to stay in front; tao's is empty and ordered out.
    super::native::keep_front();
}

#[cfg(not(target_os = "macos"))]
pub fn keep_everywhere(app: &AppHandle) {
    follow_everywhere(&window(app));
}

/// Make the overlay exist on every Space, above full-screen apps and open menus.
///
/// This is the difference between a demo and a product. Nudge is pitched at people
/// working full-screen in Blender or a DAW -- and a full-screen app is its own
/// Space on macOS. By default a window belongs to the Space it was created on, so
/// the companion simply vanishes the moment you switch to the app you wanted help
/// with, which is precisely when it is needed.
///
/// `canJoinAllSpaces` follows the user between desktops; `fullScreenAuxiliary` is
/// the separate flag that allows floating *over* a full-screen app. Both are
/// required -- the first alone still leaves the overlay behind at the one moment
/// that matters. The window level is raised above normal floating windows so it
/// also clears other apps' panels.
#[cfg(target_os = "macos")]
fn follow_everywhere(win: &WebviewWindow) {
    use objc2_app_kit::{
        NSScreenSaverWindowLevel, NSWindow, NSWindowStyleMask,
    };

    let Ok(ptr) = win.ns_window() else { return };
    if ptr.is_null() {
        return;
    }
    // Tauri hands back the NSWindow it owns; borrow it, never take ownership.
    let window: &NSWindow = unsafe { &*(ptr as *const NSWindow) };
    // All three, matching what Clicky's OverlayWindow uses -- worth stating because
    // an earlier version of this file dropped FullScreenAuxiliary after concluding
    // it was harmful. It was not: at the time, Tauri's alwaysOnTop was still
    // resetting the level out from under us, and the wrong flag got the blame.
    //
    //   CanJoinAllSpaces   put the window in every Space, including full-screen ones
    //   Stationary         do not slide it around during Space transitions
    //   FullScreenAuxiliary  permit it alongside a full-screen window
    //   IgnoresCycle       never a cmd-tab target; it is a cursor, not a window
    window.setCollectionBehavior(behavior());

    // Borderless, matching Clicky's OverlayWindow exactly.
    //
    // tao builds this window as FullSizeContentView|Miniaturizable (32772), which
    // also makes it key-capable. A key-capable window is a window the system
    // believes belongs to a Space -- and it gets evicted when a full-screen Space
    // takes over, which is precisely what the window-server poll showed. Borderless
    // makes canBecomeKeyWindow false, and the overlay stops being treated as a
    // window someone might switch to.
    window.setStyleMask(NSWindowStyleMask::Borderless);

    // The missing piece. Nudge is an accessory app, so it is *never* the active
    // application -- and a window that hides on deactivation is therefore a window
    // that is hidden almost all the time. It happened to survive on the desktop and
    // vanish in full screen, which is what sent the last three attempts hunting
    // through collection-behaviour flags.
    window.setHidesOnDeactivate(false);

    // Window level is what decides both "does it survive full screen" and "is it
    // above that menu", and the ladder is unforgiving:
    //
    //   floating 3  ·  status 25  ·  pop-up menu 101  ·  screen saver 1000
    //
    // At status level the overlay sits *below* every open menu and dropdown --
    // which is exactly where it must not be, since pointing into a menu is the
    // whole job. A full-screen Space raises the bar again. Screen-saver level
    // clears all of it; the window is transparent and click-through, so being this
    // high costs the user nothing.
    window.setLevel(NSScreenSaverWindowLevel);

    if std::env::var("NUDGE_DEBUG_WINDOW").is_ok() {
        println!(
            "nudge: styleMask={:?} canBecomeKeyWindow={} onActiveSpace={} hidesOnDeactivate={}",
            window.styleMask(),
            window.canBecomeKeyWindow(),
            window.isOnActiveSpace(),
            window.hidesOnDeactivate(),
        );
    }
    if std::env::var("NUDGE_DEBUG_WINDOW_VERBOSE").is_ok() {
        let class: &objc2_foundation::NSString = unsafe { objc2::msg_send![window, className] };
        println!(
            "nudge: window class={class} level={} behavior={:?} visible={}",
            window.level(),
            window.collectionBehavior(),
            window.isVisible(),
        );
    }
}

#[cfg(not(target_os = "macos"))]
fn follow_everywhere(win: &WebviewWindow) {
    let _ = win.set_visible_on_all_workspaces(true);
}

/// The overlay's Space membership, in one place so the setup and the upkeep cannot
/// drift apart.
#[cfg(target_os = "macos")]
fn behavior() -> objc2_app_kit::NSWindowCollectionBehavior {
    use objc2_app_kit::NSWindowCollectionBehavior as B;
    let default = B::CanJoinAllSpaces | B::Stationary | B::FullScreenAuxiliary | B::IgnoresCycle;
    // Escape hatch for trying combinations without a rebuild.
    match std::env::var("NUDGE_WINDOW_BEHAVIOR").ok().and_then(|v| v.parse().ok()) {
        Some(bits) => B(bits),
        None => default,
    }
}
