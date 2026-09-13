//! Everything that touches the screen and the input devices.
//!
//! Grouped because they share a constraint nothing else here does: they are all
//! macOS talking back. Each one needs a permission, fails in a way only this
//! platform fails, and is untestable without a real display -- which is why so
//! many of their tests are `#[ignore]`d and driven by hand.
//!
//! [`ax`] is the exception in kind rather than degree: it asks the system what
//! is on screen instead of photographing it.
pub mod ax;
pub mod capture;
#[cfg(target_os = "macos")]
pub mod fast;
pub mod click;
pub mod facts;
pub mod haptics;
pub mod keyboard;
pub mod launch;
pub mod privacy;

use crate::config::Config;
use crate::error::{Error, Result};

/// One observation of the screen: the picture, and the facts that describe the
/// same moment.
///
/// They travel together because they have to. `facts` outranks the picture in
/// the prompt -- the model is told to believe it over its own eyes -- so facts
/// gathered a second away from the shot would authoritatively describe a
/// different screen.
pub struct Look {
    pub facts: facts::Facts,
    pub shot: capture::Shot,
    /// What the system says is on screen, in global points. Empty when the app
    /// exposes nothing, which is a normal answer and not a failure -- plenty of
    /// interfaces are drawn rather than built.
    pub controls: Vec<ax::Control>,
    /// When it was taken. A look can be taken ahead of the turn that uses it,
    /// and a photograph of a screen nobody is looking at any more is worse than
    /// no photograph.
    pub taken: std::time::Instant,
}

/// Check, gather, photograph -- in that order, and never any other.
///
/// The privacy check comes first because there is no un-sending a screenshot.
/// It has to happen while the only thing that exists is a window title, which is
/// why it lives here rather than at the call sites: every caller routes through
/// this function, so no future one can forget it.
pub fn look(cfg: &Config) -> Result<Look> {
    let front = privacy::frontmost_window();
    if let Some((_, app, title)) = &front {
        if let Some(reason) = privacy::blocked_by(cfg, app, title) {
            return Err(Error::Blocked(reason));
        }
    }
    let facts = facts::gather();
    let shot = capture::grab(cfg.max_edge)?;
    // After the picture, not before. Reading a tree can take the best part of a
    // second, and the two will disagree by however long that took -- so the
    // fresher of the pair should be the one whose coordinates we click.
    let controls = front
        .map(|(pid, _, _)| ax::controls(pid))
        .unwrap_or_default();
    Ok(Look {
        facts,
        shot,
        controls,
        taken: std::time::Instant::now(),
    })
}
