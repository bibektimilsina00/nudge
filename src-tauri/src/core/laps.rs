//! Where a turn's time actually went.
//!
//! Not logging machinery. A turn is one model call and a handful of waits, so a
//! line naming each of them costs nothing next to the call itself.
//!
//! It exists because the two slowest things this project has found were both
//! invisible: fifty thousand characters of dead history re-sent every turn, and
//! a second spent waiting for our own voice to stop. Neither was a hard problem.
//! Both survived for months because nothing printed a stage breakdown, and the
//! only number anyone had was the total.
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

pub struct Laps {
    last: Instant,
    marks: Vec<(&'static str, Duration)>,
    /// Already reported. The next `start` is then a new turn, rather than a
    /// second caller announcing the beginning of the same one.
    spent: bool,
}

impl Default for Laps {
    fn default() -> Self {
        Self {
            last: Instant::now(),
            marks: Vec::new(),
            spent: true,
        }
    }
}

impl Laps {
    /// A turn began. Ignored while one is already being timed -- a spoken turn
    /// starts in the hotkey and then passes through `advance`, and both would
    /// otherwise claim to be the start, throwing away the transcription.
    pub fn start(&mut self) {
        if !self.spent {
            return;
        }
        self.last = Instant::now();
        self.marks.clear();
        self.spent = false;
    }

    /// That stage is over. Measured from the previous mark, so the sum of the
    /// marks is the turn.
    pub fn mark(&mut self, stage: &'static str) {
        let now = Instant::now();
        self.marks
            .push((stage, now.saturating_duration_since(self.last)));
        self.last = now;
    }

    /// `wav 0.01s · heard 1.28s · still 0.24s · hush 0.98s · brain 4.43s · total 6.94s`
    ///
    /// Every stage, including the ones that cost nothing. A measurement that
    /// hides the small numbers cannot tell you that five of them add up, and
    /// four zeroes on a line is itself the answer to "where is the time going".
    ///
    /// `None` when no turn was being timed, so a step taken outside one prints
    /// nothing rather than a line of noise. Spends the turn either way.
    ///
    /// Does not spend it: the same turn is both printed and written down, and
    /// whichever went second used to get nothing.
    /// Is a turn being timed right now? Which is the same question as "is a
    /// turn happening", asked of the one thing that knows from the moment the
    /// key comes up rather than from the moment a session opens.
    pub fn running(&self) -> bool {
        !self.spent
    }

    pub fn stages(&self) -> Vec<(&'static str, Duration)> {
        match self.spent {
            true => Vec::new(),
            false => self.marks.clone(),
        }
    }

    pub fn line(&mut self) -> Option<String> {
        if self.spent {
            return None;
        }
        self.spent = true;
        if self.marks.is_empty() {
            return None;
        }
        let total: Duration = self.marks.iter().map(|(_, d)| *d).sum();
        let stages: Vec<String> = self
            .marks
            .iter()
            .map(|(stage, d)| format!("{stage} {:.2}s", d.as_secs_f32()))
            .collect();
        Some(format!(
            "{} · total {:.2}s",
            stages.join(" · "),
            total.as_secs_f32()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_turn_is_timed_once_however_many_callers_think_they_started_it() {
        let mut l = Laps::default();
        l.start();
        l.mark("heard");
        l.start(); // `advance`, inside a turn the hotkey already began
        l.mark("brain");

        let line = l.line().expect("a turn was timed");
        assert!(
            line.contains("heard"),
            "the second start wiped the first: {line}"
        );
        assert!(line.contains("brain"));
        assert!(line.contains("total"));

        // Spent. Reporting twice would print the same turn under the next one.
        assert_eq!(l.line(), None);

        // And the next turn stands alone.
        l.start();
        l.mark("shot");
        let next = l.line().expect("a second turn");
        assert!(!next.contains("heard"), "stale marks carried over: {next}");
    }

    /// A step outside a turn -- the privacy guard refusing before anything was
    /// marked -- must still spend the clock, or the next turn inherits its start.
    #[test]
    fn a_turn_with_nothing_marked_prints_nothing_and_still_ends() {
        let mut l = Laps::default();
        l.start();
        assert_eq!(
            l.line(),
            None,
            "nothing was measured, so there is nothing to say"
        );

        l.start();
        l.mark("shot");
        assert!(l.line().is_some(), "the clock was never released");
    }
}

/// One turn, as a line in a file somebody can add up later.
///
/// The printed line answers "why was that slow" while it is still on screen.
/// This answers the questions that need more than one turn to ask: is the model
/// call slower on Tuesdays, does transcription get worse with a longer recording,
/// did that change actually help. Neither replaces the other -- a number you have
/// to catch as it scrolls past is not a measurement.
///
/// `~/.config/nudge/timings.jsonl`, beside the audit log and for the same
/// reasons: one process appending whole lines, readable with `jq`, and a line
/// that will not parse costs that line rather than the file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Turn {
    /// Milliseconds since the epoch. A number rather than a formatted time, so
    /// the file does not carry a timezone that was true on one machine only.
    pub at: u64,
    /// Each stage and how long it took, in the order they happened. Ordered
    /// rather than a map, because the order *is* the pipeline and sorting it
    /// alphabetically would throw that away.
    pub stages: Vec<(String, f32)>,
    pub total: f32,
    /// Which model answered, so a file that spans a change of provider can be
    /// split by it rather than averaged across it.
    pub provider: String,
    /// How long the request was, in characters. Not the request itself: this
    /// file is for arithmetic, and the audit log's rule about contents applies
    /// here with less reason to bend it.
    pub said: usize,
    /// Whether something was drawn on the screenshot, which costs a decode and
    /// an encode and should show up in `shot`.
    pub drew: bool,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or_default()
}

impl Turn {
    pub fn new(
        stages: Vec<(&'static str, Duration)>,
        provider: &str,
        said: usize,
        drew: bool,
    ) -> Self {
        Self {
            at: now_ms(),
            total: stages.iter().map(|(_, d)| d.as_secs_f32()).sum(),
            stages: stages
                .into_iter()
                .map(|(s, d)| (s.to_string(), d.as_secs_f32()))
                .collect(),
            provider: provider.to_string(),
            said,
            drew,
        }
    }
}

/// Where the turns are kept. `None` only if there is no home directory.
pub fn path() -> Option<std::path::PathBuf> {
    dirs::home_dir().map(|d| d.join(".config/nudge/timings.jsonl"))
}

/// Append one turn. Silent on failure: a measurement that can break the thing
/// being measured is worse than no measurement.
pub fn keep(turn: &Turn) {
    use std::io::Write;
    let Some(path) = path() else { return };
    let Ok(line) = serde_json::to_string(turn) else {
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
