//! Signing in, from the panel's point of view.
//!
//! Thin, like the rest of this directory. The one thing that is genuinely here
//! rather than in `core` is the shape of GitHub's device flow: it is two calls
//! with a person in between, because somebody has to read a code off the screen
//! and type it into a browser. The panel drives that gap, so the split into
//! "start" and "wait" lives at this layer.
use tauri::{AppHandle, Emitter};

use crate::core::account::{self, Account};
use crate::core::signin;

/// The same GitHub App the integration signs in with, read from the catalogue
/// rather than written down a second time.
///
/// Two copies of a client id is two things to change and one that gets missed,
/// and the failure when they disagree is a device flow that starts, shows a
/// code, and is rejected at the end for belonging to a different application.
fn github_client() -> Result<&'static str, String> {
    crate::core::connect::offer("github")
        .and_then(|offer| offer.sign_in)
        .ok_or_else(|| "GitHub sign-in is not configured".to_string())
}

/// Who this Mac is signed in as, from local files only.
///
/// The panel's first paint calls this, so it must not wait on the network.
#[tauri::command]
pub fn account() -> Option<Account> {
    account::current()
}

/// Ask the server whether the stored session is still good.
///
/// Called after the first paint. `None` means it is not -- revoked, expired, or
/// signed out elsewhere -- and the local copy is gone by the time this returns.
#[tauri::command]
pub async fn check_account(app: AppHandle) -> Option<Account> {
    match account::check().await {
        Ok(found) => {
            // Told either way. The panel is already drawn by now, so this is
            // what moves it to the sign-in page when a session has gone.
            app.emit("account", &found).ok();
            found
        }
        // Offline. Being unreachable is not being signed out, so nothing is
        // emitted and the panel keeps drawing whoever it already had.
        Err(why) => {
            eprintln!("account: could not check the session -- {why}");
            account::current()
        }
    }
}

/// The whole Google flow: browser, redirect, exchange.
///
/// One call because there is no gap in it that needs a person. It returns when
/// they have finished in the browser, which can be minutes -- the panel shows a
/// waiting state for exactly this reason.
#[tauri::command]
pub async fn sign_in_google(app: AppHandle) -> Result<Account, String> {
    let proof = account::google::proof().await?;
    let who = account::exchange("google", &proof).await?;
    app.emit("account", Some(&who)).ok();
    Ok(who)
}

/// The whole GitHub flow: ask for a code, show it, wait for them to type it.
///
/// One call, like Google's, even though there is a person in the middle. The
/// obvious alternative -- return the code, then have the panel call back to
/// start polling -- cannot work: the device code is the half GitHub expects to
/// stay private, so `Waiting` does not serialise it and the panel never has it
/// to hand back. Emitting the part that is meant to be read, and keeping the
/// part that is not, is the whole shape of this flow.
#[tauri::command]
pub async fn sign_in_github(app: AppHandle) -> Result<Account, String> {
    let client = github_client()?;
    // No scope string. A GitHub App's reach is the permissions it was installed
    // with, so asking here would be asking for something it does not grant that
    // way -- the same reasoning as `sign_in_begin` in `commands/connect.rs`.
    let waiting = signin::begin(client, "").await?;
    // The code, the URL and how long it lasts. Everything the panel draws.
    app.emit("github_code", &waiting).ok();

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(waiting.expires_in);
    let granted = loop {
        if std::time::Instant::now() > deadline {
            // GitHub would start answering `expired_token` here anyway. Saying
            // it ourselves means the panel can offer a fresh code rather than
            // showing a dead one next to a shrug.
            return Err("that code expired. Try again for a new one.".into());
        }
        match signin::poll(client, &waiting.device_code).await {
            signin::Poll::Token(granted) => break granted,
            signin::Poll::Pending { interval } => {
                tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
            }
            signin::Poll::Stopped(why) => return Err(why),
        }
    };

    let who = account::exchange("github", &granted.access).await?;
    app.emit("account", Some(&who)).ok();
    Ok(who)
}

/// Sign out, locally whatever the server says.
#[tauri::command]
pub async fn sign_out(app: AppHandle) {
    account::sign_out().await;
    app.emit("account", None::<Account>).ok();
}
