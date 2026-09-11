//! The menu bar item. Nudge has no Dock icon and no window, so this is the only
//! place it visibly exists when idle.
use super::state::{Auto, Voice, VoiceMode};
use crate::core::click;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

pub fn install(app: &AppHandle, hotkey: &str) -> tauri::Result<()> {
    let ask = MenuItem::with_id(app, "ask", &format!("Ask Nudge  ({hotkey})"), true, None::<&str>)?;
    let auto = CheckMenuItem::with_id(
        app,
        "auto",
        "Click for me",
        true,
        app.state::<Auto>().0.on(),
        None::<&str>,
    )?;
    // Checkboxes behaving as a radio group: macOS menus have no native radio item,
    // so exclusivity is kept by hand in the handler below.
    let current = app.state::<Voice>().get();
    let labels = ["Off", "System (free, offline)", "Natural (uses API)"];
    let options: Vec<CheckMenuItem<_>> = VoiceMode::ALL
        .iter()
        .zip(labels)
        .map(|((id, mode), label)| {
            CheckMenuItem::with_id(app, id, label, true, *mode == current, None::<&str>)
        })
        .collect::<tauri::Result<_>>()?;
    let refs: Vec<&dyn tauri::menu::IsMenuItem<_>> =
        options.iter().map(|i| i as &dyn tauri::menu::IsMenuItem<_>).collect();
    let voice = Submenu::with_items(app, "Voice", true, &refs)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Nudge", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[&ask, &auto, &voice, &PredefinedMenuItem::separator(app)?, &quit],
    )?;
    let toggle = auto.clone();

    TrayIconBuilder::with_id("nudge")
        // A template icon is rendered from its alpha channel alone, so this is a
        // transparent PNG with an opaque glyph -- the app icon would come out as a
        // solid blob. Embedded rather than bundled as a resource: one less path to
        // get wrong at runtime.
        .icon(tauri::image::Image::from_bytes(include_bytes!("../../icons/tray.png"))?)
        .icon_as_template(true) // follows the light/dark menu bar like a native item
        .tooltip("Nudge")
        .menu(&menu)
        // Left click opens the panel; the menu stays on right click. Without this
        // the menu eats the left click and the panel can never be reached.
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            eprintln!("nudge: tray event {event:?}");
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                rect,
                ..
            } = event
            {
                // Convert with the real scale factor, not 1.0. `to_physical`
                // passes physical values through untouched but multiplies logical
                // ones -- so a hardcoded 1.0 puts the panel at half the correct x
                // on a Retina display, which is off the side of the screen for a
                // menu bar item on the right.
                let scale = tray
                    .app_handle()
                    .primary_monitor()
                    .ok()
                    .flatten()
                    .map(|m| m.scale_factor())
                    .unwrap_or(2.0);
                let size = rect.size.to_physical::<f64>(scale);
                let pos = rect.position.to_physical::<f64>(scale);
                super::panel::toggle(
                    tray.app_handle(),
                    Some((pos.x, pos.y + size.height, size.width)),
                );
            }
        })
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "ask" => {
                app.emit("ask", ()).ok();
            }
            id if id.starts_with("voice:") => {
                let Some((_, mode)) = VoiceMode::ALL.iter().find(|(k, _)| *k == id) else {
                    return;
                };
                app.state::<Voice>().set(*mode);
                // Tick the chosen one and clear the rest; a menu showing two voices
                // selected is worse than one showing none.
                for (item, (_, m)) in options.iter().zip(VoiceMode::ALL) {
                    item.set_checked(m == *mode).ok();
                }
            }
            "auto" => {
                let want = !app.state::<Auto>().0.on();
                // Posting input events needs Accessibility permission. Ask on the
                // way in, and refuse to show the box ticked if it was not granted --
                // a toggle that claims to be on while silently doing nothing is
                // worse than one that will not turn on.
                let allowed = !want || click::may_click() || click::request_click_permission();
                app.state::<Auto>().0.set(want && allowed);
                toggle.set_checked(want && allowed).ok();
                if want && !allowed {
                    app.emit(
                        "error",
                        "Clicking needs Accessibility: System Settings > Privacy & \
                         Security > Accessibility, then reopen Nudge.",
                    )
                    .ok();
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}
