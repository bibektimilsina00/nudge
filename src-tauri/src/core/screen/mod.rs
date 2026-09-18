//! Everything that touches the screen and the input devices.
//!
//! Grouped because they share a constraint nothing else here does: they are all
//! macOS talking back. Each one needs a permission, fails in a way only this
//! platform fails, and is untestable without a real display -- which is why so
//! many of their tests are `#[ignore]`d and driven by hand.
//!
//! [`ax`] is the exception in kind rather than degree: it asks the system what
//! is on screen instead of photographing it.
//!
//! # The platform seam
//!
//! Three of these -- [`click`], [`keyboard`] and [`launch`] -- have a second
//! implementation under `elsewhere/`, chosen by `cfg`. That is where a port
//! starts: the stubs are the list of what another platform has to answer, each
//! one documenting what the macOS version had to get right and which crate
//! covers it elsewhere.
//!
//! Everything else here already declines gracefully off macOS -- `capture`
//! returns nothing, `ax` returns no controls, `facts` knows nothing -- so the
//! app runs, blind, rather than failing to build.
//!
//! The two sides are held together by a test rather than by discipline: add a
//! function to `click` and forget `elsewhere/click.rs`, and the build breaks
//! here rather than on someone else's machine months later.
/// The modifier keys, as a set.
///
/// Its own type rather than a `bool` per key, because the question is always
/// "exactly these and no others" -- the hotkey names a set, the keyboard holds a
/// set, and the gesture is the two being equal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Mods(u8);

impl Mods {
    pub const CONTROL: Mods = Mods(1 << 0);
    pub const OPTION: Mods = Mods(1 << 1);
    pub const SHIFT: Mods = Mods(1 << 2);
    pub const COMMAND: Mods = Mods(1 << 3);

    pub const fn empty() -> Self {
        Mods(0)
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Does this set contain that one?
    pub const fn has(self, other: Mods) -> bool {
        self.0 & other.0 == other.0
    }
}

impl std::ops::BitOr for Mods {
    type Output = Mods;
    fn bitor(self, other: Mods) -> Mods {
        Mods(self.0 | other.0)
    }
}

impl std::ops::BitOrAssign for Mods {
    fn bitor_assign(&mut self, other: Mods) {
        self.0 |= other.0;
    }
}

pub mod ax;
pub mod capture;
#[cfg(all(target_os = "macos", not(feature = "portable")))]
pub mod click;
#[cfg(any(not(target_os = "macos"), feature = "portable"))]
#[path = "elsewhere/click.rs"]
pub mod click;
pub mod facts;
#[cfg(target_os = "macos")]
pub mod fast;
pub mod haptics;
#[cfg(all(target_os = "macos", not(feature = "portable")))]
pub mod keyboard;
#[cfg(any(not(target_os = "macos"), feature = "portable"))]
#[path = "elsewhere/keyboard.rs"]
pub mod keyboard;
#[cfg(all(target_os = "macos", not(feature = "portable")))]
pub mod launch;
#[cfg(any(not(target_os = "macos"), feature = "portable"))]
#[path = "elsewhere/launch.rs"]
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

#[cfg(test)]
mod seam {
    /// Every platform-specific module must offer the same functions on both
    /// sides, or a port compiles on macOS and fails everywhere else -- which is
    /// exactly the kind of breakage nobody finds until they try.
    ///
    /// Reads the source, because the other side is not compiled on this machine
    /// and so cannot be type-checked here. Crude, and it catches the one thing
    /// that actually goes wrong: a function added to one side only.
    #[test]
    fn both_sides_of_the_platform_seam_offer_the_same_functions() {
        fn names(src: &str) -> Vec<String> {
            src.lines()
                .filter_map(|l| l.trim().strip_prefix("pub fn "))
                .filter_map(|l| l.split(['(', '<']).next())
                .map(str::to_string)
                .collect()
        }

        for (module, mac, elsewhere) in [
            (
                "click",
                include_str!("click.rs"),
                include_str!("elsewhere/click.rs"),
            ),
            (
                "keyboard",
                include_str!("keyboard.rs"),
                include_str!("elsewhere/keyboard.rs"),
            ),
            (
                "launch",
                include_str!("launch.rs"),
                include_str!("elsewhere/launch.rs"),
            ),
        ] {
            let (mut here, mut there) = (names(mac), names(elsewhere));
            here.sort();
            there.sort();
            assert_eq!(
                here, there,
                "{module}: the two sides have drifted -- anything only on one list \
                 is a function a port cannot supply, or one nobody needs"
            );
        }
    }
}
