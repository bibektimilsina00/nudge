//! Signing in to a service that does not want a pasted token.
//!
//! GitHub's device flow, which exists for exactly this shape of program: a
//! desktop app cannot keep a client secret, because the secret ships inside a
//! file anybody can download and read. Google solved that with PKCE; GitHub did
//! not, and offers this instead -- no secret, no redirect, no local web server.
//!
//! What happens: ask GitHub for a code, show the person a short string to type at
//! a URL, then poll until they have. The client id is public and is meant to be,
//! so there is nothing here worth hiding.
//!
//! The token that comes back goes straight to the Keychain and is handed to the
//! MCP server in the same environment variable a pasted one would have used --
//! GitHub's API does not distinguish them, so nothing downstream changes.
use serde::{Deserialize, Serialize};

/// What GitHub says when asked to start.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Waiting {
    /// The short string somebody types. Shown, not used by us.
    pub user_code: String,
    /// Where they type it.
    pub verification_uri: String,
    /// Ours, and not theirs to see -- this is what the polling asks about.
    #[serde(skip_serializing)]
    pub device_code: String,
    /// Seconds between polls. GitHub says five and means it: poll faster and it
    /// answers `slow_down` and lengthens the interval.
    pub interval: u64,
    /// Seconds before the code stops working, usually 900.
    pub expires_in: u64,
}

#[derive(Debug, Deserialize)]
struct Answer {
    access_token: Option<String>,
    error: Option<String>,
    interval: Option<u64>,
}

/// Ask GitHub to start a sign-in.
pub async fn begin(client_id: &str, scope: &str) -> Result<Waiting, String> {
    let res = crate::core::http()
        .post("https://github.com/login/device/code")
        .header("Accept", "application/json")
        .form(&[("client_id", client_id), ("scope", scope)])
        .send()
        .await
        .map_err(|e| format!("could not reach GitHub: {e}"))?;

    let text = res.text().await.map_err(|e| e.to_string())?;
    serde_json::from_str::<Waiting>(&text)
        // GitHub answers a bad client id with a JSON error rather than a status,
        // so the parse failing is the likeliest way to learn the id is wrong.
        .map_err(|_| format!("GitHub refused to start a sign-in: {text}"))
}

/// What a single poll found.
pub enum Poll {
    /// Not yet. Wait `interval` seconds and ask again.
    Pending { interval: u64 },
    /// Done, and this is the token.
    Token(String),
    /// Over, for a reason worth showing: refused, expired, or a real failure.
    Stopped(String),
}

/// Ask once whether they have finished.
///
/// Deliberately one step rather than a loop: the caller owns the waiting, so it
/// can be cancelled, reported on, and kept off whatever thread it must not block.
pub async fn poll(client_id: &str, device_code: &str) -> Poll {
    let res = crate::core::http()
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .form(&[
            ("client_id", client_id),
            ("device_code", device_code),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ])
        .send()
        .await;

    let Ok(res) = res else {
        // A dropped connection mid-wait is not a refusal. Treat it as "not yet"
        // so a flaky minute does not throw away a sign-in somebody is part way
        // through, and let the expiry be the thing that ends it.
        return Poll::Pending { interval: 5 };
    };
    let Ok(a) = res.json::<Answer>().await else {
        return Poll::Pending { interval: 5 };
    };

    if let Some(token) = a.access_token {
        return Poll::Token(token);
    }
    match a.error.as_deref() {
        Some("authorization_pending") => Poll::Pending { interval: 5 },
        // GitHub sets the new interval itself when we have been too eager.
        Some("slow_down") => Poll::Pending {
            interval: a.interval.unwrap_or(10),
        },
        Some("expired_token") => Poll::Stopped("That code expired. Start again.".into()),
        Some("access_denied") => Poll::Stopped("Sign-in was refused on GitHub.".into()),
        Some(other) => Poll::Stopped(format!("GitHub said: {other}")),
        None => Poll::Stopped("GitHub answered with neither a token nor a reason.".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_device_code_is_never_serialised_towards_the_window() {
        // It is the half that proves the sign-in is ours. The user code is meant
        // to be read aloud; this one is not, and a struct that goes to the UI is
        // the easiest place to leak it by accident.
        let w = Waiting {
            user_code: "WDJB-MJHT".into(),
            verification_uri: "https://github.com/login/device".into(),
            device_code: "secret-half".into(),
            interval: 5,
            expires_in: 900,
        };
        let json = serde_json::to_string(&w).unwrap();
        assert!(json.contains("WDJB-MJHT"));
        assert!(!json.contains("secret-half"), "{json}");
    }
}
