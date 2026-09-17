//! Signing in with Google, from a program with no server behind it.
//!
//! The shape is the one Google specifies for installed applications: open the
//! browser at Google, have it redirect to a port on this machine, and catch the
//! code there. No secret is needed to be secret -- a desktop client's is shipped
//! inside every copy of the app and Google treats it as public -- so the thing
//! that actually proves the code was redeemed by whoever asked for it is PKCE.
//!
//! Why PKCE is not optional here: the redirect goes to `127.0.0.1`, which any
//! other program on this Mac can also listen on. Without a verifier, whoever
//! wins the race for the port holds a code that Google will exchange. With one,
//! a stolen code is worth nothing to anybody who did not generate the secret it
//! is bound to.
//!
//! What comes back is an ID token -- a signed statement from Google about who
//! signed in. It is sent to our server and nothing else; the access token beside
//! it is dropped, because reading anybody's mail is not what signing in is for.
use base64::Engine as _;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::{Duration, Instant};

/// How long the browser has. Fifteen minutes is long enough to find a password
/// manager and short enough that an abandoned sign-in does not hold a port for
/// the rest of the session.
const PATIENCE: Duration = Duration::from_secs(900);

/// Enough for signing in and nothing more.
///
/// Notably absent is every Workspace scope. Connecting Gmail is a separate act
/// with a separate consent screen, and rolling the two together would mean
/// asking for somebody's inbox at the moment they are deciding whether to trust
/// the app at all.
const SCOPE: &str = "openid email profile";

/// The desktop OAuth client, as `connect.rs` already reads it.
struct Keys {
    id: String,
    secret: String,
}

fn keys() -> Result<Keys, String> {
    let path = dirs::home_dir()
        .ok_or("no home directory")?
        .join(".config/gcp-oauth.keys.json");
    let raw = std::fs::read_to_string(&path)
        .map_err(|_| format!("no Google client at {}", path.display()))?;
    let parsed: serde_json::Value =
        serde_json::from_str(&raw).map_err(|_| "the Google client file is not JSON".to_string())?;
    // Google writes either key depending on which button made the file.
    let client = parsed
        .get("installed")
        .or_else(|| parsed.get("web"))
        .ok_or("the Google client file has no client in it")?;
    Ok(Keys {
        id: client["client_id"].as_str().unwrap_or_default().to_string(),
        secret: client["client_secret"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
    })
}

fn random() -> Result<String, String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|e| format!("no randomness available: {e}"))?;
    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes))
}

fn challenge(verifier: &str) -> String {
    use sha2::{Digest, Sha256};
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

/// Percent-encoding, for the handful of characters a URL query cannot carry.
///
/// Small on purpose. The values encoded here are a scope string, a redirect and
/// two base64url blobs; pulling in a crate to escape three characters would be
/// more surface than the problem has.
fn escaped(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn unescaped(value: &str) -> String {
    let mut out = String::new();
    let mut bytes = value.bytes();
    while let Some(b) = bytes.next() {
        match b {
            b'+' => out.push(' '),
            b'%' => {
                let hex: String = bytes.by_ref().take(2).map(|c| c as char).collect();
                match u8::from_str_radix(&hex, 16) {
                    Ok(c) => out.push(c as char),
                    Err(_) => out.push('%'),
                }
            }
            _ => out.push(b as char),
        }
    }
    out
}

/// One parameter out of a query string.
fn param(query: &str, want: &str) -> Option<String> {
    query.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        (key == want).then(|| unescaped(value))
    })
}

/// What the browser lands on once Google sends it back.
///
/// Served from here rather than from our site because the redirect has to be
/// loopback -- and because a sign-in that finishes while the network is down
/// should still finish.
fn landing(message: &str) -> String {
    let body = format!(
        "<!doctype html><meta charset=utf-8><title>Nudge</title>\
         <style>html{{height:100%}}body{{margin:0;height:100%;display:grid;place-items:center;\
         background:#0b0b0d;color:#f4f4f5;font:15px/1.6 -apple-system,BlinkMacSystemFont,sans-serif}}\
         p{{opacity:.55;margin:.4rem 0 0}}</style>\
         <div><strong>{message}</strong><p>You can close this tab.</p></div>"
    );
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    )
}

