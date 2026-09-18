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

/// `None` when there was nothing intelligible in the audio.
///
/// Not an error: the recogniser hearing only room noise is the same everyday
/// non-event as not speaking at all, and being told off for it is worse than it
/// passing unremarked.
/// Did the last transcription happen on this machine, or over the network?
///
/// Worth recording, because the two are not close: measured on this Mac, the
/// on-device recogniser answers in about a quarter of a second and Gemini takes
/// five to seven, which makes it the largest single difference in how long a
/// turn feels. A timing file that calls both of them "heard" cannot tell you how
/// often the fast one is actually working, and the answer turns out to be "less
/// often than you would think".
static ON_DEVICE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn was_on_device() -> bool {
    ON_DEVICE.load(std::sync::atomic::Ordering::Relaxed)
}

pub async fn speech_to_text(cfg: &Config, wav: &[u8]) -> Result<Option<String>> {
    // This machine first, when it will. No upload, no round trip, and it works
    // with the wifi off -- see `ear`, which declines rather than guesses.
    let here = super::ear::transcribe(wav);
    ON_DEVICE.store(here.is_some(), std::sync::atomic::Ordering::Relaxed);
    if let Some(heard) = here {
        return Ok(Some(heard));
    }

    let relay = crate::core::relay::Relay::choose(cfg)
        .ok_or_else(|| Error::Voice(crate::core::relay::Relay::missing()))?;

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

    let resp: serde_json::Value = relay
        .authorise(crate::core::http().post(relay.url(&cfg.voice_model)))
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
    Ok((!text.is_empty()).then_some(text))
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
