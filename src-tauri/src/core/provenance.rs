//! Whether the thing about to be run is a thing this run wrote.
//!
//! The judge is never shown file contents, so `python3 setup.py` cannot be
//! judged from its text: the effect lives inside a file nobody is showing it.
//! But something here knows one fact neither the judge nor the person does —
//! **whether Nudge itself wrote that file, minutes ago.**
//!
//! A script somebody asked for is ordinary work. One the agent wrote for
//! reasons of its own, and is now about to execute, is the shape of a task that
//! has been steered: read an instruction off a web page, write it to a file,
//! run the file. Each step is unremarkable and the sequence is not.
//!
//! So this keeps that record and renders one line of fixed vocabulary.
//!
//! ## Deliberately not here
//!
//! Reading file contents, working out what a script does, or tracing values out
//! of untrusted text. A miss leaves behaviour exactly as it is today, so partial
//! coverage only ever moves towards caution — unlike a detector, whose false
//! negatives would breed false confidence in the thing it missed.
use std::path::Path;

/// A file this run made, and when.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ours {
    pub path: String,
    pub step: usize,
}

/// Does this command name something this run wrote?
///
/// Returns the sentence to show, in fixed vocabulary — never the file's name as
/// the file gave it, never its contents.
pub fn names_ours(command: &str, made: &[Ours], now: usize) -> Option<String> {
    // Every token, because a path can arrive as an argument, after a flag, or as
    // the program itself. Splitting on whitespace is cruder than a shell parser
    // and errs the right way: more things to check can only surface a match,
    // never hide one.
    let tokens: Vec<&str> = command
        .split_whitespace()
        .map(|t| t.trim_matches(|c| c == '"' || c == '\'' || c == ';'))
        .collect();

    // Newest first: if a command names two, the one written most recently is the
    // one worth mentioning.
    let mut theirs: Vec<&Ours> = made.iter().collect();
    theirs.sort_by_key(|o| std::cmp::Reverse(o.step));

    for ours in theirs {
        let name = Path::new(&ours.path)
            .file_name()?
            .to_string_lossy()
            .to_string();
        let hit = tokens.iter().any(|t| {
            // Either the whole path, or the bare filename. A relative mention of
            // a file in the workspace is the ordinary case and looks like the
            // latter.
            *t == ours.path || *t == name || t.ends_with(&format!("/{name}"))
        });
        if hit {
            return Some(said(&name, now.saturating_sub(ours.step)));
        }
    }
    None
}

/// One line, fixed vocabulary.
fn said(name: &str, ago: usize) -> String {
    let when = match ago {
        0 => "just now".to_string(),
        1 => "one step ago".to_string(),
        n => format!("{n} steps ago"),
    };
    format!("{name} was written by this task {when}, not by the user")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn made() -> Vec<Ours> {
        vec![
            Ours {
                path: "/Users/x/Work/setup.py".into(),
                step: 2,
            },
            Ours {
                path: "/Users/x/Work/notes.txt".into(),
                step: 5,
            },
        ]
    }

    /// The sequence this exists for: write a file, then run it.
    #[test]
    fn running_something_this_task_wrote_is_noticed() {
        let out = names_ours("python3 setup.py", &made(), 3).unwrap();
        assert!(out.contains("setup.py"), "{out}");
        assert!(out.contains("one step ago"), "{out}");
        assert!(out.contains("not by the user"), "{out}");
    }

    /// However it is spelled.
    #[test]
    fn a_full_path_and_a_bare_name_are_the_same_file() {
        for command in [
            "python3 /Users/x/Work/setup.py",
            "python3 ./setup.py",
            "sh -c 'python3 setup.py'",
            "cat setup.py",
        ] {
            assert!(
                names_ours(command, &made(), 4).is_some(),
                "missed: {command}"
            );
        }
    }

    /// A file somebody else put there is not this task's doing.
    #[test]
    fn a_file_this_task_did_not_write_says_nothing() {
        assert!(names_ours("python3 manage.py", &made(), 4).is_none());
        assert!(names_ours("ls -la", &made(), 4).is_none());
        assert!(names_ours("", &made(), 4).is_none());
    }

    /// A near-miss must not read as a match. `setup.py.bak` is a different file.
    #[test]
    fn a_name_that_merely_contains_another_is_not_it() {
        assert!(names_ours("cat setup.py.bak", &made(), 4).is_none());
        assert!(names_ours("cat mysetup.py", &made(), 4).is_none());
    }

    /// When two are named, the newest is the one worth saying.
    #[test]
    fn the_most_recent_one_is_the_one_mentioned() {
        let out = names_ours("diff setup.py notes.txt", &made(), 6).unwrap();
        assert!(out.contains("notes.txt"), "{out}");
    }

    #[test]
    fn how_long_ago_reads_as_english() {
        assert!(said("a", 0).contains("just now"));
        assert!(said("a", 1).contains("one step ago"));
        assert!(said("a", 9).contains("9 steps ago"));
    }
}
