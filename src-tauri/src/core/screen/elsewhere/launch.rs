//! Starting applications and opening links, anywhere.
//!
//! `installed_apps` is the one with real substance. The model is told what this
//! machine can launch and nothing more, so that it offers something that exists
//! instead of inventing a plausible name -- and every platform keeps that list
//! somewhere different.
use crate::error::{Error, Result};

pub fn open_url(url: &str) -> Result<()> {
    open::that_detached(url).map_err(|e| Error::Launch(format!("could not open {url}: {e}")))
}

/// Where each platform keeps the list of things a person can start.
///
/// Windows: the Start menu, as shortcut files, in a machine-wide folder and a
/// per-user one. Linux: XDG desktop entries, in the same two flavours. Both are
/// directories of files named after the application, which is all we need -- the
/// name is what the model is shown and what `open_app` is given back.
fn app_dirs() -> Vec<std::path::PathBuf> {
    let home = dirs::home_dir();
    #[cfg(target_os = "windows")]
    {
        let mut out = Vec::new();
        if let Ok(d) = std::env::var("ProgramData") {
            out.push(std::path::PathBuf::from(d).join("Microsoft/Windows/Start Menu/Programs"));
        }
        if let Ok(d) = std::env::var("AppData") {
            out.push(std::path::PathBuf::from(d).join("Microsoft/Windows/Start Menu/Programs"));
        }
        let _ = home;
        out
    }
    #[cfg(not(target_os = "windows"))]
    {
        let mut out = vec![std::path::PathBuf::from("/usr/share/applications")];
        if let Some(h) = home {
            out.push(h.join(".local/share/applications"));
        }
        out
    }
}

const EXTENSION: &str = if cfg!(target_os = "windows") {
    "lnk"
} else {
    "desktop"
};

/// Read once. The list changes when something is installed, which is not
/// something that happens between two turns of a conversation.
pub fn installed_apps() -> &'static [String] {
    static APPS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    APPS.get_or_init(|| {
        let mut found: Vec<String> = app_dirs()
            .iter()
            .flat_map(|d| walk(d, 0))
            .filter_map(|p| {
                (p.extension()?.eq_ignore_ascii_case(EXTENSION))
                    .then(|| p.file_stem()?.to_str().map(str::to_string))
                    .flatten()
            })
            .collect();
        found.sort();
        found.dedup();
        found
    })
}

/// Shallow on purpose. Start menus nest a folder deep per vendor and no deeper
/// that anyone looks, and walking a whole filesystem to name an application
/// would cost more than the turn it feeds.
fn walk(dir: &std::path::Path, depth: usize) -> Vec<std::path::PathBuf> {
    if depth > 2 {
        return Vec::new();
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for e in entries.flatten() {
        let path = e.path();
        if path.is_dir() {
            out.extend(walk(&path, depth + 1));
        } else {
            out.push(path);
        }
    }
    out
}

pub fn open_app(name: &str) -> Result<()> {
    // Whatever the desktop does when you double-click it. `open` knows the
    // per-platform incantation, and the entry it is given came from the list
    // above, so it names something that exists.
    let target = app_dirs()
        .iter()
        .flat_map(|d| walk(d, 0))
        .find(|p| {
            p.file_stem()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s.eq_ignore_ascii_case(name))
        })
        .ok_or_else(|| Error::Launch(format!("{name} is not installed on this machine")))?;
    open::that_detached(&target)
        .map_err(|e| Error::Launch(format!("could not start {name}: {e}")))
}
