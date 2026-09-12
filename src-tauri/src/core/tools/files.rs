//! Writing files, which is the first thing Nudge does that cannot be undone by
//! looking away.
//!
//! Three rules, all enforced here rather than asked for in a prompt:
//!
//! 1. **Inside the workspace, and nowhere else.** Resolved and normalised before
//!    anything is touched, so `../../.ssh/authorized_keys` is a refusal rather
//!    than a surprise.
//! 2. **Overwriting needs the user to say so.** Creating a file is additive and
//!    the worst case is clutter. Replacing one destroys something that was
//!    already there, and nobody asked for that when they asked for a landing
//!    page.
//! 3. **Anything overwritten is kept.** Permission is not the same as safety --
//!    people say yes to things they have misunderstood. A copy goes to
//!    `.nudge-backups` first, so "yes" is recoverable and not final.
use crate::error::{Error, Result};
use std::path::{Component, Path, PathBuf};

/// Biggest file Nudge will write. A landing page is a few KB; anything past this
/// is a model that has lost the thread, and a 4GB file is not a thing to find out
/// about afterwards.
const MAX_BYTES: usize = 256 * 1024;

/// Where overwritten files go.
const BACKUPS: &str = ".nudge-backups";

#[derive(Debug)]
pub enum Wrote {
    /// Written. `backup` is where the previous contents went, if there were any.
    Done {
        path: PathBuf,
        backup: Option<PathBuf>,
    },
    /// The file exists and nobody has agreed to replace it.
    NeedsPermission { path: PathBuf },
}

/// Turn a requested path into a real one inside the workspace, or refuse.
///
/// Normalised without touching the filesystem, because the file usually does not
/// exist yet and `canonicalize` fails on those -- which is exactly the case that
/// needs checking.
pub fn resolve(workspace: &Path, requested: &str) -> Result<PathBuf> {
    let requested = requested.trim();
    if requested.is_empty() {
        return Err(Error::Click("no path given".into()));
    }
    let lower = requested.to_lowercase();
    if let Some(secret) = super::shell::SECRETS
        .iter()
        .find(|s| lower.contains(&s.to_lowercase()))
    {
        return Err(Error::Click(format!(
            "{requested} looks like a secret ({secret}) -- I will not write there"
        )));
    }

    let joined = if Path::new(requested).is_absolute() {
        PathBuf::from(requested)
    } else {
        workspace.join(requested)
    };

    // Resolve `.` and `..` by hand. A `..` that walks above the workspace leaves
    // the stack empty, which is the escape this is here to catch.
    let mut out = PathBuf::new();
    for part in joined.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    return Err(Error::Click(format!("{requested} leaves the workspace")));
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    if !out.starts_with(workspace) {
        return Err(Error::Click(format!(
            "{requested} is outside the workspace ({})",
            workspace.display()
        )));
    }
    Ok(out)
}

/// Write `content` to `path` inside `workspace`.
///
/// `permitted` says the user has agreed to replace this particular file; it is
/// ignored when the file does not exist, because creating one needs no
/// permission.
pub fn write(workspace: &Path, requested: &str, content: &str, permitted: bool) -> Result<Wrote> {
    let path = resolve(workspace, requested)?;
    if content.len() > MAX_BYTES {
        return Err(Error::Click(format!(
            "that is {} KB and the limit is {} KB",
            content.len() / 1024,
            MAX_BYTES / 1024
        )));
    }
    let exists = path.is_file();
    if exists && !permitted {
        return Ok(Wrote::NeedsPermission { path });
    }

    let backup = if exists {
        keep_a_copy(workspace, &path)?
    } else {
        None
    };

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, content)?;
    Ok(Wrote::Done { path, backup })
}

/// Copy a file aside before changing it.
///
/// Counted rather than timestamped: `SystemTime` in a file name is a clock
/// dependency in something that otherwise has none, and the count reads better
/// anyway -- index.html.2 is the second time this was replaced.
fn keep_a_copy(workspace: &Path, path: &Path) -> Result<Option<PathBuf>> {
    let dir = workspace.join(BACKUPS);
    std::fs::create_dir_all(&dir)?;
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let mut n = 1;
    let mut to = dir.join(format!("{name}.{n}"));
    while to.exists() {
        n += 1;
        to = dir.join(format!("{name}.{n}"));
    }
    std::fs::copy(path, &to)?;
    Ok(Some(to))
}

