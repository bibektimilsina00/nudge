//! Is that tour any good?
//!
//! A wrong box is the most expensive thing a tour can do. It is said with the
//! same confidence as a right one, it is drawn over an application the person
//! does not know yet -- that is why they asked -- and it is the one part of the
//! answer they cannot check. A sentence that is slightly wrong gets corrected by
//! the screen in front of them; a box round the wrong panel teaches them the
//! wrong name for it.
//!
//! So the tours get scored, the way the clicks already do. Not against labelled
//! answers -- nobody is going to draw the true rectangle round "the sidebar" for
//! fifty screenshots -- but against the things that are checkable without a
//! label, which turn out to be most of the ways a tour actually goes wrong.
use crate::core::provider::Step;

/// Something wrong with a tour, in the words somebody would use about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Complaint {
    /// Which part, counted from one as a person would. `None` for the tour as a
    /// whole.
    pub part: Option<usize>,
    pub about: String,
}

/// Past this, a sentence is a paragraph, and this is read aloud.
const LONGEST: usize = 240;
/// Fewer than this is not a tour of anything, more is a lecture.
const FEWEST: usize = 3;
const MOST: usize = 14;
/// Two boxes sharing this much of their area are describing one thing twice.
const SAME: f64 = 0.55;
/// A box bigger than this much of the screen outlines everything and therefore
/// points at nothing.
const WHOLE: f64 = 0.66;

type Rect = (f64, f64, f64, f64);

fn rect(at: (f64, f64), size: (f64, f64)) -> Rect {
    (
        at.0 - size.0 / 2.0,
        at.1 - size.1 / 2.0,
        at.0 + size.0 / 2.0,
        at.1 + size.1 / 2.0,
    )
}

fn area(r: Rect) -> f64 {
    (r.2 - r.0).max(0.0) * (r.3 - r.1).max(0.0)
}

fn overlap(a: Rect, b: Rect) -> f64 {
    let hit = (a.0.max(b.0), a.1.max(b.1), a.2.min(b.2), a.3.min(b.3));
    let shared = area(hit);
    match area(a).min(area(b)) {
        n if n <= 0.0 => 0.0,
        n => shared / n,
    }
}

