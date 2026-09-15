//! What is actually on this machine.
//!
//! A model that is not told invents. Asked to open a pull request it reaches for
//! `gh`, asked to convert a video it reaches for `ffmpeg`, and on a machine
//! without either it spends a turn finding out -- then usually spends another
//! one guessing at a second name. The same failure as applications and as coding
//! agents, which are already told rather than guessed at.
//!
//! ## Usable, not merely installed
//!
//! This reports what can be run **now**, which is not the same as what exists.
//! `ffmpeg` on a machine where the shell is still read-only is a refusal waiting
//! to happen, and naming it in the prompt buys a wasted turn and a confusing
//! error. So the list is what is installed *and* currently allowed, and it grows
//! by itself when somebody grants a full shell -- which is the right shape: the
//! grant does not merely permit more, it visibly *offers* more.
//!
//! ## Absence is not reported
//!
//! Only what is here is named. A list of what is missing would be longer, would
//! be mostly irrelevant, and would cost tokens on every turn to say that a
//! machine is a normal machine. When something missing would have helped, that
//! belongs at the moment it would have helped -- which is 3.2, not here.
use std::path::Path;

/// Things whose presence changes how a task should be approached.
///
/// Not everything on the PATH -- that is thousands of entries and most of them
/// are noise. These are the ones a model reaches for by name, grouped so the
/// prompt line reads as something rather than as an inventory.
const INTERESTING: &[(&str, &[&str])] = &[
    ("version control", &["git", "gh", "glab", "jj"]),
    ("running things", &["docker", "make", "just", "kubectl"]),
    (
        "languages",
        &[
            "node", "npm", "pnpm", "yarn", "bun", "deno", "python3", "pip3", "uv", "cargo",
            "rustc", "go", "java", "ruby", "php", "swift",
        ],
    ),
    ("searching", &["rg", "fd", "jq", "yq", "sqlite3"]),
    ("files and media", &["ffmpeg", "magick", "pandoc", "qpdf", "zip", "unzip"]),
    ("the web", &["curl", "wget"]),
    ("this Mac", &["osascript", "shortcuts", "pbcopy", "pbpaste", "mdfind", "brew"]),
];

/// How to install something, for the ones we can honestly say.
///
/// Only where the answer is not a guess. A wrong instruction is worse than
/// none -- somebody runs it, it fails, and now they have a broken command and a
/// reason to distrust the next thing they are told. Anything not here is named
/// without advice, which is still the useful half: **the machine does not have
/// this** is what they did not know.
const HOW: &[(&str, &str)] = &[
    ("gh", "brew install gh"),
    ("glab", "brew install glab"),
    ("jj", "brew install jj"),
    ("just", "brew install just"),
    ("kubectl", "brew install kubectl"),
    ("rg", "brew install ripgrep"),
    ("fd", "brew install fd"),
    ("jq", "brew install jq"),
    ("yq", "brew install yq"),
    ("ffmpeg", "brew install ffmpeg"),
    ("magick", "brew install imagemagick"),
    ("pandoc", "brew install pandoc"),
    ("qpdf", "brew install qpdf"),
    ("wget", "brew install wget"),
    ("sqlite3", "brew install sqlite"),
    ("docker", "brew install --cask docker"),
    ("pnpm", "npm install -g pnpm"),
    ("yarn", "npm install -g yarn"),
    ("bun", "brew install oven-sh/bun/bun"),
    ("deno", "brew install deno"),
    ("uv", "brew install uv"),
];

/// Say that something is not here, and how to change that if we know.
///
/// The sentence a person can act on, at the moment it would have helped -- which
/// is the only moment it is worth saying. Nothing here is proactive: this is only
/// ever reached because something was actually reached for.
pub fn missing(program: &str) -> String {
    let install = HOW.iter().find(|(n, _)| *n == program).map(|(_, h)| *h);
    match install {
        Some(how) if installed("brew") || !how.starts_with("brew") => {
            format!("{program} is not on this Mac. I can do that once it is -- `{how}`")
        }
        // Advice that cannot be followed is not advice. Without Homebrew, "brew
        // install x" is a second thing to go and find out about.
        _ => format!("{program} is not on this Mac"),
    }
}

/// Is this on the PATH?
///
/// Walks `PATH` rather than running `which`. Thirty names would be thirty forks
/// and thirty process spawns at startup, against a few hundred `stat` calls that
/// cost nothing -- and this is asked on a path where a person is waiting.
pub fn installed(name: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| {
        let full = dir.join(name);
        // Executable, not merely present: a directory called `go` on the PATH is
        // not the Go toolchain.
        std::fs::metadata(&full).is_ok_and(|m| m.is_file()) && is_executable(&full)
    })
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path).is_ok_and(|m| m.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| matches!(e.to_str(), Some("exe" | "cmd" | "bat")))
}

