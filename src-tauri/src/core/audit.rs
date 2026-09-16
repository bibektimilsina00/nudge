//! What Nudge actually did, written down where a person can read it.
//!
//! The test this exists to pass was set a long time ago and never met: *can the
//! person see afterwards what was done with it?* Until now an agent reported
//! what it did and Nudge believed it — the only record was the model's own
//! account of itself, which is the one witness with a reason to be generous.
//!
//! It is also what makes the permission work legible. A gate that decides
//! quietly is a gate nobody can check, and "it asked me something last Tuesday"
//! is not a thing anybody can audit from memory.
//!
//! ## The contents never go in
//!
//! This is the design, and it is the part a first attempt gets wrong. Tool
//! inputs and outputs carry API tokens, session cookies, mail bodies and whole
//! files. A log that copies them is a log that has collected every secret the
//! program has touched into one plaintext file that nothing guards — the audit
//! trail becomes the leak it was meant to catch.
//!
//! So: **the call, its shape and its outcome. Never its contents.** A fetch
//! records the host and how much came back, not the page. A command records the
//! command, not its output. Enough to recognise what happened and to notice what
//! should not have, and not enough to be worth stealing.
//!
//! ## One line at a time
//!
//! JSON Lines rather than a database, which is a deliberate departure from the
//! two projects this was read from. Both use SQLite, and both are servers with
//! concurrent writers and a query surface. This is one process appending to one
//! file and reading it back newest-first; SQLite would buy a dependency, a
//! schema and a migration story in exchange for queries nobody is going to run.
//!
//! Append-only matters more than the format: a line is written whole or not at
//! all, so a process killed mid-write loses that line rather than the file, and
//! a line that will not parse is skipped rather than taken as the end.
use serde::{Deserialize, Serialize};

/// How much of a thing's description is kept.
///
/// Long enough to recognise a call, short enough that nothing worth stealing
/// fits. A command is usually well under this; a base64 blob pasted into one is
/// not, and that is the case this bounds.
const MOST_SAID: usize = 160;

/// How much of an outcome is kept. A little more, because "refused because ..."
/// is the sentence somebody will actually be reading.
const MOST_OUTCOME: usize = 300;

/// What happened, in the only three shapes that matter afterwards.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "lowercase")]
pub enum Outcome {
    /// It happened. `detail` is a measurement, never a payload — "3040 chars",
    /// "14 files".
    Did { detail: String },
    /// A gate said no, and this is what it said.
    Refused { why: String },
    /// A person was asked. Whether they said yes is its own later entry, because
    /// the gap between the two is the interesting part.
    Asked { question: String },
}

/// One thing that happened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// Milliseconds since the epoch. A number rather than a formatted time, so
    /// the file does not carry a timezone that was true on the machine that
    /// wrote it and nowhere else.
    pub at: u64,
    /// Which agent run, when there was one. Absent for foreground turns.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run: Option<u64>,
    /// The kind of thing: `fetch`, `shell`, `write`, `tool`.
    pub kind: String,
    /// What was asked for, clipped and redacted. Never a result.
    pub said: String,
    pub outcome: Outcome,
    /// Which grant allowed it, when one had to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule: Option<String>,
    /// What class of thing it was -- read, egress, write, exec, external.
    ///
    /// The `kind` beside it names the tool; this names the *effect*, which is
    /// the question somebody scanning a log is actually asking. It is also the
    /// only thing that can be said about a tool this repository has never heard
    /// of, whose name is a stranger's claim.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk: Option<String>,
}

impl Entry {
    pub fn new(kind: &str, said: &str, outcome: Outcome) -> Self {
        Self {
            at: now_ms(),
            run: None,
            kind: kind.to_string(),
            said: keep(said, MOST_SAID),
            outcome: match outcome {
                Outcome::Did { detail } => Outcome::Did {
                    detail: keep(&detail, MOST_OUTCOME),
                },
                Outcome::Refused { why } => Outcome::Refused {
                    why: keep(&why, MOST_OUTCOME),
                },
                Outcome::Asked { question } => Outcome::Asked {
                    question: keep(&question, MOST_OUTCOME),
                },
            },
            rule: None,
            risk: None,
        }
    }

