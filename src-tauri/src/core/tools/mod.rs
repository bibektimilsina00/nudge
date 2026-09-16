//! What Nudge can do that is not on the screen.
//!
//! The only group here with no macOS in it: commands, files and the web. That is
//! not an accident of filing -- it is the half of the product that works the same
//! everywhere, and the half whose safety rules are written in Rust rather than
//! asked for in a prompt.
pub mod fetch;
pub mod files;
pub mod mcp;
pub mod paper;
pub mod present;
pub mod running;
pub mod secret;
pub mod shell;
