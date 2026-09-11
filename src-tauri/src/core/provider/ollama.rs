//! Local, free, offline, no key. The default -- and the privacy story: for a tool
//! that screenshots your work, "nothing leaves this machine" is a feature.
use super::{first_json, no_point, prompt, Ask, Provider, Step};
use crate::core::capture::{Point, Shot};
use crate::config::Config;
use crate::error::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct Ollama {
    model: String,
    host: String,
    http: reqwest::Client,
}

impl Ollama {
    pub fn new(cfg: &Config) -> Self {
        Self {
            model: cfg.model.clone().unwrap_or_else(|| "qwen3-vl:4b".into()),
            host: std::env::var("OLLAMA_HOST")
                .unwrap_or_else(|_| "http://localhost:11434".into()),
            // Local inference on a laptop is slow; the default 30s timeout is not enough.
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(180))
                .build()
                .expect("http client"),
        }
    }
}

#[async_trait]
impl Provider for Ollama {
    fn name(&self) -> &'static str {
        "ollama"
    }

    async fn next_step(&self, shot: &Shot, ask: &Ask<'_>) -> Result<Step> {
        let (w, h) = shot.sent;
        let body = json!({
            "model": self.model,
            "stream": false,
            "format": "json",
            "messages": [{
                "role": "user",
                "content": format!(
                    "{}\n\nReply with only one of:\n\
                     {{\"kind\":\"point\",\"x\":<pixels>,\"y\":<pixels>,\"act\":\"click|doubleClick|hover\",\"say\":\"...\"}}\n\
                     {{\"kind\":\"done\",\"say\":\"...\"}}\n\
                     {{\"kind\":\"unsure\",\"say\":\"...\"}}\n\
                     {{\"kind\":\"launch\",\"app\":\"Blender\",\"say\":\"...\"}}\n\
                     {{\"kind\":\"open\",\"url\":\"https://...\",\"say\":\"...\"}}\n\
                     {{\"kind\":\"reply\",\"say\":\"...\"}}\n\
                     {{\"kind\":\"type\",\"text\":\"...\",\"submit\":true,\"say\":\"...\"}}\n\
                     Coordinates are pixels in this {w}x{h} image, origin top-left.",
                    prompt(ask)
                ),
                "images": [shot.b64()],
            }],
        });

        let text = self
            .http
            .post(format!("{}/api/chat", self.host))
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?["message"]["content"]
            .as_str()
            .unwrap_or_default()
            .to_string();

        let v = first_json(&text).ok_or_else(|| no_point("ollama", text.clone()))?;
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
        // ponytail: assumes absolute pixels, which is what Qwen-VL is trained on.
        // If every ring lands at a constant fraction of where it should, the model
        // answered on a 0-1000 grid instead -- scale by (w/1000, h/1000) and move on.
        match (v["x"].as_f64(), v["y"].as_f64()) {
            (Some(x), Some(y)) => Ok(Step::Point {
                at: Point { x, y },
                say,
                act: super::act_from(v["act"].as_str()),
            }),
            _ => Err(no_point("ollama", text)),
        }
    }
}
