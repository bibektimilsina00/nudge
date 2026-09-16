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
    /// Environment that is not a secret: where a server looks for its own
    /// credential file, mostly, which differs per server and is not something
    /// anybody should have to discover.
    ///
    /// A value starting `~/` is expanded against the home directory, because
    /// these are paths and a static string cannot know whose home.
    pub env: &'static [(&'static str, &'static str)],
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
            env: &[],
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
            env: &[],
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
            env: &[],
            token: Some("SLACK_BOT_TOKEN"),
            where_from: Some("api.slack.com/apps → your app → OAuth & Permissions"),
            setup: None,
        },
        Offer {
            key: "gmail",
            name: "Gmail",
            about: "Search, read, draft and send mail.",
            // The scopes are named because they were read out of the package
            // rather than taken from its description, and because "Gmail access"
            // and `gmail.modify` are not the same sentence to anybody who has to
            // decide. Verified 2026-09-17 against v1.1.11.
            access: "Your mail, and the labels and filters on it: gmail.modify \
                     and gmail.settings.basic. Not your Drive, your Calendar or \
                     anything else in the account.",
            command: "npx",
            args: &["-y", "@gongrzhe/server-gmail-autoauth-mcp"],
            env: &[],
            token: None,
            where_from: None,
            setup: Some(
                "Uses the same Desktop OAuth client as the rest of Google, and asks \
                 only for mail. Put its JSON at ~/.gmail-mcp/gcp-oauth.keys.json, \
                 then run: npx @gongrzhe/server-gmail-autoauth-mcp auth",
            ),
        },
        Offer {
            key: "google_calendar",
            name: "Google Calendar",
            about: "Read availability, summarise schedules and create events.",
            access: "Your calendars and their events: calendar and \
                     calendar.events. No access to mail or files.",
            command: "npx",
            args: &["-y", "@cocal/google-calendar-mcp"],
            env: &[],
            token: None,
            where_from: None,
            setup: Some(
                "Same Desktop OAuth client, calendar scopes only. Point \
                 GOOGLE_OAUTH_CREDENTIALS at its JSON -- \
                 ~/.config/gcp-oauth.keys.json works -- then run: \
                 npx @cocal/google-calendar-mcp auth",
            ),
        },
        Offer {
            key: "google_sheets",
            name: "Google Sheets",
            about: "Read and write spreadsheets.",
            // `drive.file` rather than `drive`: per-file access to what this app
            // created or you opened with it, not the whole disk. The difference
            // is the point of having a Sheets entry at all.
            access: "Your spreadsheets, and only the Drive files this created or \
                     you opened with it: spreadsheets and drive.file. Not the \
                     rest of your Drive, and not mail.",
            command: "npx",
            args: &["-y", "mcp-google-sheets"],
            env: &[("CREDENTIALS_PATH", "~/.config/gcp-oauth.keys.json")],
            token: None,
            where_from: None,
            setup: Some(
                "Same Desktop OAuth client as the rest of Google. Put its JSON at \
                 ~/.config/gcp-oauth.keys.json, then sign in once in a browser: \
                 CREDENTIALS_PATH=~/.config/gcp-oauth.keys.json npx mcp-google-sheets",
            ),
        },
        Offer {
            key: "google_tasks",
            name: "Google Tasks",
            about: "Read, create and complete tasks and task lists.",
            access: "Your task lists and their contents: tasks. Nothing else in \
                     the account.",
            command: "npx",
            args: &["-y", "mcp-google-tasks"],
            env: &[],
            token: None,
            where_from: None,
            setup: Some(
                "This one wants a refresh token rather than doing the browser \
                 flow itself, so it needs GOOGLE_TASKS_CLIENT_ID, \
                 GOOGLE_TASKS_CLIENT_SECRET and GOOGLE_TASKS_REFRESH_TOKEN in the \
                 environment. Note it starts and lists its tools without them and \
                 only fails when something is actually asked of it.",
            ),
        },
        Offer {
            key: "google_search_console",
            name: "Google Search Console",
            about: "Search performance, indexing and sitemaps for your sites.",
            access: "Read-only: webmasters.readonly. It cannot change anything, \
                     and reaches only the properties the service account is added \
                     to rather than everything you own.",
            command: "npx",
            args: &["-y", "mcp-server-gsc"],
            env: &[("GOOGLE_APPLICATION_CREDENTIALS", "~/.config/gsc-service-account.json")],
            token: None,
            where_from: None,
            setup: Some(
                "The odd one out: a service account, not the Desktop OAuth client \
                 the other Google entries use. In Cloud Console create a service \
                 account, download its JSON key to \
                 ~/.config/gsc-service-account.json, then in Search Console add \
                 that account's email as a user on the property you want read.",
            ),
        },
        Offer {
            key: "google",
            name: "Google Workspace",
            about: "Docs, Sheets, Slides, Drive and Forms, in one grant.",
            access: "Whatever you consent to when you sign in, for the account you \
                     sign in with. The consent screen lists it before you agree, and \
                     that screen is Google's rather than ours.",
            command: "npx",
            // One server for all of it rather than seven entries. The official
            // `server-gdrive` is deprecated and covers only Drive.
            args: &["-y", "google-workspace-mcp", "serve"],
            env: &[],
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
    /// Which of those tools may actually be used.
    ///
    /// `None` is "nobody has reviewed this yet", and means all of them -- which
    /// is what every connection made before tool review looks like, so an older
    /// file keeps working without being migrated.
    ///
    /// Once reviewed this is an **include list**, so a tool the server ships in
    /// a later version arrives excluded rather than quietly enabled. A server
    /// that grows a `deleteEverything` next month does not get it for free.
    #[serde(default)]
    pub allowed: Option<Vec<String>>,
    /// Tools that were reviewed and turned down.
    ///
    /// Redundant against `allowed` for deciding what runs, and not redundant at
    /// all for the person reading the list: without it, "I said no to this" and
    /// "the server added this since I looked" are both merely absent, so every
    /// tool you decline comes back wearing a `new` badge forever.
    #[serde(default)]
    pub declined: Vec<String>,
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
    for (k, v) in offer.env {
        let v = match v.strip_prefix("~/") {
            Some(rest) => match dirs::home_dir() {
                Some(home) => home.join(rest).display().to_string(),
                // No home directory is not a reason to pass a path that means
                // something else; leave it out and let the server say so.
                None => continue,
            },
            None => v.to_string(),
        };
        env.insert(k.to_string(), v);
    }
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
        allowed: made.allowed.clone(),
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
            allowed: None,
            declined: Vec::new(),
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
            allowed: None,
            declined: Vec::new(),
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
            allowed: None,
            declined: Vec::new(),
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
            allowed: None,
            declined: Vec::new(),
            at: 0,
        })
        .is_none());
    }
}
