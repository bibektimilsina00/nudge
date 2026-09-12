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
    if let Some((app, title)) = privacy::frontmost() {
        if let Some(reason) = privacy::blocked_by(cfg, &app, &title) {
            return Err(Error::Blocked(reason));
        }
    }
    Ok(Look {
        facts: facts::gather(),
        shot: capture::grab(cfg.max_edge)?,
        taken: std::time::Instant::now(),
    })
}
