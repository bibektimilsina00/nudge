//! Free tier, no card. Note the coordinate format below -- it is not pixels.
use super::first_json;
use super::no_point;
use super::prompt;
use super::Ask;
use super::Provider;
use super::Step;
use crate::config::Config;
use crate::core::screen::capture::{Point, Shot};
use crate::error::{Error, Result};
use async_trait::async_trait;
use serde_json::json;

pub struct Gemini {
    model: String,
    /// Kept whole rather than reduced to a key at startup: where a call goes is
    /// decided per call, because signing in, signing out and pasting a key into
    /// Settings all change the answer and none of them should need a restart.
    cfg: Config,
    think: Option<String>,
    http: reqwest::Client,
}

impl Gemini {
    /// What goes in `generationConfig`, with the thinking budget folded in when
    /// one is configured.
    ///
    /// Left out entirely when unset, rather than sent as a default we invented:
    /// a budget we guess at is a change to the model's behaviour that nothing
    /// measured.
    fn generation(&self) -> serde_json::Value {
        let mut cfg = json!({"responseMimeType": "application/json"});
        if let Some(level) = &self.think {
            cfg["thinkingConfig"] = json!({"thinkingLevel": level});
        }
        cfg
    }
}

impl Gemini {
    /// One request, wherever this copy is entitled to send it.
    ///
    /// Not `Result` at the call sites' expense: every one of them is already in
    /// a function that returns `Result`, and a relay that cannot be chosen is
    /// the same failure as a missing key always was.
    fn send(&self, model: &str, body: &serde_json::Value) -> Result<reqwest::RequestBuilder> {
        let relay = crate::core::relay::Relay::choose(&self.cfg)
            .ok_or_else(|| Error::Config(crate::core::relay::Relay::missing()))?;
        Ok(relay.authorise(self.http.post(relay.url(model))).json(body))
    }

    pub fn new(cfg: &Config) -> Result<Self> {
        // No key needed to build one any more. A signed-in copy borrows the
        // server's, and whether this copy has either is a question for the
        // moment it actually sends something.
        Ok(Self {
            // Measured against the same screenshot and goal: 3.8-flash 9.6s,
            // 3.5-flash 5.1s, 3.5-flash-lite 3.3s -- and the lite model still put
            // the ring dead centre on a 20px icon. Speed is the whole experience
            // here; a nudge that arrives after you have gone hunting is worthless.
            model: cfg
                .model
                .clone()
                .unwrap_or_else(|| "gemini-3.5-flash-lite".into()),
            cfg: cfg.clone(),
            think: cfg.think.clone(),
            http: reqwest::Client::new(),
        })
    }
}

/// Gemini points are `[y, x]` on a 0-1000 grid -- normalised, and y first. Both
/// halves catch people out, so the conversion lives alone and is tested.
fn denorm(point: &[f64], w: u32, h: u32) -> Point {
    Point {
        x: point[1] / 1000.0 * w as f64,
        y: point[0] / 1000.0 * h as f64,
    }
}

