//! The Tauri adapter: windows, tray, hotkeys, IPC. Everything here exists to plug
//! `core` into the OS, and `core` never reaches back the other way.

pub mod commands;
mod native;
mod panel;
mod cursor;
mod hotkey;
pub(crate) mod overlay;
mod state;
mod tray;

use crate::config::Config;
use crate::core::session::Nudge;
use state::{Auto, Docked, Flag, Mic, Screen, Settle, Voice, VoiceMode};
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
            // Give tao's overlay a parent window that full-screen Spaces do not
            // evict; see native.rs for what was measured to get here.
            println!("nudge: overlay anchored = {}", native::anchor_overlay(handle));
            app.manage(Mic::default());
            app.manage(auto);
            app.manage(voice);
            app.manage(Settle(std::sync::Mutex::new(None)));
            // Starts undocked: an app that does nothing until you find a button is
            // an app most people never see working.
            app.manage(Docked(Flag::new(false)));

            println!(
                "nudge: windows = {:?}",
                app.webview_windows().keys().collect::<Vec<_>>()
            );
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
            commands::docked,
            commands::set_docked,
            commands::fit_panel,
            commands::auto,
            commands::set_auto,
        ])
        .run(tauri::generate_context!())
        .expect("nudge failed to start");
}
