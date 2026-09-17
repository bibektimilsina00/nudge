//! The frontend's entire API surface. Thin on purpose: every one of these is a
//! translation from an IPC call into a `core` call, and nothing more.
//!
//! Split three ways because they are three different things that happened to
//! share a file: the loop that does the work, the controls for an agent that is
//! already running, and the settings. `advance` and `quit` do not belong
//! together.
mod account;
mod agents;
mod connect;
mod settings;
mod step;

pub use account::*;
pub use agents::*;
pub use connect::*;
pub use settings::*;
pub use step::*;