/// The mark a sentence carries, when it carries one.
///
/// An area becomes an outline, a pixel becomes a ring, and anything else is
/// `None` -- a sentence about nothing you can point at, which is a third of any
/// real tour. Shared by a plain step and by every part of a tour, so there is
/// one place where normalised corners become a place on the screen.
fn mark(v: &serde_json::Value, shot: &Shot, say: String) -> Option<Step> {
    let nums = |key: &str| -> Vec<f64> {
        v[key]
            .as_array()
            .map(|a| a.iter().filter_map(|n| n.as_f64()).collect())
            .unwrap_or_default()
    };
    // Named by the model during a tour; absent on an ordinary step, where the
    // name comes off the control list instead.
    let control = v["name"].as_str().map(str::to_string);
    let act = super::act_from(v["act"].as_str());

    let r = nums("region");
    if r.len() == 4 {
        let a = denorm(&r[..2], shot.sent.0, shot.sent.1);
        let b = denorm(&r[2..], shot.sent.0, shot.sent.1);
        let (w, h) = ((b.x - a.x).abs(), (b.y - a.y).abs());
        // A region has to be a part of the screen to mean anything.
        //
        // Asked to explain DaVinci Resolve it came back with the whole display,
        // 1512x982, which outlines everything and therefore points at nothing.
        // Past two thirds of the screen this is not somewhere to look, so the
        // box is dropped and the sentence stands on its own.
        let whole = shot.sent.0 as f64 * shot.sent.1 as f64;
        let size = (w * h <= whole * 0.66).then(|| shot.to_global_size(w, h));
        return Some(Step::Point {
            at: Point {
                x: (a.x + b.x) / 2.0,
                y: (a.y + b.y) / 2.0,
            },
            size,
            control,
            say,
            act,
        });
    }

    let p = nums("point");
    (p.len() == 2).then(|| Step::Point {
        at: denorm(&p, shot.sent.0, shot.sent.1),
        // A guessed pixel has no bounds to draw.
        size: None,
        control,
        say,
        act,
    })
}

