//! Adding a provider = one file here + one arm in `build()`. That is the whole
//! extension story; there is deliberately no registry, no plugin loader, no DSL.

mod anthropic;
mod gemini;
mod ollama;

use crate::core::capture::{Point, Shot};
use crate::config::Config;
use crate::error::{Error, Result};
use async_trait::async_trait;
use serde::Serialize;

/// What to do at the point.
///
/// Three, not a `double: bool`, because hovering is genuinely a third thing and
/// not a kind of click. Inside an open macOS menu, hovering is how a submenu is
/// revealed and a click can dismiss the whole menu -- so treating every target as
/// clickable does not merely fail, it undoes the previous step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Act {
    Click,
    DoubleClick,
    /// Rest the pointer here. Advances on dwell, never on a click.
    Hover,
}

/// One nudge.
///
/// An enum rather than `Option<Point>` because there are genuinely three answers,
/// and the two-state version had no room for the third: shown a screen with no
/// matching control, a model that may only point or finish will invent
/// coordinates. Giving "I cannot see it" a name is what makes admitting it
/// possible.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Step {
    /// Do something at this point next.
    Point { at: Point, say: String, act: Act },
    /// The goal is already achieved.
    Done { say: String },
    /// The control is not on this screen. Not an error -- often the right answer.
    Unsure { say: String },
    /// Open an application. Some goals ("open Blender") cannot be satisfied by
    /// pointing at anything, because the thing to point at does not exist yet.
    Launch { app: String, say: String },
    /// Open a web page. Most "open X" goals turn out to be this: the machine has
    /// no X app, and the browser was always the right answer.
    Open { url: String, say: String },
    /// Type into whatever is focused. `submit` presses Return afterwards, which
    /// is what an address bar or a search box almost always wants.
    Type { text: String, submit: bool, say: String },
    /// Just talking. Not every hotkey press is a task -- sometimes it is a
    /// question, a greeting, or someone bored at 2am.
    Reply { say: String },
}

impl Step {
    pub fn say(&self) -> &str {
        match self {
            Step::Point { say, .. }
            | Step::Done { say }
            | Step::Unsure { say }
            | Step::Launch { say, .. }
            | Step::Open { say, .. }
            | Step::Type { say, .. }
            | Step::Reply { say } => say,
        }
    }

    /// Rewrites the coordinate through `f`; other outcomes pass through untouched.
    pub fn map_point(self, f: impl FnOnce(Point) -> Point) -> Self {
        match self {
            Step::Point { at, say, act } => Step::Point { at: f(at), say, act },
            other => other,
        }
    }
}

/// Everything the model is told, besides the screenshot.
pub struct Ask<'a> {
    pub goal: &'a str,
    /// What we have already walked the user through, so the model advances
    /// instead of re-pointing at step one.
    pub done: &'a [String],
    /// The screen looks identical to before the last step. Either the click
    /// missed, or it landed on something that does nothing.
    pub stalled: bool,
}

#[async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &'static str;

    async fn next_step(&self, shot: &Shot, ask: &Ask<'_>) -> Result<Step>;
}

pub fn build(cfg: &Config) -> Result<Box<dyn Provider>> {
    Ok(match cfg.provider.as_str() {
        "ollama" => Box::new(ollama::Ollama::new(cfg)),
        "gemini" => Box::new(gemini::Gemini::new(cfg)?),
        "anthropic" => Box::new(anthropic::Anthropic::new(cfg)?),
        other => {
            return Err(Error::Config(format!(
                "unknown provider {other:?} -- expected ollama, gemini or anthropic"
            )))
        }
    })
}

