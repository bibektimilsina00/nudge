//! The Tauri adapter: windows, tray, hotkeys, IPC. Everything here exists to plug
//! `core` into the OS, and `core` never reaches back the other way.

pub(crate) mod agent;
pub mod commands;
mod input;
pub(crate) mod state;
mod ui;

use crate::config::Config;
use crate::core::run::session::Nudge;
use state::{Background, Docked, Flag, Grants, Mic, Screen, Settle, Voice, VoiceMode};
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // Menu-bar app: no Dock icon, no app switcher, no window chrome.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let cfg = Config::load()?;
            let hotkey = cfg.hotkey.clone();
            let cfg_engine = cfg.speech_engine.clone();
            let voice = Voice(VoiceMode::from_config(cfg.speak, &cfg.speech_engine).into());
            let nudge = Nudge::new(cfg)?;
            println!("nudge: provider = {}", nudge.provider_name());
            println!(
                "nudge: accessibility = {}, microphone = {:?}",
                crate::core::screen::click::may_click(),
                crate::core::voice::access(),
            );

            // Asked once, at startup, and never waited on. Transcription happens
            // on this machine when this is granted and over the network when it
            // is not, so a refusal costs a second a turn and nothing else -- which
            // is not worth interrupting a hotkey press for.
            crate::core::voice::ear::request_access();
            println!(
                "nudge: on-device speech = {}",
                crate::core::voice::ear::status()
            );
            println!(
                "nudge: voice = {:?} ({})",
                crate::core::voice::speech::chosen_voice(),
                cfg_engine,
            );

            let handle = app.handle();
            app.manage(nudge);
            app.manage::<Screen>(ui::overlay::fit(handle)?);
            // Give tao's overlay a parent window that full-screen Spaces do not
            // evict; see native.rs for what was measured to get here.
            println!(
                "nudge: overlay anchored = {}",
                ui::native::anchor_overlay(handle)
            );
            app.manage(Mic::default());
            app.manage(voice);
            app.manage(Settle(std::sync::Mutex::new(None)));
            app.manage(Grants::default());
            app.manage(Background::default());
            // Starts undocked: an app that does nothing until you find a button is
            // an app most people never see working.
            app.manage(Docked(Flag::new(false)));
            app.manage(crate::core::run::agent::Agents::default());

            println!(
                "nudge: windows = {:?}",
                app.webview_windows().keys().collect::<Vec<_>>()
            );
            ui::panel::dock_to_notch(handle);
            agent::place_window(handle);
            ui::tray::install(handle, &hotkey)?;
            input::cursor::follow(handle);
            input::hotkey::register(handle, &hotkey)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::start,
            commands::advance,
            commands::cancel,
            commands::docked,
            commands::set_docked,
            commands::fit_panel,
            commands::set_open_size,
            commands::notch_height,
            commands::open_artifact,
            commands::voice_mode,
            commands::set_voice_mode,
            commands::microphone,
            commands::version,
            commands::quit,
            commands::agents,
            commands::answer_agent,
            commands::stop_agent,
            commands::dismiss_agent,
        ])
        .run(tauri::generate_context!())
        .expect("nudge failed to start");
}