/// How much of a file `read` returns at once.
const MAX_READ: usize = 40_000;
/// Lines returned when no range is asked for.
const DEFAULT_LINES: usize = 1_000;
/// Longest single line returned.
///
/// Without this, one minified file is one line, and a whole-file budget spent on
/// it comes back as a single truncated line that looks like the file was read.
/// Truncating per line means the shape of the file survives even when its
/// contents do not.
const MAX_LINE: usize = 2_000;

/// Read part of a file, with line numbers.
///
/// `run` can already `cat`, and its output caps at 4000 characters with no way
/// to ask for the rest -- so a long file simply stopped mid-way and the model
/// had no idea there was more. Line numbers are what make `edit` targetable and
/// what let a second read ask for the part it actually wants.
pub fn read(workspace: &Path, requested: &str, from: usize, lines: usize) -> Result<String> {
    let path = resolve(workspace, requested)?;
    if !path.is_file() {
        return Err(Error::Click(format!("{} is not a file", path.display())));
    }
    let body = std::fs::read_to_string(&path)
        .map_err(|e| Error::Click(format!("couldn't read {}: {e}", path.display())))?;

    let all: Vec<&str> = body.lines().collect();
    let total = all.len();
    let from = from.max(1);
    let lines = if lines == 0 { DEFAULT_LINES } else { lines };

    let mut out = String::new();
    let mut truncated = false;
    for (i, line) in all.iter().enumerate().skip(from - 1).take(lines) {
        if out.len() >= MAX_READ {
            truncated = true;
            break;
        }
        // Per line, on a character boundary.
        let shown: String = if line.chars().count() > MAX_LINE {
            format!(
                "{}… (line truncated)",
                line.chars().take(MAX_LINE).collect::<String>()
            )
        } else {
            line.to_string()
        };
        out.push_str(&format!("{:>5}  {shown}\n", i + 1));
    }
    if out.is_empty() {
        return Ok(format!(
            "{} has {total} lines; nothing at line {from}.",
            path.display()
        ));
    }
    // Said plainly, because a model shown the first 200 lines of a 2000-line
    // file will otherwise reason as though it has seen the file.
    let shown = out.lines().count();
    let last = from + shown - 1;
    if last < total || truncated {
        out.push_str(&format!(
            "… lines {last}-{total} not shown; ask for them if you need them\n"
        ));
    }
    Ok(out)
}

/// Replace an exact piece of text in a file.
///
/// The everyday change is a line, not a file. Rewriting the whole file to alter
/// one heading means regenerating hundreds of lines the model cannot see, and
/// every one of them is a chance to quietly lose something -- which is why
/// `write` needs permission and keeps a backup.
///
/// `old` must appear EXACTLY ONCE. Two matches means the model is guessing which
/// one it meant, and picking for it is how the wrong line gets changed. It is
/// told how many it found so it can ask for a longer, unique piece.
pub fn edit(
    workspace: &Path,
    requested: &str,
    old: &str,
    new: &str,
    permitted: bool,
) -> Result<Wrote> {
    let path = resolve(workspace, requested)?;
    if !path.is_file() {
        return Err(Error::Click(format!(
            "{} does not exist -- use write to create it",
            path.display()
        )));
    }
    if old.is_empty() {
        return Err(Error::Click("nothing to replace".into()));
    }
    let body = std::fs::read_to_string(&path)
        .map_err(|e| Error::Click(format!("couldn't read {}: {e}", path.display())))?;

    match body.matches(old).count() {
        1 => {}
        0 => {
            return Err(Error::Click(format!(
                "that text is not in {} -- read it first, and match it exactly",
                path.display()
            )))
        }
        n => {
            return Err(Error::Click(format!(
                "that text appears {n} times in {} -- include enough around it to be unique",
                path.display()
            )))
        }
    }

    // Replacing everything is a rewrite wearing an edit's clothes, and the
    // permission rule exists for exactly that. Anything smaller is surgical,
    // reversible from the backup, and does not need asking.
    if old.trim() == body.trim() && !permitted {
        return Ok(Wrote::NeedsPermission { path });
    }

    let backup = keep_a_copy(workspace, &path)?;
    std::fs::write(&path, body.replacen(old, new, 1))?;
    Ok(Wrote::Done { path, backup })
}

