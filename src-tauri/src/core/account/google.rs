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
    // Built into the binary, because a copy of this app on somebody else's Mac
    // has none of this machine's files.
    //
    // Signing in with Google failed for the first person who downloaded a
    // release with "no Google client at /Users/<them>/.config/gcp-oauth.keys.json"
    // -- a path that exists here and nowhere else. GitHub's client id has
    // always shipped inside the app; Google's was being read off the developer's
    // disk, so it worked in every test and for nobody else.
    //
    // Compiled in rather than written down, because this repository is public.
    // Google says a desktop client's secret "is obviously not treated as a
    // secret" and expects it embedded, which is true of the shipped binary and
    // not a reason to put it in a public git history.
    //
    // The file still answers when nothing was compiled in, which is what a
    // checkout without the build secrets has.
    match (
        option_env!("NUDGE_GOOGLE_CLIENT_ID"),
        option_env!("NUDGE_GOOGLE_CLIENT_SECRET"),
    ) {
        (Some(id), Some(secret)) if !id.is_empty() && !secret.is_empty() => {
            return Ok(Keys {
                id: id.to_string(),
                secret: secret.to_string(),
            });
        }
        _ => {}
    }

    let path = dirs::home_dir()
        .ok_or("no home directory")?
        .join(".config/gcp-oauth.keys.json");
    let raw = std::fs::read_to_string(&path).map_err(|_| {
        format!(
            "this build has no Google sign-in configured, and there is no \
                 client at {}",
            path.display()
        )
    })?;
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

/// Which of the three moments this page is.
#[derive(Clone, Copy)]
enum Tone {
    /// It worked.
    Good,
    /// Something in the browser asked before Google came back -- a favicon, a
    /// refresh. Nothing has happened yet.
    Waiting,
    /// Refused, cancelled, or aimed at the wrong window.
    Bad,
}

impl Tone {
    /// The accent, taken from the icon rather than from the app's interface.
    ///
    /// Nudge's own amber, which is the cat's nose and the shadow it sits in. The
    /// interface blue would have been the easy pick and would have made this
    /// page look like every other confirmation screen; the icon is the only
    /// thing on here that belongs to nobody else, so the colour comes from it.
    fn accent(self) -> &'static str {
        match self {
            Tone::Good => "#e0913a",
            Tone::Waiting => "#8b8b93",
            Tone::Bad => "#dd4060",
        }
    }

    /// The badge on the corner of the icon, the way macOS marks one.
    ///
    /// Two strokes and no library, which is the exception a self-contained page
    /// has to make: nothing here may be fetched, so an icon package is not an
    /// option. Kept to marks simple enough that drawing them is not a risk.
    fn badge(self) -> &'static str {
        match self {
            Tone::Good => r#"<polyline points="9,14.5 12.5,18 19,10.5"/>"#,
            Tone::Bad => r#"<path d="M10 10 L18 18 M18 10 L10 18"/>"#,
            Tone::Waiting => "",
        }
    }
}

/// Text that came from somewhere else, made safe to put in a page.
///
/// The failure message can be Google's own words, which is the whole reason it
/// is worth showing -- and the whole reason it cannot go in raw.
fn escaped_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The app icon, carried inside the binary and inlined into the page.
///
/// Two sizes, because the page needs it twice for different jobs: 256px is what
/// a 92px picture needs to stay sharp on a Retina display, and 32px is all a
/// tab icon will ever draw. Inlining the big one for both put 26KB of base64
/// into the page to render it at sixteen points.
///
/// Both are files `tauri.conf.json` lists, which is the reason to prefer them
/// over `256x256.png`: the sizes nothing references drifted a whole icon behind
/// the rest of the set and nobody noticed for a week.
///
/// Encoded once each. A sign-in bounces through here several times while the
/// browser asks for a favicon and settles the redirect, and re-encoding per
/// request would be work for nothing.
fn icon(px: u32) -> &'static str {
    static BIG: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    static SMALL: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    let encode = |bytes: &[u8]| {
        format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(bytes)
        )
    };
    match px {
        32 => SMALL.get_or_init(|| encode(include_bytes!("../../../icons/32x32.png"))),
        _ => BIG.get_or_init(|| encode(include_bytes!("../../../icons/128x128@2x.png"))),
    }
}

/// What the browser lands on once Google sends it back.
///
/// Served from here rather than from our site because the redirect has to be
/// loopback, and because a sign-in that finishes while the network is down
/// should still finish. Nothing is fetched: no font, no script, no stylesheet,
/// and the icon is inlined rather than linked.
///
/// It follows the browser's own light or dark setting instead of being dark
/// because the app is. Somebody on a light Mac who clicks a button and gets a
/// black tab has been handed a page that looks like it belongs to something
/// else, at the exact moment it needs to look like it belongs to us.
fn landing(tone: Tone, title: &str, sub: &str) -> String {
    let body = PAGE
        .replace("__ACCENT__", tone.accent())
        .replace("__BADGE__", tone.badge())
        .replace("__FAVICON__", icon(32))
        .replace("__ICON__", icon(256))
        .replace("__TITLE__", &escaped_html(title))
        .replace("__SUB__", &escaped_html(sub));
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    )
}

