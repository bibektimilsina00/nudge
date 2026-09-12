//! The switches and the window plumbing. Nothing here decides anything; it
//! translates a click in the panel into a call and back.
use crate::app::state::{Docked, Voice};
use crate::core::run::session::Nudge;
use crate::core::tools::files;
use crate::error::Result;
use tauri::{AppHandle, Emitter, Manager};

/// Is the companion parked in the panel?
#[tauri::command]
pub fn docked(app: AppHandle) -> bool {
    app.state::<Docked>().0.on()
}

/// Park the companion in the panel, or release it onto the screen.
#[tauri::command]
pub fn set_docked(app: AppHandle, docked: bool) {
    app.state::<Docked>().0.set(docked);
    // Both windows draw the companion, so both need to know which of them owns it.
    app.emit("docked", docked).ok();

    // The panel window is not a panel: it is the notch, and the notch is the only
    // way back into Nudge.
    //
    // Releasing the companion used to hide this window, which read as "get the
    // settings sheet out of the way" and actually meant "delete the dock". The
    // pill went with it, docking again never called `show`, and the companion had
    // nowhere to sit -- so it vanished from the cursor and from the notch at
    // once, with nothing left to click.
    //
    // The sheet collapsing is the frontend's job and it already does it. This
    // window stays.
    if let Some(panel) = crate::app::ui::panel::window(&app) {
        let _ = panel.show();
        #[cfg(target_os = "macos")]
        crate::app::ui::native::float_everywhere(&panel);
    }
}

/// Which voice is in use: "off", "system" or "gemini".
#[tauri::command]
pub fn voice_mode(app: AppHandle) -> &'static str {
    match app.state::<Voice>().get() {
        crate::app::state::VoiceMode::Off => "off",
        crate::app::state::VoiceMode::System => "system",
        crate::app::state::VoiceMode::Gemini => "gemini",
    }
}

/// Change it. Unknown names are ignored rather than guessed at.
#[tauri::command]
pub fn set_voice_mode(app: AppHandle, mode: String) {
    let picked = match mode.as_str() {
        "off" => crate::app::state::VoiceMode::Off,
        "system" => crate::app::state::VoiceMode::System,
        "gemini" => crate::app::state::VoiceMode::Gemini,
        _ => return,
    };
    app.state::<Voice>().set(picked);
    // The menu bar shows the same setting and has to follow.
    app.emit("voice-mode", mode).ok();
}

/// The microphone recording will use.
#[tauri::command]
pub fn microphone() -> String {
    crate::core::voice::input_name().unwrap_or_else(|| "No input device".into())
}

/// Version, for the footer.
#[tauri::command]
pub fn version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}

#[tauri::command]
pub fn quit(app: AppHandle) {
    app.exit(0);
}

/// Tell the hover test how big the panel currently is.
#[tauri::command]
pub fn set_open_size(w: f64, h: f64) {
    crate::app::ui::notch::set_open_size(w, h);
}

/// The real notch's height in points, so the docked pill can match the hardware
/// exactly rather than guessing at it. Sync, so it runs on the main thread --
/// `measure` needs AppKit.
#[tauri::command]
pub fn notch_height() -> f64 {
    crate::app::ui::notch::measure().height
}

/// Open a file an agent made, in whatever app owns it.
///
/// Checked against the workspace rather than trusted: this is reachable from the
/// UI, and a path arriving from anywhere is a path that has to be proven.
#[tauri::command]
pub fn open_artifact(app: AppHandle, path: String) -> Result<()> {
    let workspace = app.state::<Nudge>().cfg.workspace_dir();
    let resolved = files::resolve(&workspace, &path)?;
    if !resolved.is_file() {
        return Err(crate::error::Error::Click(format!(
            "{} is not there any more",
            resolved.display()
        )));
    }
    std::process::Command::new("open").arg(&resolved).spawn()?;
    Ok(())
}

/// Let the panel size itself to its content.
#[tauri::command]
pub fn fit_panel(app: AppHandle, height: f64) {
    crate::app::ui::panel::fit(&app, height);
}