#[async_trait]
impl Provider for Gemini {
    fn name(&self) -> &'static str {
        "gemini"
    }

    async fn next_step(&self, shot: &Shot, ask: &Ask<'_>) -> Result<Step> {
        // Asking in Gemini's own native point format, rather than forcing pixels,
        // keeps it on the output shape it was actually trained to ground in.
        let instruction = format!(
            "{}\n\nEvery reply is an object with \"screen\" plus one of these \
             shapes:\n\
             {{\"kind\":\"point\",\"control\":7,\"act\":\"click|doubleClick|hover\",\"say\":\"...\"}}\n\
             {{\"kind\":\"point\",\"point\":[y,x],\"act\":\"click|doubleClick|hover\",\"say\":\"...\"}}\n\
             {{\"kind\":\"point\",\"region\":[y0,x0,y1,x1],\"act\":\"hover\",\"say\":\"...\"}}\n\
             {{\"kind\":\"tour\",\"say\":\"the chapter, in a few words\",\"parts\":[{{\"say\":\"...\",\"region\":[y0,x0,y1,x1],\"name\":\"...\"}},{{\"say\":\"...\",\"point\":[y,x],\"name\":\"...\",\"act\":\"click\"}},{{\"say\":\"...\"}}],\"next\":\"...\"}}\n\
             {{\"kind\":\"done\",\"say\":\"...\"}}\n\
             {{\"kind\":\"unsure\",\"say\":\"...\"}}\n\
             {{\"kind\":\"launch\",\"app\":\"...\",\"say\":\"...\"}}\n\
             {{\"kind\":\"open\",\"url\":\"https://...\",\"say\":\"...\"}}\n\
             {{\"kind\":\"reply\",\"say\":\"...\"}}\n\
             {{\"kind\":\"agent\",\"title\":\"Playing the song\",\"say\":\"...\"}}\n\
             {{\"kind\":\"ask\",\"question\":\"...\"}}\n\
             {{\"kind\":\"type\",\"text\":\"...\",\"submit\":true,\"say\":\"...\"}}\n\
             {{\"kind\":\"press\",\"keys\":\"cmd+shift+n\",\"say\":\"...\"}}\n\
             {{\"kind\":\"run\",\"command\":\"find . -name '*.ts' | wc -l\",\"say\":\"...\"}}\n\
             {{\"kind\":\"start\",\"command\":\"npm run dev\",\"say\":\"...\"}}\n\
             {{\"kind\":\"output\",\"id\":1,\"say\":\"...\"}}\n\
             {{\"kind\":\"await\",\"id\":1,\"say\":\"...\"}}\n\
             {{\"kind\":\"kill\",\"id\":1,\"say\":\"...\"}}\n\
             {{\"kind\":\"fetch\",\"url\":\"https://...\",\"say\":\"...\"}}\n\
             {{\"kind\":\"search\",\"query\":\"...\",\"say\":\"...\"}}\n\
             {{\"kind\":\"mcp\",\"tool\":\"server/name\",\"args\":{{}},\"say\":\"...\"}}\n\
             {{\"kind\":\"tools\",\"server\":\"gmail\",\"say\":\"...\"}}\n\
             {{\"kind\":\"request\",\"method\":\"POST\",\"url\":\"https://...\",\"headers\":{{}},\"body\":{{}},\"say\":\"...\"}}\n\
             {{\"kind\":\"delegate\",\"task\":\"the whole job, written out\",\"say\":\"...\"}}\n\
             {{\"kind\":\"remember\",\"about\":\"AppName\",\"note\":\"what would have saved you\",\"say\":\"...\"}}\n\
             {{\"kind\":\"skill\",\"name\":\"exact skill name\",\"say\":\"...\"}}\n\
             {{\"kind\":\"task\",\"task\":\"...\",\"say\":\"...\"}}\n\
             {{\"kind\":\"show\",\"path\":\"index.html\",\"say\":\"...\"}}\n\
             {{\"kind\":\"workspace\",\"path\":\"~/projects/thing\",\"say\":\"...\"}}\n\
             {{\"kind\":\"write\",\"path\":\"index.html\",\"content\":\"...\",\"say\":\"...\"}}\n\
             {{\"kind\":\"read\",\"path\":\"...\",\"from\":1,\"lines\":200,\"say\":\"...\"}}\n\
             {{\"kind\":\"plan\",\"todos\":[{{\"text\":\"...\",\"status\":\"active\"}}],\"say\":\"...\"}}\n\
             {{\"kind\":\"edit\",\"path\":\"...\",\"old\":\"...\",\"new\":\"...\",\"say\":\"...\"}}\n\
             e.g. {{\"screen\":\"a list of search results; the goal is not met \
             yet\",\"kind\":\"point\",...}}\n\
             where y and x are normalised to 0-1000. Use \"control\" with a \
             number from the list above whenever the thing you want is on it -- \
             a number is exact and a guess at a pixel is not. \"point\" is for \
             everything the list does not contain.\n\
             \"region\" is a *part* of a window -- a sidebar, a toolbar, a \
             panel, a row of tabs. Never the whole screen and never the whole \
             window: an outline round everything points at nothing. \
             It is for showing somebody an area rather than one \
             control: the media pool, a sidebar, a toolbar. Give the corners as \
             [top, left, bottom, right], normalised to 0-1000 like \"point\". \
             Use it while explaining what part of a window is for, where a ring \
             on a single pixel would point at nothing in particular. Many \
             applications publish no control list at all, and this is how you \
             show somebody around one of those.",
            prompt(ask)
        );
        let body = json!({
            "contents": [{"parts": [
                {"inline_data": {"mime_type": crate::core::screen::capture::MIME, "data": shot.b64()}},
                {"text": instruction},
            ]}],
            "generationConfig": self.generation(),
        });

        let resp: serde_json::Value = self
            .send(&self.model, &body)?
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let text = resp["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let v = first_json(&text).ok_or_else(|| no_point("gemini", text.clone()))?;
        let say = v["say"].as_str().unwrap_or("Here.").to_string();
        // What the model says it sees, next to what it decided to do. The single
        // most useful line in the log: a wrong action with an accurate `screen`
        // is a reasoning problem, and the same action with a `screen` describing
        // the previous step's expected result is a grounding problem.
        if let Some(seen) = v["screen"].as_str() {
            eprintln!("  saw: {seen}");
        }

        // A tour: one narration cut into parts, each with its own mark or none.
        // Parsed here rather than in `simple_step` because a part carries
        // coordinates, and coordinates are the one thing the providers do not
        // share.
        if v["kind"] == "tour" {
            let parts = v["parts"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .map(|p| {
                            let said = p["say"].as_str().unwrap_or_default().to_string();
                            // No mark is a real answer, not a failure to give
                            // one: the opening line, and the sentence that joins
                            // two panels, are about nothing you can point at.
                            mark(p, shot, said.clone()).unwrap_or(Step::Reply { say: said })
                        })
                        .collect()
                })
                .unwrap_or_default();
            return Ok(Step::Tour {
                say,
                parts,
                next: v["next"]
                    .as_str()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string),
            });
        }

        if let Some(step) = super::simple_step(v["kind"].as_str().unwrap_or(""), &v, say.clone()) {
            return Ok(step);
        }

        // A number from the list beats a guess at a pixel, so it is tried first.
        // Resolved through the same function that numbered the list, and
        // converted into the picture's coordinates -- everything downstream maps
        // back out again, and a control that skipped that would be off by the
        // display's origin on any screen but the first.
        if let Some(n) = v["control"].as_u64() {
            if let Some(c) = super::control_at(ask.controls, n) {
                eprintln!("  control {n}: {} {:?}", c.role, c.label);
                return Ok(Step::Point {
                    // Named, so this can be pressed rather than clicked at.
                    control: Some(c.label.clone()),
                    size: Some(c.size),
                    at: shot.to_image(Point {
                        x: c.at.0,
                        y: c.at.1,
                    }),
                    say,
                    act: super::act_from(v["act"].as_str()),
                });
            }
            // Out of range. Falling through to `point` is right when the model
            // gave both, and the error below is right when it did not -- either
            // way, better than clicking a control we cannot identify.
            eprintln!("  control {n} is not on the list of {}", ask.controls.len());
        }

        // An area or a pixel, whichever it gave. Shared with a tour's parts, so
        // a box drawn during a tour lands in the same place as one drawn on its
        // own.
        if let Some(step) = mark(&v, shot, say.clone()) {
            return Ok(step);
        }

        Err(no_point("gemini", text))
    }

    /// The same loop without a picture.
    ///
    /// Sends the shapes a subagent may use and no others, so the model is not
    /// choosing between tools it cannot reach. Telling it afterwards that
    /// pointing is unavailable would be a refusal it could have been spared.
    async fn next_step_blind(&self, ask: &Ask<'_>) -> Result<Step> {
        let instruction = format!(
            "{}\n\nEvery reply is an object with \"screen\" -- one line on what you \
             know so far -- plus one of these shapes:\n\
             {{\"kind\":\"run\",\"command\":\"...\",\"say\":\"...\"}}\n\
             {{\"kind\":\"read\",\"path\":\"...\",\"from\":1,\"lines\":200,\"say\":\"...\"}}\n\
             {{\"kind\":\"write\",\"path\":\"...\",\"content\":\"...\",\"say\":\"...\"}}\n\
             {{\"kind\":\"edit\",\"path\":\"...\",\"old\":\"...\",\"new\":\"...\",\"say\":\"...\"}}\n\
             {{\"kind\":\"fetch\",\"url\":\"https://...\",\"say\":\"...\"}}\n\
             {{\"kind\":\"search\",\"query\":\"...\",\"say\":\"...\"}}\n\
             {{\"kind\":\"mcp\",\"tool\":\"server/name\",\"args\":{{}},\"say\":\"...\"}}\n\
             {{\"kind\":\"request\",\"method\":\"POST\",\"url\":\"https://...\",\"headers\":{{}},\"body\":{{}},\"say\":\"...\"}}\n\
             {{\"kind\":\"delegate\",\"task\":\"the whole job, written out\",\"say\":\"...\"}}\n\
             {{\"kind\":\"remember\",\"about\":\"AppName\",\"note\":\"what would have saved you\",\"say\":\"...\"}}\n\
             {{\"kind\":\"skill\",\"name\":\"exact skill name\",\"say\":\"...\"}}\n\
             {{\"kind\":\"done\",\"say\":\"what you found, in full\"}}\n\
             {{\"kind\":\"unsure\",\"say\":\"why you could not\",\"needed\":\"Calendar (only if a service would have done it)\"}}",
            prompt(ask)
        );
        let body = json!({
            "contents": [{"parts": [{"text": instruction}]}],
            "generationConfig": self.generation(),
        });
        let resp: serde_json::Value = self
            .send(&self.model, &body)?
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let text = resp["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let v = super::first_json(&text).ok_or_else(|| no_point("gemini", text.clone()))?;
        let say = v["say"].as_str().unwrap_or_default().to_string();
        super::simple_step(v["kind"].as_str().unwrap_or(""), &v, say)
            .ok_or_else(|| no_point("gemini", text))
    }

    fn aside(&self) -> bool {
        true
    }

    /// One action, judged by the same model that drives the agent.
    ///
    /// The same model on purpose: if it is trusted to decide what Nudge does, it
    /// is strong enough to say whether one action follows from what was asked.
    /// A second key and a second model to configure would be two more things to
    /// get wrong for a defence people would then switch off.
    ///
    /// What makes this safe is not the model, it is what it is shown -- see
    /// [`crate::core::judge`]. This call carries no screenshot and no history:
    /// one prompt, built by `judge::prompt`, and nothing this method can add to.
    async fn ask_aside(&self, prompt: &str) -> Result<String> {
        let body = json!({
            "contents": [{"parts": [{"text": prompt}]}],
            // Not `self.generation()`: that asks for JSON *and* carries the
            // thinking level configured for driving the agent. A verdict is a
            // small judgement and the reply shape is spelled out in the
            // instructions, so this stays plain and cheap.
            "generationConfig": {"responseMimeType": "application/json"},
        });
        let resp: serde_json::Value = self
            .send(&self.model, &body)?
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(resp["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .unwrap_or_default()
            .to_string())
    }

    /// Google Search, through the key that is already configured.
    ///
    /// A separate call rather than grounding the main loop: the step request
    /// carries a screenshot and wants one small JSON object back, and asking it
    /// to also search would slow every screen action for the rare turn that
    /// needed a fact. This costs one request, and only when something is
    /// actually being looked up.
    ///
    /// Comes back as an answer with its sources rather than a list of links --
    /// which is what was wanted anyway, and the sources are there to fetch when
    /// the answer is not enough.
    async fn search(&self, query: &str) -> Result<String> {
        let body = json!({
            "contents": [{"parts": [{"text": query}]}],
            "tools": [{"google_search": {}}],
        });
        let resp: serde_json::Value = self
            .send(&self.model, &body)?
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let c = &resp["candidates"][0];
        let answer: String = c["content"]["parts"]
            .as_array()
            .map(|parts| {
                parts
                    .iter()
                    .filter_map(|p| p["text"].as_str())
                    .collect::<Vec<_>>()
                    .join("")
            })
            .unwrap_or_default();
        if answer.trim().is_empty() {
            return Err(Error::Config(format!("gemini found nothing for {query:?}")));
        }

        // Named so the model can fetch one for detail, and so a spoken answer
        // can say where it came from.
        let sources: Vec<String> = c["groundingMetadata"]["groundingChunks"]
            .as_array()
            .map(|chunks| {
                chunks
                    .iter()
                    .filter_map(|ch| ch["web"]["title"].as_str())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        Ok(match sources.is_empty() {
            true => answer,
            false => format!("{answer}\n\nSources: {}", sources.join(", ")),
        })
    }
}

#[cfg(test)]
mod live_tests {
    use super::*;
    use crate::config::Config;

    /// A real search, against the real key.
    ///
    /// Grounding is a provider feature, so the only thing that proves it works
    /// is asking it something no model could know from training -- a date after
    /// its cutoff.
    ///
    ///     cargo test searches_the_real_web -- --ignored --nocapture
    #[tokio::test]
    #[ignore]
    async fn searches_the_real_web() {
        let cfg = Config::load().unwrap_or_default();
        if cfg.provider != "gemini" {
            eprintln!("skipped: provider is {}", cfg.provider);
            return;
        }
        let g = Gemini::new(&cfg).expect("gemini");
        let out = g
            .search("what is the current version of the Rust compiler")
            .await
            .expect("search");
        eprintln!("{out}");
        assert!(out.len() > 20, "suspiciously short: {out}");
        assert!(out.contains("Sources:"), "no sources came back: {out}");
    }
}

#[cfg(test)]
mod shape_tests {
    /// Every outcome the parser accepts must appear in the shapes the model is
    /// shown. `fetch` was added to the enum, the parser, the prompt prose and
    /// the executor -- and not to this list, so the model never knew it existed
    /// and kept opening a browser to read a page it could have fetched.
    ///
    /// Reads the source rather than calling anything: the shapes are a string
    /// literal, and a string literal is what has to be checked.
    #[test]
    fn every_outcome_appears_in_the_shapes_shown_to_the_model() {
        let src = include_str!("gemini.rs");
        let shapes = src.split("shapes:").nth(1).expect("the shape list moved");
        for kind in [
            "point",
            "done",
            "unsure",
            "launch",
            "open",
            "reply",
            "agent",
            "ask",
            "type",
            "press",
            "run",
            "write",
            "fetch",
            "read",
            "edit",
            "plan",
            "search",
            "task",
            "show",
            "workspace",
            "start",
            "output",
            "kill",
        ] {
            assert!(
                shapes.contains(&format!("kind\\\":\\\"{kind}")),
                "{kind} is missing from the shapes the model is shown"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{denorm, mark};
    use crate::core::provider::Step;
    use crate::core::screen::capture::Shot;
    use serde_json::json;

    fn shot() -> Shot {
        Shot {
            bytes: Vec::new(),
            sent: (1000, 1000),
            logical: (1000.0, 1000.0),
            origin: (0.0, 0.0),
        }
    }

    /// The three shapes a tour's parts come in, which is the whole format.
    #[test]
    fn a_region_is_a_box_a_point_is_a_ring_and_neither_is_a_sentence_on_its_own() {
        let box_ = json!({"region": [0, 0, 400, 400], "name": "Media Pool"});
        match mark(&box_, &shot(), "an area".into()) {
            Some(Step::Point {
                at, size, control, ..
            }) => {
                assert_eq!((at.x, at.y), (200.0, 200.0));
                assert_eq!(size, Some((400.0, 400.0)));
                assert_eq!(control.as_deref(), Some("Media Pool"));
            }
            other => panic!("not a box: {other:?}"),
        }
        let ring = json!({"point": [600, 300], "name": "Play"});
        match mark(&ring, &shot(), "a control".into()) {
            Some(Step::Point { at, size: None, .. }) => assert_eq!((at.x, at.y), (300.0, 600.0)),
            other => panic!("not a ring: {other:?}"),
        }
        assert!(mark(&json!({}), &shot(), "just talking".into()).is_none());
    }

    /// An outline round everything points at nothing -- and this is exactly what
    /// came back the first time DaVinci Resolve was explained.
    #[test]
    fn a_region_covering_the_screen_loses_its_box() {
        let whole = json!({"region": [0, 0, 1000, 1000]});
        assert!(matches!(
            mark(&whole, &shot(), "everything".into()),
            Some(Step::Point { size: None, .. })
        ));
    }

    #[test]
    fn y_comes_first_and_the_grid_is_1000_not_pixels() {
        let p = denorm(&[250.0, 500.0], 800, 600);
        assert_eq!((p.x, p.y), (400.0, 150.0), "y/x order is swapped");
        let p = denorm(&[1000.0, 1000.0], 800, 600);
        assert_eq!((p.x, p.y), (800.0, 600.0));
    }
}
