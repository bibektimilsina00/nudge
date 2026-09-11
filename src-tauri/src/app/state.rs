//! Values Tauri holds for the lifetime of the app.
use crate::core::voice;

/// Screen geometry in points -- the space the overlay draws in -- plus the Retina
/// factor. The only place physical pixels are allowed to appear.
pub struct Screen {
    pub w: f64,
    pub h: f64,
    pub scale: f64,
}

/// A setting the menu bar can flip at runtime. The config value is only ever the
/// starting point -- these exist so testing does not mean editing a TOML file and
/// restarting.
pub struct Flag(std::sync::atomic::AtomicBool);

impl Flag {
    pub fn new(on: bool) -> Self {
        Self(on.into())
    }
    pub fn on(&self) -> bool {
        self.0.load(std::sync::atomic::Ordering::Relaxed)
    }
    pub fn set(&self, on: bool) {
        self.0.store(on, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Whether the companion is sitting in the panel rather than following the cursor.
/// Undocking is how you put it to work; docking is how you get your screen back
/// without quitting.
pub struct Docked(pub Flag);

/// Whether Nudge clicks for you, or only points.
pub struct Auto(pub Flag);

/// How Nudge speaks. Three states rather than a checkbox, because "off" is a
/// genuinely different choice from "which voice" -- folding it into the engine
/// name would make `speech_engine = "none"` mean two things at once.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum VoiceMode {
    /// Silent.
    Off,
    /// macOS `say`: free, offline, instant.
    System,
    /// Gemini TTS: much better, a network round trip, and a charge per nudge.
    Gemini,
}

impl VoiceMode {
    /// Stable identifiers, shared by the menu bar and the panel so the two cannot
    /// disagree about what the modes are called.
    pub const ALL: [(&'static str, VoiceMode); 3] = [
        ("voice:off", VoiceMode::Off),
        ("voice:system", VoiceMode::System),
        ("voice:gemini", VoiceMode::Gemini),
    ];

    /// What the config file asked for, before the menu bar overrides it.
    pub fn from_config(speak: bool, engine: &str) -> Self {
        match (speak, engine) {
            (false, _) => VoiceMode::Off,
            (_, "gemini") => VoiceMode::Gemini,
            _ => VoiceMode::System,
        }
    }

    /// Applied to a throwaway clone, so the file stays the source of truth for
    /// everything the menu does not cover.
    pub fn apply(self, cfg: &mut crate::config::Config) {
        match self {
            VoiceMode::Off => cfg.speak = false,
            VoiceMode::System => cfg.speech_engine = "system".into(),
            VoiceMode::Gemini => cfg.speech_engine = "gemini".into(),
        }
    }
}

pub struct Voice(pub std::sync::Mutex<VoiceMode>);

impl Voice {
    pub fn get(&self) -> VoiceMode {
        *self.0.lock().unwrap()
    }
    pub fn set(&self, mode: VoiceMode) {
        *self.0.lock().unwrap() = mode;
    }
}

/// When the screen is expected to have finished reacting to the last action.
///
/// A click that opens a menu takes a moment to actually draw it; screenshotting
/// straight afterwards captures the screen as it was, and the model is asked to
/// find a menu that is not on screen yet. It then reports the menu missing, which
/// looks like the model being wrong when it was simply shown a stale picture.
pub struct Settle(pub std::sync::Mutex<Option<std::time::Instant>>);

impl Settle {
    /// ponytail: fixed waits per kind of action, tuned by eye. The thorough
    /// version screenshots in a loop until two frames match -- worth doing if these
    /// ever feel slow, at the cost of a capture every 120ms while waiting.
    pub fn after(&self, wait: std::time::Duration) {
        *self.0.lock().unwrap() = Some(std::time::Instant::now() + wait);
    }

    /// How long is left, if anything.
    pub fn remaining(&self) -> Option<std::time::Duration> {
        self.0
            .lock()
            .unwrap()
            .take()
            .and_then(|t| t.checked_duration_since(std::time::Instant::now()))
    }
}

/// The in-flight recording, if the hotkey is currently held down.
#[derive(Default)]
pub struct Mic(pub std::sync::Mutex<Option<(std::time::Instant, voice::Recording)>>);