/// Everything interesting that is installed, cached for the life of the process.
///
/// Nothing is installed or removed while Nudge is running often enough to matter,
/// and the alternative is a few hundred filesystem calls on every single turn.
fn found() -> &'static [(&'static str, Vec<&'static str>)] {
    static FOUND: std::sync::OnceLock<Vec<(&'static str, Vec<&'static str>)>> =
        std::sync::OnceLock::new();
    FOUND.get_or_init(|| {
        INTERESTING
            .iter()
            .map(|(group, names)| {
                let here: Vec<&'static str> =
                    names.iter().copied().filter(|n| installed(n)).collect();
                (*group, here)
            })
            .filter(|(_, here)| !here.is_empty())
            .collect()
    })
}

/// What is installed and currently runnable, grouped.
///
/// Separate from [`line`] so that a test can look at the names themselves. The
/// first version tested the finished sentence and reported that `gh` was being
/// offered when it was not: the word "right" contains "gh", and a substring
/// search does not know the difference.
pub fn usable(anything: bool) -> Vec<(&'static str, Vec<&'static str>)> {
    found()
        .iter()
        .filter_map(|(group, names)| {
            let here: Vec<&'static str> = names
                .iter()
                .copied()
                .filter(|n| anything || super::shell::may_run(n))
                .collect();
            (!here.is_empty()).then_some((*group, here))
        })
        .collect()
}

/// What the prompt says about this machine's tools.
///
/// `anything` is [`crate::core::reach::Grant::Shell`]. Without it the list is
/// narrowed to what the read-only allow-list would actually let through, because
/// a name the model cannot use is worse than no name at all.
pub fn line(anything: bool) -> String {
    let groups: Vec<String> = usable(anything)
        .into_iter()
        .map(|(group, names)| format!("{group}: {}", names.join(", ")))
        .collect();

    if groups.is_empty() {
        return String::new();
    }
    let note = match anything {
        true => "",
        false => " Only what can be run without changing anything is listed; \
                  there is more here that a wider grant would reach.",
    };
    format!(
        "Installed and usable right now -- {}.{note} Anything not named is either \
         not here or cannot be run, so do not reach for it on the chance.\n\n",
        groups.join("; ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_program_is_named_and_the_fix_offered() {
        let said = missing("ffmpeg");
        assert!(said.contains("ffmpeg is not on this Mac"));
        // Only claimed when Homebrew is actually here to run it.
        assert_eq!(said.contains("brew install ffmpeg"), installed("brew"));
    }

    /// Advice we do not have is left out rather than invented. Being told the
    /// machine lacks something is the useful half on its own.
    #[test]
    fn something_we_cannot_advise_on_is_still_named() {
        let said = missing("some-private-cli");
        assert_eq!(said, "some-private-cli is not on this Mac");
    }

    /// Not an assertion -- a look at what this actually says on this machine.
    ///
    ///     cargo test --lib present -- --ignored --nocapture
    #[test]
    #[ignore = "prints rather than asserts"]
    fn show_me() {
        println!("\n-- read-only --\n{}", line(false));
        println!("-- full shell --\n{}", line(true));
    }

    /// Whatever else is true of the machine running this, these are.
    #[test]
    fn it_finds_things_that_are_certainly_here() {
        assert!(installed("ls"));
        assert!(installed("sh"));
    }

    #[test]
    fn it_does_not_find_things_that_are_certainly_not() {
        assert!(!installed("definitely-not-a-real-program-9a7f"));
        // A name with a path in it is not a name; `which` would reject it too.
        assert!(!installed("bin/ls"));
    }

    /// The point of the module: a name the model cannot use is worse than no name.
    #[test]
    fn a_narrow_shell_lists_less_than_a_wide_one() {
        let narrow = line(false);
        let wide = line(true);
        assert!(wide.len() >= narrow.len());
        assert!(narrow.contains("a wider grant would reach"));
        assert!(!wide.contains("a wider grant would reach"));
    }

    #[test]
    fn nothing_is_listed_that_the_shell_would_refuse() {
        for (group, names) in usable(false) {
            for name in names {
                assert!(
                    super::super::shell::may_run(name),
                    "{group} offered {name}, which the shell would refuse"
                );
            }
        }
    }

    /// Granting a shell should not merely permit more, it should offer more.
    #[test]
    fn a_wider_grant_offers_at_least_as_much() {
        let narrow: Vec<&str> = usable(false).into_iter().flat_map(|(_, n)| n).collect();
        let wide: Vec<&str> = usable(true).into_iter().flat_map(|(_, n)| n).collect();
        for name in &narrow {
            assert!(wide.contains(name), "{name} was lost by granting more");
        }
    }
}
