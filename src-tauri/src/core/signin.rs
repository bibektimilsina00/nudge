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
    /// Only sent when the app expires user tokens, and then it is the only way
    /// back once it has. Dropping it made every sign-in work for eight hours and
    /// then fail with "Bad credentials", which reads as a revoked token rather
    /// than an expired one and sends you looking in the wrong place.
    refresh_token: Option<String>,
    /// Seconds. GitHub says 28800 -- eight hours -- when expiry is on.
    expires_in: Option<u64>,
    error: Option<String>,
    interval: Option<u64>,
}

/// A token, and what is needed to get another one.
#[derive(Debug, Clone)]
pub struct Granted {
    pub access: String,
    /// Absent when the app does not expire tokens, in which case `access` is
    /// good until somebody revokes it.
    pub refresh: Option<String>,
    /// Milliseconds since the epoch, or `None` when it does not expire.
    pub until: Option<u64>,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

impl Granted {
    fn from(a: Answer) -> Option<Self> {
        Some(Granted {
            access: a.access_token?,
            refresh: a.refresh_token,
            // A minute early on purpose: a token that expires between the check
            // and the call is the same bug with a smaller window.
            until: a
                .expires_in
                .map(|s| now_ms() + s.saturating_sub(60) * 1000),
        })
    }

    /// Whether this needs replacing before it is used again.
    pub fn stale(&self) -> bool {
        self.until.is_some_and(|u| now_ms() >= u)
    }
}

/// Trade a refresh token for a new one.
///
/// The refresh token is single-use -- GitHub returns a new one each time and
/// invalidates the old -- so whatever calls this has to store what comes back or
/// the next refresh fails and the only way out is signing in again.
pub async fn refresh(client_id: &str, refresh: &str) -> Result<Granted, String> {
    let res = crate::core::http()
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .form(&[
            ("client_id", client_id),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh),
        ])
        .send()
        .await
        .map_err(|e| format!("could not reach GitHub: {e}"))?;

    let a: Answer = res.json().await.map_err(|e| e.to_string())?;
    if let Some(why) = a.error {
        return Err(format!("GitHub would not renew that sign-in: {why}"));
    }
    Granted::from(a).ok_or_else(|| "GitHub renewed nothing".to_string())
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
    /// Done, and this is the token and how to renew it.
    Token(Granted),
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

    if a.access_token.is_some() {
        return match Granted::from(a) {
            Some(g) => Poll::Token(g),
            None => Poll::Stopped("GitHub answered with an empty token.".into()),
        };
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
    fn a_token_with_no_expiry_never_goes_stale() {
        // Apps that do not expire user tokens send no `expires_in`, and treating
        // that as "expired now" would refresh on every single session.
        let g = Granted { access: "t".into(), refresh: None, until: None };
        assert!(!g.stale());
    }

    #[test]
    fn a_token_is_stale_a_little_before_it_actually_expires() {
        // The margin is the point: a token that expires between the check and
        // the call is the same bug with a smaller window.
        let a = Answer {
            access_token: Some("t".into()),
            refresh_token: Some("r".into()),
            expires_in: Some(30),
            error: None,
            interval: None,
        };
        let g = Granted::from(a).unwrap();
        assert!(g.stale(), "30s of life is inside the one-minute margin");

        let b = Answer {
            access_token: Some("t".into()),
            refresh_token: Some("r".into()),
            expires_in: Some(28_800),
            error: None,
            interval: None,
        };
        assert!(!Granted::from(b).unwrap().stale(), "a fresh 8 hour token is not stale");
    }

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
