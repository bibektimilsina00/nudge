//! The accuracy ceiling, and the reference the free providers get measured against.
//! No official Rust SDK exists, so this is raw HTTP -- which is why "support any
//! model" stayed three request shapes instead of three SDK abstractions.
use super::no_point;
use super::prompt;
use super::Act;
use super::Ask;
use super::Provider;
use super::Step;
use crate::config::Config;
use crate::core::screen::capture::{Point, Shot};
use crate::error::{Error, Result};
use async_trait::async_trait;
use serde_json::json;

pub struct Anthropic {
    model: String,
    key: String,
    http: reqwest::Client,
}

impl Anthropic {
    pub fn new(cfg: &Config) -> Result<Self> {
        let key = cfg.key("ANTHROPIC_API_KEY").ok_or_else(|| {
            Error::Config("set ANTHROPIC_API_KEY or api_key in config.toml".into())
        })?;
        Ok(Self {
            model: cfg.model.clone().unwrap_or_else(|| "claude-opus-5".into()),
            key,
            http: reqwest::Client::new(),
        })
    }
}

#[async_trait]
impl Provider for Anthropic {
    fn name(&self) -> &'static str {
        "anthropic"
    }

    async fn next_step(&self, shot: &Shot, ask: &Ask<'_>) -> Result<Step> {
        let (w, h) = shot.sent;
        // The computer-use tool is *trained* for coordinate grounding -- markedly
        // better than asking any model to describe a box in JSON. Its coordinates
        // come back 1:1 with the image we sent, provided the long edge is under
        // 2576px; past that the server downscales and every point silently drifts.
        // config.max_edge is what keeps us under that line.
        let body = json!({
            "model": self.model,
            "max_tokens": 1024,
            "tools": [
                {
                    "type": "computer_20251124",
                    "name": "computer",
                    "display_width_px": w,
                    "display_height_px": h,
                },
                // A declared tool rather than a phrase to pattern-match: the
                // computer tool can only click, so "open Blender" has no way to
                // come back as anything but prose otherwise.
                {
                    "name": "run_agent",
                    "description": "Carry out a whole task unattended, step by step, \
                                    rather than pointing at one control.",
                    "input_schema": {
                        "type": "object",
                        "properties": {
                            "title": {"type": "string", "description": "Short name for the task."},
                        },
                        "required": ["title"],
                    },
                },
                {
                    "name": "ask_user",
                    "description": "Ask the user something you cannot see or decide. \
                                    Only when genuinely blocked.",
                    "input_schema": {
                        "type": "object",
                        "properties": {
                            "question": {"type": "string"},
                        },
                        "required": ["question"],
                    },
                },
                {
                    "name": "open_url",
                    "description": "Open a web page in the browser. Prefer this when \
                                    the machine has no application for what was asked.",
                    "input_schema": {
                        "type": "object",
                        "properties": {
                            "url": {"type": "string", "description": "An http(s) URL."},
                        },
                        "required": ["url"],
                    },
                },
                {
                    "name": "launch_app",
                    "description": "Open an application that is not running yet.",
                    "input_schema": {
                        "type": "object",
                        "properties": {
                            "app": {"type": "string", "description": "Application name."},
                        },
                        "required": ["app"],
                    },
                },
            ],
            "messages": [{"role": "user", "content": [
                {"type": "image", "source": {
                    "type": "base64", "media_type": crate::core::screen::capture::MIME, "data": shot.b64(),
                }},
                {"type": "text", "text": format!(
                    "{}\n\nUse the computer tool's left_click action to show exactly where, \
                     double_click when a single click would only select, or \
                     mouse_move for anything inside an open menu -- submenus open \
                     on hover and a click can close the menu. Use type to enter \
                     text, after clicking the field on an earlier step. \
                     If the goal is already achieved, or the control is not on this \
                     screen, reply in words and do not click.",
                    prompt(ask)
                )},
            ]}],
        });

        let resp: serde_json::Value = self
            .http
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.key)
            .header("anthropic-version", "2023-06-01")
            .header("anthropic-beta", "computer-use-2025-11-24")
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(read_step(&resp).ok_or_else(|| no_point("anthropic", resp.to_string()))?)
    }
}

