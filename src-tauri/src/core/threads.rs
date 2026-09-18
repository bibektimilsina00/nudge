//! What was going on before, after the process that was doing it has gone.
//!
//! A session's continuity was a value in memory with a five-minute timer on it:
//! ask again inside the window and the last few lines came with you, ask outside
//! it and there had never been a conversation. Quitting the app forgot
//! everything, every time.
//!
//! That is visible in the one place Nudge deliberately holds a thread across
//! turns. A tour ends by offering the next chapter -- "shall I show you how to
//! import a clip?" -- and the answer to that offer is only useful if the next
//! turn knows what the last one covered. Six minutes later, or after a restart,
//! it did not, and the offer became a lie.
//!
//! ## The conversation you are in, and nothing older
//!
//! Only the current thread is carried: what was being done a few minutes ago,
//! which is what makes "that", "it" and "the next bit" resolve. When the subject
//! changes, the new conversation starts clean.
//!
//! An earlier version carried a line about every thread from the last eight
//! hours, on the reasoning that "carry on with what we were doing" might mean
//! this morning. In use it meant a question about the screen arrived with three
//! unrelated errands attached, and the model reached for them -- so the window
//! is one conversation again. The file keeps the rest; nothing reads it.
//!
//! ## One line at a time, like everything else here
//!
//! Appended whole to `~/.config/nudge/threads.jsonl`, beside the audit log and
//! the timings, for the reasons written down there: a line is written whole or
//! not at all, a line that will not parse costs that line, and there is no schema
//! to migrate.
use serde::{Deserialize, Serialize};

/// A finished piece of work.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thread {
    /// Milliseconds since the epoch, so the file carries no timezone.
    pub at: u64,
    /// What they asked for, in their own words -- so this field is trusted too,
    /// and for an ordinary turn it is the whole of what they said.
    pub goal: String,
    /// Anything else they said: answers typed to an agent's questions. **The
    /// trusted channel** -- see
    /// `Session::said`. Kept apart on disk for the same reason it is kept apart
    /// in memory: it is the one list a web page cannot write to, and it stays
    /// that way only if nothing merges it with the other one.
    pub said: Vec<String>,
    /// The tail of what was done. Untrusted: it holds tool output, page text and
    /// whatever a search returned.
    pub tail: Vec<String>,
    /// A line that means "stop reading here".
    ///
    /// Written when somebody says they are changing the subject. Forgetting used
    /// to clear a value in memory, which the file has just made pointless: quit
    /// and reopen and the forgotten thread was back. A marker in the file is
    /// forgotten in the same place the threads live, and stays forgotten.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub wall: bool,
}

/// How many lines of the untrusted tail are kept.
///
/// The same six the warm carry used, because that is what it is for. Anything
/// older is better re-read than half-remembered.
const TAIL: usize = 6;
/// And a ceiling on them together.
const TAIL_CHARS: usize = 2000;

/// How long the detail stays relevant.
const FRESH: u64 = 5 * 60 * 1000;

/// How many threads are read back. The file grows forever; the question never
/// reaches further back than the last few.
const RECENT: usize = 8;

/// Where the threads are kept.
///
/// A value rather than a constant path, for the reason the audit log is one: a
/// test that writes to the real file is a test that reads somebody's actual
/// conversations and leaves its own behind for the next run to find.
#[derive(Clone, Debug, Default)]
pub struct Threads {
    path: Option<std::path::PathBuf>,
}

impl Threads {
    /// Beside the config, the audit log and the timings.
    pub fn open() -> Self {
        Threads {
            path: dirs::home_dir().map(|d| d.join(".config/nudge/threads.jsonl")),
        }
    }

    /// Remembers nothing. For anything that must not touch the real file.
    pub fn nowhere() -> Self {
        Threads { path: None }
    }

