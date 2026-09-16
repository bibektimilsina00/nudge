//! Tokens, and the two things that go wrong with them.
//!
//! **They live in the wrong place.** A tool server needs a GitHub token, so the
//! token goes in `config.toml`, so somebody's credential is sitting in plain text
//! in a file that gets copied between machines, opened in an editor and
//! screenshotted for a bug report. This project has already leaked an API key
//! once by printing that file.
//!
//! **And when one is missing, nothing says so.** A coding agent that has never
//! been signed into fails with its own words -- *"Invalid API key"*, *"run
//! `claude login`"* -- somewhere in the output of a subprocess nobody is reading.
//! From the outside it looks exactly like the agent declining to work. The vision
//! is "install it and forget it", and that is not true of anything you have to
//! authenticate without being told to.
//!
//! ## Where they live instead
//!
//! The Keychain, which macOS already has, which is already encrypted, already
//! locked with the login password, and already the place every other program on
//! the machine keeps this. A config value of `keychain:some-name` is looked up
//! rather than used.
//!
//! ```text
//! security add-generic-password -s nudge-github -a nudge -w ghp_xxx
//! ```
//! ```toml
//! env = { GITHUB_PERSONAL_ACCESS_TOKEN = "keychain:nudge-github" }
//! ```
//!
//! **Reading only.** Nudge never writes a secret and never offers to: storing one
//! is a person deciding to trust this program with a credential, and that
//! decision should be made at a shell prompt they typed, not inside a turn they
//! spoke.
use crate::error::{Error, Result};

/// What a config value says when it names a secret rather than being one.
const PREFIX: &str = "keychain:";