/// Did the user say yes?
///
/// Fails closed on purpose: anything that is not clearly agreement is a no. The
/// cost of misreading a mumble as "yes" is someone's file; the cost of misreading
/// it as "no" is being asked again.
pub fn is_yes(answer: &str) -> bool {
    let a = answer
        .trim()
        .to_lowercase()
        .replace(['.', ',', '!', '?'], "");
    const NO: &[&str] = &[
        "no", "not", "don't", "dont", "stop", "cancel", "never", "wait",
    ];
    if NO.iter().any(|n| a.split_whitespace().any(|w| w == *n)) {
        return false;
    }
    const YES: &[&str] = &[
        "yes",
        "yeah",
        "yep",
        "yup",
        "sure",
        "ok",
        "okay",
        "please",
        "do it",
        "go ahead",
        "go for it",
        "fine",
        "correct",
        "confirm",
        "overwrite",
        "replace",
    ];
    YES.iter()
        .any(|y| a == *y || a.starts_with(&format!("{y} ")) || a.contains(y))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One directory per test, named after it.
    ///
    /// Sharing one made two tests write `index.html` at the same time and read
    /// each other's bytes -- "originalh1>". Cargo runs tests in parallel, so any
    /// shared mutable thing outside the test is a race waiting for a slow day.
    fn workspace(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nudge-files-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Every one of these is a way out of the workspace, and each would put a
    /// file somewhere nobody agreed to.
    #[test]
    fn nothing_escapes_the_workspace() {
        let w = workspace("escape");
        for bad in [
            "../outside.txt",
            "sub/../../outside.txt",
            "/etc/hosts",
            "/tmp/elsewhere.txt",
            "~/.ssh/authorized_keys",
            "../../../../etc/passwd",
        ] {
            assert!(resolve(&w, bad).is_err(), "{bad} should be refused");
        }
        // And the ordinary cases still work, including a `..` that stays inside.
        assert!(resolve(&w, "index.html").is_ok());
        assert!(resolve(&w, "site/css/main.css").is_ok());
        assert!(resolve(&w, "site/../index.html").is_ok());
    }

    #[test]
    fn secret_looking_paths_are_refused_even_inside_the_workspace() {
        let w = workspace("secrets");
        for bad in [
            ".env",
            "config/credentials.json",
            "keys/server.pem",
            ".npmrc",
        ] {
            assert!(resolve(&w, bad).is_err(), "{bad} should be refused");
        }
    }

    /// Creating is additive; replacing destroys. Only the second needs asking.
    #[test]
    fn a_new_file_is_written_and_an_existing_one_asks_first() {
        let w = workspace("new-vs-existing");
        let made = write(&w, "index.html", "<h1>hi</h1>", false).unwrap();
        assert!(
            matches!(made, Wrote::Done { backup: None, .. }),
            "new file, no asking"
        );
        assert_eq!(
            std::fs::read_to_string(w.join("index.html")).unwrap(),
            "<h1>hi</h1>"
        );

        let again = write(&w, "index.html", "<h1>replaced</h1>", false).unwrap();
        assert!(
            matches!(again, Wrote::NeedsPermission { .. }),
            "must ask to replace"
        );
        assert_eq!(
            std::fs::read_to_string(w.join("index.html")).unwrap(),
            "<h1>hi</h1>",
            "and must not have written anything while asking"
        );
    }

    /// Permission is not safety. People say yes to things they misunderstood.
    #[test]
    fn saying_yes_still_keeps_a_copy() {
        let w = workspace("backup");
        write(&w, "index.html", "original", false).unwrap();
        let done = write(&w, "index.html", "replaced", true).unwrap();
        let Wrote::Done {
            backup: Some(b), ..
        } = done
        else {
            panic!("expected a backup");
        };
        assert_eq!(std::fs::read_to_string(&b).unwrap(), "original");
        assert_eq!(
            std::fs::read_to_string(w.join("index.html")).unwrap(),
            "replaced"
        );

        // A second replacement does not clobber the first backup.
        let done = write(&w, "index.html", "third", true).unwrap();
        let Wrote::Done {
            backup: Some(b2), ..
        } = done
        else {
            panic!("expected a second backup");
        };
        assert_ne!(b, b2);
        assert_eq!(std::fs::read_to_string(&b2).unwrap(), "replaced");
    }

    #[test]
    fn a_runaway_file_is_refused() {
        let w = workspace("runaway");
        let huge = "x".repeat(MAX_BYTES + 1);
        assert!(write(&w, "big.txt", &huge, false).is_err());
    }

    /// Two matches means the model is guessing which one it meant, and choosing
    /// for it is how the wrong line gets changed.
    #[test]
    fn an_edit_must_be_unambiguous() {
        let w = workspace("edit-unique");
        write(&w, "a.txt", "one\ntwo\none\n", false).unwrap();

        let twice = edit(&w, "a.txt", "one", "1", false)
            .unwrap_err()
            .to_string();
        assert!(twice.contains("appears 2 times"), "got: {twice}");
        assert_eq!(
            std::fs::read_to_string(w.join("a.txt")).unwrap(),
            "one\ntwo\none\n",
            "must not have changed anything while refusing"
        );

        let missing = edit(&w, "a.txt", "three", "3", false)
            .unwrap_err()
            .to_string();
        assert!(missing.contains("not in"), "got: {missing}");

        // Enough context to be unique, and only that one changes.
        edit(&w, "a.txt", "two\none", "two\nuno", false).unwrap();
        assert_eq!(
            std::fs::read_to_string(w.join("a.txt")).unwrap(),
            "one\ntwo\nuno\n"
        );
    }

    /// An edit keeps a copy like a write does, so a wrong one is recoverable.
    #[test]
    fn an_edit_keeps_the_previous_version() {
        let w = workspace("edit-backup");
        write(&w, "a.txt", "hello world", false).unwrap();
        let Wrote::Done {
            backup: Some(b), ..
        } = edit(&w, "a.txt", "world", "there", false).unwrap()
        else {
            panic!("expected a backup");
        };
        assert_eq!(std::fs::read_to_string(&b).unwrap(), "hello world");
        assert_eq!(
            std::fs::read_to_string(w.join("a.txt")).unwrap(),
            "hello there"
        );
    }

    /// Replacing the whole file through `edit` would be a rewrite dodging the
    /// permission that `write` asks for.
    #[test]
    fn an_edit_that_replaces_everything_still_asks() {
        let w = workspace("edit-whole");
        write(&w, "a.txt", "everything", false).unwrap();
        let out = edit(&w, "a.txt", "everything", "something else", false).unwrap();
        assert!(matches!(out, Wrote::NeedsPermission { .. }));
        assert_eq!(
            std::fs::read_to_string(w.join("a.txt")).unwrap(),
            "everything"
        );
    }

    /// A model shown the first 200 lines of a 2000-line file will otherwise
    /// reason as though it has seen the file.
    #[test]
    fn reading_says_what_it_did_not_show() {
        let w = workspace("read");
        let body: String = (1..=500).map(|i| format!("line {i}\n")).collect();
        write(&w, "big.txt", &body, false).unwrap();

        let head = read(&w, "big.txt", 1, 10).unwrap();
        assert!(head.contains("    1  line 1"), "no line numbers: {head}");
        assert!(head.contains("   10  line 10"));
        assert!(!head.contains("line 11"), "returned more than asked");
        assert!(head.contains("not shown"), "did not admit there was more");

        // A window in the middle, and the end that admits nothing is missing.
        let mid = read(&w, "big.txt", 250, 2).unwrap();
        assert!(mid.contains("  250  line 250"));
        let all = read(&w, "big.txt", 1, 500).unwrap();
        assert!(
            !all.contains("not shown"),
            "claimed more when it showed everything"
        );
    }

    /// A minified file is one line. Without a per-line cap the whole budget goes
    /// on it, and what comes back is a single truncated line that reads like the
    /// file was successfully read.
    #[test]
    fn one_enormous_line_does_not_eat_the_whole_file() {
        let w = workspace("read-minified");
        let body = format!("{}\nsecond line\nthird line\n", "x".repeat(50_000));
        write(&w, "min.js", &body, false).unwrap();

        let out = read(&w, "min.js", 1, 10).unwrap();
        assert!(
            out.contains("(line truncated)"),
            "did not cap the line: {}",
            &out[..80]
        );
        assert!(out.contains("second line"), "lost the rest of the file");
        assert!(out.contains("third line"));
    }

    #[test]
    fn reading_obeys_the_same_boundaries_as_writing() {
        let w = workspace("read-bounds");
        assert!(read(&w, "../../../etc/passwd", 1, 5).is_err());
        assert!(read(&w, "~/.ssh/id_rsa", 1, 5).is_err());
        assert!(read(&w, "nope.txt", 1, 5).is_err(), "missing file");
    }

    /// Fails closed: only clear agreement counts, because the cost of reading a
    /// mumble as "yes" is somebody's file.
    #[test]
    fn only_a_clear_yes_is_a_yes() {
        for yes in [
            "yes",
            "Yes.",
            "yeah go ahead",
            "sure",
            "ok",
            "do it",
            "please do",
        ] {
            assert!(is_yes(yes), "{yes:?} should be yes");
        }
        for no in [
            "no",
            "no don't",
            "not that one",
            "wait",
            "stop",
            "hmm",
            "what",
            "the other file",
            "",
        ] {
            assert!(!is_yes(no), "{no:?} should not be yes");
        }
    }
}
