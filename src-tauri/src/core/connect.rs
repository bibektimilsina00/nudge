//! What Nudge can be connected to, and what connecting one actually grants.
//!
//! A tool server is three lines in `config.toml` today. That is right for the
//! protocol and wrong for a person: there is no notion of an account, the token
//! sits in a file anybody can `cat`, and nothing says what is connected or how
//! to take it away.
//!
//! **Not thirty-five hand-written integrations.** `mcp.rs` already makes that
//! argument in its own header — six written by hand buy six, and speaking the
//! protocol buys the ones written next year. What is missing is the account
//! model around it, which is this.
//!
//! ## An entry cannot overclaim, because connecting checks
//!
//! OpenWorker's connector catalogue carries About/Access copy for every entry
//! and its tests fail the build if one has no Access line, on the rule that
//! *"overclaiming here is a product bug"*. That is the right instinct and it
//! still relies on somebody keeping prose true.
//!
//! So here the claim is checked rather than reviewed: connecting starts the
//! server and asks it what it can do. If it will not start, or offers nothing,
//! there is no connection — and what is recorded afterwards is the tool list the
//! server itself reported, not what this file hoped for. A catalogue entry is a
//! way to *reach* a server, never a promise about it.
use serde::{Deserialize, Serialize};

/// Something Nudge knows how to connect to.
#[derive(Debug, Clone, Serialize)]
pub struct Offer {
    /// Stable, and the key everything else hangs off. Do not change a shipped one.
    pub key: &'static str,
    pub name: &'static str,
    /// What it is for, in one plain sentence about behaviour.
    pub about: &'static str,
    /// **What access it gets.** Required — a catalogue entry without one cannot
    /// be shown, because consent to something undescribed is not consent.
    pub access: &'static str,
    /// The program that speaks the protocol, and its arguments.
    pub command: &'static str,
    pub args: &'static [&'static str],
    /// The environment variable the token goes into, when one is needed.
    pub token: Option<&'static str>,
    /// Where a person gets that token. A link and a sentence, because "paste
    /// your token" is not an instruction anybody can follow.
    pub where_from: Option<&'static str>,
    /// For the ones that are not a token at all.
    ///
    /// Google's server keeps its own credential file and its own account store,
    /// and signing in means a browser and a consent screen -- which Nudge cannot
    /// do on somebody's behalf and should not try to. So the entry says what to
    /// run instead, and connecting still proves itself the same way: the server
    /// is started and asked what it can do, and an unconfigured one fails there
    /// rather than later.
    pub setup: Option<&'static str>,
}

/// Everything on offer.
///
/// Short on purpose. Each entry is a claim that this command exists and speaks
/// the protocol, and an entry that does not work is worse than no entry: it
/// spends somebody's afternoon before it admits it.
pub fn catalogue() -> Vec<Offer> {
    vec![
        Offer {
            key: "files",
            name: "Files",
            about: "Read and write files in one folder you choose.",
            access: "Everything inside the folder you pick, and nothing outside it.",
            command: "npx",
            args: &["-y", "@modelcontextprotocol/server-filesystem"],
            token: None,
            where_from: None,
            setup: None,
        },
        Offer {
            key: "github",
            name: "GitHub",
            about: "Work with issues, pull requests and repository files.",
            access: "Whatever your token allows — the scopes you tick when you \
                     create it are the whole of the limit.",
            command: "npx",
            args: &["-y", "@modelcontextprotocol/server-github"],
            token: Some("GITHUB_PERSONAL_ACCESS_TOKEN"),
            where_from: Some("github.com → Settings → Developer settings → Personal access tokens"),
            setup: None,
        },
        Offer {
            key: "slack",
            name: "Slack",
            about: "Read channels and post messages as a bot you install.",
            access: "The channels the bot is invited to. It cannot read anything \
                     it has not been added to.",
            command: "npx",
            args: &["-y", "@modelcontextprotocol/server-slack"],
            token: Some("SLACK_BOT_TOKEN"),
            where_from: Some("api.slack.com/apps → your app → OAuth & Permissions"),
            setup: None,
        },
        Offer {
            key: "google",
            name: "Google Workspace",
            about: "Docs, Sheets, Slides, Drive, Gmail, Calendar and Forms.",
            access: "Whatever you consent to when you sign in, for the account you \
                     sign in with. The consent screen lists it before you agree, and \
                     that screen is Google's rather than ours.",
            command: "npx",
            // One server for all of it rather than seven entries. The official
            // `server-gdrive` is deprecated and covers only Drive.
            args: &["-y", "google-workspace-mcp", "serve"],
            token: None,
            where_from: None,
            setup: Some(
                "Google needs a browser and a consent screen, which is yours to give \
                 rather than mine to take. In Google Cloud Console create an OAuth \
                 client of type Desktop app — not iOS, which wants a Bundle ID this \
                 is not — download its JSON to ~/.google-mcp/credentials.json, then \
                 run: npx google-workspace-mcp accounts add me",
            ),
        },
    ]
}

/// Find one by key.
pub fn offer(key: &str) -> Option<Offer> {
    catalogue().into_iter().find(|o| o.key == key)
}

/// The Keychain item a connection's token lives in.
///
/// Namespaced, so it is obvious in Keychain Access what put it there and what
/// deleting it would break.
pub fn keychain_item(key: &str) -> String {
    format!("nudge-{key}")
}

