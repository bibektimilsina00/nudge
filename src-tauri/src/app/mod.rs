//! The Tauri adapter: windows, tray, hotkeys, IPC. Everything here exists to plug
//! `core` into the OS, and `core` never reaches back the other way.

pub(crate) mod agent;
pub mod commands;
mod input;
mod state;
pub mod supervise;
mod tour;
mod ui;
pub mod update;

use crate::config::Config;
use crate::core::run::session::Nudge;
use state::{
    Background, Docked, Flag, Grants, Hotkey, Look, Mic, Reviewing, Screen, Settle, Suggesting,
    Voice, VoiceMode,
};
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
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
            let reviewing = cfg.review;
            keep_a_log();
            // Before anything else. If the last run was killed rather than
            // closed, its pointer is still hidden and nothing else will fix it.
            crate::core::screen::click::show_the_pointer();
            // What this run is holding, so nothing can carry it back out.
            //
            // Collected before the providers are built, because after that the
            // key is inside one and this is the last place it is plainly a
            // string. Env vars as well as the config file: most people set the
            // variable and never write the key down here at all.
            crate::core::tools::secret::remember(
                cfg.api_key.clone().into_iter().chain(
                    ["GEMINI_API_KEY", "ANTHROPIC_API_KEY", "OPENAI_API_KEY"]
                        .iter()
                        .filter_map(|k| std::env::var(k).ok()),
                ),
            );

            let voice = Voice(VoiceMode::from_config(cfg.speak, &cfg.speech_engine).into());
            let nudge_look = cfg.companion.clone();
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
            #[cfg(target_os = "macos")]
            ui::native::watch_spaces(handle);
            app.manage(Mic::default());
            app.manage(voice);
            app.manage(Settle(std::sync::Mutex::new(None)));
            app.manage(Grants::default());
            // What it actually did, beside everything else it keeps.
            app.manage(crate::core::audit::Audit::open());
            app.manage(crate::app::state::Offering::default());
            app.manage(crate::core::offers::Offers::load());
            app.manage(Background::default());
            // Starts undocked: an app that does nothing until you find a button is
            // an app most people never see working.
            app.manage(Docked(Flag::new(false)));
            app.manage(Reviewing(Flag::new(reviewing)));
            app.manage(commands::Connections {
                path: crate::core::connect::store(),
            });
            app.manage(Hotkey(std::sync::Mutex::new(hotkey.clone())));
            app.manage(Suggesting(Flag::new(true)));
            app.manage(Look(std::sync::Mutex::new(
                nudge_look.unwrap_or_else(|| "cat".into()),
            )));
            let agents = crate::core::run::agent::Agents::default();
            // Before it is managed, so nothing can look at an empty list first.
            agents.remember();
            app.manage(agents);

            // Off the startup path on purpose. `npx` may spend a minute fetching
            // a server it has never run, and the hotkey has to work during that
            // minute -- so the window appears, the assistant answers, and the
            // tools arrive when they arrive.
            let starting = handle.clone();
            let shown = hotkey.clone();
            tauri::async_runtime::spawn(async move {
                starting.state::<Nudge>().connect_tools().await;
                let n = starting.state::<Nudge>().tools().len();
                if n > 0 {
                    println!("nudge: {n} tools from mcp servers");
                }
                // The menu was built before any of this existed, listing each
                // server as "starting…". Now it can say what they brought.
                ui::tray::refresh(&starting, &shown);
            });

            println!(
                "nudge: windows = {:?}",
                app.webview_windows().keys().collect::<Vec<_>>()
            );
            ui::panel::dock_to_notch(handle);
            agent::place_window(handle);
            ui::tray::install(handle, &hotkey)?;
            input::inject::start(handle);
            ui::connect::place(handle);

            // `NUDGE_OFFER=GitHub` puts a connect prompt on screen and leaves it
            // there. *When* one of these should appear is not decided yet, and
            // the shape is worth getting right first -- so this is how it is
            // looked at, the same way the agent card is.
            if let Some(want) = std::env::var_os("NUDGE_OFFER") {
                let want = want.to_string_lossy().to_lowercase();
                if let Some(offer) = ui::connect::catalogue()
                    .into_iter()
                    .find(|o| o.name.to_lowercase() == want)
                {
                    let showing = handle.clone();
                    tauri::async_runtime::spawn(async move {
                        // After the event loop exists, for the reason the agent
                        // card needs it: `show` reaches the window through the
                        // main thread, and during setup there is nothing to reach.
                        tokio::time::sleep(std::time::Duration::from_millis(700)).await;
                        showing
                            .state::<crate::app::state::Offering>()
                            .set(offer.clone());
                        ui::connect::ask(&showing, &offer);
                        println!("nudge: offering {} (NUDGE_OFFER)", offer.name);
                    });
                }
            }

            // Anything that was mid-task when Nudge last ended has come back as
            // interrupted, and the offer to carry on has to reach the window.
            //
            // From a task, for the same reason the example below is: `publish`
            // reaches the window through `run_on_main_thread`, which posts to an
            // event loop that does not exist yet during setup -- so publishing
            // here would simply be dropped, the run would sit in the list, and
            // nothing would appear.
            if handle
                .state::<crate::core::run::agent::Agents>()
                .list()
                .iter()
                .any(|a| a.interrupted())
            {
                let back = handle.clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(600)).await;
                    agent::publish(&back);
                });
            }

            // `NUDGE_CARD=1` puts a worked example on the agent card and leaves it
            // there, so how it looks can be changed without racing a real task.
            //
            // Published from a task rather than from here. `publish` reaches the
            // window through `run_on_main_thread`, which posts to an event loop
            // that does not exist yet during setup -- so the first attempt was
            // simply dropped, the agent existed, and the card never appeared.
            if std::env::var_os("NUDGE_CARD").is_some() {
                let showing = handle.clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(600)).await;
                    let id = showing
                        .state::<crate::core::run::agent::Agents>()
                        .demonstrate();
                    println!("nudge: agent#{id} is a standing example (NUDGE_CARD)");
                    agent::publish(&showing);
                });
            }
            input::cursor::follow(handle);
            input::hotkey::install(handle)?;
            input::hotkey::bind(handle, &hotkey)?;
            update::look(handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::account,
            commands::check_account,
            commands::sign_in_google,
            commands::sign_in_github,
            commands::sign_out,
            commands::start,
            commands::advance,
            commands::cancel,
            commands::docked,
            commands::set_docked,
            commands::fit_panel,
            commands::fit_agents,
            commands::pending_offer,
            commands::answer_offer,
            commands::skills,
            commands::open_skills_folder,
            commands::set_open_size,
            commands::model_access,
            commands::set_api_key,
            commands::notch_height,
            commands::open_artifact,
            commands::voice_mode,
            commands::set_voice_mode,
            commands::microphone,
            commands::version,
            commands::reviewing,
            commands::set_reviewing,
            update::take_update,
            commands::quit,
            commands::shot_for_report,
            commands::send_report,
            commands::reach,
            commands::set_reach,
            commands::brain,
            commands::retune,
            commands::agent_setup,
            commands::set_suggesting,
            commands::pick_workspace,
            commands::shortcuts,
            commands::set_shortcut,
            commands::permits,
            commands::relaunch,
            commands::ask_permit,
            commands::open_permit,
            commands::look,
            commands::set_look,
            commands::servers,
            commands::agents,
            commands::answer_agent,
            commands::stop_agent,
            commands::dismiss_agent,
            commands::trail,
            commands::resume_agent,
            commands::connections,
            commands::connect,
            commands::choose_tools,
            commands::sign_in_begin,
            commands::disconnect,
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
