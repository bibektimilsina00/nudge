//! Connecting an account, and taking it away again.
//!
//! The rule the whole thing turns on: **a connection is not made until the
//! server has said what it can do.** Connecting starts it, waits, and asks. If
//! it will not start, or offers nothing, nothing is written down and the token
//! does not stay in the Keychain -- so a catalogue entry naming a package that
//! moved, or a token with the wrong scopes, fails at the moment somebody is
//! looking at it rather than in the middle of a task next week.
use crate::core::connect::{self, Made};
use crate::core::tools::{mcp, secret};
use serde::Serialize;
use tauri::{AppHandle, Manager};

/// One row on the integrations page.
#[derive(Serialize)]
pub struct Listed {
    pub key: String,
    pub name: String,
    pub about: String,
    /// What connecting grants, shown *before* consent.
    pub access: String,
    pub needs_token: bool,
    pub where_from: Option<String>,
    /// For the ones signed into rather than pasted. Shown instead of a field.
    pub setup: Option<String>,
    /// A folder or team this one needs, which the catalogue cannot know.
    pub needs_folder: bool,
    pub connected: bool,
    /// What the server itself said it could do. Empty until connected.
    pub tools: Vec<String>,
    /// Which of those may be used. `None` means all of them -- a connection made
    /// before any of this existed, which keeps working.
    pub allowed: Option<Vec<String>>,
    /// Which were looked at and turned down. Separate from "not allowed" so the
    /// page can tell a decision from a tool that only appeared afterwards.
    pub declined: Vec<String>,
}

/// Everything on offer, and what is already connected.
#[tauri::command]
pub fn connections(app: AppHandle) -> Vec<Listed> {
    let made = connect::read(app.state::<Connections>().path.as_deref());
    connect::catalogue()
        .into_iter()
        .map(|o| {
            let mine = made.iter().find(|m| m.key == o.key);
            Listed {
                key: o.key.into(),
                name: o.name.into(),
                about: o.about.into(),
                access: o.access.into(),
                needs_token: o.token.is_some(),
                where_from: o.where_from.map(Into::into),
                setup: o.setup.map(Into::into),
                // The filesystem server is handed the folder it may touch, and
                // there is no sensible default for "which of your folders".
                needs_folder: o.key == "files",
                connected: mine.is_some(),
                tools: mine.map(|m| m.tools.clone()).unwrap_or_default(),
                allowed: mine.and_then(|m| m.allowed.clone()),
                declined: mine.map(|m| m.declined.clone()).unwrap_or_default(),
            }
        })
        .collect()
}

/// Where the connections file lives. Managed so tests and the app can differ.
pub struct Connections {
    pub path: Option<std::path::PathBuf>,
}

