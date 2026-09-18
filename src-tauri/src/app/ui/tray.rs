//! The menu bar item. Nudge has no Dock icon and no window, so this is the only
//! place it visibly exists when idle.
//!
//! It is also where everything Nudge has been allowed to do is written down. A
//! limit lifted in a config file read once at startup is a thing you have to
//! remember; a ticked line here is a thing you can look at.
//!
//! ## The menu is built from state, never edited
//!
//! Every click changes state and then rebuilds the whole menu. The older way --
//! keeping handles to the items and ticking them by hand -- meant the menu and
//! the truth were two things that had to be kept in step, and a menu that says
//! one thing while a gate enforces another is the worst available failure for
//! something whose whole job is being visible.
//!
//! It is also what makes the tool servers work at all. They connect in the
//! background, long after this is first built, and their names arrive with a
//! count of what they can do; [`refresh`] is called when they land and the menu
//! simply says more than it did a minute ago.
use crate::app::state::{Voice, VoiceMode};
use crate::core::reach::Grant;
use crate::core::run::session::{Nudge, ServerState};
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Wry};

/// The id the tray is registered under, so [`refresh`] can find it again.
const TRAY: &str = "nudge";

fn build(app: &AppHandle, hotkey: &str) -> tauri::Result<Menu<Wry>> {
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
    // so only one is ticked and clicking another rebuilds this.
    let current = app.state::<Voice>().get();
    let labels = ["Off", "System (free, offline)", "Natural (uses API)"];
    let options: Vec<CheckMenuItem<_>> = VoiceMode::ALL
        .iter()
        .zip(labels)
        .map(|((id, mode), label)| {
            CheckMenuItem::with_id(app, id, label, true, *mode == current, None::<&str>)
        })
        .collect::<tauri::Result<_>>()?;
    let voice = Submenu::with_items(app, "Voice", true, &as_items(&options))?;

    let nudge = app.state::<Nudge>();

    // The fixed grants: shell, files, http. Closed unless somebody opened them.
    let grants: Vec<CheckMenuItem<_>> = Grant::ALL
        .iter()
        .map(|g| {
            CheckMenuItem::with_id(
                app,
                format!("reach:{}", g.key()),
                g.menu(),
                true,
                nudge.reach.has(*g),
                None::<&str>,
            )
        })
        .collect::<tauri::Result<_>>()?;
    let reach = Submenu::with_items(app, "Allowed to", true, &as_items(&grants))?;

    // And the tool servers, which are whatever the config named.
    //
    // The count is the "see what it can do" half. A server described only by its
    // name is a thing you have to trust; one that says it brought fourteen tools
    // is a thing you can weigh.
    let servers = nudge.tool_servers();
    let tools: Vec<CheckMenuItem<_>> = servers
        .iter()
        .map(|(name, count)| {
            let label = match count {
                ServerState::Ready(0) => format!("{name} — nothing offered"),
                ServerState::Ready(1) => format!("{name} — 1 tool"),
                ServerState::Ready(n) => format!("{name} — {n} tools"),
                ServerState::Starting => format!("{name} — starting…"),
                // Short, because a menu item is not a log line -- but present,
                // because a server that is simply absent reads as one nobody
                // configured.
                ServerState::Failed(_) => format!("{name} — did not start"),
            };
            CheckMenuItem::with_id(
                app,
                format!("server:{name}"),
                label,
                true,
                nudge.reach.server(name),
                None::<&str>,
            )
        })
        .collect::<tauri::Result<_>>()?;

    // What it has learned, and how to make it forget.
    //
    // Memory changes what Nudge does, so it gets the same treatment as everything
    // else that does: visible, with a count, and undoable in one click. Something
    // that silently learns is something you cannot reason about when it starts
    // behaving oddly.
    //
    // Plain items rather than checkboxes: a tick would imply the note can be
    // switched off and back on, and forgetting is not reversible.
    let learned = nudge.memory.everything();
    let notes: Vec<MenuItem<_>> = learned
        .iter()
        .map(|(about, count)| {
            MenuItem::with_id(
                app,
                format!("forget:{about}"),
                match count {
                    1 => format!("Forget 1 note about {about}"),
                    n => format!("Forget {n} notes about {about}"),
                },
                true,
                None::<&str>,
            )
        })
        .collect::<tauri::Result<_>>()?;

    // The tour script, played straight. It is how the segmented format gets
    // looked at without waiting on a model to produce one.
    let tour = MenuItem::with_id(app, "tour", "Play tour", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Nudge", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;

    // The submenu is left out entirely when nothing is configured, rather than
    // shown empty. An empty "Tools" reads as something broken; its absence reads
    // as a feature not in use, which is what it is.
    let mut items: Vec<&dyn tauri::menu::IsMenuItem<Wry>> = vec![&ask, &voice, &reach];
    // Built here so the borrows outlive the vector that points at them.
    let servers = match tools.is_empty() {
        true => None,
        false => Some(Submenu::with_items(app, "Tools", true, &as_items(&tools))?),
    };
    if let Some(servers) = &servers {
        items.push(servers);
    }
    let remembered = match notes.is_empty() {
        true => None,
        false => Some(Submenu::with_items(
            app,
            "Learned",
            true,
            &as_items(&notes),
        )?),
    };
    if let Some(remembered) = &remembered {
        items.push(remembered);
    }
    items.push(&sep);
    items.push(&tour);
    items.push(&quit);
    Menu::with_items(app, &items)
}

fn as_items<T: tauri::menu::IsMenuItem<Wry>>(
    items: &[T],
) -> Vec<&dyn tauri::menu::IsMenuItem<Wry>> {
    items
        .iter()
        .map(|i| i as &dyn tauri::menu::IsMenuItem<Wry>)
        .collect()
}

/// Rebuild the menu from what is true now.
///
/// Called after every click, and once more when the tool servers finish
/// connecting -- which is the only reason this is public.
pub fn refresh(app: &AppHandle, hotkey: &str) {
    let Some(tray) = app.tray_by_id(TRAY) else {
        return;
    };
    match build(app, hotkey) {
        Ok(menu) => {
            let _ = tray.set_menu(Some(menu));
        }
        // A menu that failed to rebuild leaves the previous one in place, which
        // is stale rather than absent. Better than a menu bar item with no menu.
        Err(e) => eprintln!("tray: could not rebuild the menu: {e}"),
    }
}

pub fn install(app: &AppHandle, hotkey: &str) -> tauri::Result<()> {
    let menu = build(app, hotkey)?;
    let hotkey = hotkey.to_string();

    TrayIconBuilder::with_id(TRAY)
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
        .on_menu_event(move |app, event| {
            let id = event.id.as_ref();
            match id {
                "tour" => {
                    crate::app::tour::demo(app);
                    return;
                }
                "quit" => {
                    app.exit(0);
                    return;
                }
                _ if id.starts_with("voice:") => {
                    if let Some((_, mode)) = VoiceMode::ALL.iter().find(|(k, _)| *k == id) {
                        app.state::<Voice>().set(*mode);
                    }
                }
                _ if id.starts_with("reach:") => {
                    if let Some(g) = Grant::ALL
                        .iter()
                        .find(|g| format!("reach:{}", g.key()) == id)
                    {
                        let nudge = app.state::<Nudge>();
                        nudge.reach.set(*g, !nudge.reach.has(*g));
                    }
                }
                _ if id.starts_with("forget:") => {
                    app.state::<Nudge>().memory.forget(&id["forget:".len()..]);
                }
                _ if id.starts_with("server:") => {
                    let name = &id["server:".len()..];
                    let nudge = app.state::<Nudge>();
                    nudge.reach.set_server(name, !nudge.reach.server(name));
                }
                _ => return,
            }
            // One rebuild, from whatever is now true, for every kind of change.
            refresh(app, &hotkey);
        })
        .build(app)?;
    Ok(())
}
