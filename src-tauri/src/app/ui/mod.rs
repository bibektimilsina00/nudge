//! Everything Nudge puts on screen, and the AppKit it takes to keep it there.
//!
//! Grouped because they share one hard-won problem: a window that behaves in a
//! normal app does not behave here. These have to float above full-screen
//! Spaces, never take focus, never appear in the Dock, and survive macOS
//! quietly resetting their level -- see `native` for what was measured to get
//! there.
pub mod connect;
#[cfg(target_os = "macos")]
pub mod native;
#[cfg(not(target_os = "macos"))]
#[path = "elsewhere/native.rs"]
pub mod native;
pub mod notch;
pub mod overlay;
pub mod panel;
pub mod tray;
