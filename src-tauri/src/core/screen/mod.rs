//! Everything that touches the screen and the input devices.
//!
//! Grouped because they share a constraint nothing else here does: they are all
//! macOS talking back. Each one needs a permission, fails in a way only this
//! platform fails, and is untestable without a real display -- which is why so
//! many of their tests are `#[ignore]`d and driven by hand.
pub mod capture;
pub mod click;
pub mod facts;
pub mod haptics;
pub mod keyboard;
pub mod launch;
pub mod privacy;