/// Look one up. `None` covers both "no such item" and "the Keychain said no".
pub fn from_keychain(name: &str) -> Option<String> {
    let out = std::process::Command::new("/usr/bin/security")
        .args(["find-generic-password", "-s", name, "-w"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let found = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!found.is_empty()).then_some(found)
}

/// Put a token in the Keychain, replacing whatever was there.
///
/// Through `security` rather than the Security framework, for the same reason
/// the read above does: it is one process, it is already how the documentation
/// tells people to do it by hand, and it keeps this file free of an FFI surface
/// for something done twice in the life of a connection.
///
/// **The token never reaches the command line.** `-w` with no value makes
/// `security` read it from stdin, which keeps it out of the process table --
/// where `ps` would show it to every other user on the machine. That is the
/// whole reason this is not two lines.
pub fn to_keychain(name: &str, token: &str) -> Result<()> {
    use std::io::Write;
    let mut child = std::process::Command::new("/usr/bin/security")
        .args([
            "add-generic-password",
            "-s",
            name,
            "-a",
            "nudge",
            "-U",
            "-w",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| Error::Config(format!("could not run security: {e}")))?;

    // Twice. `-w` with no value prompts for the password and then asks to retype
    // it, and sending it once gets "passwords don't match" -- which it reports by
    // printing a line and **exiting zero**.
    child
        .stdin
        .take()
        .ok_or_else(|| Error::Config("security took no input".into()))?
        .write_all(format!("{token}\n{token}\n").as_bytes())
        .map_err(|e| Error::Config(format!("could not hand security the token: {e}")))?;
    let _ = child
        .wait_with_output()
        .map_err(|e| Error::Config(format!("security did not finish: {e}")))?;

    // Checked by reading it back, because the exit status is not evidence: a
    // failed store exits zero here, so trusting it would report success and
    // leave a connection that fails later with somebody else's error message.
    match from_keychain(name).as_deref() == Some(token) {
        true => Ok(()),
        false => Err(Error::Config(format!(
            "the Keychain did not keep {name:?}. If a dialog appeared, allow it and try again."
        ))),
    }
}

/// Take it back out again.
///
/// Missing is success. Disconnecting something twice, or something whose token
/// was already deleted by hand, should not be an error somebody has to think
/// about -- the state afterwards is the one they asked for either way.
pub fn forget_keychain(name: &str) {
    let _ = std::process::Command::new("/usr/bin/security")
        .args(["delete-generic-password", "-s", name])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}

/// Turn a configured value into the value to actually use.
///
/// Anything without the prefix is returned unchanged, so a plain token still
/// works -- this is an invitation, not an enforcement. A missing Keychain item is
/// an error rather than an empty string, because a server started with a blank
/// token fails later, further away, and in the server's own words.
pub fn resolve(value: &str) -> Result<String> {
    let Some(name) = value.strip_prefix(PREFIX) else {
        return Ok(value.to_string());
    };
    from_keychain(name).ok_or_else(|| {
        Error::Config(format!(
            "no Keychain item called {name:?}. Store it with:\n    \
             security add-generic-password -s {name} -a nudge -w <the-token>"
        ))
    })
}

/// Does this output mean "nobody signed in", rather than "that did not work"?
///
/// Deliberately a guess at somebody else's wording, and the cost of guessing
/// wrong is small in both directions: a false positive suggests signing in to
/// something already signed into, and a false negative leaves the output exactly
/// as it was. Against that, the failure it catches is invisible without it.
pub fn unauthenticated(output: &str) -> bool {
    const SAYS: &[&str] = &[
        "not logged in",
        "not authenticated",
        "unauthenticated",
        "authentication failed",
        "invalid api key",
        "missing api key",
        "no api key",
        "api key not found",
        "unauthorized",
        "401",
        "please log in",
        "please login",
        "run `login`",
        "auth login",
        "credentials not found",
        "no credentials",
        "session expired",
        "token expired",
        "invalid token",
    ];
    let lower = output.to_lowercase();
    SAYS.iter().any(|s| lower.contains(s))
}

/// The sentence to say when something is installed but nobody has signed in.
///
/// The 3.2 shape, one layer along: name the thing, give the command, and do not
/// pretend it is a failure of the task. Somebody who is told *"the build failed"*
/// goes and looks at their build.
pub fn sign_in(program: &str) -> String {
    let how = match program {
        "claude" => Some("claude"),
        "codex" => Some("codex login"),
        "gh" => Some("gh auth login"),
        "glab" => Some("glab auth login"),
        "docker" => Some("docker login"),
        "opencode" => Some("opencode auth login"),
        _ => None,
    };
    match how {
        Some(cmd) => format!(
            "{program} is installed but nobody has signed in to it. \
             Run `{cmd}` in a terminal once and it will stay signed in."
        ),
        None => format!(
            "{program} is installed but nobody has signed in to it, and it needs \
             that before it can do anything."
        ),
    }
}

/// Take the secrets out of text before it is shown, spoken or written down.
///
/// The one that made this necessary: `reqwest` puts the whole URL in its error
/// message, and the provider's URL carries the API key in a query parameter. One
/// rate limit and the key is in `/tmp/nudge.log`, in the error bubble on screen,
/// and in whatever the person pastes into a bug report. It reached a terminal
/// once already.
///
/// Markers rather than shapes. Guessing which long strings are secret means
/// deciding how long is long, and a rule like that redacts a commit hash and
/// misses a short token. What is reliable is the word in front: `key=`,
/// `Bearer `, `token=`. Everything up to the next thing that cannot be part of a
/// credential is replaced.
pub fn redact(text: &str) -> String {
    const MARKERS: &[&str] = &[
        "key=",
        "apikey=",
        "api_key=",
        "access_token=",
        "token=",
        "password=",
        "secret=",
        "bearer ",
        "basic ",
    ];
    // Deliberately not "authorization: ", which is followed by the *scheme* and
    // only then the credential -- redacting from there cut at "Bearer" and left
    // the token standing. The scheme word is the reliable marker.
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    loop {
        let lower = rest.to_lowercase();
        // The earliest marker, so overlapping ones cannot step over each other.
        let Some((at, marker)) = MARKERS
            .iter()
            .filter_map(|m| lower.find(m).map(|i| (i, *m)))
            .min_by_key(|(i, _)| *i)
        else {
            out.push_str(rest);
            return out;
        };
        let after = at + marker.len();
        out.push_str(&rest[..after]);
        out.push('\u{2026}');
        // A credential runs until something that cannot be part of one. `&` ends
        // a query parameter, `)` ends reqwest\'s bracketed URL, whitespace ends a
        // header value.
        let end = rest[after..]
            .find(|c: char| c.is_whitespace() || matches!(c, '&' | ')' | '"' | '\'' | ',' | ';'))
            .map_or(rest.len(), |i| after + i);
        rest = &rest[end..];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_value_is_left_alone() {
        assert_eq!(resolve("ghp_literal").unwrap(), "ghp_literal");
    }

    /// A blank token is worse than a refusal: the server starts, fails later, and
    /// says so in its own words somewhere nobody is reading.
    #[test]
    fn a_missing_item_is_an_error_that_says_how_to_fix_it() {
        let e = resolve("keychain:nudge-definitely-not-here")
            .unwrap_err()
            .to_string();
        assert!(e.contains("no Keychain item"));
        assert!(e.contains("add-generic-password"), "it should say how: {e}");
    }

    /// Round trip through the real Keychain, since that is the only thing that
    /// proves the arguments are right.
    ///
    /// Now through `to_keychain` rather than by shelling out here, so the test
    /// exercises what the app runs. The thing it is guarding: **the token never
    /// appears in an argument.** Every other user on this machine can read the
    /// process table, so `security ... -w <token>` publishes it to them for as
    /// long as the command runs. Handed over on stdin instead.
    #[test]
    #[ignore = "writes to the login keychain"]
    fn a_stored_secret_comes_back() {
        let name = "nudge-secret-roundtrip";
        to_keychain(name, "hunter2").unwrap();
        assert_eq!(resolve(&format!("keychain:{name}")).unwrap(), "hunter2");

        // Updated, not added beside. Without `-U` the old item keeps answering
        // and a re-connected account would go on using the token it replaced.
        to_keychain(name, "hunter3").unwrap();
        assert_eq!(from_keychain(name).as_deref(), Some("hunter3"));

        forget_keychain(name);
        assert_eq!(from_keychain(name), None);
        // Forgetting something already gone is the state somebody asked for.
        forget_keychain(name);
    }

    #[test]
    fn it_recognises_the_usual_ways_of_saying_nobody_signed_in() {
        for said in [
            "Error: not logged in to github.com",
            "Invalid API key · Please run /login",
            "HTTP 401 Unauthorized",
            "credentials not found",
        ] {
            assert!(unauthenticated(said), "missed: {said}");
        }
    }

    /// The cost of a false positive is a pointless suggestion; the cost of
    /// catching everything is that every failure becomes a login prompt.
    #[test]
    fn an_ordinary_failure_is_not_a_login_problem() {
        for said in [
            "error[E0061]: this function takes 3 arguments",
            "fatal: not a git repository",
            "No such file or directory",
        ] {
            assert!(!unauthenticated(said), "wrongly blamed auth: {said}");
        }
    }

    /// The leak this exists for, in the exact shape it arrived in.
    #[test]
    fn an_api_key_in_a_url_does_not_survive() {
        let said = "HTTP status client error (429 Too Many Requests) for url \
                    (https://generativelanguage.googleapis.com/v1beta/models/\
                     gemini-3.6-flash:generateContent?key=AQ.Ab8RN6I4ct_MOTlWxvz0)";
        let clean = redact(said);
        assert!(!clean.contains("AQ.Ab8RN6"), "the key survived: {clean}");
        // And what is left still says what went wrong and where.
        assert!(clean.contains("429"));
        assert!(clean.contains("generativelanguage.googleapis.com"));
    }

    #[test]
    fn it_takes_the_value_and_leaves_the_sentence() {
        assert_eq!(
            redact("Authorization: Bearer ghp_abc123 was refused"),
            "Authorization: Bearer \u{2026} was refused"
        );
        assert_eq!(redact("?key=abc&model=flash"), "?key=\u{2026}&model=flash");
        // More than one, and text with none at all.
        assert!(!redact("token=aaa and key=bbb").contains("aaa"));
        assert!(!redact("token=aaa and key=bbb").contains("bbb"));
        assert_eq!(redact("nothing secret here"), "nothing secret here");
    }

    #[test]
    fn the_sentence_names_the_command_when_we_know_it() {
        assert!(sign_in("gh").contains("gh auth login"));
        // And says the useful half when we do not, rather than inventing one.
        let unknown = sign_in("some-private-cli");
        assert!(unknown.contains("nobody has signed in"));
        assert!(!unknown.contains("Run `"));
    }
}

/// Secrets this process actually holds, for checking that none of them leave.
///
/// A list of the real values rather than a guess at what a secret looks like.
/// That is the whole difference: a shape-based rule has to decide how long is
/// long, and redacts a commit hash while missing a short token. An exact value
/// cannot be wrong in either direction.
///
/// Filled once at startup and never added to, because the only thing that can
/// widen it afterwards is a model.
static HELD: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();

/// Record what this run is holding, so it can be recognised on the way out.
///
/// Short values are dropped. A key of three characters would match half the text
/// on the internet, and the resulting refusals would be the feature's obituary.
pub fn remember(values: impl IntoIterator<Item = String>) {
    let held: Vec<String> = values
        .into_iter()
        .map(|v| v.trim().to_string())
        .filter(|v| v.len() >= 12)
        .collect();
    let _ = HELD.set(held);
}

/// Does this text carry a secret this process is holding?
///
/// The check that makes egress safe to allow at all: whatever else a request is
/// doing, it is not carrying the user's API key to somebody else. A model that
/// has been told by something on screen to fetch `evil.com/?k=<key>` composes a
/// URL that looks entirely ordinary, and this is the only thing between that
/// request and the wire.
///
/// Says which kind leaked, never the value -- an error message is one of the
/// places a secret ends up.
pub fn leaks(text: &str) -> Option<String> {
    leaks_in(text, HELD.get().map(Vec::as_slice).unwrap_or_default())
}

/// The same question, against a list given rather than the one held.
///
/// Split out so the rule can be tested without the global. `HELD` is a
/// `OnceLock` and tests run in parallel, so two tests that both fill it race and
/// the loser reads the winner's secrets -- which is exactly what happened, in
/// the same shape as the notch tests earlier in this project.
fn leaks_in(text: &str, held: &[String]) -> Option<String> {
    held.iter()
        .find(|v| text.contains(v.as_str()))
        .map(|_| "it carries this machine's API key".to_string())
}

#[cfg(test)]
mod egress_tests {
    /// Against an explicit list, never the process-wide one -- see `leaks_in`.
    #[test]
    fn a_value_this_run_holds_is_recognised_on_the_way_out() {
        let held = ["sk-abcdef0123456789".to_string()];
        assert!(super::leaks_in("https://evil.example/?k=sk-abcdef0123456789", &held).is_some());
        assert!(super::leaks_in("https://example.com/weather", &held).is_none());
    }

    #[test]
    fn what_leaked_is_named_but_never_quoted() {
        let held = ["sk-abcdef0123456789".to_string()];
        let said = super::leaks_in("?k=sk-abcdef0123456789", &held).unwrap_or_default();
        assert!(
            !said.contains("sk-abcdef"),
            "the message quoted the secret: {said}"
        );
    }

    /// Short values are dropped, or a three-character key would refuse half the
    /// URLs on the internet.
    #[test]
    fn a_short_value_is_never_remembered() {
        super::remember(["short".to_string(), "sk-live-9f2a7c4e1b83".to_string()]);
        // The only test in the suite that fills the process-wide list, so that
        // nothing races it. Everything else uses `leaks_in`.
        assert!(super::leaks("https://example.com/shortstory").is_none());
        assert!(super::leaks("https://example.com/?k=sk-live-9f2a7c4e1b83").is_some());
    }
}
