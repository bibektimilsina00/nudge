//! The menu bar item. Nudge has no Dock icon and no window, so this is the only
//! place it visibly exists when idle.
use crate::app::state::{Voice, VoiceMode};
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager};

pub fn install(app: &AppHandle, hotkey: &str) -> tauri::Result<()> {
    // A bare modifier is held, not pressed, and the menu should say so.
    let label = if crate::app::input::hotkey::is_bare_modifier(hotkey) {
        "Ask Nudge  (hold \u{2303})".to_string()
    } else {
        format!("Ask Nudge  ({hotkey})")
    };
    // Disabled on purpose: it states the shortcut rather than offering a click.
    // There is nothing to click any more -- speaking is the only way in.
    let ask = MenuItem::with_id(app, "ask", &label, false, None::<&str>)?;
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
    let refs: Vec<&dyn tauri::menu::IsMenuItem<_>> = options
        .iter()
        .map(|i| i as &dyn tauri::menu::IsMenuItem<_>)
        .collect();
    let voice = Submenu::with_items(app, "Voice", true, &refs)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Nudge", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[&ask, &voice, &PredefinedMenuItem::separator(app)?, &quit],
    )?;

    TrayIconBuilder::with_id("nudge")
        // A template icon is rendered from its alpha channel alone, so this is a
        // transparent PNG with an opaque glyph -- the app icon would come out as a
        // solid blob. Embedded rather than bundled as a resource: one less path to
        // get wrong at runtime.
        .icon(tauri::image::Image::from_bytes(include_bytes!(
            "../../../icons/tray.png"
        ))?)
        .icon_as_template(true) // follows the light/dark menu bar like a native item
        .tooltip("Nudge")
        .menu(&menu)
        .on_menu_event(move |app, event| match event.id.as_ref() {
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
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}
