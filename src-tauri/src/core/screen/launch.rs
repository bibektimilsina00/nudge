//! Opening an application, for goals like "open Blender" that no amount of
//! pointing can satisfy.
//!
//! This is the one thing Nudge does *to* your machine rather than showing you, so
//! it is deliberately the narrowest possible action: `open -a <name>` starts an
//! installed application and nothing else. It cannot open a file, follow a URL,
//! run a binary, or pass an argument to anything.
use crate::error::{Error, Result};

/// The name arrives from a language model, so it is treated as untrusted input.
/// Allowing a path would turn "open an app" into "run this", and `open` happily
/// launches whatever a path points at.
fn is_safe_name(name: &str) -> bool {
    let name = name.trim();
    !name.is_empty()
        && name.len() <= 64
        && !name.starts_with('-') // never let it read as a flag to `open`
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || " .+-_".contains(c))
}

/// Only ever a web page. `open` will happily hand `file://` a local path, `ftp://`
/// a server, and a custom scheme whatever app claims it -- so the scheme is an
/// allow-list of exactly two, not a blocklist of the ones we thought of.
fn is_safe_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    (lower.starts_with("https://") || lower.starts_with("http://"))
        && url.len() <= 2048
        && !url.chars().any(|c| c.is_whitespace() || c.is_control())
        // A host has to follow the scheme; "https://" alone is not a page.
        && url.len() > lower.find("//").map(|i| i + 2).unwrap_or(0)
}

/// Open a web page in the default browser. The escape hatch for goals like "open
/// TikTok" on a machine with no TikTok app -- which is most of them.
pub fn open_url(url: &str) -> Result<()> {
    let url = url.trim();
    if !is_safe_url(url) {
        return Err(Error::Launch(format!("refusing to open {url:?}")));
    }
    let out = std::process::Command::new("open").arg(url).output()?;
    if out.status.success() {
        return Ok(());
    }
    Err(Error::Launch(format!(
        "couldn't open {url}: {}",
        String::from_utf8_lossy(&out.stderr).trim()
    )))
}

/// Every application this machine can actually launch.
///
/// Without this the model guesses. Asked to "open TikTok" it invents a TikTok app,
/// the launch fails, and the user is told something useless -- when the right
/// answer was always the browser. Naming what exists is what lets it choose.
///
/// ponytail: a flat scan of the usual directories, cached for the process. It
/// misses apps nested deeper; if that starts to matter, `mdfind
/// "kMDItemContentType == 'com.apple.application-bundle'"` is the thorough
/// version, and slower.
pub fn installed_apps() -> &'static [String] {
    static APPS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    APPS.get_or_init(|| {
        let home = dirs::home_dir();
        let roots = [
            std::path::PathBuf::from("/Applications"),
            std::path::PathBuf::from("/System/Applications"),
            std::path::PathBuf::from("/System/Applications/Utilities"),
            home.map(|h| h.join("Applications")).unwrap_or_default(),
        ];
        let mut names: Vec<String> = roots
            .iter()
            .filter_map(|dir| std::fs::read_dir(dir).ok())
            .flatten()
            .filter_map(|e| {
                let path = e.ok()?.path();
                let name = path.file_name()?.to_str()?;
                name.strip_suffix(".app").map(str::to_string)
            })
            .collect();
        names.sort_unstable();
        names.dedup();
        names
    })
}

pub fn open_app(name: &str) -> Result<()> {
    let name = name.trim();
    if !is_safe_name(name) {
        return Err(Error::Launch(format!("refusing to open {name:?}")));
    }
    // `-a` means "application named this". Without it, `open` would treat the
    // argument as a path or URL.
    let out = std::process::Command::new("open")
        .arg("-a")
        .arg(name)
        .output()?;
    if out.status.success() {
        return Ok(());
    }
    Err(Error::Launch(format!(
        "couldn't open {name}: {}",
        String::from_utf8_lossy(&out.stderr).trim()
    )))
}

#[cfg(test)]
mod tests {
    use super::is_safe_name;
    use super::is_safe_url;

    #[test]
    fn accepts_ordinary_web_pages() {
        for ok in [
            "https://tiktok.com",
            "http://localhost:3000/path?q=1",
            "https://www.google.com/search?q=hello+world%20thing",
        ] {
            assert!(is_safe_url(ok), "{ok} should be allowed");
        }
    }

    #[test]
    fn refuses_every_scheme_that_is_not_the_web() {
        for bad in [
            "javascript:alert(1)",
            "file:///etc/passwd",
            "data:text/html,<script>",
            "ftp://example.com",
            "vscode://file/etc",
            "tiktok.com",           // no scheme: `open` would treat it as a path
            "https://",             // no host
            "https://x.com\nmore",  // embedded newline
            "https://exa mple.com", // whitespace
        ] {
            assert!(!is_safe_url(bad), "{bad:?} should be refused");
        }
    }

    #[test]
    fn accepts_ordinary_application_names() {
        for ok in [
            "Blender",
            "Visual Studio Code",
            "DaVinci Resolve",
            "Logic Pro X",
            "Figma",
        ] {
            assert!(is_safe_name(ok), "{ok} should be allowed");
        }
    }

    #[test]
    fn rejects_anything_that_is_not_just_a_name() {
        for bad in [
            "",
            "   ",
            "../../../bin/sh",        // path traversal
            "/Applications/Mail.app", // absolute path
            "-W",                     // reads as a flag to `open`
            "Blender; rm -rf ~",      // shell metacharacters
            "Terminal\n/bin/sh",      // embedded newline
            "https://example.com",    // URL, not an app
        ] {
            assert!(!is_safe_name(bad), "{bad:?} should be refused");
        }
    }
}