/// Shared instruction. Kept in one place so a provider comparison measures the
/// *model*, not three people's prompt-writing.
pub(crate) fn prompt(ask: &Ask<'_>) -> String {
    let (goal, done) = (ask.goal, ask.done);
    let history = if done.is_empty() {
        "Nothing yet.".to_string()
    } else {
        done.iter()
            .enumerate()
            .map(|(i, s)| format!("{}. {s}", i + 1))
            .collect::<Vec<_>>()
            .join("\n")
    };
    // Saying it plainly beats letting the model assume its last instruction worked.
    // Without this it keeps pointing at the same control, because from the
    // screenshot alone nothing distinguishes "not done yet" from "did not work".
    let stalled = if ask.stalled {
        "\n\nThe screen has not changed since the last step. That instruction did not \
         work -- the control may have moved, been the wrong one, or need a double \
         click. Do not repeat it unchanged; find another way.\n"
    } else {
        ""
    };
    let apps = crate::core::launch::installed_apps().join(", ");
    format!(
        "You are Nudge: a small companion living on someone's screen, who can see \
         what they are looking at and point at things.\n\n\
         You are warm and quick-witted. A dry aside is welcome; a paragraph of one \
         is not. When you are giving an instruction, clarity wins over the joke \
         every time -- be funny around the edges, never in the middle of the step.\n\n\
         Not every request is a task. If they are chatting, greeting you, asking \
         about you, or asking something the screen cannot answer, just reply. Do \
         not invent a control to point at so you have something to do.\n\n\
         Their goal: {goal}\n\n\
         Steps already completed:\n{history}{stalled}\n\n\
         Point at the SINGLE next control they must click -- the control itself, \
         not the panel around it. Assume the screenshot is current. \
         Say what to do in one short sentence, and why in at most one more.\n\n\
         Only point at a control you can actually see in the screenshot. If the \
         screen does not contain it -- the menu is not open yet, or this is just a \
         desktop -- say that instead and mark it unsure. Guessing a location is \
         worse than admitting you cannot see it.\n\
         Say how to reach it: click, doubleClick, or hover.\n\
         - doubleClick to open a file, folder or application from Finder or the \
           desktop; a single click there only selects it.\n\
         - hover for anything inside an already-open menu -- a submenu opens on \
           hover, and clicking a menu can close it and undo the previous step. \
           Clicking is right only for the final item that performs the action.\n\
         If the goal needs an application that is not open yet, launch it by name \
         instead of pointing -- but only one from this machine's list below. When \
         what they want has no application here and lives on the web, open the URL \
         instead; a browser is the right answer far more often than an app is.\n\
         To put text somewhere -- a search box, an address bar, a filename -- click \
         the field first, then type on the next step. Set submit when Return should \
         follow, which an address bar or a search box almost always wants.\n\
         If the goal is already achieved, say so and mark it done.\n\
         If it was never a task at all, reply and leave it there.\n\n\
         Applications installed on this machine:\n{apps}"
    )
}

/// Models wrap JSON in prose and code fences no matter how firmly you ask.
pub(crate) fn first_json(text: &str) -> Option<serde_json::Value> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    serde_json::from_str(text.get(start..=end)?).ok()
}

/// Unknown or missing means click: it is what most targets want, and a wrong
/// click is recoverable where a wrong hover just stalls.
pub(crate) fn act_from(raw: Option<&str>) -> Act {
    match raw.unwrap_or("") {
        "doubleClick" | "double_click" | "double" => Act::DoubleClick,
        "hover" | "mouse_move" | "move" => Act::Hover,
        _ => Act::Click,
    }
}

pub(crate) fn no_point(provider: &'static str, detail: impl Into<String>) -> Error {
    Error::NoPoint { provider, detail: detail.into() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digs_json_out_of_fences_and_chatter() {
        let v = first_json("Sure!\n```json\n{\"x\": 12, \"y\": 7}\n```\nHope that helps")
            .expect("should find the object");
        assert_eq!(v["x"], 12);
        assert_eq!(v["y"], 7);
        assert!(first_json("no object here").is_none());
    }

    fn ask<'a>(goal: &'a str, done: &'a [String], stalled: bool) -> Ask<'a> {
        Ask { goal, done, stalled }
    }

    #[test]
    fn prompt_numbers_completed_steps() {
        let p = prompt(&ask("unwrap UVs", &["Opened the UV editor".into()], false));
        assert!(p.contains("1. Opened the UV editor"));
        assert!(prompt(&ask("x", &[], false)).contains("Nothing yet."));
    }

    #[test]
    fn prompt_names_the_apps_that_actually_exist() {
        // Guessing at an app that is not installed is the failure this prevents.
        let p = prompt(&ask("open something", &[], false));
        assert!(p.contains("Applications installed on this machine:"));
        assert!(p.contains("Safari"), "a Mac always has Safari; got a short list?");
    }

    #[test]
    fn a_stall_is_stated_only_when_it_happened() {
        assert!(prompt(&ask("x", &[], true)).contains("has not changed"));
        assert!(!prompt(&ask("x", &[], false)).contains("has not changed"));
    }
}