/// One template, six holes. Placeholders rather than `format!` because the
/// stylesheet is mostly braces, and escaping every one of them to keep a
/// formatter happy makes the CSS unreadable for no gain.
const PAGE: &str = r##"<!doctype html>
<html lang="en"><head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Nudge</title>
<link rel="icon" href="__FAVICON__">
<style>
  :root {
    color-scheme: light dark;
    --accent: __ACCENT__;
    --bg: #fbfbfc;
    --fg: #101013;
    --muted: #6b6b73;
    --lift: 0 10px 28px rgba(0, 0, 0, .12), 0 1px 3px rgba(0, 0, 0, .07);
  }
  @media (prefers-color-scheme: dark) {
    :root {
      --bg: #0a0a0c;
      --fg: #f4f4f6;
      --muted: rgba(255, 255, 255, .5);
      --lift: 0 10px 30px rgba(0, 0, 0, .5);
    }
  }
  * { box-sizing: border-box }
  html, body { height: 100% }
  body {
    margin: 0; min-height: 100dvh; display: grid; place-items: center;
    background: var(--bg); color: var(--fg);
    font: 400 15px/1.6 -apple-system, BlinkMacSystemFont, "SF Pro Text", system-ui, sans-serif;
    -webkit-font-smoothing: antialiased;
  }
  /* Flat. The icon carries the colour, and the one accent on the page is the
     badge on its corner; a wash behind it was one decoration too many. */
  main { width: min(21rem, 86vw); padding: 2rem; text-align: center }
  .app { position: relative; width: 92px; margin: 0 auto 1.7rem }
  .app img { display: block; width: 92px; height: 92px; border-radius: 21px; box-shadow: var(--lift) }
  /* Badged on the corner, the way macOS marks an icon that has something to
     say. It is the app being marked, not a tick floating on its own. */
  .badge {
    position: absolute; right: -5px; bottom: -5px; width: 32px; height: 32px;
    border-radius: 50%; background: var(--accent);
    border: 3px solid var(--bg); box-shadow: 0 3px 10px rgba(0, 0, 0, .28);
  }
  .badge svg { display: block; width: 100%; height: 100%; fill: none; stroke: #fff;
               stroke-width: 2.6; stroke-linecap: round; stroke-linejoin: round }
  h1 { margin: 0; font-size: 21px; font-weight: 600; letter-spacing: -.021em }
  p { margin: .5rem 0 0; font-size: 14px; color: var(--muted); text-wrap: pretty }
  .app, h1, p { animation: rise .6s cubic-bezier(.23, 1, .32, 1) both }
  h1 { animation-delay: .07s }
  p  { animation-delay: .13s }
  .badge { animation: land .42s cubic-bezier(.23, 1, .32, 1) .3s both }
  @keyframes rise { from { opacity: 0; transform: translateY(9px) } to { opacity: 1; transform: none } }
  @keyframes land { from { opacity: 0; transform: scale(.5) } to { opacity: 1; transform: scale(1) } }
  /* The whole page is one entrance and nothing loops, so reduced motion simply
     arrives at the end state. */
  @media (prefers-reduced-motion: reduce) {
    .app, h1, p, .badge { animation: none }
  }
</style>
</head><body>
  <main>
    <div class="app">
      <img src="__ICON__" alt="Nudge">
      <span class="badge"><svg viewBox="0 0 28 28" aria-hidden="true">__BADGE__</svg></span>
    </div>
    <h1>__TITLE__</h1>
    <p>__SUB__</p>
  </main>
</body></html>"##;

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
            let _ = stream.write_all(
                landing(Tone::Waiting, "Almost there", "Finishing up with Google.").as_bytes(),
            );
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
            match &answer {
                Ok(_) => landing(
                    Tone::Good,
                    "You\u{2019}re signed in",
                    "Nudge is ready in your menu bar. You can close this tab.",
                ),
                Err(why) => landing(Tone::Bad, why, "Go back to Nudge and try again."),
            }
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
    fn every_hole_in_the_page_is_filled() {
        // A placeholder that survives is not a crash -- it is a browser tab
        // showing somebody the word __TITLE__ at the end of signing in.
        for tone in [Tone::Good, Tone::Waiting, Tone::Bad] {
            let page = landing(tone, "Signed in", "You can close this tab.");
            assert!(!page.contains("__"), "a placeholder was left unfilled");
            assert!(page.contains("Signed in"));
        }
    }

    #[test]
    fn a_message_from_google_cannot_write_its_own_html() {
        // The failure line can be Google's words, which is exactly why it is
        // worth showing and exactly why it cannot go in raw.
        let page = landing(Tone::Bad, "<script>alert(1)</script>", "try again");
        assert!(!page.contains("<script>"));
        assert!(page.contains("&lt;script&gt;"));
    }

    #[test]
    fn the_page_declares_the_length_it_actually_sends() {
        // A Content-Length that disagrees with the body leaves the browser
        // waiting on bytes that are never coming, which looks like a hang at
        // the exact moment somebody has finished signing in.
        let page = landing(Tone::Good, "Signed in", "You can close this tab.");
        let (head, body) = page.split_once("\r\n\r\n").expect("no header break");
        let stated: usize = head
            .lines()
            .find_map(|l| l.strip_prefix("Content-Length: "))
            .expect("no Content-Length")
            .trim()
            .parse()
            .unwrap();
        assert_eq!(stated, body.len());
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
