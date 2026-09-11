//! Nudge: points at the thing you need to click.
//!
//! Two layers, one rule:
//!
//! - [`core`] is what Nudge does -- capture, ask a model, listen, track a session.
//!   No Tauri, no windows, testable on its own.
//! - [`app`] wires that to the OS: the overlay window, the menu bar, the hotkey, IPC.
//!
//! `core` must never depend on `app`. [`config`] and [`error`] are shared by both.

pub mod app;
pub mod config;
pub mod core;
pub mod error;

pub use app::run;
