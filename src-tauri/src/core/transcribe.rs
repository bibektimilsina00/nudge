//! Speech -> text. Separate from `provider` on purpose: the model that hears you
//! and the model that finds the button are independent choices, and you may well
//! want a local ear with a hosted eye.
//!
//! ponytail: Gemini over the network. macOS ships on-device speech recognition
//! (SFSpeechRecognizer) which would be free, offline and lower latency -- that is
//! the upgrade, and it costs an objc2 binding rather than an HTTP call.
use crate::config::Config;
use crate::error::{Error, Result};
use base64::Engine;
use serde_json::json;

pub async fn speech_to_text(cfg: &Config, wav: &[u8]) -> Result<String> {
    let key = cfg
        .key("GEMINI_API_KEY")
        .ok_or_else(|| Error::Voice("voice needs GEMINI_API_KEY (or api_key) set".into()))?;

    let body = json!({
        "contents": [{"parts": [
            {"inline_data": {
                "mime_type": "audio/wav",
                "data": base64::engine::general_purpose::STANDARD.encode(wav),
            }},
            {"text": "Transcribe the speech in this audio verbatim. \
                      Reply with only the words spoken and nothing else. \
                      If there is no speech, reply with an empty string."},
        ]}],
    });

    let resp: serde_json::Value = reqwest::Client::new()
        .post(format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
            cfg.voice_model
        ))
        .header("x-goog-api-key", &key)
        .json(&body)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let text = resp["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .unwrap_or_default();
    let text = clean(text);
    if text.is_empty() {
        return Err(Error::Voice("didn't catch that".into()));
    }
    Ok(text)
}

/// Models like to answer a transcription request with a sentence about the
/// transcription. Strip the usual wrappers before it reaches the prompt.
fn clean(raw: &str) -> String {
    raw.trim()
        .trim_matches(|c| c == '"' || c == '\'' || c == '`')
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::clean;

    #[test]
    fn strips_quotes_and_whitespace() {
        assert_eq!(clean("  \"open the UV editor\" \n"), "open the UV editor");
        assert_eq!(clean("open the UV editor"), "open the UV editor");
        assert_eq!(clean(""), "");
    }
}
