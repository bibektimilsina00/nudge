//! How much of each thing goes in front of the model.
//!
//! Every part of a prompt was assembled by being interpolated into one string,
//! which meant nothing decided how big any of it was allowed to be. That held
//! while the parts were small and stopped holding the day one of them was not:
//! 197 tools came to 24,651 characters, forty-seven per cent of everything sent,
//! and nothing noticed because there was nothing to notice with.
//!
//! Those particular sections are cut at the source now -- see
//! `tools::relevant` -- but the shape of the problem is general. The
//! accessibility tree of a busy application, a long history, a folder of skills:
//! each is bounded only by how big it happens to be today.
//!
//! ## Ranks, not a queue
//!
//! When it does not all fit, what goes is decided by what the turn is *for*. A
//! turn is about the person in front of the screen: what they asked, what they
//! said before, what the screen is, what has been learned about it. Those are
//! kept whatever else goes. A catalogue is a convenience and is cut first.
//!
//! ## Trimmed on a line, and said out loud
//!
//! A part that does not fit is cut at a line boundary and told to say so. A list
//! silently missing its last third is worse than a shorter list: the model
//! cannot tell the difference between "there is no such tool" and "the tool is
//! past where this was cut", and will confidently report the first.

/// How hard a part fights for its room.
///
/// Nothing is exempt. A rank that could never be trimmed would make the ceiling
/// a suggestion -- which it was, on the first attempt: history came in as "never
/// cut" and a thousand steps took the prompt to 138,000 characters with a budget
/// of 20,000 sitting right there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Keep {
    /// Cut last, and only once everything else is down to its floor: the goal,
    /// the screen, what they said, what was learned.
    Last,
    /// Useful, and the turn survives without it.
    IfItFits,
    /// A catalogue. The first thing to go.
    Spare,
}

#[derive(Debug, Clone)]
pub struct Part {
    pub name: &'static str,
    pub keep: Keep,
    pub text: String,
    /// Which end matters when there is not room for all of it.
    ///
    /// A list reads from the top, so it keeps its first lines. A history reads
    /// from the bottom -- the newest step is the last line -- and trimming it
    /// from the end would throw away the only part anybody needed.
    pub newest_last: bool,
}

impl Part {
    pub fn new(name: &'static str, keep: Keep, text: impl Into<String>) -> Self {
        Part {
            name,
            keep,
            text: text.into(),
            newest_last: false,
        }
    }

    /// A part whose end is the part worth keeping.
    pub fn ending(name: &'static str, keep: Keep, text: impl Into<String>) -> Self {
        Part {
            newest_last: true,
            ..Part::new(name, keep, text)
        }
    }
}

/// What a trimmed part says about itself.
fn cut_to(text: &str, room: usize, newest_last: bool) -> String {
    let mut kept: Vec<&str> = Vec::new();
    let mut dropped = 0usize;
    let mut spent = 0usize;
    let lines: Vec<&str> = match newest_last {
        true => text.lines().rev().collect(),
        false => text.lines().collect(),
    };
    for line in lines {
        // The +1 is the newline the line will be joined with.
        if spent + line.len() < room {
            spent += line.len() + 1;
            kept.push(line);
        } else {
            dropped += 1;
        }
    }
    if newest_last {
        kept.reverse();
    }
    let mut out = kept.join("\n");
    out.push('\n');
    match dropped {
        0 => out,
        n => match newest_last {
            true => format!("[{n} earlier lines, cut to fit]\n{out}"),
            false => format!("{out}[{n} more lines, cut to fit -- ask for what you need]\n"),
        },
    }
}

/// Fit the parts inside a ceiling, cheapest first.
///
/// Returns them in the order they came, so the caller can interpolate each one
/// where it belongs -- the budget decides how much of a part there is, never
/// where it goes.
pub fn fit(mut parts: Vec<Part>, ceiling: usize) -> Vec<Part> {
    let mut room: Vec<usize> = parts.iter().map(|p| p.text.len()).collect();
    let total: usize = room.iter().sum();
    if total <= ceiling {
        return parts;
    }

    // Spare first, then IfItFits, then Last -- and the biggest within a rank,
    // because cutting the largest thing is what makes room and cutting three
    // small ones to save one large one costs three sections for no gain.
    let mut order: Vec<usize> = (0..parts.len()).collect();
    order.sort_by_key(|&i| {
        (
            std::cmp::Reverse(parts[i].keep),
            std::cmp::Reverse(parts[i].text.len()),
        )
    });

    // How much room each part may have, decided before anything is cut.
    //
    // Deciding and cutting in one pass meant cutting a part that had already
    // been cut: the second pass counted the first pass's own "[6 more lines]"
    // note as a line, dropped it, and wrote "[1 earlier lines]" over the top of
    // a part missing seven. A part is measured as often as necessary and cut
    // exactly once.
    let mut over = total - ceiling;
    // A tenth first; then no floor at all, because a ceiling that gives way to a
    // floor is not a ceiling.
    for floor in [10, usize::MAX] {
        for &i in &order {
            if over == 0 {
                break;
            }
            let take = (room[i] - room[i] / floor).min(over);
            room[i] -= take;
            over -= take;
        }
    }

    for (i, part) in parts.iter_mut().enumerate() {
        if room[i] < part.text.len() {
            part.text = cut_to(&part.text, room[i], part.newest_last);
        }
    }
    parts
}