/// Connect one, and prove it works before saying so.
#[tauri::command]
pub async fn connect(
    app: AppHandle,
    key: String,
    token: Option<String>,
    folder: Option<String>,
) -> Result<Vec<String>, String> {
    let offer = connect::offer(&key).ok_or_else(|| format!("no such integration: {key}"))?;

    // The token goes in first, because the server is started with a reference to
    // it -- but it is taken back out if the server does not work, so a failed
    // attempt leaves nothing behind.
    // Nothing to paste, and nothing Nudge can do on somebody's behalf: signing
    // in happens in their browser, with their account, on Google's own consent
    // screen. Said plainly rather than failing at the server with whatever that
    // server's own words happen to be.
    if let Some(how) = offer.setup {
        let ready = mcp::Servers::start(std::slice::from_ref(
            &connect::spec(&Made {
                key: key.clone(),
                extra: Vec::new(),
                tools: Vec::new(),
                // This one only asks whether the thing starts at all, so it must
                // see everything -- a server is not broken for offering a tool
                // somebody later turned off.
                allowed: None,
                declined: Vec::new(),
                at: 0,
            })
            .ok_or_else(|| "could not build that server".to_string())?,
        ))
        .await;
        if ready.tools().is_empty() {
            return Err(how.to_string());
        }
    }

    if offer.token.is_some() {
        let token = token
            .as_deref()
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .ok_or_else(|| format!("{} needs a token", offer.name))?;
        secret::to_keychain(&connect::keychain_item(&key), token).map_err(|e| e.to_string())?;
    }

    let made = Made {
        key: key.clone(),
        extra: folder.into_iter().filter(|f| !f.is_empty()).collect(),
        tools: Vec::new(),
        // Unreviewed while connecting, because the tools are not known until the
        // server has answered. Filled in below, once they are.
        allowed: None,
        declined: Vec::new(),
        at: now_ms(),
    };
    let spec = connect::spec(&made).ok_or_else(|| "could not build that server".to_string())?;

    // The check. Everything above was preparation; this is the only thing that
    // decides whether a connection exists.
    let started = mcp::Servers::start(std::slice::from_ref(&spec)).await;
    let tools: Vec<String> = started.tools().iter().map(|t| t.name.clone()).collect();
    if tools.is_empty() {
        undo(&key, offer.token.is_some());
        return Err(match started.failed(&key) {
            Some(why) => format!("{} did not start: {why}", offer.name),
            None => format!("{} started but offered no tools", offer.name),
        });
    }

    let mut all = connect::read(app.state::<Connections>().path.as_deref());
    all.retain(|m| m.key != key);
    all.push(Made {
        tools: tools.clone(),
        // Everything it offered, written down rather than left as "all of them".
        // The two look identical today and stop being identical the moment the
        // server updates: consent was given to this list, so a tool added next
        // month arrives switched off and asks for a fresh look.
        allowed: Some(tools.clone()),
        ..made
    });
    connect::write(app.state::<Connections>().path.as_deref(), &all);

    eprintln!("connected {key}: {} tools", tools.len());
    Ok(tools)
}

/// Choose which of a connected server's tools may be used.
///
/// Takes only the allowed list; what was turned down is worked out from what the
/// server offered, because the two together are the whole of it and a caller that
/// sends both can send a pair that disagree.
///
/// Takes effect on the next start of that server. Nothing is re-probed here: the
/// tools are already known, and asking a server to prove itself again to answer a
/// checkbox would make a cheap thing slow.
#[tauri::command]
pub fn choose_tools(app: AppHandle, key: String, allowed: Vec<String>) -> Result<(), String> {
    let path = app.state::<Connections>().path.clone();
    let mut all = connect::read(path.as_deref());
    let Some(made) = all.iter_mut().find(|m| m.key == key) else {
        return Err(format!("{key} is not connected"));
    };

    // Only names the server actually offered. A name from anywhere else is not a
    // permission, it is a typo or something worse, and letting it into the file
    // would make the list disagree with the server for the rest of its life.
    let allowed: Vec<String> = made
        .tools
        .iter()
        .filter(|t| allowed.iter().any(|a| a == *t))
        .cloned()
        .collect();
    made.declined = made
        .tools
        .iter()
        .filter(|t| !allowed.iter().any(|a| a == *t))
        .cloned()
        .collect();
    made.allowed = Some(allowed);

    eprintln!(
        "{key}: {} of {} tools allowed",
        made.allowed.as_ref().map_or(0, Vec::len),
        made.tools.len()
    );
    connect::write(path.as_deref(), &all);
    Ok(())
}

/// Take it away: the record, and the token with it.
#[tauri::command]
pub fn disconnect(app: AppHandle, key: String) {
    let path = app.state::<Connections>().path.clone();
    let mut all = connect::read(path.as_deref());
    let before = all.len();
    all.retain(|m| m.key != key);
    connect::write(path.as_deref(), &all);
    // Always, not only when a record existed. Disconnecting is somebody asking
    // for the token to be gone, and a token left behind because the bookkeeping
    // disagreed is the worst way to answer that.
    secret::forget_keychain(&connect::keychain_item(&key));
    eprintln!("disconnected {key} ({} of {before} left)", all.len());
}

/// Undo a half-made connection.
fn undo(key: &str, had_token: bool) {
    if had_token {
        secret::forget_keychain(&connect::keychain_item(key));
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
