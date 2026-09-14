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
            //
            // Except while a permission is still unanswered. macOS will not put a
            // privacy dialog on screen for an application that has no presence --
            // the request returns, nothing appears, and the status stays
            // "not asked" forever, which is indistinguishable from the user
            // ignoring a prompt they were never shown. So we stay an ordinary
            // application, with a Dock icon, until both questions have answers,
            // and disappear after.
            #[cfg(target_os = "macos")]
            let asking = crate::core::voice::access() == crate::core::voice::Access::Unasked
                || crate::core::voice::ear::status() == "not asked yet";
            #[cfg(target_os = "macos")]
            if !asking {
                app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            }

            let cfg = Config::load()?;
            let hotkey = cfg.hotkey.clone();
            let cfg_engine = cfg.speech_engine.clone();
            keep_a_log();
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
            #[cfg(target_os = "macos")]
            if asking {
                // Front and centre, so the dialog has something to belong to.
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    for _ in 0..120 {
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        let answered = crate::core::voice::access()
                            != crate::core::voice::Access::Unasked
                            && crate::core::voice::ear::status() != "not asked yet";
                        if answered {
                            break;
                        }
                    }
                    println!(
                        "nudge: permissions settled -- microphone = {:?}, speech = {}",
                        crate::core::voice::access(),
                        crate::core::voice::ear::status()
                    );
                    let _ = handle.set_activation_policy(tauri::ActivationPolicy::Accessory);
                });
            }

            // Both grants, asked for at launch rather than mid-hotkey.
            //
            // The hotkey path asks too, and that was the only place asking until
            // now -- which put a system dialog on screen at the exact moment
            // someone was holding a key and talking, where it opens behind
            // whatever they were looking at and gets dismissed unread. A dismissed
            // microphone prompt is recorded as a refusal, and macOS never asks
            // twice.
            if crate::core::voice::access() == crate::core::voice::Access::Unasked {
                crate::core::voice::request_access();
            }
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

/// Send everything printed to a file as well as wherever it was going.
///
/// Not for the user -- for us. Launched from a terminal the output is right
/// there, but a permission prompt is attributed to whatever *started* the app,
/// so anything needing a new grant has to be opened the normal way, and then
/// stdout goes nowhere. Every measurement this project has made was read out of
/// a terminal, and the first one that mattered was lost exactly this way.
#[cfg(target_os = "macos")]
fn keep_a_log() {
    use std::os::unix::io::AsRawFd;
    // Not `temp_dir`, which is a per-user folder buried under /var/folders with
    // a name nobody can type. This file exists to be tailed.
    let path = std::path::Path::new("/tmp/nudge.log");
    let Ok(file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    else {
        return;
    };
    // Only when nothing is watching. A terminal that launched us is a better
    // place for this than a file nobody opens, and duplicating it into both is
    // how you end up reading a stale copy.
    let attached = unsafe { libc::isatty(std::io::stderr().as_raw_fd()) } == 1;
    if attached {
        return;
    }
    unsafe {
        libc::dup2(file.as_raw_fd(), std::io::stdout().as_raw_fd());
        libc::dup2(file.as_raw_fd(), std::io::stderr().as_raw_fd());
    }
    std::mem::forget(file);
    println!("\n--- nudge started {} ---", std::process::id());
}

#[cfg(not(target_os = "macos"))]
fn keep_a_log() {}