/// Find a part by name, for interpolating it back where it belongs.
pub fn text<'a>(parts: &'a [Part], name: &str) -> &'a str {
    parts
        .iter()
        .find(|p| p.name == name)
        .map(|p| p.text.as_str())
        .unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(n: usize, word: &str) -> String {
        (0..n).map(|i| format!("{word} {i}\n")).collect()
    }

    #[test]
    fn under_the_ceiling_nothing_is_touched() {
        let parts = vec![
            Part::new("goal", Keep::Last, "open safari"),
            Part::new("tools", Keep::Spare, lines(3, "tool")),
        ];
        let fitted = fit(parts.clone(), 10_000);
        assert_eq!(fitted[1].text, parts[1].text);
    }

    /// The catalogue goes before the conversation does.
    #[test]
    fn what_the_turn_is_about_outlives_what_is_merely_available() {
        let parts = vec![
            Part::ending("history", Keep::Last, lines(20, "step")),
            Part::new("controls", Keep::IfItFits, lines(200, "control")),
            Part::new("tools", Keep::Spare, lines(400, "tool")),
        ];
        let fitted = fit(parts, 2_000);
        // The catalogue gives up almost everything; the conversation gives up
        // almost nothing and keeps its newest end. Both are trimmable -- a rank
        // that could not be cut would make the ceiling a suggestion -- so what
        // is asserted is the order, not an exemption.
        assert!(
            text(&fitted, "tools").len() < text(&fitted, "controls").len(),
            "the catalogue should have given up more than the controls"
        );
        assert!(
            text(&fitted, "history").lines().count() > 15,
            "history gave up too much: {:?}",
            text(&fitted, "history")
        );
        assert!(
            text(&fitted, "history").contains("step 19"),
            "the newest went"
        );
    }

    /// A list silently missing its end is worse than a short list: nothing can
    /// tell "no such tool" from "past where it was cut".
    #[test]
    fn a_trimmed_part_says_that_it_was_trimmed() {
        let parts = vec![Part::new("tools", Keep::Spare, lines(500, "tool"))];
        let fitted = fit(parts, 400);
        assert!(
            text(&fitted, "tools").contains("cut to fit"),
            "{:?}",
            fitted[0].text
        );
    }

    /// History reads from the bottom: the newest step is the last line, and
    /// trimming it from the end would throw away the only part anybody needed.
    #[test]
    fn a_history_keeps_its_newest_end() {
        let parts = vec![Part::ending("history", Keep::Spare, lines(200, "step"))];
        let fitted = fit(parts, 300);
        let kept = text(&fitted, "history");
        assert!(kept.contains("step 199"), "the newest went: {kept}");
        assert!(!kept.contains("step 0\n"), "the oldest stayed: {kept}");
        assert!(kept.contains("earlier lines, cut to fit"));
    }

    /// The ceiling is a ceiling, not a preference.
    #[test]
    fn everything_gives_way_before_the_ceiling_does() {
        let parts = vec![
            Part::ending("history", Keep::Last, lines(4000, "step")),
            Part::new("tools", Keep::Spare, lines(4000, "tool")),
        ];
        let fitted = fit(parts, 2_000);
        let total: usize = fitted.iter().map(|p| p.text.len()).sum();
        assert!(total <= 2_400, "{total} characters, ceiling was 2,000");
    }

    #[test]
    fn nothing_is_left_as_a_heading_with_nothing_under_it() {
        let parts = vec![
            Part::new("keep", Keep::Last, lines(100, "kept")),
            Part::new("tools", Keep::Spare, lines(100, "tool")),
        ];
        let fitted = fit(parts, 10);
        // A tenth survives rather than nothing at all.
        assert!(!text(&fitted, "tools").is_empty());
    }
}
