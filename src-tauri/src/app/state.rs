//! Values Tauri holds for the lifetime of the app.
use crate::core::voice;

/// Screen geometry in points -- the space the overlay draws in -- plus the Retina
/// factor. The only place physical pixels are allowed to appear.
/// The overlay's own geometry: the union of every display, in logical points.
///
/// One window spanning all screens rather than one per screen. The companion has
/// to follow the pointer across a monitor boundary without teleporting, and a
/// ring has to be able to land anywhere -- both are free with one big window and
/// both need bookkeeping with several.
pub struct Screen {
    pub w: f64,
    /// Kept for symmetry with `w` and because the union's height is the thing
    /// a future per-display check will want; not read today.
    #[allow(dead_code)]
    pub h: f64,
    pub scale: f64,
    /// Top-left of the union, in global points. Negative when a display sits
    /// above or to the left of the main one.
    pub origin: (f64, f64),
}

impl Screen {
    /// Global screen point to a position inside the overlay window.
    ///
    /// Zero-cost on a single display, where the union starts at the origin --
    /// which is exactly why the old code could get away with never doing it.
    pub fn to_overlay(
        &self,
        p: crate::core::screen::capture::Point,
    ) -> crate::core::screen::capture::Point {
        crate::core::screen::capture::Point {
            x: p.x - self.origin.0,
            y: p.y - self.origin.1,
        }
    }
}

/// Long-running processes Nudge started, and did not wait for.
pub type Background = crate::core::tools::running::Running;

/// Files the user has agreed to have replaced, and the one being asked about.
///
/// Per path and per grant: saying yes to one file is not saying yes to the next.
/// Held here rather than in the prompt because a permission a model can grant
/// itself is not a permission.
#[derive(Default)]
pub struct Grants {
    pub granted: std::sync::Mutex<std::collections::HashSet<std::path::PathBuf>>,
    /// The one thing currently waiting on a person, whatever kind it is.
    ///
    /// One slot rather than a queue, because a question nobody is looking at is
    /// not a question -- the agent is blocked on this one, and a second could
    /// not be answered until the first was anyway.
    ///
    /// See [`crate::core::reach::Pending`] for why the payload is held rather
    /// than regenerated.
    pub asking: std::sync::Mutex<Option<crate::core::reach::Pending>>,
}

/// The offer on screen, so a window that loads late can ask for it.
#[derive(Default)]
pub struct Offering(std::sync::Mutex<Option<crate::app::ui::connect::Offer>>);

impl Offering {
    pub fn set(&self, offer: crate::app::ui::connect::Offer) {
        *self.0.lock().unwrap() = Some(offer);
    }
    pub fn current(&self) -> Option<crate::app::ui::connect::Offer> {
        self.0.lock().unwrap().clone()
    }
    pub fn clear(&self) {
        *self.0.lock().unwrap() = None;
    }
}

/// What the gate answers for this grant, right now.
///
/// A free function rather than a method so the call site reads as the question
/// being asked -- `permits(app, Grant::Files)` -- and so there is exactly one
/// place that knows the state is managed by Tauri.
///
/// Returns a `Decision` rather than a bool. Today it only ever says allow or
/// deny, and the behaviour is exactly what `may()` did before it; what the type
/// buys is somewhere for the third answer to go, and a refusal that carries its
/// own reason instead of each call site inventing one.
pub fn permits(
    app: &tauri::AppHandle,
    grant: crate::core::reach::Grant,
) -> crate::core::reach::Decision {
    use crate::core::reach::Decision;
    use tauri::Manager;
    match app
        .state::<crate::core::run::session::Nudge>()
        .reach
        .has(grant)
    {
        true => Decision::allowed_by(grant),
        false => Decision::deny(grant.denied()),
    }
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

/// Whether Nudge may raise a suggestion nobody asked for.
///
/// On by default, because an assistant that only ever answers is one people
/// forget can do anything else -- and off is a real preference, because the same
/// thing unasked-for is an interruption.
pub struct Suggesting(pub Flag);

/// The key that summons Nudge, as it stands.
///
/// Changeable, so it cannot live in `cfg` -- and read by the pointer loop sixty
/// times a second as well as by the menu bar, which is why it is a lock around a
/// string rather than anything cleverer.
pub struct Hotkey(pub std::sync::Mutex<String>);

impl Hotkey {
    pub fn get(&self) -> String {
        self.0.lock().unwrap().clone()
    }
    pub fn set(&self, keys: &str) {
        *self.0.lock().unwrap() = keys.to_string();
    }
}

/// Which character the companion wears.
///
/// Here rather than in the interface because two windows draw it -- the overlay
/// when it is loose, the panel when it is parked -- and a preference only one of
/// them knows about is a companion that changes shape when you dock it.
///
/// A string rather than an enum: the whole point is that adding a character is
/// dropping in a file and naming it, and an enum would make Rust a place you have
/// to edit to add art.
pub struct Look(pub std::sync::Mutex<String>);

impl Look {
    pub fn get(&self) -> String {
        self.0.lock().unwrap().clone()
    }
    pub fn set(&self, key: &str) {
        *self.0.lock().unwrap() = key.to_string();
    }
}

/// Whether the companion is sitting in the panel rather than following the cursor.
/// Undocking is how you put it to work; docking is how you get your screen back
/// without quitting.
pub struct Docked(pub Flag);

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
