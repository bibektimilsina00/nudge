//! The Nudge account: who is using this copy of the app.
//!
//! Distinct from everything in `connect.rs`, and the distinction is worth
//! holding on to. Connecting Google there is *lending Nudge your inbox*;
//! signing in here is *saying which person this is*. They happen to use the same
//! provider and nothing else about them is the same -- different scopes,
//! different consent, different consequences for revoking.
//!
//! ## Where the two halves live
//!
//! The provider flows are per-provider and messy, so each has its own file.
//! What they all end in is a *proof* -- an ID token from Google, an access token
//! from GitHub -- and from that point there is one path: hand the proof to our
//! server, get a session token back, keep it.
//!
//! The provider's token is never stored and never used again. It did its one job
//! at the moment the server checked it.
//!
//! ## What is kept, and where
//!
//! The session token goes in the Keychain, because it is a credential and this
//! app already has exactly one place credentials go. The profile -- name, email,
//! picture -- goes in a plain file next to the rest of the config, because it is
//! not secret and because a panel that has to unlock the Keychain before it can
//! draw a name would be a panel that hesitates on every launch.
use serde::{Deserialize, Serialize};

use crate::core::tools::secret;

pub mod google;

/// The Keychain item holding the session token.
const ITEM: &str = "nudge-account";

/// Who is signed in, as the app draws them.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Account {
    /// "google" | "github". Shown, so somebody can tell which button they
    /// pressed last time -- signing in with the other one is a different
    /// account, and the app should not make that a surprise.
    pub provider: String,
    pub email: String,
    pub name: String,
    pub avatar_url: String,
}

/// What the server says when a sign-in works.
#[derive(Debug, Deserialize)]
struct SignedIn {
    token: String,
    #[allow(dead_code)]
    expires_at: String,
    user: Account,
}

fn profile() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".config/nudge/account.json")
}

/// Where the server is. Overridable so that running the API locally does not
/// mean editing a constant and remembering to put it back.
fn api() -> String {
    std::env::var("NUDGE_API").unwrap_or_else(|_| "https://nudge.runmycrew.com".into())
}

/// Which copy of the app this is, for the server's record of signed-in devices.
fn device() -> String {
    format!("Nudge {} on macOS", env!("CARGO_PKG_VERSION"))
}

/// The account this Mac is signed in as, as far as the local files know.
///
/// Deliberately does not ask the server. This is what the panel draws with on
/// launch, and it has to be instant; [`check`] is the one that finds out the
/// session was revoked, and it runs after the first paint.
pub fn current() -> Option<Account> {
    let raw = std::fs::read_to_string(profile()).ok()?;
    serde_json::from_str(&raw).ok()
}

/// The session token, for talking to our server.
pub fn token() -> Option<String> {
    secret::from_keychain(ITEM)
}

fn remember(account: &Account, token: &str) -> Result<(), String> {
    secret::to_keychain(ITEM, token).map_err(|e| format!("could not save the sign-in: {e}"))?;
    let path = profile();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("could not save the sign-in: {e}"))?;
    }
    let body = serde_json::to_string_pretty(account)
        .map_err(|e| format!("could not save the sign-in: {e}"))?;
    std::fs::write(&path, body).map_err(|e| format!("could not save the sign-in: {e}"))
}

/// Forget this device locally, whatever the server thinks.
///
/// Both halves, always. Leaving the profile behind after dropping the token is
/// what produces an app that shows somebody's name above a page telling them to
/// sign in.
fn forget_locally() {
    secret::forget_keychain(ITEM);
    let _ = std::fs::remove_file(profile());
}

/// Trade a provider's proof for a session, and keep it.
pub async fn exchange(provider: &str, proof: &str) -> Result<Account, String> {
    let reply = reqwest::Client::new()
        .post(format!("{}/api/auth/signin/{provider}", api()))
        .json(&serde_json::json!({ "proof": proof, "device": device() }))
        .send()
        .await
        .map_err(|e| format!("could not reach Nudge: {e}"))?;

    if !reply.status().is_success() {
        return Err(complaint(reply).await);
    }

    let signed_in: SignedIn = reply
        .json()
        .await
        .map_err(|e| format!("Nudge sent something unreadable: {e}"))?;
    remember(&signed_in.user, &signed_in.token)?;
    Ok(signed_in.user)
}

/// Ask the server whether this session is still good.
///
/// `Ok(None)` means it is not, and the local copy has been cleared -- a revoked
/// session and a signed-out app should look identical from here on. A network
/// failure is `Err` and changes nothing: being offline is not being signed out,
/// and treating it as such would throw somebody out of the app on a train.
pub async fn check() -> Result<Option<Account>, String> {
    let Some(token) = token() else {
        return Ok(None);
    };
    let reply = reqwest::Client::new()
        .get(format!("{}/api/auth/me", api()))
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|e| format!("could not reach Nudge: {e}"))?;

    if reply.status() == reqwest::StatusCode::UNAUTHORIZED {
        forget_locally();
        return Ok(None);
    }
    if !reply.status().is_success() {
        return Err(complaint(reply).await);
    }

    let account: Account = reply
        .json()
        .await
        .map_err(|e| format!("Nudge sent something unreadable: {e}"))?;
    // Written back so a name changed at the provider reaches the panel without
    // anybody signing out and in again.
    let _ = remember(&account, &token);
    Ok(Some(account))
}

/// Sign this device out.
///
/// The local half happens whatever the server says. A sign-out that fails
/// because the network is down and leaves somebody still signed in is the one
/// outcome nobody would accept -- most of the reasons for pressing it are
/// reasons you cannot wait for a round trip.
pub async fn sign_out() {
    if let Some(token) = token() {
        let _ = reqwest::Client::new()
            .post(format!("{}/api/auth/signout", api()))
            .bearer_auth(token)
            .send()
            .await;
    }
    forget_locally();
}

/// What went wrong, in the server's words where it has any.
async fn complaint(reply: reqwest::Response) -> String {
    let status = reply.status();
    let body: serde_json::Value = reply.json().await.unwrap_or_default();
    body.get("detail")
        .and_then(|d| d.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| format!("Nudge refused the sign-in ({status})"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_server_is_the_hosted_one_unless_told_otherwise() {
        // Not asserted against the env var being set -- this is about the
        // default being the real server rather than a localhost left in.
        if std::env::var("NUDGE_API").is_err() {
            assert_eq!(api(), "https://nudge.runmycrew.com");
        }
    }

    #[test]
    fn an_account_survives_the_json_it_is_stored_as() {
        let account = Account {
            provider: "google".into(),
            email: "sam@example.com".into(),
            name: "Sam".into(),
            avatar_url: "https://example.com/sam.png".into(),
        };
        let back: Account = serde_json::from_str(&serde_json::to_string(&account).unwrap()).unwrap();
        assert_eq!(back, account);
    }

    #[test]
    fn the_server_reply_is_read_the_way_the_server_writes_it() {
        // Mirrors `SignedIn` in server/app/routes/auth.py. The two are one
        // contract written twice, and this is the half that notices.
        let raw = r#"{
            "token": "abc",
            "expires_at": "2026-12-01T00:00:00+00:00",
            "user": {
                "provider": "github",
                "email": "",
                "name": "sam",
                "avatar_url": "https://example.com/a.png"
            }
        }"#;
        let parsed: SignedIn = serde_json::from_str(raw).unwrap();
        assert_eq!(parsed.token, "abc");
        assert_eq!(parsed.user.name, "sam");
        assert_eq!(parsed.user.email, "");
    }
}
