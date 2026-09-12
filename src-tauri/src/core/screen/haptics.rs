//! A tick on the trackpad.
//!
//! Arriving at the notch is an alignment, and `Alignment` is the pattern macOS
//! provides for it -- but it is tuned for a guide snapping inside a window you are
//! already looking at, and against a deliberate sweep to the top of the screen it
//! is too faint to register without concentrating. `Generic` is the firmest of the
//! three and reads as a definite arrival.
//!
//! Silent on hardware without a Force Touch trackpad, and when "Force Click and
//! haptic feedback" is off in System Settings. Both are correct, and neither needs
//! a check of our own.

/// How many ticks make up one thunk. One reads as a clean arrival; three is
/// unmissable but starts to feel like a buzz for something triggered this often,
/// which is why it is back to one.
pub const BURST: usize = 1;
/// Spacing between them. Close enough to fuse into one sensation, far enough
/// apart that the actuator has actually reset.
pub const GAP: std::time::Duration = std::time::Duration::from_millis(45);

/// The pattern dial. Firm to faint: `Generic` · `LevelChange` · `Alignment`.
#[cfg(target_os = "macos")]
const PATTERN: objc2_app_kit::NSHapticFeedbackPattern =
    objc2_app_kit::NSHapticFeedbackPattern::Generic;

/// One alignment tick. Must be called on the main thread.
#[cfg(target_os = "macos")]
pub fn tick() {
    use objc2_app_kit::{
        NSHapticFeedbackManager, NSHapticFeedbackPerformanceTime, NSHapticFeedbackPerformer,
    };
    let performer = NSHapticFeedbackManager::defaultPerformer();
    performer.performFeedbackPattern_performanceTime(PATTERN, NSHapticFeedbackPerformanceTime::Now);
}

#[cfg(not(target_os = "macos"))]
pub fn tick() {}
