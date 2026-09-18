//! Playing a tour: the narration, and the mark that goes with each sentence.
//!
//! A tour arrives as [`Step::Tour`] -- one answer, cut into parts, each part a
//! `Point` where there is something to look at and a `Reply` where the sentence
//! is about nothing in particular. This is what turns that into something
//! happening on screen: emit the part, speak it, and move on when the voice has
//! *actually* stopped.
//!
//! **Waiting on the speech is the whole trick.** A tour timed by character count
//! drifts by a sentence before the end, and a box round the wrong panel while
//! the voice names another says the wrong place confidently. Where a mark should
//! change mid-sentence, the model cuts the sentence in two: the part boundary is
//! the cue, so there is nothing to synchronise.
//!
//! The chapter is written into the session before the offer is made, because a
//! tour is an introduction and not the manual. "Want me to show you how to
//! import a clip?" is only answerable if the next turn knows what the last one
//! covered -- see `Nudge::still_warm`, which carries it.
//!
//! [`demo`] plays a script from a file instead, which is how the format gets
//! looked at without spending a model call. `~/.config/nudge/tour.json` wins
//! over the built-in one, so trying a different tour is a save and a menu click.
use crate::app::state::{Screen, Voice};
use crate::config::Config;
use crate::core::provider::{act_from, Step};
use crate::core::run::session::Nudge;
use crate::core::screen::capture::Point;
use crate::core::voice::speech;
use serde::Deserialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

/// Escape has to end a tour as completely as it ends everything else, and the
/// loop is sitting in a sleep when it is pressed.
static STOP: AtomicBool = AtomicBool::new(false);

pub fn stop() {
    STOP.store(true, Ordering::SeqCst);
}

/// Show somebody round, and hand back the mark left on screen.
///
/// The last part stays up: a tour ends at the one thing to do first, and that
/// ring is the whole point of ending there. Nothing clears it -- the caller
/// returns it like any other step, and it waits for them the way a nudge does.
pub async fn play(
    app: &AppHandle,
    chapter: &str,
    parts: Vec<Step>,
    next: Option<String>,
) -> Option<Step> {
    STOP.store(false, Ordering::SeqCst);
    let mut cfg = app.state::<Nudge>().cfg.clone();
    app.state::<Voice>().get().apply(&mut cfg);
    app.state::<Nudge>().note(covered(chapter, &parts));

    let last = parts.len().saturating_sub(1);
    let mut shown = None;
    for (i, part) in parts.into_iter().enumerate() {
        if STOP.load(Ordering::SeqCst) {
            return shown;
        }
        // The offer rides the last sentence rather than following it, so the
        // mark that ends the tour is still on screen while the next chapter is
        // being offered. Sent as its own step it cleared that mark, which is the
        // one thing the tour was building towards.
        let part = match (i == last, &next) {
            (true, Some(offer)) => {
                let said = format!("{} {offer}", part.say());
                part.saying(said)
            }
            _ => part,
        };
        app.emit("step", Some(&part)).ok();
        spoken(&cfg, part.say()).await;
        shown = Some(part);
    }
    shown
}

/// What this chapter covered, in one line, for the session's record.
fn covered(chapter: &str, parts: &[Step]) -> String {
    let names: Vec<&str> = parts
        .iter()
        .filter_map(|p| match p {
            Step::Point {
                control: Some(name),
                ..
            } => Some(name.as_str()),
            _ => None,
        })
        .collect();
    format!("Toured {chapter}, covering: {}. Already explained -- teach the next thing rather than this again, unless they ask for it again.", names.join(", "))
}

