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
//! - [`laps`] -- where a turn's time went
pub mod account;
pub mod answering;
pub mod asked;
pub mod audit;
pub mod claimed;
pub mod compact;
pub mod connect;
pub mod context;
pub mod judge;
pub mod laps;
pub mod memory;
pub mod newer;
pub mod offers;
pub mod permits;
pub mod provenance;
pub mod provider;
pub mod reach;
pub mod relay;
pub mod report;
pub mod risk;
pub mod run;
pub mod screen;
pub mod signin;
pub mod skills;
pub mod stuck;
pub mod teaching;
pub mod threads;
pub mod tools;
pub mod voice;

/// One HTTP client, shared.
///
/// `reqwest::Client` owns the connection pool, so building a fresh one per call
/// means a new TCP and TLS handshake to a host we are very likely already
/// connected to. The providers have always held theirs on the struct; both of
/// the `voice` paths built one per utterance, and paid for it every time anyone
/// spoke.
pub fn http() -> &'static reqwest::Client {
    static HTTP: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    HTTP.get_or_init(reqwest::Client::new)
}
