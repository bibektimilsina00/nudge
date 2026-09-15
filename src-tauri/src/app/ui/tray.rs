//! The menu bar item. Nudge has no Dock icon and no window, so this is the only
//! place it visibly exists when idle.
use crate::app::state::{Voice, VoiceMode};
use crate::core::reach::Grant;
use crate::core::run::session::Nudge;
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
    // What Nudge has been allowed to do, with a tick beside anything granted.
    //
    // This submenu is the whole of "see that you have". A limit lifted in a
    // config file read once at startup is a thing you have to remember; a ticked
    // line in the menu bar is a thing you can look at. Clicking it takes the
    // grant back, in the same place and with the same gesture that gave it.
    let reach_now = &app.state::<Nudge>().reach;
    let grants: Vec<CheckMenuItem<_>> = Grant::ALL
        .iter()
        .map(|g| {
            CheckMenuItem::with_id(
                app,
                format!("reach:{}", g.key()),
                g.menu(),
                true,
                reach_now.has(*g),
                None::<&str>,
            )
        })
        .collect::<tauri::Result<_>>()?;
    let grant_refs: Vec<&dyn tauri::menu::IsMenuItem<_>> = grants
        .iter()
        .map(|i| i as &dyn tauri::menu::IsMenuItem<_>)
        .collect();
    let reach = Submenu::with_items(app, "Allowed to", true, &grant_refs)?;

    let quit = MenuItem::with_id(app, "quit", "Quit Nudge", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &ask,
            &voice,
            &reach,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
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
            id if id.starts_with("reach:") => {
                let Some(grant) = Grant::ALL.iter().find(|g| format!("reach:{}", g.key()) == id)
                else {
                    return;
                };
                let nudge = app.state::<Nudge>();
                let state = &nudge.reach;
                state.set(*grant, !state.has(*grant));
                // Set from what the state now says rather than from the click:
                // the tick has to show what is true, and those are only the same
                // thing while nothing else can change it.
                for (item, g) in grants.iter().zip(Grant::ALL) {
                    item.set_checked(state.has(g)).ok();
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}
