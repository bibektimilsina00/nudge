//! Listening and speaking: the primary way in and out.
//!
//! `record` captures the microphone, `transcribe` turns that into words, and
//! `speech` says words back. Three separate things because each can be swapped
//! on its own -- on-device transcription, a different voice -- without the other
//! two noticing.
mod record;
pub mod speech;
pub mod transcribe;

// Flattened on purpose: `voice::record::start()` says "record" twice, and the
// module is named for what it does either way.
pub use record::*;