/// Sit on the port until the browser arrives, or until patience runs out.
///
/// Non-blocking with a deadline rather than a blocking accept, so abandoning the
/// sign-in leaves nothing behind. A thread parked forever on `accept` is only
/// freed by a connection that is never coming.
fn catch(listener: TcpListener, state: &str) -> Result<String, String> {
    listener
        .set_nonblocking(true)
        .map_err(|e| format!("could not listen: {e}"))?;
    let deadline = Instant::now() + PATIENCE;

    while Instant::now() < deadline {
        let mut stream = match listener.accept() {
            Ok((stream, _)) => stream,
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
            Err(e) => return Err(format!("could not listen: {e}")),
        };

        let mut buffer = [0u8; 2048];
        let read = stream.read(&mut buffer).unwrap_or(0);
        let request = String::from_utf8_lossy(&buffer[..read]);
        // "GET /?code=...&state=... HTTP/1.1"
        let target = request.split_whitespace().nth(1).unwrap_or("");
        let query = target.split_once('?').map(|(_, q)| q).unwrap_or("");

        // Browsers ask for /favicon.ico off their own bat. Answering it as if it
        // were the redirect would end the sign-in before it happened.
        if param(query, "code").is_none() && param(query, "error").is_none() {
            let _ = stream.write_all(landing("Waiting for Google...").as_bytes());
            continue;
        }

        let answer = if let Some(problem) = param(query, "error") {
            Err(match problem.as_str() {
                "access_denied" => "sign-in was cancelled".to_string(),
                other => format!("Google refused: {other}"),
            })
        } else if param(query, "state").as_deref() != Some(state) {
            // Somebody else's redirect arrived at our port. Refusing is the
            // whole reason the state is there.
            Err("that sign-in did not come from this window".to_string())
        } else {
            Ok(param(query, "code").unwrap_or_default())
        };

        let _ = stream.write_all(
            landing(match &answer {
                Ok(_) => "Signed in.",
                Err(why) => why,
            })
            .as_bytes(),
        );
        let _ = stream.flush();
        return answer;
    }
    Err("sign-in timed out".into())
}

/// Run the whole flow and come back with Google's statement of who signed in.
pub async fn proof() -> Result<String, String> {
    let keys = keys()?;
    let listener =
        TcpListener::bind("127.0.0.1:0").map_err(|e| format!("could not open a port: {e}"))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("could not open a port: {e}"))?
        .port();
    let redirect = format!("http://127.0.0.1:{port}");

    let verifier = random()?;
    let state = random()?;
    let url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}\
         &response_type=code&scope={}&code_challenge={}&code_challenge_method=S256&state={}",
        escaped(&keys.id),
        escaped(&redirect),
        escaped(SCOPE),
        escaped(&challenge(&verifier)),
        escaped(&state),
    );

    open::that(&url).map_err(|e| format!("could not open the browser: {e}"))?;

    let watching = state.clone();
    let code = tokio::task::spawn_blocking(move || catch(listener, &watching))
        .await
        .map_err(|e| format!("sign-in stopped unexpectedly: {e}"))??;

    let reply = reqwest::Client::new()
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", keys.id.as_str()),
            ("client_secret", keys.secret.as_str()),
            ("code", code.as_str()),
            ("code_verifier", verifier.as_str()),
            ("grant_type", "authorization_code"),
            ("redirect_uri", redirect.as_str()),
        ])
        .send()
        .await
        .map_err(|e| format!("could not reach Google: {e}"))?;

    let body: serde_json::Value = reply
        .json()
        .await
        .map_err(|e| format!("Google sent something unreadable: {e}"))?;

    body.get("id_token")
        .and_then(|t| t.as_str())
        .map(str::to_string)
        .ok_or_else(|| {
            // Google's own wording, when it has one. This is the error somebody
            // reads when a redirect URI is not registered, and paraphrasing it
            // would cost them the one clue that names the fix.
            body.get("error_description")
                .or_else(|| body.get("error"))
                .and_then(|e| e.as_str())
                .unwrap_or("Google did not return a sign-in")
                .to_string()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_challenge_is_the_one_google_specifies() {
        // The worked example from RFC 7636 appendix B. If this drifts, every
        // sign-in fails at the token exchange with a message about the verifier.
        assert_eq!(
            challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn a_query_value_survives_the_round_trip() {
        let awkward = "a code/with+odd chars&=%";
        assert_eq!(unescaped(&escaped(awkward)), awkward);
    }

    #[test]
    fn a_parameter_is_found_by_its_whole_name() {
        let query = "state=abc&code=xyz&scope=openid";
        assert_eq!(param(query, "code").as_deref(), Some("xyz"));
        assert_eq!(param(query, "state").as_deref(), Some("abc"));
        // Not a prefix match: "code" must not answer for "code_challenge".
        assert_eq!(param("code_challenge=no", "code"), None);
        assert_eq!(param(query, "error"), None);
    }

    #[test]
    fn a_redirect_carrying_a_plus_is_decoded_as_a_space_not_a_plus() {
        assert_eq!(param("name=one+two", "name").as_deref(), Some("one two"));
    }
}