/// A connection somebody actually made.
///
/// What the *server* said it could do, not what the catalogue claimed. Recorded
/// at connect time from the server's own reply, so the list a person reads is
/// the one thing that cannot be out of date by being written down optimistically.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Made {
    pub key: String,
    /// An argument the catalogue could not know -- a folder to work in, a team
    /// id. Appended to the entry's own arguments.
    #[serde(default)]
    pub extra: Vec<String>,
    /// Tool names the server reported when it started.
    #[serde(default)]
    pub tools: Vec<String>,
    /// Milliseconds since the epoch.
    #[serde(default)]
    pub at: u64,
}

/// The file connections are kept in.
///
/// Beside the config rather than in it. `config.toml` is handwritten and
/// commented; a program that rewrites it will eat somebody's comments the first
/// time it saves, and there is no version of that which is not a bug report.
pub fn store() -> Option<std::path::PathBuf> {
    dirs::home_dir().map(|d| d.join(".config/nudge/connections.toml"))
}

#[derive(Default, Serialize, Deserialize)]
pub struct Kept {
    #[serde(default)]
    pub made: Vec<Made>,
}

/// Read what has been connected.
pub fn read(path: Option<&std::path::Path>) -> Vec<Made> {
    let Some(path) = path else { return Vec::new() };
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    match toml::from_str::<Kept>(&text) {
        Ok(k) => k.made,
        Err(e) => {
            eprintln!("connections: {} is not readable ({e})", path.display());
            Vec::new()
        }
    }
}

/// Write it back.
pub fn write(path: Option<&std::path::Path>, made: &[Made]) {
    let Some(path) = path else { return };
    let Ok(text) = toml::to_string(&Kept {
        made: made.to_vec(),
    }) else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Err(e) = std::fs::write(path, text) {
        eprintln!("connections: could not write {}: {e}", path.display());
    }
}

/// Turn a made connection into something `mcp` can start.
pub fn spec(made: &Made) -> Option<crate::core::tools::mcp::Spec> {
    let offer = offer(&made.key)?;
    let mut args: Vec<String> = offer.args.iter().map(|a| a.to_string()).collect();
    args.extend(made.extra.iter().cloned());

    let mut env = std::collections::HashMap::new();
    if let Some(var) = offer.token {
        // Never the token itself. `mcp` resolves this against the Keychain when
        // it starts the child, so the value on disk names a secret rather than
        // being one -- see `core::tools::secret`.
        env.insert(
            var.to_string(),
            format!("keychain:{}", keychain_item(&made.key)),
        );
    }

    Some(crate::core::tools::mcp::Spec {
        name: offer.key.to_string(),
        command: offer.command.to_string(),
        args,
        env,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Consent to something undescribed is not consent.
    #[test]
    fn every_offer_says_what_access_it_gets() {
        for o in catalogue() {
            assert!(!o.access.trim().is_empty(), "{} has no access line", o.key);
            assert!(!o.about.trim().is_empty(), "{} has no description", o.key);
            // A statement of behaviour, not a slogan. Anything this short is a
            // label rather than a description of what somebody is agreeing to.
            assert!(
                o.access.len() > 30,
                "{}'s access line says nothing: {:?}",
                o.key,
                o.access
            );
        }
    }

    /// Anything needing a token says where to get one.
    ///
    /// "Paste your token" is not an instruction anybody can follow, and the
    /// moment somebody goes hunting is the moment they give up.
    #[test]
    fn anything_that_needs_a_token_says_where_to_find_it() {
        for o in catalogue() {
            assert_eq!(
                o.token.is_some(),
                o.where_from.is_some(),
                "{} asks for a token without saying where from, or the reverse",
                o.key
            );
        }
    }

    #[test]
    fn keys_are_unique_and_stable_looking() {
        let mut keys: Vec<&str> = catalogue().iter().map(|o| o.key).collect();
        keys.sort_unstable();
        let before = keys.len();
        keys.dedup();
        assert_eq!(before, keys.len(), "two offers share a key");
    }

    /// The token never lands in the file. It is named there and looked up.
    #[test]
    fn what_is_written_down_names_the_secret_rather_than_being_one() {
        let made = Made {
            key: "github".into(),
            extra: Vec::new(),
            tools: vec!["create_issue".into()],
            at: 0,
        };
        let spec = spec(&made).unwrap();
        let value = spec.env.get("GITHUB_PERSONAL_ACCESS_TOKEN").unwrap();
        assert_eq!(value, "keychain:nudge-github");
        assert!(!value.starts_with("ghp_"));
    }

    /// What somebody chose is appended to what the catalogue knows.
    #[test]
    fn a_folder_the_catalogue_could_not_know_is_carried_through() {
        let made = Made {
            key: "files".into(),
            extra: vec!["/Users/x/Work".into()],
            tools: Vec::new(),
            at: 0,
        };
        let spec = spec(&made).unwrap();
        assert_eq!(spec.args.last().unwrap(), "/Users/x/Work");
        assert!(spec.env.is_empty(), "files needs no token");
    }

    #[test]
    fn connections_survive_a_round_trip() {
        let path = std::env::temp_dir().join(format!("nudge-conn-{}.toml", std::process::id()));
        let _ = std::fs::remove_file(&path);
        assert!(read(Some(&path)).is_empty(), "nothing connected yet");

        let made = vec![Made {
            key: "files".into(),
            extra: vec!["/tmp/x".into()],
            tools: vec!["read_file".into(), "write_file".into()],
            at: 1,
        }];
        write(Some(&path), &made);
        assert_eq!(read(Some(&path)), made);

        // And what the server said it could do came back with it.
        assert_eq!(read(Some(&path))[0].tools.len(), 2);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn an_unknown_offer_makes_no_spec() {
        assert!(spec(&Made {
            key: "nothing-like-this".into(),
            extra: Vec::new(),
            tools: Vec::new(),
            at: 0,
        })
        .is_none());
    }
}