/// Speak a part and return when the voice has stopped.
///
/// With the voice off there is nothing to wait for, so fall back to a reading
/// pace -- otherwise the whole tour flashes past in half a second.
async fn spoken(cfg: &Config, line: &str) {
    speech::speak(cfg, line).await;
    if !speech::is_playing() {
        let read = line.chars().count() as u64 * 68;
        return tokio::time::sleep(Duration::from_millis(read)).await;
    }
    while speech::is_playing() && !STOP.load(Ordering::SeqCst) {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    // A beat between parts, so two sentences do not run together.
    tokio::time::sleep(Duration::from_millis(180)).await;
}

// ---------------------------------------------------------------------------
// The written-down tour, for looking at the format without a model call.

const BUILT_IN: &str = include_str!("../../tests/fixtures/tour.json");

#[derive(Deserialize)]
struct Script {
    /// The chapter, in a few words.
    say: String,
    parts: Vec<Part>,
    next: Option<String>,
}

#[derive(Deserialize)]
struct Part {
    /// What the thing is called. Rides the corner of a box, sits above a ring.
    name: Option<String>,
    say: String,
    /// An area, as `[y0, x0, y1, x1]` normalised to 0-1000. Drawn as an outline.
    region: Option<[f64; 4]>,
    /// One control, as `[y, x]`. Drawn as a ring -- which is what says "press
    /// this" where an outline round the same thing would read as a selection.
    point: Option<[f64; 2]>,
    /// Which ring: a second one for a double click, dashed and still for a hover.
    act: Option<String>,
}

pub fn demo(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let script: Script = match serde_json::from_str(&written()) {
            Ok(s) => s,
            Err(e) => {
                app.emit("error", format!("tour: {e}")).ok();
                return;
            }
        };
        // Normalised corners map onto the overlay window, not onto global points:
        // the window already covers the union of every display, and this is where
        // a model's step would have been converted on its way out of Rust.
        let (w, h) = {
            let screen = app.state::<Screen>();
            (screen.w, screen.h)
        };
        let parts = script.parts.iter().map(|p| step(p, w, h)).collect();
        play(&app, &script.say, parts, script.next).await;
    });
}

fn written() -> String {
    dirs::home_dir()
        .map(|d| d.join(".config/nudge/tour.json"))
        .and_then(|p| std::fs::read_to_string(p).ok())
        .unwrap_or_else(|| BUILT_IN.to_string())
}

fn step(p: &Part, w: f64, h: f64) -> Step {
    let at = |y: f64, x: f64| Point {
        x: x / 1000.0 * w,
        y: y / 1000.0 * h,
    };
    match (p.region, p.point) {
        (Some([y0, x0, y1, x1]), _) => Step::Point {
            at: at((y0 + y1) / 2.0, (x0 + x1) / 2.0),
            say: p.say.clone(),
            act: act_from(p.act.as_deref()),
            size: Some(((x1 - x0) / 1000.0 * w, (y1 - y0) / 1000.0 * h)),
            control: p.name.clone(),
        },
        (None, Some([y, x])) => Step::Point {
            at: at(y, x),
            say: p.say.clone(),
            act: act_from(p.act.as_deref()),
            size: None,
            control: p.name.clone(),
        },
        // A sentence with nothing to look at, which the overlay already knows how
        // to show -- and, more to the point, knows to clear the last mark for.
        (None, None) => Step::Reply { say: p.say.clone() },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn part(region: Option<[f64; 4]>, point: Option<[f64; 2]>) -> Part {
        Part {
            name: Some("Media Pool".into()),
            say: "here it is".into(),
            region,
            point,
            act: None,
        }
    }

    #[test]
    fn the_written_tour_parses_and_ends_with_something_to_do() {
        let script: Script = serde_json::from_str(BUILT_IN).expect("built-in tour");
        assert!(script.parts.len() > 2);
        assert!(script.parts.last().unwrap().point.is_some());
        // A chapter that offers nothing leads nowhere, and one tour is not a
        // course.
        assert!(script.next.is_some());
    }

    #[test]
    fn a_region_is_a_box_a_point_is_a_ring_and_neither_is_just_a_sentence() {
        match step(&part(Some([0.0, 0.0, 500.0, 500.0]), None), 1000.0, 1000.0) {
            Step::Point { at, size, .. } => {
                assert_eq!((at.x, at.y), (250.0, 250.0));
                assert_eq!(size, Some((500.0, 500.0)));
            }
            other => panic!("not a box: {other:?}"),
        }
        match step(&part(None, Some([400.0, 200.0])), 1000.0, 1000.0) {
            Step::Point { at, size: None, .. } => assert_eq!((at.x, at.y), (200.0, 400.0)),
            other => panic!("not a ring: {other:?}"),
        }
        assert!(matches!(
            step(&part(None, None), 1000.0, 1000.0),
            Step::Reply { .. }
        ));
    }

    #[test]
    fn the_record_names_what_was_shown_so_the_next_tour_moves_on() {
        let parts = vec![
            step(&part(Some([0.0, 0.0, 500.0, 500.0]), None), 100.0, 100.0),
            step(&part(None, None), 100.0, 100.0),
        ];
        let line = covered("the edit page", &parts);
        assert!(line.contains("the edit page") && line.contains("Media Pool"));
    }
}
