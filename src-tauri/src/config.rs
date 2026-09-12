//! ~/.config/nudge/config.toml -- this file *is* the "bring your own model" feature.
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// "ollama" (local, free) | "gemini" | "anthropic"
    pub provider: String,
    /// None -> each provider's own default.
    pub model: Option<String>,
    /// Prefer the env var; this is here for people who'd rather not export one.
    pub api_key: Option<String>,
    pub hotkey: String,
    /// Long edge of the screenshot sent to the model, and the main speed dial.
    ///
    /// Measured on a 3024x1964 display: 1920px took 3.6s round trip, 1280px took
    /// 2.4s, and both landed on the same 20px gear icon -- so the extra pixels were
    /// paying for nothing. Raise it if small widgets start being missed. Anthropic
    /// additionally hard-caps at 2576px and silently downscales past it, which
    /// desyncs every coordinate, so never go above that.
    pub max_edge: u32,
    /// Model that turns speech into text. Independent of `model` -- the ear and
    /// the eye are separate choices. Gemini-only for now; needs GEMINI_API_KEY.
    pub voice_model: String,
    /// Read each nudge aloud.
    pub speak: bool,
    /// "system" (macOS `say` -- free, offline, instant) or "gemini" (much better,
    /// but a network round trip and a charge per nudge). Free is the right default
    /// for something that speaks on every step; the menu bar switches it per run.
    pub speech_engine: String,
    /// Gemini TTS model, used when `speech_engine = "gemini"`.
    pub speech_model: String,
    /// Voice name for the chosen engine -- a Gemini prebuilt voice ("Kore",
    /// "Puck", "Aoede"...) or a macOS one (`say -v ?`). None picks a default.
    pub speech_voice: Option<String>,
    /// Where commands run and files are written.
    ///
    /// This is the boundary that makes shell access and file writing acceptable
    /// at all: nothing outside it is read, written or run. Point it at a folder
    /// for work Nudge produces, not at a source repository -- generated pages
    /// landing among tracked files is a mess, and the first test that wrote one
    /// dropped it straight into this project's root.
    ///
    /// Defaults to the home folder when unset, which is deliberately too broad
    /// to be comfortable; set it.
    pub workspace: Option<String>,
    /// Refuse to screenshot password managers and windows that name a secret.
    /// On by default: the cost of being wrong is not symmetric.
    pub privacy_guard: bool,
    /// Extra application names to refuse, beyond the built-in list.
    pub blocked_apps: Vec<String>,
    /// Extra window-title fragments to refuse, beyond the built-in list.
    pub blocked_titles: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            provider: "ollama".into(),
            model: None,
            api_key: None,
            // A bare modifier: hold Control to talk, tap it for the next step.
            hotkey: "ctrl".into(),
            max_edge: 1280,
            voice_model: "gemini-3.8-flash".into(),
            speak: true,
            speech_engine: "system".into(),
            speech_model: "gemini-2.5-flash-preview-tts".into(),
            speech_voice: None,
            workspace: None,
            privacy_guard: true,
            blocked_apps: Vec::new(),
            blocked_titles: Vec::new(),
        }
    }
}

impl Config {
    /// The directory commands run in, resolved and checked to exist.
    pub fn workspace_dir(&self) -> PathBuf {
        self.workspace
            .as_ref()
            .map(|w| match w.strip_prefix("~/") {
                Some(rest) => dirs::home_dir().unwrap_or_default().join(rest),
                None => PathBuf::from(w),
            })
            .filter(|p| p.is_dir())
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_default())
    }

    /// `~/.config/nudge/config.toml`, spelled out rather than via
    /// `dirs::config_dir()` -- that returns `~/Library/Application Support` on
    /// macOS, so a file written where the docs say silently does nothing and you
    /// get default settings with no warning.
    pub fn path() -> Result<PathBuf> {
        dirs::home_dir()
            .map(|d| d.join(".config/nudge/config.toml"))
            .ok_or_else(|| Error::Config("no home directory".into()))
    }

    /// Missing file is not an error -- defaults run local-only with no key. It is
    /// still worth saying so out loud: silently running on defaults is exactly how
    /// the wrong provider gets used for an hour without anyone noticing.
    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        let Ok(text) = std::fs::read_to_string(&path) else {
            eprintln!("nudge: no config at {} -- using defaults", path.display());
            return Ok(Self::default());
        };
        toml::from_str(&text).map_err(|e| Error::Config(format!("{}: {e}", path.display())))
    }

    /// Env wins over the file so a shared config can stay free of secrets.
    pub fn key(&self, env_var: &str) -> Option<String> {
        std::env::var(env_var)
            .ok()
            .filter(|k| !k.is_empty())
            .or_else(|| self.api_key.clone())
    }
}
