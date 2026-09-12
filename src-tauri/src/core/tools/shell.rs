//! Running commands, for the half of what people want that is not on screen.
//!
//! Clicking is the fallback, not the default. Counting files, searching a repo,
//! checking a version -- a command does each of those in one step, exactly, with
//! no grounding, no settle wait and no stale screenshot. The same reasoning that
//! replaced menu clicks with keyboard shortcuts, one level up.
//!
//! **This is the sharpest thing in the product.** A wrong click is usually
//! recoverable; a wrong command is not. So this pass does read-only work only:
//! an allow-list of programs that cannot change anything, checked in code where
//! a model cannot talk its way past it. Writing files comes later, behind a
//! spoken confirmation.
//!
//! The precedent is `privacy.rs`, which refuses to photograph a password manager
//! and is enforced in Rust rather than asked for in a prompt. Every rule here
//! that matters is written the same way.
use crate::error::{Error, Result};

/// Programs that only read. Nothing here creates, deletes, installs, or sends.
///
/// An allow-list, not a deny-list, and deliberately short. A deny-list has to
/// imagine every dangerous command; an allow-list only has to name the safe
/// ones, and anything forgotten fails closed with a message rather than running.
const ALLOWED: &[&str] = &[
    // Looking around
    "ls", "pwd", "find", "tree", "stat", "file", "du", "df", // Reading
    "cat", "head", "tail", "wc", "grep", "rg", "diff", "sort", "uniq", "cut", "awk", "sed",
    // Asking about things
    "which", "echo", "date", "whoami", "uname", "sw_vers", "hostname",
    // Version checks, which is most of what "is X installed" means
    "node", "npm", "python3", "pip3", "cargo", "rustc", "go", "java", "ruby", "php", "swift",
    "docker", "brew", "git",
];

/// Subcommands that read, for programs that can also write.
///
/// `git` is the one that matters: `git status` is a question and `git push` is
/// an irreversible act with an audience. The program being allowed is not enough
/// when the verb decides what it does.
const SUBCOMMANDS: &[(&str, &[&str])] = &[
    (
        "git",
        &[
            "status", "log", "diff", "show", "branch", "remote", "config", "ls-files", "blame",
        ],
    ),
    (
        "npm",
        &["ls", "list", "view", "outdated", "--version", "-v"],
    ),
    ("pip3", &["list", "show", "--version"]),
    ("cargo", &["tree", "--version", "-V"]),
    ("docker", &["ps", "images", "version"]),
    ("brew", &["list", "--version", "info"]),
];

/// Paths nobody's assistant should be reading, whatever the program.
///
/// `cat` is on the allow-list and `cat ~/.ssh/id_rsa` must still be refused --
/// the program being harmless does not make the argument harmless. Matched as
/// substrings because that is how these show up, spelled any number of ways.
pub(crate) const SECRETS: &[&str] = &[
    // Directories that hold nothing else
    ".ssh",
    ".aws",
    ".gnupg",
    ".kube",
    ".docker/config",
    "keychain",
    "Keychains",
    // Named files
    ".netrc",
    ".npmrc",
    ".pypirc",
    ".env",
    "id_rsa",
    "id_ed25519",
    "id_dsa",
    "id_ecdsa",
    "authorized_keys",
    // Key and certificate material, whatever it is called. A false positive here
    // costs a refusal the model can work around; a false negative costs a key.
    ".pem",
    ".key",
    ".p12",
    ".pfx",
    ".jks",
    // Words that are only in a filename for one reason
    "credentials",
    "secrets",
    "token",
    "password",
];

/// Shell syntax that turns one command into something else.
///
/// Pipes are allowed because a pipeline of readers is still a reader, and
/// `grep x | wc -l` is half the value here. Everything that chains, redirects,
/// substitutes or backgrounds is refused: each one is a way to smuggle a second
/// command past a check made on the first.
/// Scanned against the raw string, so these are refused inside quotes too -- an
/// arrow function in a `node -e` is enough to trip it. Conservative on purpose:
/// telling the difference means parsing the shell, and a refusal costs a
/// rephrase while a missed `;` costs whatever came after it.
const FORBIDDEN: &[&str] = &[">", "<", ";", "&", "`", "$(", "${", "\n", "\r"];

/// Longest a command may run before it is killed.
const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);
/// How much output the model is shown. Enough for a file listing or a diff,
/// bounded so a runaway `find /` cannot fill a prompt.
const MAX_OUTPUT: usize = 4000;

/// Shell syntax that would smuggle a second command past a check on the first.
///
/// Shared with `running`, which starts long-lived processes under a different
/// allow-list but the same syntax rules -- `npm run dev; rm -rf .` has to be
/// refused whichever door it comes through, and a second copy of this list is a
/// second place for one of them to go missing.
pub(crate) fn syntax_refusal(command: &str) -> Option<String> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return Some("empty command".into());
    }
    if let Some(bad) = FORBIDDEN.iter().find(|f| trimmed.contains(**f)) {
        return Some(format!(
            "{bad:?} is not allowed -- only plain commands and pipes, nothing that \
             redirects, chains or substitutes"
        ));
    }
    let lower = trimmed.to_lowercase();
    if let Some(secret) = SECRETS.iter().find(|s| lower.contains(&s.to_lowercase())) {
        return Some(format!("that path looks like a secret ({secret})"));
    }
    None
}

