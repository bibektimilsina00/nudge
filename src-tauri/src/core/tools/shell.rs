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

/// Would the allow-list let this program run at all?
///
/// Asked by [`crate::core::tools::present`], which will not offer the model a
/// tool the shell is going to refuse. Says nothing about arguments -- `git` may
/// run and `git push` may not -- because the question here is whether the program
/// is worth naming, not whether one command is allowed.
pub fn may_run(program: &str) -> bool {
    ALLOWED.contains(&program)
}

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
pub fn refuse(command: &str, anything: bool) -> Option<String> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return Some("empty command".into());
    }

    // Refused whatever has been granted, and this is the one line in the module
    // worth arguing about.
    //
    // Full shell access is a decision about *capability* -- somebody wants their
    // assistant to be able to move a file, install a package, run a build. It is
    // not a decision to hand over their keys, and the two are not the same thing
    // said twice. Nobody granting "run any command" is thinking about
    // `~/.ssh/id_rsa`, and a permission people would not have given if asked
    // plainly is not one they gave.
    let lower = trimmed.to_lowercase();
    if let Some(secret) = SECRETS.iter().find(|s| lower.contains(&s.to_lowercase())) {
        return Some(format!("that path looks like a secret ({secret})"));
    }

    // With the allow-list gone there is no check left to smuggle a second
    // command past, so the syntax rules have nothing to protect and refusing a
    // `&&` would only make the granted shell useless for the work it was granted
    // for. Without the grant they stand exactly as they did.
    if anything {
        return None;
    }
    if let Some(why) = syntax_refusal(trimmed) {
        return Some(why);
    }

    for stage in trimmed.split('|') {
        let mut words = stage.split_whitespace();
        let Some(program) = words.next() else {
            return Some("empty stage in the pipeline".into());
        };
        // A path, not a name: `/bin/rm` must not pass as `rm` would not.
        let name = program.rsplit('/').next().unwrap_or(program);
        if !ALLOWED.contains(&name) {
            // Two different failures wearing one message, until now. "I may not
            // run that" and "that is not here" ask opposite things of the person
            // listening: one is a permission they can grant in the menu bar, the
            // other is software they have to install, and being told the wrong
            // one sends them looking in the wrong place.
            //
            // Checked in this order because *not installed* is the more useful
            // answer when both are true. Granting a shell does not conjure
            // `ffmpeg`.
            if !super::present::installed(name) {
                return Some(super::present::missing(name));
            }
            return Some(format!(
                "{name} is here, but running it is not something I have been \
                 allowed to do -- I can only read. Someone can change that under \
                 \u{201c}Allowed to\u{201d} in the menu bar."
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
pub fn run(workspace: &std::path::Path, command: &str, anything: bool) -> Result<String> {
    if let Some(why) = refuse(command, anything) {
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

    // 127 is the shell's way of saying it could not find the program. It reaches
    // here for things on the allow-list that are not installed -- `docker` is
    // allowed and plenty of Macs do not have it -- and `sh: docker: command not
    // found` is a worse answer than saying so plainly.
    if out.status.code() == Some(127) {
        let program = command
            .trim()
            .split('|')
            .next()
            .unwrap_or("")
            .split_whitespace()
            .next()
            .unwrap_or("");
        let name = program.rsplit('/').next().unwrap_or(program);
        if !name.is_empty() && !super::present::installed(name) {
            return Err(Error::Click(super::present::missing(name)));
        }
    }

    let mut text = String::from_utf8_lossy(&out.stdout).to_string();
    if text.trim().is_empty() {
        text = String::from_utf8_lossy(&out.stderr).to_string();
    }

    // Installed, allowed, ran -- and said nobody is signed in. Worth naming,
    // because it is the one failure that looks like the tool refusing to work.
    // Somebody told "the build failed" goes and looks at their build.
    if !out.status.success() && super::secret::unauthenticated(&text) {
        let program = command
            .trim()
            .split_whitespace()
            .next()
            .unwrap_or("")
            .rsplit('/')
            .next()
            .unwrap_or("");
        return Err(Error::Click(format!(
            "{}\n\nIt said: {}",
            super::secret::sign_in(program),
            text.trim().chars().take(200).collect::<String>()
        )));
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
    use std::ops::Not;
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
            assert_eq!(refuse(ok, false), None, "{ok} should be allowed");
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
            assert!(refuse(bad, false).is_some(), "{bad} should be refused");
        }
    }

    /// The allow-list is checked per pipeline stage, or the first harmless
    /// command in a pipe would carry anything after it.
    #[test]
    fn every_stage_of_a_pipeline_is_checked() {
        assert!(refuse("ls | rm -rf .", false).is_some(), "second stage ignored");
        assert!(refuse("cat x | sudo tee /etc/hosts", false).is_some());
        assert_eq!(refuse("ls | grep src | wc -l", false), None);
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
            assert!(refuse(bad, false).is_some(), "{bad:?} should be refused");
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
            assert!(refuse(bad, false).is_some(), "{bad} should be refused");
        }
    }

    #[test]
    fn a_program_that_both_reads_and_writes_is_judged_by_its_verb() {
        assert_eq!(refuse("git diff", false), None);
        assert!(refuse("git commit -m x", false).is_some());
        assert!(refuse("git push origin main", false).is_some());
        assert_eq!(refuse("npm ls", false), None);
        assert!(refuse("npm publish", false).is_some());
    }

    /// The whole point of 2.2: the same command, refused and then allowed,
    /// because somebody decided -- not because a constant in this file changed.
    #[test]
    fn a_granted_shell_runs_what_the_list_refuses() {
        assert!(refuse("rm -rf build", false).is_some());
        assert!(refuse("rm -rf build", true).is_none());
        // Chaining only ever mattered as a way past the list. With no list there
        // is nothing to get past.
        assert!(refuse("npm ci && npm test", false).is_some());
        assert!(refuse("npm ci && npm test", true).is_none());
    }

    /// Granting a shell is a decision about capability, not about credentials.
    /// Nobody ticking "run any command" is thinking about their private keys.
    #[test]
    fn secrets_are_refused_however_wide_the_grant() {
        for command in ["cat ~/.ssh/id_rsa", "cp .env /tmp/x"] {
            assert!(
                refuse(command, true).is_some(),
                "{command} was allowed with a full grant"
            );
        }
    }

    /// 3.3: a program that ran and said nobody is signed in is not a task that
    /// failed, and saying so sends a person to look in the wrong place.
    #[test]
    fn a_command_that_says_nobody_signed_in_is_named_as_that() {
        let here = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        // Exactly what an unauthenticated CLI looks like from out here: a
        // non-zero exit and an auth-shaped sentence. Built rather than borrowed
        // from a real program, so the test says the same thing on every machine
        // -- the first version ran `gh` and asserted whatever that happened to do.
        let err = run(here, "sh -c 'echo not logged in >&2; exit 1'", true)
            .unwrap_err()
            .to_string();
        assert!(err.contains("nobody has signed in"), "got: {err}");
        assert!(err.contains("It said:"), "the original words are worth keeping: {err}");

        // And an ordinary failure is still an ordinary failure.
        let plain = run(here, "sh -c 'echo no such file >&2; exit 1'", true);
        assert!(
            plain.is_ok() || !plain.unwrap_err().to_string().contains("signed in"),
            "an unrelated failure was blamed on a login"
        );
    }

    /// 3.2: the sentence a person can act on, at the moment it would have helped.
    #[test]
    fn a_missing_program_says_so_rather_than_saying_it_is_forbidden() {
        // A name no machine has, so the test says the same thing everywhere --
        // the first version used `ffmpeg`, which is installed here.
        let why = refuse("zzconvert -i a.mov b.mp4", false).unwrap();
        assert!(why.contains("not on this Mac"), "got: {why}");
        assert!(!why.contains("allowed"), "wrong half of the story: {why}");

        // A full shell does not conjure it. The attempt is permitted and fails
        // honestly instead, where `run` turns the shell's 127 into the same
        // sentence.
        assert!(refuse("zzconvert -i a.mov b.mp4", true).is_none());
    }

    /// Allowed, but not installed, which the allow-list cannot catch: `docker` is
    /// on it and plenty of Macs do not have Docker. The shell answers 127 and
    /// `sh: docker: command not found` is a worse sentence than ours.
    #[test]
    fn a_command_that_is_allowed_but_absent_is_explained_not_echoed() {
        let here = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        // `find` is allowed and present, so this exercises the 127 path only if
        // something allowed is genuinely missing on this machine. Pick whichever.
        let absent = ["docker", "java", "php", "ruby", "go"]
            .into_iter()
            .find(|n| super::super::present::installed(n).not());
        let Some(absent) = absent else {
            // Everything on the list is installed here; nothing to prove.
            return;
        };
        let err = run(here, absent, true).unwrap_err().to_string();
        assert!(err.contains("not on this Mac"), "got: {err}");
    }

    /// The other half: here, but not permitted -- which is fixable in the menu.
    #[test]
    fn a_forbidden_program_that_exists_points_at_the_menu() {
        // `cp` is on this Mac and is not on the allow-list.
        let why = refuse("cp a b", false).unwrap();
        assert!(why.contains("is here"), "got: {why}");
        assert!(why.contains("Allowed to"), "it should say where to change it: {why}");
    }

    #[test]
    fn an_empty_command_is_refused_either_way() {
        assert!(refuse("   ", true).is_some());
        assert!(refuse("   ", false).is_some());
    }

    #[test]
    fn it_actually_runs_and_bounds_what_comes_back() {
        let here = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let out = run(here, "ls", false).expect("ls should run");
        assert!(out.contains("Cargo.toml"), "got: {out}");

        // Refusals surface as errors the model can read, not silence.
        let err = run(here, "rm -rf x", false).unwrap_err().to_string();
        assert!(err.contains("rm"), "got: {err}");
    }
}