/// A click means "go here"; no click means the model declined to point. It cannot
/// tell us *why* it declined through the tool interface, so the wording decides:
/// this is the one provider where done-vs-unsure is inferred rather than stated.
fn read_step(resp: &serde_json::Value) -> Option<Step> {
    let blocks = resp["content"].as_array()?;
    let say = blocks
        .iter()
        .filter_map(|b| (b["type"] == "text").then(|| b["text"].as_str()).flatten())
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();

    let tool = |name: &str| {
        blocks
            .iter()
            .find(|b| b["type"] == "tool_use" && b["name"] == name)
    };

    if let Some(q) = tool("ask_user").and_then(|b| b["input"]["question"].as_str()) {
        return Some(Step::Question {
            question: q.to_string(),
        });
    }

    if let Some(title) = tool("run_agent").and_then(|b| b["input"]["title"].as_str()) {
        return Some(Step::Agent {
            title: title.to_string(),
            say: if say.is_empty() {
                format!("Starting: {title}")
            } else {
                say
            },
            background: tool("run_agent")
                .and_then(|b| b["input"]["background"].as_bool())
                .unwrap_or(false),
        });
    }

    if let Some(url) = tool("open_url").and_then(|b| b["input"]["url"].as_str()) {
        return Some(Step::Open {
            url: url.to_string(),
            say: if say.is_empty() {
                "Opening that page.".into()
            } else {
                say
            },
        });
    }

    if let Some(app) = tool("launch_app").and_then(|b| b["input"]["app"].as_str()) {
        return Some(Step::Launch {
            app: app.to_string(),
            say: if say.is_empty() {
                format!("Opening {app}.")
            } else {
                say
            },
        });
    }

    let computer = tool("computer");

    // The computer tool carries typing as an action with its own payload, so it is
    // read from there rather than asked for a second way.
    if let Some(text) = computer
        .filter(|b| b["input"]["action"] == "type")
        .and_then(|b| b["input"]["text"].as_str())
    {
        return Some(Step::Type {
            text: text.to_string(),
            // The tool has no "submit" of its own; a separate key press would be
            // the model's next step.
            submit: false,
            say: if say.is_empty() {
                format!("Type “{text}”.")
            } else {
                say
            },
        });
    }

    let point = computer
        .and_then(|b| b["input"]["coordinate"].as_array())
        .and_then(|c| {
            Some(Point {
                x: c.first()?.as_f64()?,
                y: c.get(1)?.as_f64()?,
            })
        });
    // The computer tool names the action itself, so it is read from there rather
    // than asked for twice.
    let act = match computer.map(|b| &b["input"]["action"]) {
        Some(a) if a == "double_click" => Act::DoubleClick,
        Some(a) if a == "mouse_move" => Act::Hover,
        _ => Act::Click,
    };

    let say = if say.is_empty() { "Here.".into() } else { say };
    Some(match point {
        Some(at) => Step::Point {
            at,
            say,
            act,
            control: None,
        },
        None if reads_as_unsure(&say) => Step::Unsure { say, needed: None },
        // No `recalled` here, unlike the JSON providers: this reads a
        // computer-use tool response, which has no room for a key we invented.
        // A recalled fact from this provider is marked only when it comes back
        // through a subagent, where it is worked out from what was consulted
        // rather than asked for.
        None => Step::Done { say, next: None },
    })
}

/// Crude on purpose: a second model call to classify one sentence would cost more
/// than the mistake does. Misfiling "done" as "unsure" shows a slightly wrong
/// message and nothing else breaks.
fn reads_as_unsure(say: &str) -> bool {
    let s = say.to_lowercase();
    [
        "can't",
        "cannot",
        "not visible",
        "no ",
        "don't see",
        "unable",
        "not on",
    ]
    .iter()
    .any(|n| s.contains(n))
}

#[cfg(test)]
mod tests {
    use super::read_step;
    use serde_json::json;

    #[test]
    fn pulls_coordinate_out_of_the_tool_call() {
        let r = json!({"content": [
            {"type": "text", "text": "Open the UV menu."},
            {"type": "tool_use", "name": "computer",
             "input": {"action": "left_click", "coordinate": [412, 96]}},
        ]});
        match read_step(&r).unwrap() {
            super::Step::Point { at, say, .. } => {
                assert_eq!((at.x, at.y), (412.0, 96.0));
                assert_eq!(say, "Open the UV menu.");
            }
            other => panic!("expected a point, got {other:?}"),
        }
    }

    #[test]
    fn no_click_means_finished() {
        let r = json!({"content": [{"type": "text", "text": "Already unwrapped."}]});
        assert!(matches!(read_step(&r).unwrap(), super::Step::Done { .. }));
    }

    #[test]
    fn a_double_click_action_is_carried_through() {
        // Finder selects on a single click and opens on a double; losing this makes
        // "open the file" look like it silently did nothing.
        let r = json!({"content": [
            {"type": "tool_use", "name": "computer",
             "input": {"action": "double_click", "coordinate": [10, 20]}},
        ]});
        assert!(matches!(
            read_step(&r).unwrap(),
            super::Step::Point {
                act: super::Act::DoubleClick,
                ..
            }
        ));
    }

    #[test]
    fn a_menu_move_is_a_hover_not_a_click() {
        // Clicking a submenu parent closes the menu; this is the difference
        // between advancing and undoing the previous step.
        let r = json!({"content": [
            {"type": "tool_use", "name": "computer",
             "input": {"action": "mouse_move", "coordinate": [5, 5]}},
        ]});
        assert!(matches!(
            read_step(&r).unwrap(),
            super::Step::Point {
                act: super::Act::Hover,
                ..
            }
        ));
    }

    #[test]
    fn reads_a_launch_tool_call() {
        let r = json!({"content": [
            {"type": "tool_use", "name": "launch_app", "input": {"app": "Blender"}},
        ]});
        match read_step(&r).unwrap() {
            super::Step::Launch { app, .. } => assert_eq!(app, "Blender"),
            other => panic!("expected a launch, got {other:?}"),
        }
    }

    #[test]
    fn declining_because_it_cannot_see_is_not_success() {
        // The bug this whole enum exists for: shown a bare desktop, the two-state
        // version reported "goal achieved" and cleared the overlay.
        let r = json!({"content": [
            {"type": "text", "text": "I can't see a UV editor on this screen."},
        ]});
        assert!(matches!(read_step(&r).unwrap(), super::Step::Unsure { .. }));
    }
}