    pub fn at(path: impl Into<std::path::PathBuf>) -> Self {
        Threads {
            path: Some(path.into()),
        }
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or_default()
}

impl Thread {
    pub fn new(goal: String, said: Vec<String>, done: &[String]) -> Self {
        let mut tail = Vec::new();
        let mut spent = 0usize;
        for line in done.iter().rev().take(TAIL) {
            let room = TAIL_CHARS.saturating_sub(spent);
            if room < 40 {
                break;
            }
            let kept: String = line.chars().take(room).collect();
            spent += kept.chars().count();
            tail.push(kept);
        }
        tail.reverse();
        Thread {
            at: now_ms(),
            goal,
            said,
            tail,
            wall: false,
        }
    }
}

impl Threads {
    /// Draw a line: nothing before this is carried into another turn.
    pub fn forget(&self) {
        self.keep(&Thread {
            at: now_ms(),
            goal: String::new(),
            said: Vec::new(),
            tail: Vec::new(),
            wall: true,
        });
    }

    /// Write one down. Silent on failure: losing the record of a finished task must
    /// not fail the task.
    pub fn keep(&self, thread: &Thread) {
        use std::io::Write;
        let Some(path) = &self.path else { return };
        let Ok(line) = serde_json::to_string(thread) else {
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

    /// The last few, newest first.
    pub fn recent(&self) -> Vec<Thread> {
        let Some(path) = &self.path else {
            return Vec::new();
        };
        let Ok(text) = std::fs::read_to_string(path) else {
            return Vec::new();
        };
        let mut found: Vec<Thread> = text
            .lines()
            .rev()
            .filter_map(|line| serde_json::from_str(line).ok())
            .take(RECENT)
            .collect();
        found.sort_by_key(|t| std::cmp::Reverse(t.at));
        found
    }
}

/// What to put in front of the model about work that is already finished.
///
/// Newest first, and two shapes: the freshest thread in full, and the ones
/// before it as a line each. Pure so the windows can be tested without waiting
/// five minutes.
pub fn carry(threads: &[Thread], now: u64) -> Vec<String> {
    let Some(thread) = threads.first() else {
        return Vec::new();
    };
    if thread.wall || now.saturating_sub(thread.at) > FRESH {
        return Vec::new();
    }
    // The goal first, because it is what makes "that" and "it" resolve, and what
    // makes an offer of a next chapter answerable.
    let mut out = vec![format!("They had asked: {}", thread.goal)];
    out.extend(thread.tail.iter().cloned());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn thread(at: u64, goal: &str) -> Thread {
        Thread {
            at,
            goal: goal.into(),
            said: vec![goal.into()],
            tail: vec!["did a thing".into(), "did another".into()],
            wall: false,
        }
    }

    #[test]
    fn nothing_before_a_wall_is_carried() {
        let now = 200_000_000;
        let threads = [
            Thread {
                at: now - 1000,
                goal: String::new(),
                said: Vec::new(),
                tail: Vec::new(),
                wall: true,
            },
            thread(now - 60_000, "the thread they changed the subject on"),
        ];
        assert!(carry(&threads, now).is_empty());
    }

    #[test]
    fn a_fresh_thread_brings_its_detail() {
        let carried = carry(&[thread(1_000_000, "open safari")], 1_000_000 + 60_000);
        assert_eq!(carried.len(), 3, "{carried:?}");
        assert!(carried[0].contains("open safari"));
        assert!(carried.contains(&"did a thing".to_string()));
    }

    /// An hour later is a new conversation, not a continuation of the old one.
    #[test]
    fn a_cold_thread_is_not_carried_at_all() {
        let hour = 60 * 60 * 1000;
        assert!(carry(&[thread(1_000_000, "teach me davinci")], 1_000_000 + hour).is_empty());
    }

    /// Only the one being had. An earlier version carried a line about every
    /// thread from the last eight hours, and a question about the screen arrived
    /// with three unrelated errands attached to it.
    #[test]
    fn only_the_conversation_you_are_in_comes_with_you() {
        let now = 200_000_000;
        let threads = [
            thread(now - 60_000, "the one we are having"),
            thread(now - 120_000, "the one before it"),
        ];
        let carried = carry(&threads, now);
        assert!(carried[0].contains("the one we are having"));
        assert!(
            !carried.iter().any(|l| l.contains("the one before it")),
            "{carried:?}"
        );
    }
}
