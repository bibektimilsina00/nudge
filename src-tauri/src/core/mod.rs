//! Everything Nudge does, with no Tauri in it.
//!
//! The split is enforced by `tests/layering.rs`, not by convention: nothing here
//! may import the GUI layer. That keeps the interesting parts testable without a
//! window, and it is why the harnesses in `bin/` can drive the same code the app
//! does.
//!
//! - [`run`] -- the loop, and who is driving it
//! - [`provider`] -- the models, one file each
//! - [`screen`] -- capture and input; everything that needs a permission
//! - [`voice`] -- microphone, transcription, speech
//! - [`tools`] -- commands, files and the web
pub mod provider;
pub mod run;
pub mod screen;
pub mod tools;
pub mod voice;
