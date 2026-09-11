//! Everything Nudge actually does, with no Tauri in sight: take a screenshot, ask a
//! model where to click, record speech, turn it into text, track a session.
//!
//! Nothing here may depend on `crate::app`. That one rule is what keeps this layer
//! unit-testable without spawning a window, and it is why the tests in here run in
//! milliseconds.

pub mod agent;
pub mod capture;
pub mod click;
pub mod haptics;
pub mod keyboard;
pub mod launch;
pub mod privacy;
pub mod provider;
pub mod session;
pub mod speech;
pub mod transcribe;
pub mod voice;
