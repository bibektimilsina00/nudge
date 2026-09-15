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
    /// `"low"` or `"high"`. `None` leaves the model's own default alone.
    ///
    /// Flash models in this family reason before they answer, and those tokens
    /// come out before the first character of the JSON -- so they are paid in
    /// full on the critical path of every screen action.
    ///
    /// It cannot be switched off: this model rejects a budget of zero outright.
    /// Thinking is part of what it is, and the only question is how much. That is
    /// a question for the bench rather than an opinion --
    /// `NUDGE_THINK=low cargo run --bin bench` scores hits and latency together.
    ///
    /// It is also the most expensive line in the file. Thinking bills at the
    /// output rate, five times input, so it outweighs the choice of model.
    /// Measured on 3.6-flash against a 1280px screenshot, two calls per level:
    /// minimal and low emit **no thinking tokens at all** and cost $0.00102 a
    /// call; medium emits between 446 and 2,079 and costs $0.0027 to $0.0088.
    /// Erratic as well as dear -- those two medium numbers are the same request
    /// twice.
    pub think: Option<String>,
    /// What Nudge may do beyond its defaults, at startup.
    ///
    /// Empty means today's Nudge: the shell reads and does not change, files
    /// stay in the workspace, the web is fetched with GET. Each name here lifts
    /// one of those on purpose, and the menu bar can lift or drop any of them
    /// while it runs -- nothing is written back here, so a restart returns to
    /// whatever this line says.
    ///
    /// ```text
    /// reach = ["shell", "files", "http"]
    /// ```
    #[serde(default)]
    pub reach: Vec<String>,
    /// Servers speaking the Model Context Protocol, started at launch.
    ///
    /// Each one is three lines and brings its own tools, which is the point:
    /// six integrations written here would buy six integrations, and this buys
    /// the ones that exist now and the ones written next year.
    ///
    /// ```text
    /// [[mcp]]
    /// name = "files"
    /// command = "npx"
    /// args = ["-y", "@modelcontextprotocol/server-filesystem", "/Users/me/Notes"]
    /// ```
    ///
    /// Tokens go in `env` beside the server that needs them, rather than in
    /// Nudge, which has no business holding somebody else's credentials.
    #[serde(default)]
    pub mcp: Vec<crate::core::tools::mcp::Spec>,
    /// Have a second pass try to refute an answer before it is given.
    ///
    /// Off by default, and the reason is the cost. Measured on one full trace:
    /// answering took two model calls carrying 428 characters of search results;
    /// checking took four more carrying 7,059, because every checker turn
    /// re-sends the whole prompt plus everything found so far. Three times the
    /// calls and worse than that in tokens.
    ///
    /// Against which it has not yet caught a single real error -- it agreed on
    /// one run of the macOS case and disagreed on the next, and during
    /// development it damaged two answers before the guards went in. A three-fold
    /// bill for an unproven save is not a default; it is an experiment, and it
    /// stays behind a switch until `make truth` shows it earning the money.
    pub verify: bool,
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
    /// Where "Report a bug" and "Request a feature" send what people write.
    ///
    /// One URL for both -- they arrive with a `kind` of `"bug"` or `"idea"` and
    /// differ by a word, so whoever reads them wants them in one place anyway.
    /// The body is JSON:
    ///
    /// ```text
    /// report_url = "https://example.com/report"
    ///
    /// { "kind": "bug", "text": "...", "image": "data:image/jpeg;base64,...",
    ///   "version": "0.1.0", "os": "macOS 27.0" }
    /// ```
    ///
    /// `image` is absent unless somebody attached one. Nothing else is collected:
    /// no identifier, no screen, nothing about what they were doing.
    ///
    /// **The default is a placeholder and accepts everything.** `httpbin.org/post`
    /// answers 200 and throws the body away, so Send works end to end today and
    /// nobody has to see a half-built feature -- but nothing sent to it is kept by
    /// anyone. Point this at something real before shipping, or reports are going
    /// into a bin on purpose.
    pub report_url: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            provider: "ollama".into(),
            model: None,
            think: None,
            verify: false,
            mcp: Vec::new(),
            reach: Vec::new(),
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
            report_url: Some("https://httpbin.org/post".into()),
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
