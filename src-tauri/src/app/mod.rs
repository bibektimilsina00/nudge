//! The Tauri adapter: windows, tray, hotkeys, IPC. Everything here exists to plug
//! `core` into the OS, and `core` never reaches back the other way.

pub mod commands;
mod cursor;
mod hotkey;
pub(crate) mod overlay;
mod state;
mod tray;

use crate::config::Config;
use crate::core::session::Nudge;
use state::{Auto, Flag, Mic, Screen, Settle, Voice, VoiceMode};
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // Menu-bar app: no Dock icon, no app switcher, no window chrome.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let cfg = Config::load()?;
            let hotkey = cfg.hotkey.clone();
            let cfg_auto = cfg.auto_click;
            let cfg_engine = cfg.speech_engine.clone();
            let auto = Auto(Flag::new(cfg_auto));
            let voice = Voice(VoiceMode::from_config(cfg.speak, &cfg.speech_engine).into());
            let nudge = Nudge::new(cfg)?;
            println!("nudge: provider = {}", nudge.provider_name());
            println!(
                "nudge: click-for-me = {}, accessibility = {}, microphone = {:?}",
                cfg_auto,
                crate::core::click::may_click(),
                crate::core::voice::access(),
            );
            println!(
                "nudge: voice = {:?} ({})",
                crate::core::speech::chosen_voice(),
                cfg_engine,
            );

            let handle = app.handle();
            app.manage(nudge);
            app.manage::<Screen>(overlay::fit(handle)?);
            app.manage(Mic::default());
            app.manage(auto);
            app.manage(voice);
            app.manage(Settle(std::sync::Mutex::new(None)));

            tray::install(handle, &hotkey)?;
            cursor::follow(handle);
            hotkey::register(handle, &hotkey)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::start,
            commands::advance,
            commands::cancel,
            commands::set_interactive,
        ])
        .run(tauri::generate_context!())
        .expect("nudge failed to start");
}
