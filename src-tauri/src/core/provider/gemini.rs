//! Free tier, no card. Note the coordinate format below -- it is not pixels.
use super::{first_json, no_point, prompt, Ask, Provider, Step};
use crate::core::capture::{Point, Shot};
use crate::config::Config;
use crate::error::{Error, Result};
use async_trait::async_trait;
use serde_json::json;

pub struct Gemini {
    model: String,
    key: String,
    http: reqwest::Client,
}

impl Gemini {
    pub fn new(cfg: &Config) -> Result<Self> {
        let key = cfg
            .key("GEMINI_API_KEY")
            .ok_or_else(|| Error::Config("set GEMINI_API_KEY or api_key in config.toml".into()))?;
        Ok(Self {
            // Measured against the same screenshot and goal: 3.8-flash 9.6s,
            // 3.5-flash 5.1s, 3.5-flash-lite 3.3s -- and the lite model still put
            // the ring dead centre on a 20px icon. Speed is the whole experience
            // here; a nudge that arrives after you have gone hunting is worthless.
            model: cfg.model.clone().unwrap_or_else(|| "gemini-3.5-flash-lite".into()),
            key,
            http: reqwest::Client::new(),
        })
    }
}

/// Gemini points are `[y, x]` on a 0-1000 grid -- normalised, and y first. Both
/// halves catch people out, so the conversion lives alone and is tested.
fn denorm(point: &[f64], w: u32, h: u32) -> Point {
    Point { x: point[1] / 1000.0 * w as f64, y: point[0] / 1000.0 * h as f64 }
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
            "{}\n\nReply with only one of:\n\
             {{\"kind\":\"point\",\"point\":[y,x],\"act\":\"click|doubleClick|hover\",\"say\":\"...\"}}\n\
             {{\"kind\":\"done\",\"say\":\"...\"}}\n\
             {{\"kind\":\"unsure\",\"say\":\"...\"}}\n\
             {{\"kind\":\"launch\",\"app\":\"Blender\",\"say\":\"...\"}}\n\
             {{\"kind\":\"open\",\"url\":\"https://...\",\"say\":\"...\"}}\n\
             {{\"kind\":\"reply\",\"say\":\"...\"}}\n\
             {{\"kind\":\"type\",\"text\":\"...\",\"submit\":true,\"say\":\"...\"}}\n\
             where y and x are normalised to 0-1000.",
            prompt(ask)
        );
        let body = json!({
            "contents": [{"parts": [
                {"inline_data": {"mime_type": crate::core::capture::MIME, "data": shot.b64()}},
                {"text": instruction},
            ]}],
            "generationConfig": {"responseMimeType": "application/json"},
        });

        let resp: serde_json::Value = self
            .http
            .post(format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
                self.model
            ))
            .header("x-goog-api-key", &self.key)
            .json(&body)
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
        match v["kind"].as_str() {
            Some("done") => return Ok(Step::Done { say }),
            Some("unsure") => return Ok(Step::Unsure { say }),
            Some("reply") => return Ok(Step::Reply { say }),
            Some("type") => {
                return Ok(Step::Type {
                    text: v["text"].as_str().unwrap_or_default().to_string(),
                    submit: v["submit"].as_bool().unwrap_or(false),
                    say,
                })
            }
            Some("launch") => {
                let app = v["app"].as_str().unwrap_or_default().to_string();
                return Ok(Step::Launch { app, say });
            }
            Some("open") => {
                let url = v["url"].as_str().unwrap_or_default().to_string();
                return Ok(Step::Open { url, say });
            }
            _ => {}
        }

        let pt: Vec<f64> = v["point"]
            .as_array()
            .map(|a| a.iter().filter_map(|n| n.as_f64()).collect())
            .unwrap_or_default();
        if pt.len() != 2 {
            return Err(no_point("gemini", text));
        }
        let (w, h) = shot.sent;
        Ok(Step::Point {
            at: denorm(&pt, w, h),
            say,
            act: super::act_from(v["act"].as_str()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::denorm;

    #[test]
    fn y_comes_first_and_the_grid_is_1000_not_pixels() {
        let p = denorm(&[250.0, 500.0], 800, 600);
        assert_eq!((p.x, p.y), (400.0, 150.0), "y/x order is swapped");
        let p = denorm(&[1000.0, 1000.0], 800, 600);
        assert_eq!((p.x, p.y), (800.0, 600.0));
    }
}
