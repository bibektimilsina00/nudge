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
    /// The client id to sign in with, for services that will hand over a token
    /// rather than make somebody go and mint one.
    ///
    /// Public on purpose. GitHub's device flow has no client secret, which is
    /// exactly why it suits a program anybody can download and read.
    pub sign_in: Option<&'static str>,
    /// A cheap read that proves the credential works, not just that the server
    /// runs.
    ///
    /// The whole reason this exists: Gmail, Calendar and GitHub all start
    /// happily and list every tool they have with no valid token at all. So
    /// "the server answered" was never evidence of a working connection, and
    /// the card went green on connections that failed at the first real task.
    ///
    /// Named here rather than guessed, because there is no convention -- one
    /// server's cheapest read is `list_email_labels` and another's is
    /// `list-calendars`, and calling the wrong thing fails for the wrong reason.
    pub check: Option<(&'static str, &'static str)>,
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
            check: None,
            sign_in: None,
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
            check: None,
            sign_in: Some("Iv23liZbf0DVYN4oVPZt"),
            token: Some("GITHUB_PERSONAL_ACCESS_TOKEN"),
            // Still offered, because a fine-grained token you minted yourself is
            // a narrower thing than what signing in gives, and somebody who
            // wants that should not have to sign in to get it.
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
            check: None,
            sign_in: None,
            token: Some("SLACK_BOT_TOKEN"),
            where_from: Some("api.slack.com/apps → your app → OAuth & Permissions"),
            setup: None,
        },
        Offer {
            key: "airtable",
            name: "Airtable",
            about: "Read and write records, and inspect table schemas.",
            // Airtable is one of the few whose token is genuinely scoped, and it
            // is worth saying so -- most of the tokens in this catalogue are all
            // or nothing.
            access: "Only the bases you tick when you create the token, and only \
                     the scopes you tick with them. Not your whole workspace.",
            command: "npx",
            args: &["-y", "airtable-mcp-server"],
            env: &[],
            check: Some(("list_bases", "{}")),
            sign_in: None,
            token: Some("AIRTABLE_API_KEY"),
            where_from: Some("airtable.com/create/tokens → Create token → pick bases and scopes"),
            setup: None,
        },
        Offer {
            key: "asana",
            name: "Asana",
            about: "Search tasks and projects; create, update and comment.",
            access: "Everything you can see in Asana. Their personal access token \
                     is not scoped, so it carries your whole account.",
            command: "npx",
            args: &["-y", "@roychri/mcp-server-asana"],
            env: &[],
            check: Some(("asana_list_workspaces", "{}")),
            sign_in: None,
            token: Some("ASANA_ACCESS_TOKEN"),
            where_from: Some("app.asana.com/0/my-apps → Personal access tokens → Create"),
            setup: None,
        },
        Offer {
            key: "miro",
            name: "Miro",
            about: "Read and edit boards, frames, shapes and sticky notes.",
            access: "The boards the token's app has been given, and what you tick \
                     when you install it.",
            command: "npx",
            args: &["-y", "@k-jarzyna/mcp-miro"],
            env: &[],
            check: None,
            sign_in: None,
            token: Some("MIRO_ACCESS_TOKEN"),
            where_from: Some("miro.com/app/settings/user-profile/apps → your app → Install and get OAuth token"),
            setup: None,
        },
        Offer {
            key: "zeplin",
            name: "Zeplin",
            about: "Read design specs, components and screen annotations.",
            access: "The projects and styleguides your Zeplin account can see.",
            command: "npx",
            args: &["-y", "@zeplin/mcp-server"],
            env: &[],
            check: None,
            sign_in: None,
            token: Some("ZEPLIN_ACCESS_TOKEN"),
            where_from: Some("app.zeplin.com/profile/developer → Personal access tokens"),
            setup: None,
        },
        Offer {
            key: "youtube",
            name: "YouTube",
            about: "Your channel, videos, playlists and subscriptions — and search.",
            // Read-only on purpose, and the line says which half is missing.
            // Upload, delete and comment are a separate decision; inheriting them
            // from "let it see my channel" is how a connector becomes a risk
            // nobody agreed to.
            access: "Read-only: youtube.readonly. Your channel, your videos \
                     including private and unlisted ones, your playlists and \
                     subscriptions. It cannot upload, delete or comment.",
            // No server: YouTube's API is plain REST, so it is a table. See
            // `core::tools::services`.
            command: "",
            args: &[],
            env: &[("GOOGLE_TOKEN_FILE", "~/.config/nudge/youtube-token.json")],
            check: Some(("youtube_my_channel", "{}")),
            sign_in: None,
            token: Some("YOUTUBE_OAUTH_TOKEN"),
            where_from: None,
            setup: Some(
                "Sign in once, asking only for YouTube:\nSCOPES=https://www.googleapis.com/auth/youtube.readonly \
                 OUT=~/.config/nudge/youtube-token.json python3 scripts/google-token.py",
            ),
        },
        Offer {
            key: "notion",
            name: "Notion",
            about: "Search pages, read content, query databases and create pages.",
            // Notion's is the narrowest model of the lot and worth saying plainly,
            // because it is the opposite of what people expect from a token.
            access: "Only the pages and databases you share with the integration. \
                     A new integration can reach nothing until you add it to \
                     something, page by page.",
            command: "npx",
            args: &["-y", "@notionhq/notion-mcp-server"],
            env: &[],
            check: None,
            sign_in: None,
            token: Some("AUTH_TOKEN"),
            where_from: Some(
                "notion.so → Settings → Connections → Develop or manage integrations \
                 → New integration → Internal Integration Secret",
            ),
            setup: None,
        },
        Offer {
            key: "todoist",
            name: "Todoist",
            about: "Read, create and complete tasks, projects and labels.",
            access: "Everything in your Todoist account. Their API token is not \
                     scoped, so this is all of it or none of it.",
            command: "npx",
            args: &["-y", "@doist/todoist-mcp"],
            env: &[],
            check: Some(("get-projects", "{}")),
            sign_in: None,
            token: Some("TODOIST_API_KEY"),
            where_from: Some("todoist.com → Settings → Integrations → Developer → API token"),
            setup: None,
        },
        Offer {
            key: "hubspot",
            name: "HubSpot",
            about: "Search CRM records, log notes and tasks, update records.",
            access: "Whatever scopes you tick when you create the private app — \
                     that list is the whole of the limit, and it is worth ticking \
                     narrowly.",
            command: "npx",
            args: &["-y", "@hubspot/mcp-server"],
            env: &[],
            check: None,
            sign_in: None,
            token: Some("HUBSPOT_ACCESS_TOKEN"),
            where_from: Some(
                "hubspot.com → Settings → Integrations → Private Apps → Create → \
                 Auth → Access token",
            ),
            setup: None,
        },
        Offer {
            key: "calcom",
            name: "Cal.com",
            about: "Read availability and manage bookings and event types.",
            access: "Your bookings, availability and event types.",
            command: "npx",
            args: &["-y", "@calcom/cal-mcp"],
            env: &[],
            check: None,
            sign_in: None,
            token: Some("CAL_API_KEY"),
            where_from: Some("cal.com → Settings → Developer → API keys"),
            setup: None,
        },
        Offer {
            key: "supabase",
            name: "Supabase",
            about: "Query databases, inspect schemas and manage projects.",
            // The broadest thing in this catalogue, and it would be dishonest to
            // describe it in the same shape as the others.
            access: "Every project in your account, including the ability to run \
                     SQL and change schemas. A personal access token is not scoped \
                     to one project. Consider a read-only token, and consider \
                     turning off the write tools once connected.",
            command: "npx",
            args: &["-y", "@supabase/mcp-server-supabase"],
            env: &[],
            check: None,
            sign_in: None,
            token: Some("SUPABASE_ACCESS_TOKEN"),
            where_from: Some("supabase.com/dashboard/account/tokens → Generate new token"),
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
            check: Some(("list_email_labels", "{}")),
            sign_in: None,
            token: None,
            where_from: None,
            setup: Some(
                "Desktop OAuth client JSON at ~/.gmail-mcp/gcp-oauth.keys.json, then:\n\
                 npx @gongrzhe/server-gmail-autoauth-mcp auth",
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
            // Nudge points it at the credentials itself. Asking somebody to
            // export a variable before clicking a button is asking them to do
            // the part that is not theirs to do.
            env: &[("GOOGLE_OAUTH_CREDENTIALS", "~/.config/gcp-oauth.keys.json")],
            check: Some(("list-calendars", "{}")),
            sign_in: None,
            token: None,
            where_from: None,
            setup: Some(
                "Desktop OAuth client JSON at ~/.config/gcp-oauth.keys.json, then:\n\
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
            env: &[
                ("CREDENTIALS_PATH", "~/.config/gcp-oauth.keys.json"),
                // Its default is `token.json` *relative to the working
                // directory*, so signing in from a terminal puts the token
                // somewhere the server Nudge starts will never look. Pinned, or
                // this works once by hand and never again from the app.
                ("TOKEN_PATH", "~/.mcp-google-sheets-token.json"),
            ],
            check: Some(("list_spreadsheets", "{}")),
            sign_in: None,
            token: None,
            where_from: None,
            setup: Some(
                "Desktop OAuth client JSON at ~/.config/gcp-oauth.keys.json, then \
                 sign in once:\npython3 scripts/google-sheets-token.py\n\n\
                 The server itself cannot do this. It checks for a token file and, \
                 if there is none, raises an error telling you to sign in through \
                 a browser it has no code to open.",
            ),
        },
        Offer {
            key: "google_tasks",
            name: "Google Tasks",
            about: "Read, create and complete tasks and task lists.",
            access: "Your task lists and their contents: tasks. Nothing else in \
                     the account.",
            // A table, not a server: the npm one wants a client id, a client
            // secret and a refresh token as three variables, and the API under
            // it is four plain calls. See `core::tools::services`.
            command: "",
            args: &[],
            env: &[("GOOGLE_TOKEN_FILE", "~/.config/nudge/tasks-gsc-token.json")],
            check: Some(("tasks_lists", "{}")),
            sign_in: None,
            token: Some("GOOGLE_TASKS_TOKEN"),
            where_from: None,
            setup: Some(
                "Sign in once, asking only for Tasks and Search Console:\n\
                 SCOPES=\"https://www.googleapis.com/auth/tasks \
                 https://www.googleapis.com/auth/webmasters.readonly\" \
                 OUT=~/.config/nudge/tasks-gsc-token.json python3 scripts/google-token.py",
            ),
        },
        Offer {
            key: "google_search_console",
            name: "Google Search Console",
            about: "Search performance, indexing and sitemaps for your sites.",
            access: "Read-only: webmasters.readonly. The properties you already \
                     own in Search Console, and nothing else — it cannot change a \
                     property, submit a sitemap or add a user.",
            // A table on the same sign-in as Tasks. The npm server wants a
            // service account key: a second kind of credential to create,
            // download, and then grant separately on every property -- for an
            // API that answers an ordinary OAuth token perfectly well.
            command: "",
            args: &[],
            env: &[("GOOGLE_TOKEN_FILE", "~/.config/nudge/tasks-gsc-token.json")],
            check: Some(("gsc_sites", "{}")),
            sign_in: None,
            token: Some("GSC_TOKEN"),
            where_from: None,
            setup: Some(
                "Signed in together with Tasks — one sign-in covers both:\n\
                 SCOPES=\"https://www.googleapis.com/auth/tasks \
                 https://www.googleapis.com/auth/webmasters.readonly\" \
                 OUT=~/.config/nudge/tasks-gsc-token.json python3 scripts/google-token.py",
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
            check: None,
            sign_in: None,
            token: None,
            where_from: None,
            setup: Some(
                "Create an OAuth client of type Desktop app — not iOS, which wants \
                 a Bundle ID this is not.\nIts JSON at ~/.google-mcp/credentials.json, \
                 then:\nnpx google-workspace-mcp accounts add me",
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
/// Where the renewal half of a sign-in is kept.
///
/// Separate from the token rather than packed in with it, so that reading a
/// token stays a read of one thing and the shape on disk does not change for
/// the connectors that have no refresh token at all.
pub fn refresh_item(key: &str) -> String {
    format!("{}-refresh", keychain_item(key))
}

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
/// Renew any sign-in that has gone stale, before the servers are started with it.
///
/// The token is handed to a server as an environment variable when the process
/// is spawned, so there is no later moment to fix it in -- a server started with
/// a dead token stays dead for as long as it runs, and reports it as "Bad
/// credentials", which reads as revoked rather than expired.
///
/// Refresh tokens are single-use: GitHub returns a new one and invalidates the
/// old, so the new one is written before the access token. Losing the access
/// token means one failed call; losing the refresh token means signing in again.
pub async fn freshen() {
    for made in read(store().as_deref()) {
        let Some(offer) = offer(&made.key) else { continue };

        // Google's table-backed connectors keep a refresh token in a file and
        // need a fresh access token per session. Minted here for the same reason
        // GitHub's is: the token is read once when tools are wired up, and there
        // is no later moment to notice it has gone stale.
        if let Some((_, path)) = offer.env.iter().find(|(k, _)| *k == "GOOGLE_TOKEN_FILE") {
            match google_access(path).await {
                // Not `let _ =`. Storing can fail -- it did, silently, for every
                // token longer than 128 characters -- and a swallowed failure
                // here leaves a stale token and no sign of why.
                Ok(access) => match crate::core::tools::secret::to_keychain(
                    &keychain_item(&made.key),
                    &access,
                ) {
                    Ok(()) => eprintln!("renewed the {} sign-in", made.key),
                    Err(e) => eprintln!("renewed {} but could not store it: {e}", made.key),
                },
                Err(why) => eprintln!("could not renew {}: {why}", made.key),
            }
            continue;
        }

        let Some(client_id) = offer.sign_in else { continue };
        let name = keychain_item(&made.key);
        let refresh_name = refresh_item(&made.key);
        let Some(refresh) = crate::core::tools::secret::from_keychain(&refresh_name) else {
            // No refresh half: either the app does not expire tokens, or this
            // connection predates keeping it. Nothing to do either way.
            continue;
        };
        // Cheap and definitive: ask for a new one rather than tracking a clock
        // across restarts. GitHub is happy to be asked.
        match crate::core::signin::refresh(client_id, &refresh).await {
            Ok(fresh) => {
                if let Some(next) = &fresh.refresh {
                    let _ = crate::core::tools::secret::to_keychain(&refresh_name, next);
                }
                let _ = crate::core::tools::secret::to_keychain(&name, &fresh.access);
                eprintln!("renewed the {} sign-in", made.key);
            }
            Err(why) => eprintln!("could not renew {}: {why}", made.key),
        }
    }
}

/// Trade a stored Google refresh token for an access token.
///
/// The refresh token is what was granted; the access token is what a call
/// carries, and it lasts an hour. Kept out of the Keychain in favour of the
/// refresh token only because the file is what the sign-in script writes -- the
/// access token goes to the Keychain, which is where everything else reads from.
async fn google_access(path: &str) -> Result<String, String> {
    let path = match path.strip_prefix("~/") {
        Some(rest) => dirs::home_dir().ok_or("no home directory")?.join(rest),
        None => std::path::PathBuf::from(path),
    };
    let stored: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&path).map_err(|_| format!("no sign-in at {}", path.display()))?,
    )
    .map_err(|e| e.to_string())?;
    let refresh = stored["refresh_token"]
        .as_str()
        .ok_or("that sign-in has no refresh token; sign in again")?;

    let keys: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(
            dirs::home_dir().ok_or("no home directory")?.join(".config/gcp-oauth.keys.json"),
        )
        .map_err(|_| "no OAuth client at ~/.config/gcp-oauth.keys.json".to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let c = &keys["installed"];
    let (id, secret) = (
        c["client_id"].as_str().ok_or("that client has no id")?,
        c["client_secret"].as_str().ok_or("that client has no secret")?,
    );

    let res = crate::core::http()
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", id),
            ("client_secret", secret),
            ("refresh_token", refresh),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await
        .map_err(|e| format!("could not reach Google: {e}"))?;
    let got: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    got["access_token"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| format!("Google renewed nothing: {}", got["error"]))
}

pub fn spec(made: &Made) -> Option<crate::core::tools::mcp::Spec> {
    let offer = offer(&made.key)?;
    // A table has no process. Handing an empty command to the spawner produced
    // `could not run "": No such file or directory` on every session -- which
    // reads as a broken connector, for one that works.
    if offer.command.is_empty() {
        return None;
    }
    let mut args: Vec<String> = offer.args.iter().map(|a| a.to_string()).collect();
    // Slack is the one whose extra is not an argument. Its server wants the
    // workspace id in the environment, and unlike a folder nobody knows theirs
    // -- so `connect` reads it off the token rather than asking. Kept here
    // rather than in `Offer` because one entry needing this is a special case,
    // and a second one would be the moment to make it a field.
    let team = matches!(made.key.as_str(), "slack");
    if !team {
        args.extend(made.extra.iter().cloned());
    }

    let mut env = std::collections::HashMap::new();
    if team {
        if let Some(id) = made.extra.first() {
            env.insert("SLACK_TEAM_ID".to_string(), id.clone());
        }
    }
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
        // The rule is "somebody can find out how to get this", not "there is a
        // `where_from`". Signing in is the other way of answering it -- YouTube
        // mints its token with a script rather than sending you to a page --
        // and pairing `token` strictly with `where_from` would have forced a
        // link to somewhere that does not explain it.
        // One way round, not both. Wanting a token obliges the entry to say how
        // to get one; explaining something does not oblige it to want a token --
        // Gmail's instructions are a command to run, and it needs nothing pasted.
        for o in catalogue() {
            if o.token.is_none() {
                continue;
            }
            assert!(
                o.where_from.is_some() || o.setup.is_some() || o.sign_in.is_some(),
                "{} asks for a token and says nothing about where one comes from",
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
    fn a_server_that_needs_pointing_at_credentials_is_pointed_at_them() {
        // The bug this exists for: Calendar declared no environment, so it was
        // started without GOOGLE_OAUTH_CREDENTIALS, refused to start, and handed
        // back its own setup text as a failure -- which read as "do this by
        // hand" for something Nudge is perfectly able to do itself.
        let spec = spec(&Made {
            key: "google_calendar".into(),
            extra: Vec::new(),
            tools: Vec::new(),
            allowed: None,
            declined: Vec::new(),
            at: 0,
        })
        .unwrap();
        let path = spec
            .env
            .get("GOOGLE_OAUTH_CREDENTIALS")
            .expect("calendar is started without being told where its credentials are");
        assert!(
            path.starts_with('/'),
            "a child process does not expand ~, so this has to arrive absolute: {path}"
        );
        assert!(path.ends_with("/.config/gcp-oauth.keys.json"), "{path}");
    }

    #[test]
    fn plain_environment_is_passed_through_untouched() {
        // Only a leading `~/` means anything. A value that merely contains one is
        // a value, not a path to rewrite.
        let offer = offer("google_search_console").unwrap();
        assert!(!offer.env.is_empty());
        for (_, v) in offer.env {
            assert!(!v.contains("~/") || v.starts_with("~/"), "{v}");
        }
    }

    #[test]
    fn every_proving_call_has_arguments_that_parse() {
        // The arguments are a string in a table, so a stray brace is a typo the
        // compiler cannot see. Left unchecked it would surface as a connection
        // that fails for a reason having nothing to do with the credential.
        for o in catalogue() {
            let Some((tool, args)) = o.check else { continue };
            let parsed: Result<serde_json::Value, _> = serde_json::from_str(args);
            assert!(parsed.is_ok(), "{}/{tool} has unparseable arguments: {args}", o.key);
            assert!(
                parsed.unwrap().is_object(),
                "{}/{tool} arguments must be an object",
                o.key
            );
        }
    }

    /// Against the real Keychain and the real Google, because the parts that
    /// break here are the ones no unit test reaches: whether the token file is
    /// where the entry says, whether the client is readable, and whether what
    /// comes back is a token that works.
    ///
    ///     cargo test --lib connect -- --ignored --nocapture
    #[tokio::test]
    #[ignore = "needs a signed-in Google connector and the network"]
    async fn freshen_replaces_an_expired_google_token_with_a_working_one() {
        let table: Vec<_> = catalogue()
            .into_iter()
            .filter(|o| o.env.iter().any(|(k, _)| *k == "GOOGLE_TOKEN_FILE"))
            .collect();
        assert!(!table.is_empty(), "no table-backed Google connector to test");

        freshen().await;

        for o in table {
            let Some(token) = crate::core::tools::secret::from_keychain(&keychain_item(o.key))
            else {
                println!("  {}: not signed in, skipped", o.key);
                continue;
            };
            let res = crate::core::http()
                .get("https://www.googleapis.com/oauth2/v3/tokeninfo")
                .query(&[("access_token", token.as_str())])
                .send()
                .await
                .expect("could not reach Google");
            println!("  {}: tokeninfo says {}", o.key, res.status());
            assert!(
                res.status().is_success(),
                "{} was left with a token Google does not accept",
                o.key
            );
        }
    }

    #[test]
    fn a_table_is_not_offered_to_the_process_spawner() {
        // It has no command, and an empty one reached `Command::new("")`, which
        // fails with "No such file or directory" -- a working connector
        // reporting itself broken on every session.
        for o in catalogue().into_iter().filter(|o| o.command.is_empty()) {
            let made = Made {
                key: o.key.into(),
                extra: Vec::new(),
                tools: Vec::new(),
                allowed: None,
                declined: Vec::new(),
                at: 0,
            };
            assert!(spec(&made).is_none(), "{} was handed to the spawner", o.key);
            assert!(
                crate::core::tools::services::find(o.key).is_some(),
                "{} has no command and no table either, so nothing can run it",
                o.key
            );
        }
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