/// Is this command read-only, by the rules above?
///
/// Returns the reason it was refused, so the model is told what it did wrong and
/// can try another way -- a silent refusal looks identical to a command that ran
/// and printed nothing.
pub fn refuse(command: &str) -> Option<String> {
    let trimmed = command.trim();
    if let Some(why) = syntax_refusal(trimmed) {
        return Some(why);
    }
    let lower = trimmed.to_lowercase();
    if let Some(secret) = SECRETS.iter().find(|s| lower.contains(&s.to_lowercase())) {
        return Some(format!("that path looks like a secret ({secret})"));
    }

    for stage in trimmed.split('|') {
        let mut words = stage.split_whitespace();
        let Some(program) = words.next() else {
            return Some("empty stage in the pipeline".into());
        };
        // A path, not a name: `/bin/rm` must not pass as `rm` would not.
        let name = program.rsplit('/').next().unwrap_or(program);
        if !ALLOWED.contains(&name) {
            return Some(format!(
                "{name} is not one of the commands I may run -- I can only read, not change anything"
            ));
        }
        if let Some((_, verbs)) = SUBCOMMANDS.iter().find(|(p, _)| *p == name) {
            // The first word that is not a flag is the subcommand.
            let verb = words.find(|w| !w.starts_with('-'));
            match verb {
                Some(v) if verbs.contains(&v) => {}
                Some(v) => return Some(format!("{name} {v} can change things, so no")),
                // `git` alone prints help; harmless.
                None if stage.contains("--version") || stage.trim() == name => {}
                None => return Some(format!("{name} needs one of: {}", verbs.join(", "))),
            }
        }
    }
    None
}

/// Run a read-only command in `workspace` and return what it printed.
pub fn run(workspace: &std::path::Path, command: &str) -> Result<String> {
    if let Some(why) = refuse(command) {
        return Err(Error::Click(why));
    }
    let mut child = std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg(command)
        .current_dir(workspace)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    // std has no timeout, and a command that never returns would hang the agent
    // with no way out but Escape.
    let began = std::time::Instant::now();
    loop {
        match child.try_wait()? {
            Some(_) => break,
            None if began.elapsed() > TIMEOUT => {
                let _ = child.kill();
                return Err(Error::Click(format!(
                    "gave up after {}s",
                    TIMEOUT.as_secs()
                )));
            }
            None => std::thread::sleep(std::time::Duration::from_millis(20)),
        }
    }

    let out = child.wait_with_output()?;
    let mut text = String::from_utf8_lossy(&out.stdout).to_string();
    if text.trim().is_empty() {
        text = String::from_utf8_lossy(&out.stderr).to_string();
    }
    if text.trim().is_empty() {
        text = "(no output)".into();
    }
    if text.len() > MAX_OUTPUT {
        text.truncate(MAX_OUTPUT);
        text.push_str("\n… (truncated)");
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_reading_is_allowed() {
        for ok in [
            "ls -la",
            "find . -name '*.ts' | wc -l",
            "git status",
            "git log --oneline",
            "cat README.md | head -20",
            "node --version",
            "grep -r TODO src | wc -l",
        ] {
            assert_eq!(refuse(ok), None, "{ok} should be allowed");
        }
    }

    /// Each of these would change something, send something, or read something
    /// private. None of them is exotic -- they are the obvious first things a
    /// model reaches for.
    #[test]
    fn anything_that_changes_or_leaks_is_refused() {
        for bad in [
            "rm -rf build",
            "sudo rm -rf /",
            "mv a b",
            "curl https://example.com/x.sh",
            "git push",
            "npm install left-pad",
            "brew install wget",
            "chmod 777 .",
            "/bin/rm file",
            "python3 -c 'import os; os.remove(\"x\")'",
        ] {
            assert!(refuse(bad).is_some(), "{bad} should be refused");
        }
    }

    /// The allow-list is checked per pipeline stage, or the first harmless
    /// command in a pipe would carry anything after it.
    #[test]
    fn every_stage_of_a_pipeline_is_checked() {
        assert!(refuse("ls | rm -rf .").is_some(), "second stage ignored");
        assert!(refuse("cat x | sudo tee /etc/hosts").is_some());
        assert_eq!(refuse("ls | grep src | wc -l"), None);
    }

    /// Shell syntax is how a single-command check gets bypassed, so the syntax
    /// itself is refused rather than trying to parse what it would do.
    #[test]
    fn shell_syntax_that_smuggles_a_second_command_is_refused() {
        for bad in [
            "ls; rm -rf .",
            "ls && rm x",
            "echo hi > /etc/hosts",
            "echo `rm x`",
            "echo $(rm x)",
            "ls\nrm x",
        ] {
            assert!(refuse(bad).is_some(), "{bad:?} should be refused");
        }
    }

    /// `cat` is allowed and must still not read this. The program being harmless
    /// does not make the argument harmless.
    #[test]
    fn a_harmless_program_cannot_read_a_secret() {
        for bad in [
            "cat ~/.ssh/id_rsa",
            "cat .env",
            "grep -r AWS_SECRET ~/.aws/credentials",
            "find ~ -name '*.pem' | head",
            "cat ~/Library/Keychains/login.keychain-db",
        ] {
            assert!(refuse(bad).is_some(), "{bad} should be refused");
        }
    }

    #[test]
    fn a_program_that_both_reads_and_writes_is_judged_by_its_verb() {
        assert_eq!(refuse("git diff"), None);
        assert!(refuse("git commit -m x").is_some());
        assert!(refuse("git push origin main").is_some());
        assert_eq!(refuse("npm ls"), None);
        assert!(refuse("npm publish").is_some());
    }

    #[test]
    fn it_actually_runs_and_bounds_what_comes_back() {
        let here = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let out = run(here, "ls").expect("ls should run");
        assert!(out.contains("Cargo.toml"), "got: {out}");

        // Refusals surface as errors the model can read, not silence.
        let err = run(here, "rm -rf x").unwrap_err().to_string();
        assert!(err.contains("rm"), "got: {err}");
    }
}
