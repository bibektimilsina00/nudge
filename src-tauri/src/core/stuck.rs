//! Noticing that a supervised tool has stopped getting anywhere.
//!
//! A person watching a terminal notices three things a program does not: that
//! nothing has happened for a long time, that the same thing keeps happening,
//! and that it is asking something it already asked. The third is the
//! supervisor's; the first is a clock; this is the second.
//!
//! **Repetition is not failure and a run must not be killed for it lightly.** A
//! build prints the same line for every file, a spinner redraws, a download
//! reports the same percentage twice. What matters is a *cycle with no progress*
//! — the same short block of output arriving over and over with nothing new
//! between the repeats.
//!
//! So the bar is deliberately high: the tail has to be the same handful of lines
//! several times over, with nothing else among them. Missing a loop costs the
//! fifteen-minute cap; calling a working build a loop costs somebody's work.

/// How many identical repeats before it counts.
///
/// Three is a coincidence. Six is a program going round.
const ENOUGH: usize = 6;

/// The most lines a repeating unit may be.
///
/// A cycle of two or three lines is a program stuck; a hundred identical lines
/// is a log, and one of those is worth stopping.
const UNIT: usize = 4;

/// Is this output going round in circles? Returns what is repeating.
///
/// Given the tail of what a process has printed.
pub fn looping(text: &str) -> Option<String> {
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim_end)
        .filter(|l| !l.trim().is_empty())
        .collect();
    if lines.len() < ENOUGH {
        return None;
    }

    // Try the shortest cycle first: one line repeating is the commonest, and a
    // longer "cycle" made of a repeating single line is the same fact said
    // twice.
    for unit in 1..=UNIT {
        if lines.len() < unit * ENOUGH {
            continue;
        }
        let tail = &lines[lines.len() - unit * ENOUGH..];
        let first = &tail[..unit];
        if tail.chunks(unit).all(|c| c == first) {
            return Some(first.join("\n"));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_line_going_round_is_noticed() {
        let out = "starting\n".to_string() + &"waiting for lock...\n".repeat(8);
        assert_eq!(looping(&out).as_deref(), Some("waiting for lock..."));
    }

    #[test]
    fn a_cycle_of_several_lines_is_noticed() {
        let cycle = "retrying\nconnection refused\n";
        let out = format!("connecting\n{}", cycle.repeat(7));
        let found = looping(&out).expect("a two-line cycle is still a cycle");
        assert!(found.contains("retrying"), "{found}");
    }

    /// The important negative. A build that prints a line per file is working.
    #[test]
    fn ordinary_progress_is_not_a_loop() {
        let out: String = (0..40).map(|i| format!("Compiling crate_{i}\n")).collect();
        assert!(looping(&out).is_none());
    }

    /// Nor is a repeat that stopped repeating.
    #[test]
    fn something_that_repeated_and_then_moved_on_is_fine() {
        let out = "same\n".repeat(10) + "and then something else happened\n";
        assert!(looping(&out).is_none(), "the tail is what matters");
    }

    /// Three is a coincidence.
    #[test]
    fn a_few_repeats_are_not_enough() {
        assert!(looping(&"tick\n".repeat(3)).is_none());
        assert!(looping(&"tick\n".repeat(5)).is_none());
        assert!(looping(&"tick\n".repeat(6)).is_some());
    }

    #[test]
    fn nothing_and_almost_nothing_are_not_loops() {
        assert!(looping("").is_none());
        assert!(looping("one line\n").is_none());
        assert!(looping("\n\n\n\n\n\n\n\n").is_none());
    }

    /// Blank lines between repeats do not hide a cycle.
    #[test]
    fn a_cycle_padded_with_blanks_is_still_a_cycle() {
        let out = "stuck\n\n".repeat(8);
        assert_eq!(looping(&out).as_deref(), Some("stuck"));
    }
}
