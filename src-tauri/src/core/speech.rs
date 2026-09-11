//! Reading the nudge out loud.
//!
//! Two engines. `system` is the default: free, offline, instant, and no charge for
//! a thing that talks on every single step. `gemini` sounds far better and is one
//! menu-bar click away when it matters.
//!
//! The catch with `system` is that macOS ships nothing good -- the built-in voices
//! are a 1990s "compact" set, and the modern Siri voices offered in Manage Voices
//! are deliberately unavailable to `say` and to every third-party app. Installing a
//! *non-Siri* Premium English voice (Ava, Allison, Zoe, Evan, Nathan, Joelle, Tom)
//! is what makes this engine sound current, and `best_voice` picks it up with no
//! config change.
//!
//! `system` is also the automatic fallback whenever the network, the key, or the
//! quota is not there.
use crate::config::Config;
use base64::Engine as _;
use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

/// Whatever is making noise right now.
static PLAYER: Mutex<Option<Child>> = Mutex::new(None);

/// Bumped every time speech is superseded or silenced. A network round trip means
/// audio can arrive *after* the nudge it belongs to is gone; without this, Escape
/// would be followed a second later by a sentence about a ring you already
/// dismissed.
static TURN: AtomicU64 = AtomicU64::new(0);

/// Silence anything playing and claim the next turn.
fn begin() -> u64 {
    let turn = TURN.fetch_add(1, Ordering::SeqCst) + 1;
    if let Some(mut old) = PLAYER.lock().unwrap().take() {
        let _ = old.kill();
        let _ = old.wait(); // reap it, or every nudge leaves a zombie
    }
    turn
}

fn current(turn: u64) -> bool {
    TURN.load(Ordering::SeqCst) == turn
}

/// Stop mid-sentence. Escape should silence Nudge as completely as it hides it.
pub fn hush() {
    begin();
}

/// Fire-and-forget: callers spawn this rather than awaiting it, so fetching audio
/// never delays the ring.
pub async fn speak(cfg: &Config, text: &str) {
    let text = text.trim();
    if !cfg.speak || text.is_empty() {
        return;
    }
    let turn = begin();
    let loud = std::env::var("NUDGE_DEBUG_SPEECH").is_ok();
    if loud {
        eprintln!("speak[{turn}] engine={} text={text:?}", cfg.speech_engine);
    }

    if cfg.speech_engine != "system" {
        match gemini(cfg, text).await {
            Ok(wav) => return play(&wav, turn),
            // Offline, no key, or out of quota -- a worse voice beats no voice.
            Err(e) => eprintln!("nudge: gemini tts unavailable ({e}), using system voice"),
        }
    }
    system(cfg, text, turn);
    if loud {
        eprintln!("speak[{turn}] system voice={:?}", chosen_voice());
    }
}

async fn gemini(cfg: &Config, text: &str) -> Result<Vec<u8>, String> {
    let key = cfg
        .key("GEMINI_API_KEY")
        .ok_or_else(|| "no GEMINI_API_KEY".to_string())?;
    let voice = cfg.speech_voice.as_deref().unwrap_or("Kore");

    let body = serde_json::json!({
        "contents": [{"parts": [{"text": text}]}],
        "generationConfig": {
            "responseModalities": ["AUDIO"],
            "speechConfig": {
                "voiceConfig": {"prebuiltVoiceConfig": {"voiceName": voice}},
            },
        },
    });

    let resp: serde_json::Value = reqwest::Client::new()
        .post(format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
            cfg.speech_model
        ))
        .header("x-goog-api-key", key)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let part = &resp["candidates"][0]["content"]["parts"][0]["inlineData"];
    let pcm = base64::engine::general_purpose::STANDARD
        .decode(part["data"].as_str().unwrap_or_default())
        .map_err(|e| e.to_string())?;
    if pcm.is_empty() {
        return Err("no audio returned".into());
    }
    // Returned as headerless PCM (`audio/L16;codec=pcm;rate=24000`), so it needs a
    // container before anything will play it.
    Ok(wrap_pcm(&pcm, sample_rate(part["mimeType"].as_str().unwrap_or(""))))
}

/// The rate lives in the mime type rather than a field of its own. Parsed instead
/// of hardcoded: a model that answers at 16kHz would otherwise sound chipmunked.
fn sample_rate(mime: &str) -> u32 {
    mime.split(';')
        .find_map(|p| p.trim().strip_prefix("rate="))
        .and_then(|r| r.parse().ok())
        .unwrap_or(24_000)
}

/// Minimal RIFF/WAVE header for 16-bit mono PCM.
fn wrap_pcm(pcm: &[u8], rate: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(pcm.len() + 44);
    let data = pcm.len() as u32;
    out.extend(b"RIFF");
    out.extend((36 + data).to_le_bytes());
    out.extend(b"WAVEfmt ");
    out.extend(16u32.to_le_bytes()); // fmt chunk size
    out.extend(1u16.to_le_bytes()); // PCM
    out.extend(1u16.to_le_bytes()); // mono
    out.extend(rate.to_le_bytes());
    out.extend((rate * 2).to_le_bytes()); // byte rate: rate * channels * 2
    out.extend(2u16.to_le_bytes()); // block align
    out.extend(16u16.to_le_bytes()); // bits per sample
    out.extend(b"data");
    out.extend(data.to_le_bytes());
    out.extend(pcm);
    out
}

