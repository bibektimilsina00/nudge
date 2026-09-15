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
