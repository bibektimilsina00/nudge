//! Listening and speaking: the primary way in and out.
//!
//! `record` captures the microphone, `ear` turns that into words on this machine
//! and `transcribe` does it over the network when `ear` cannot, and
//! `speech` says words back. Three separate things because each can be swapped
//! on its own -- on-device transcription, a different voice -- without the other
//! two noticing.
pub mod ear;
mod record;
pub mod speech;
pub mod transcribe;

// Flattened on purpose: `voice::record::start()` says "record" twice, and the
// module is named for what it does either way.
pub use record::*;