fn play(wav: &[u8], turn: u64) {
    if !current(turn) {
        return; // dismissed while the audio was in flight
    }
    let path = std::env::temp_dir().join("nudge-say.wav");
    if let Err(e) = std::fs::File::create(&path).and_then(|mut f| f.write_all(wav)) {
        eprintln!("nudge: couldn't write audio: {e}");
        return;
    }
    spawn(Command::new("afplay").arg(&path), turn);
}

fn system(cfg: &Config, text: &str, turn: u64) {
    let mut cmd = Command::new("say");
    if let Some(v) = cfg.speech_voice.clone().or_else(|| best_voice().clone()) {
        cmd.arg("-v").arg(v);
    }
    cmd.arg("--").arg(text); // `--` stops a leading "-" reading as a flag
    spawn(&mut cmd, turn);
}

fn spawn(cmd: &mut Command, turn: u64) {
    let child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    match child {
        Ok(child) => {
            let mut slot = PLAYER.lock().unwrap();
            if current(turn) {
                *slot = Some(child);
            } else {
                let mut child = child;
                let _ = child.kill();
                let _ = child.wait();
            }
        }
        // Losing the voice is not worth failing a nudge over.
        Err(e) => eprintln!("nudge: couldn't speak: {e}"),
    }
}

/// Which voice the system engine will use, for reporting at startup. A silently
/// wrong choice here sounds exactly like the feature being broken.
pub fn chosen_voice() -> Option<String> {
    best_voice().clone()
}

/// Best `say` voice actually installed. Probed once -- shelling out to `say -v ?`
/// per sentence would add latency to the one thing that should feel immediate.
fn best_voice() -> &'static Option<String> {
    static VOICE: OnceLock<Option<String>> = OnceLock::new();
    VOICE.get_or_init(|| {
        let out = Command::new("say").arg("-v").arg("?").output().ok()?;
        let listing = String::from_utf8_lossy(&out.stdout);
        // A downloaded Premium voice silently *replaces* the compact one under the
        // same name -- the listing gains no suffix and no new row -- so quality
        // cannot be detected, only preferred by name. These are the voices whose
        // Premium variants exist; if the user never downloaded one, the compact
        // version answers to the same name and simply sounds worse.
        // In preference order. All of these have Premium/Enhanced downloads; the
        // compact fallback answers to the same name and simply sounds worse, so
        // quality cannot be detected here -- only preferred by name.
        const GOOD: [&str; 14] = [
            "Ava", "Allison", "Serena", "Zoe", "Joelle", "Noelle", "Evan", "Nathan",
            "Tom", "Susan", "Samantha", "Stephanie", "Oliver", "Daniel",
        ];
        GOOD.iter()
            .find_map(|name| pick(&listing, |line| line.starts_with(name)))
    })
}

/// Voice names are everything before the locale column. They contain spaces, and
/// the column is not aligned -- `Samantha (English (US)) en_US` has a single space
/// where `Albert` has many -- so the locale marker is the only reliable boundary.
fn pick(listing: &str, want: impl Fn(&str) -> bool) -> Option<String> {
    listing
        .lines()
        .filter(|l| want(l))
        .filter_map(|l| Some(l[..l.find(" en_")?].trim().to_string()))
        .find(|n| !n.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{pick, sample_rate, wrap_pcm};

    const LISTING: &str = "\
Albert              en_US    # Hello!
Samantha (English (US)) en_US    # Hello!
Anna                de_DE    # Hallo!";

    #[test]
    fn keeps_multi_word_voice_names_despite_ragged_columns() {
        assert_eq!(
            pick(LISTING, |l| l.starts_with("Samantha")).as_deref(),
            Some("Samantha (English (US))"),
        );
        assert_eq!(pick(LISTING, |l| l.starts_with("Albert")).as_deref(), Some("Albert"));
    }

    #[test]
    fn ignores_voices_that_do_not_speak_english() {
        assert_eq!(pick(LISTING, |l| l.starts_with("Anna")), None);
    }

    #[test]
    fn reads_the_rate_out_of_the_mime_type() {
        assert_eq!(sample_rate("audio/L16;codec=pcm;rate=24000"), 24_000);
        assert_eq!(sample_rate("audio/L16;codec=pcm;rate=16000"), 16_000);
        assert_eq!(sample_rate("audio/L16"), 24_000, "falls back, never panics");
    }

    #[test]
    fn builds_a_header_a_player_will_accept() {
        let wav = wrap_pcm(&[1, 2, 3, 4], 24_000);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[36..40], b"data");
        assert_eq!(wav.len(), 48, "44-byte header plus the samples");
        // Sizes must count the payload, not the whole file, or playback truncates.
        assert_eq!(u32::from_le_bytes(wav[40..44].try_into().unwrap()), 4);
        assert_eq!(u32::from_le_bytes(wav[4..8].try_into().unwrap()), 40);
    }
}