    pub fn at_risk(mut self, risk: crate::core::risk::Risk) -> Self {
        self.risk = Some(risk.name().to_string());
        self
    }

    pub fn during(mut self, run: u64) -> Self {
        self.run = Some(run);
        self
    }

    pub fn allowed_by(mut self, rule: Option<String>) -> Self {
        self.rule = rule;
        self
    }
}

/// Clip to a length and redact what should never have been there.
///
/// Redaction first, so a credential sitting past the clip point is removed
/// rather than being cut in half and left recognisable. Clipping says that it
/// clipped, because a silently truncated line reads as the whole truth.
fn keep(text: &str, most: usize) -> String {
    let clean = crate::core::tools::secret::redact(text.trim());
    // Character boundaries, not bytes: cutting a multi-byte character in half
    // produces a string that will not serialise.
    if clean.chars().count() <= most {
        return clean;
    }
    let cut: String = clean.chars().take(most).collect();
    format!("{cut}… (clipped)")
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// The log itself.
#[derive(Debug, Clone)]
pub struct Audit {
    /// `None` in tests, so a test run cannot write to somebody's real log. The
    /// same shape the agent ledger uses, and for the same reason: a suite that
    /// appends to a live file is a suite that corrupts it.
    path: Option<std::path::PathBuf>,
}

impl Audit {
    /// Where it lives beside everything else Nudge keeps.
    pub fn open() -> Self {
        Self {
            path: dirs::home_dir().map(|d| d.join(".config/nudge/audit.jsonl")),
        }
    }

    pub fn nowhere() -> Self {
        Self { path: None }
    }

    pub fn at(path: impl Into<std::path::PathBuf>) -> Self {
        Self {
            path: Some(path.into()),
        }
    }

    /// Append one line.
    ///
    /// Failures are swallowed, and that is a decision rather than laziness:
    /// nothing Nudge does should stop because it could not write to its own
    /// diary. The cost is that silent loss is undetectable, which is why the
    /// write is a single `append` of one whole line rather than anything that
    /// can half-succeed.
    pub fn note(&self, entry: Entry) {
        use std::io::Write;
        let Some(path) = &self.path else { return };
        let Ok(line) = serde_json::to_string(&entry) else {
            return;
        };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = writeln!(f, "{line}");
        }
    }

    /// The most recent entries, newest first.
    ///
    /// A line that will not parse is skipped rather than ending the read — a
    /// half-written line from a process that was killed must not hide everything
    /// written before it.
    pub fn recent(&self, most: usize) -> Vec<Entry> {
        let Some(path) = &self.path else {
            return Vec::new();
        };
        let Ok(text) = std::fs::read_to_string(path) else {
            return Vec::new();
        };
        let mut out: Vec<Entry> = text
            .lines()
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect();
        out.reverse();
        out.truncate(most);
        out
    }

    /// Everything one agent run did, oldest first — which is the order somebody
    /// reads a story in.
    pub fn of_run(&self, run: u64) -> Vec<Entry> {
        let mut out: Vec<Entry> = self
            .recent(usize::MAX)
            .into_iter()
            .filter(|e| e.run == Some(run))
            .collect();
        out.reverse();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> std::path::PathBuf {
        let p =
            std::env::temp_dir().join(format!("nudge-audit-{name}-{}.jsonl", std::process::id()));
        let _ = std::fs::remove_file(&p);
        p
    }

    /// The rule the whole module exists for.
    #[test]
    fn a_credential_never_reaches_the_file() {
        let path = tmp("secret");
        let audit = Audit::at(&path);
        audit.note(Entry::new(
            "fetch",
            "https://example.com/?api_key=sk-live-9f2a7c4e1b83DEADBEEF",
            Outcome::Did {
                detail: "3040 chars".into(),
            },
        ));

        let on_disk = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(
            !on_disk.contains("sk-live-9f2a7c4e1b83"),
            "the key is in the log: {on_disk}"
        );
        // And the entry is still useful -- the host survives.
        assert!(on_disk.contains("example.com"), "{on_disk}");
        let _ = std::fs::remove_file(&path);
    }

    /// Long things are bounded and say that they were.
    #[test]
    fn nothing_long_enough_to_be_worth_stealing_is_kept() {
        let huge = "x".repeat(10_000);
        let e = Entry::new(
            "shell",
            &huge,
            Outcome::Did {
                detail: huge.clone(),
            },
        );
        assert!(e.said.chars().count() < MOST_SAID + 20, "{}", e.said.len());
        assert!(
            e.said.ends_with("(clipped)"),
            "a silent truncation reads as the whole truth"
        );
        match e.outcome {
            Outcome::Did { detail } => assert!(detail.chars().count() < MOST_OUTCOME + 20),
            _ => unreachable!(),
        }
    }

    /// A killed process leaves a half-written line; it must not hide the rest.
    #[test]
    fn a_broken_line_does_not_end_the_read() {
        let path = tmp("broken");
        let audit = Audit::at(&path);
        audit.note(Entry::new(
            "shell",
            "ls -la",
            Outcome::Did {
                detail: "ok".into(),
            },
        ));
        {
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .open(&path)
                .unwrap();
            writeln!(f, "{{\"at\": 1, \"kind\": \"half").unwrap();
        }
        audit.note(Entry::new(
            "fetch",
            "example.com",
            Outcome::Did {
                detail: "1 char".into(),
            },
        ));

        let back = audit.recent(10);
        assert_eq!(
            back.len(),
            2,
            "the unparseable line took a real one with it"
        );
        // Newest first.
        assert_eq!(back[0].kind, "fetch");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_run_reads_as_a_story_in_order() {
        let path = tmp("run");
        let audit = Audit::at(&path);
        audit.note(
            Entry::new(
                "shell",
                "first",
                Outcome::Did {
                    detail: "ok".into(),
                },
            )
            .during(7),
        );
        audit.note(
            Entry::new(
                "shell",
                "other run",
                Outcome::Did {
                    detail: "ok".into(),
                },
            )
            .during(8),
        );
        audit.note(
            Entry::new(
                "fetch",
                "second",
                Outcome::Did {
                    detail: "ok".into(),
                },
            )
            .during(7),
        );

        let story: Vec<String> = audit.of_run(7).into_iter().map(|e| e.said).collect();
        assert_eq!(story, vec!["first", "second"]);
        let _ = std::fs::remove_file(&path);
    }

    /// Refusals and questions are the entries worth having, so they round-trip.
    #[test]
    fn every_outcome_survives_the_round_trip() {
        let path = tmp("shapes");
        let audit = Audit::at(&path);
        audit.note(
            Entry::new(
                "shell",
                "rm -rf /",
                Outcome::Refused {
                    why: "read-only".into(),
                },
            )
            .allowed_by(None),
        );
        audit.note(Entry::new(
            "fetch",
            "evil.example",
            Outcome::Asked {
                question: "Shall I fetch something from evil.example?".into(),
            },
        ));

        let back = audit.recent(10);
        assert!(matches!(back[0].outcome, Outcome::Asked { .. }));
        assert!(matches!(back[1].outcome, Outcome::Refused { .. }));
        let _ = std::fs::remove_file(&path);
    }

    /// A log with nowhere to write is silent, not a crash.
    #[test]
    fn nowhere_is_a_valid_place_to_write() {
        let audit = Audit::nowhere();
        audit.note(Entry::new(
            "shell",
            "ls",
            Outcome::Did {
                detail: "ok".into(),
            },
        ));
        assert!(audit.recent(10).is_empty());
    }
}
