//! Noticing a newer build, and taking it.
//!
//! The plugin does the work -- asking the endpoint, checking the minisign
//! signature, swapping the bundle. What is here is the policy around it, which
//! is the part with opinions in it.
//!
//! **Nothing installs itself.** An app that replaces its own binary while
//! somebody is mid-sentence, or mid-agent, is an app that loses their work to
//! be helpful. So the check is quiet and automatic; the install is a button.
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::UpdaterExt;

/// How long after launch to look.
///
/// Not at launch. The first seconds after a menu-bar app starts are the ones
/// where somebody is waiting for the hotkey to work, and a network round trip
/// on the runtime's threads is a thing that can be done later for free.
const SETTLE: std::time::Duration = std::time::Duration::from_secs(20);

/// What the panel is told when there is something newer.
#[derive(Debug, Clone, Serialize)]
pub struct Available {
    pub version: String,
    pub notes: String,
}

/// Look once, in the background, and say so if there is something.
pub fn look(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(SETTLE).await;

        // Pointed somewhere else for testing. The endpoint is compiled into the
        // config, which makes the install path the one thing that cannot be
        // tried without publishing to production first -- and "can it install"
        // is exactly the half that strands people if it is wrong.
        let builder = match std::env::var("NUDGE_UPDATE_FROM") {
            Ok(url) if !url.is_empty() => {
                eprintln!("update: asking {url} instead of the configured endpoint");
                app.updater_builder().endpoints(
                    url.split(',')
                        .filter_map(|u| u.trim().parse().ok())
                        .collect(),
                )
            }
            _ => Ok(app.updater_builder()),
        };
        let updater = match builder.and_then(|b| b.build()) {
            Ok(u) => u,
            // No endpoint configured, or a build with no public key. Not worth
            // saying out loud on every launch of a development build.
            Err(e) => return eprintln!("update: not configured ({e})"),
        };

        match updater.check().await {
            Ok(Some(found)) => {
                let version = found.version.clone();
                // Checked here as well, rather than trusting the answer. The
                // server decides what to offer; this decides what to take, and
                // an endpoint that starts serving a downgrade should not be
                // able to walk everybody backwards.
                if !crate::core::newer::newer(&app.package_info().version.to_string(), &version) {
                    return eprintln!("update: {version} is not newer than what is running");
                }
                eprintln!("update: {version} is available");
                let _ = app.emit(
                    "update",
                    Available {
                        version,
                        notes: found.body.clone().unwrap_or_default(),
                    },
                );
                *WAITING.lock().unwrap() = Some(found);
            }
            Ok(None) => eprintln!("update: up to date"),
            // Said, not shown. Somebody's café wifi is not their problem to
            // solve, and a dialog about it every launch is noise.
            Err(e) => eprintln!("update: could not check ({e})"),
        }
    });
}

/// The update found at the last check, held until somebody says yes.
static WAITING: std::sync::Mutex<Option<tauri_plugin_updater::Update>> =
    std::sync::Mutex::new(None);

/// Take it. Downloads, verifies, installs, and restarts.
#[tauri::command]
pub async fn take_update(app: AppHandle) -> Result<(), String> {
    let found = WAITING.lock().unwrap().take();
    let Some(found) = found else {
        return Err("nothing to install".into());
    };

    eprintln!("update: installing {}", found.version);
    found
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| {
            eprintln!("update: failed -- {e}");
            e.to_string()
        })?;

    // Only reached if the install worked; otherwise the error above is the end
    // of it and the app carries on being the version it already was.
    app.restart();
}