/// Everything wrong with this tour, in the order somebody would notice it.
///
/// Every rule here is checkable from the answer and the screen's size alone.
/// That is not a limitation, it is what makes this runnable fifty times: the
/// alternative needs somebody to draw the true rectangle round "the sidebar" for
/// every screenshot, which is a corpus nobody will ever finish.
pub fn score(parts: &[Step], screen: (f64, f64)) -> Vec<Complaint> {
    let mut found = Vec::new();
    let whole = screen.0 * screen.1;

    let whole_tour = |about: &str| Complaint {
        part: None,
        about: about.to_string(),
    };

    if parts.len() < FEWEST {
        found.push(whole_tour(&format!(
            "only {} parts -- that is a sentence, not a tour",
            parts.len()
        )));
    }
    if parts.len() > MOST {
        found.push(whole_tour(&format!("{} parts is a lecture", parts.len())));
    }

    // A tour ends by putting somebody at the start of something.
    match parts.last() {
        Some(Step::Point { size: None, .. }) => {}
        Some(_) => found.push(whole_tour(
            "ends on something to look at, not something to do",
        )),
        None => found.push(whole_tour("no parts at all")),
    }

    // A tour with no marks is a monologue, whatever else is true of it.
    let marked = parts
        .iter()
        .filter(|p| matches!(p, Step::Point { .. }))
        .count();
    if marked * 2 < parts.len() {
        found.push(whole_tour(&format!(
            "only {marked} of {} parts point at anything",
            parts.len()
        )));
    }

    let mut boxes: Vec<(usize, Rect)> = Vec::new();
    for (i, part) in parts.iter().enumerate() {
        let n = i + 1;
        if part.say().chars().count() > LONGEST {
            found.push(Complaint {
                part: Some(n),
                about: format!("{} characters in one breath", part.say().chars().count()),
            });
        }
        let Step::Point { at, size, .. } = part else {
            continue;
        };
        let Some(size) = size else { continue };
        let r = rect((at.x, at.y), *size);

        if area(r) > whole * WHOLE {
            found.push(Complaint {
                part: Some(n),
                about: format!(
                    "the box covers {:.0}% of the screen, which points at nothing",
                    area(r) / whole * 100.0
                ),
            });
        }
        // Off the screen entirely, or mostly. A box half in the void is a box
        // drawn from coordinates that were never checked.
        let visible = overlap(r, (0.0, 0.0, screen.0, screen.1));
        if visible < 0.9 {
            found.push(Complaint {
                part: Some(n),
                about: format!(
                    "{:.0}% of the box is off the screen",
                    (1.0 - visible) * 100.0
                ),
            });
        }
        // There was a rule here: a box should contain at least one control the
        // system reports. It was measured and it is wrong. VS Code publishes
        // ninety-nine controls and almost all of them are toolbar buttons along
        // the top, so a correct box round the terminal panel contains none of
        // them -- the rule fired twice in five tours, both times on a box that
        // was right. A check that cries wolf on correct work teaches people to
        // ignore the checker, which costs more than the check was worth.
        for (other, was) in &boxes {
            if overlap(r, *was) > SAME {
                found.push(Complaint {
                    part: Some(n),
                    about: format!("the same area as part {other}"),
                });
            }
        }
        boxes.push((n, r));
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::provider::Act;
    use crate::core::screen::capture::Point;

    fn boxed(say: &str, at: (f64, f64), size: (f64, f64)) -> Step {
        Step::Point {
            at: Point { x: at.0, y: at.1 },
            say: say.into(),
            act: Act::Click,
            size: Some(size),
            control: None,
        }
    }

    fn ring(say: &str) -> Step {
        Step::Point {
            at: Point { x: 10.0, y: 10.0 },
            say: say.into(),
            act: Act::Click,
            size: None,
            control: None,
        }
    }

    fn good() -> Vec<Step> {
        vec![
            boxed("the sidebar", (100.0, 300.0), (180.0, 500.0)),
            boxed("the editor", (600.0, 300.0), (600.0, 500.0)),
            ring("click here to start"),
        ]
    }

    #[test]
    fn a_tour_that_is_fine_is_not_complained_about() {
        assert_eq!(score(&good(), (1000.0, 800.0)), Vec::new());
    }

    #[test]
    fn a_box_round_everything_is_caught() {
        let mut parts = good();
        parts[0] = boxed("all of it", (500.0, 400.0), (950.0, 760.0));
        let said = score(&parts, (1000.0, 800.0));
        assert!(
            said.iter().any(|c| c.about.contains("points at nothing")),
            "{said:?}"
        );
    }

    /// Two boxes round one panel is one of them being wrong, and which one is
    /// not knowable from here -- but the tour is.
    #[test]
    fn the_same_area_twice_is_caught() {
        let mut parts = good();
        parts[1] = boxed("the editor again", (110.0, 300.0), (180.0, 500.0));
        let said = score(&parts, (1000.0, 800.0));
        assert!(
            said.iter().any(|c| c.about.contains("same area")),
            "{said:?}"
        );
    }

    #[test]
    fn a_tour_that_never_points_at_anything_is_caught() {
        let parts = vec![
            Step::Reply { say: "one".into() },
            Step::Reply { say: "two".into() },
            Step::Reply {
                say: "three".into(),
            },
        ];
        let said = score(&parts, (1000.0, 800.0));
        assert!(
            said.iter().any(|c| c.about.contains("point at anything")),
            "{said:?}"
        );
        assert!(
            said.iter().any(|c| c.about.contains("something to do")),
            "and it ends nowhere: {said:?}"
        );
    }

    /// A tree is not evidence about a box. Half the applications worth touring
    /// publish none, and the ones that do publish a toolbar and call it a day.
    #[test]
    fn a_tour_of_an_application_with_no_tree_is_judged_the_same_way() {
        // Which is the point: the applications most worth touring -- a video
        // editor, a game engine, anything drawn rather than built -- publish
        // nothing about themselves, and a scorer that needed a tree would be
        // silent exactly where a tour matters most.
        assert_eq!(score(&good(), (1000.0, 800.0)), Vec::new());
    }
}
