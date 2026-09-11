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
    use objc2_app_kit::{NSScreenSaverWindowLevel, NSWindow, NSWindowCollectionBehavior};

    let Ok(ptr) = win.ns_window() else { return };
    if ptr.is_null() {
        return;
    }
    // Tauri hands back the NSWindow it owns; borrow it, never take ownership.
    let window: &NSWindow = unsafe { &*(ptr as *const NSWindow) };
    // Deliberately WITHOUT FullScreenAuxiliary. Its name misleads: it means
    // "auxiliary to this application's own full-screen window", not "may float
    // over full-screen". Setting it does not grant floating over another app's
    // Space, and it suppresses the all-spaces behaviour that does -- verified by
    // reading the behaviour back (NUDGE_DEBUG_WINDOW=1) while the overlay stayed
    // invisible in full screen.
    //
    // CanJoinAllSpaces puts the window in every Space including full-screen ones;
    // Stationary stops it sliding around during Space transitions. That pair plus
    // an accessory activation policy is what screen-annotation tools use.
    let behavior = NSWindowCollectionBehavior::CanJoinAllSpaces
        | NSWindowCollectionBehavior::Stationary
        // Never a target for cmd-tab or the window cycler. It is a cursor, not a
        // window anyone means to switch to.
        | NSWindowCollectionBehavior::IgnoresCycle;
    // Escape hatch for trying combinations without a rebuild; the bits are in
    // NSWindowCollectionBehavior.
    let behavior = match std::env::var("NUDGE_WINDOW_BEHAVIOR").ok().and_then(|v| v.parse().ok()) {
        Some(bits) => NSWindowCollectionBehavior(bits),
        None => behavior,
    };
    window.setCollectionBehavior(behavior);

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
