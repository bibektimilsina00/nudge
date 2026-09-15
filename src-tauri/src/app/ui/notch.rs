//! The notch dock: a pill that lives in the notch and opens when you point at it.
//!
//! The panel is never resized to expand. It is always the full expanded size,
//! transparent, and the *content* grows -- so the open and close are a CSS
//! transition rather than a sequence of window resizes, which cannot be animated
//! and tear on a Retina display.
use objc2_app_kit::NSScreen;
use objc2_foundation::MainThreadMarker;
use std::sync::Mutex;

/// Where the pill sits, in logical points from the top-left of the main screen.
#[derive(Debug, Clone, Copy)]
pub struct Notch {
    pub center_x: f64,
    pub width: f64,
    pub height: f64,
}

/// How much wider than the notch the hover target is. Aiming at the notch is
/// aiming at a gap between two menu bars; a little slack costs nothing.
const SLACK: f64 = 90.0;

/// How far past the open panel the pointer may stray before it counts as leaving.
///
/// The controls people reach for last are in the corners -- the gear, the back
/// arrow, the close -- and a corner is the one place where a hit region exactly
/// the size of the panel is not big enough. Aiming at something 21 points across
/// at the edge of a window means routinely crossing that edge by a few points on
/// the way, and with the region flush to the panel that shut the sheet being
/// aimed at. Wider than tall, because the horizontal edges are where the corner
/// controls live and the bottom edge has nothing on it.
const OPEN_SLACK_X: f64 = 44.0;
const OPEN_SLACK_Y: f64 = 20.0;

/// The open panel's live size, in points.
///
/// Reported by the panel itself whenever it changes shape, rather than fixed at
/// the tallest view. Held constant, leaving the notch kept the panel open all the
/// way across a 700px box -- because that is how tall the *settings sheet* is, and
/// the home view is a third of that. The region that keeps it open has to be the
/// panel you can actually see.
static OPEN: Mutex<(f64, f64)> = Mutex::new((560.0, 300.0));

pub fn set_open_size(w: f64, h: f64) {
    *OPEN.lock().unwrap() = (w, h);
}

pub fn measure() -> Notch {
    let fallback = Notch {
        center_x: 720.0,
        width: 180.0,
        height: 32.0,
    };
    let Some(mtm) = MainThreadMarker::new() else {
        return fallback;
    };
    let Some(screen) = NSScreen::mainScreen(mtm) else {
        return fallback;
    };

    let frame = screen.frame();
    let center_x = frame.size.width / 2.0;
    let top = screen.safeAreaInsets().top;

    // The auxiliary areas are the usable menu bar either side of the notch; they
    // come back empty on a display without one.
    let left = screen.auxiliaryTopLeftArea().size.width;
    let right = screen.auxiliaryTopRightArea().size.width;
    let gap = frame.size.width - left - right;

    if top > 0.0 && left > 0.0 && right > 0.0 && gap > 60.0 {
        Notch {
            center_x,
            width: gap,
            height: top,
        }
    } else {
        // No notch: a pill where one would be.
        Notch {
            center_x,
            width: 180.0,
            height: 32.0,
        }
    }
}

impl Notch {
    /// Should the dock be open, given where the pointer is and whether it is
    /// already open?
    ///
    /// The target is not one region but two. Closed, it is the notch plus slack --
    /// generous, because aiming at a notch means aiming at a gap between two menu
    /// bars. Open, it is the whole panel, or the dock would shut the instant the
    /// pointer moved down onto the very thing it just revealed.
    pub fn is_hovered(&self, x: f64, y: f64, open: bool) -> bool {
        let (w, h) = if open {
            let (w, h) = *OPEN.lock().unwrap();
            (w + OPEN_SLACK_X * 2.0, h + OPEN_SLACK_Y)
        } else {
            (self.width + SLACK * 2.0, self.height + 8.0)
        };
        y <= h && (x - self.center_x).abs() <= w / 2.0
    }
}

#[cfg(test)]
mod tests {
    use super::Notch;

    const N: Notch = Notch {
        center_x: 756.0,
        width: 180.0,
        height: 32.0,
    };

    /// One test, not three, because `OPEN` is a process-wide static and cargo
    /// runs tests in parallel -- three tests each setting their own panel size
    /// read each other's values and failed at random.
    #[test]
    fn the_open_region_follows_the_panel_that_is_actually_showing() {
        // The bug: the region was fixed at the tallest view, so the home panel --
        // a third that height -- stayed open far below anything visible.
        super::set_open_size(530.0, 266.0);
        assert!(N.is_hovered(756.0, 200.0, true), "still on the home panel");
        assert!(
            !N.is_hovered(756.0, 400.0, true),
            "well past its bottom edge"
        );

        // The settings sheet is far taller, and the region has to cover the
        // tallest thing the panel can become -- not the shortest.
        super::set_open_size(530.0, 700.0);
        // Reaching for a button inside the open panel must not leave the region
        // and shut the very thing being reached for.
        assert!(
            !N.is_hovered(756.0, 200.0, false),
            "closed: well below the notch"
        );
        assert!(N.is_hovered(756.0, 200.0, true), "open: still on the panel");
        assert!(
            N.is_hovered(756.0, 650.0, true),
            "open: deep in the settings sheet"
        );
        assert!(
            !N.is_hovered(756.0, 800.0, true),
            "below even the settings sheet"
        );
        assert!(!N.is_hovered(1200.0, 100.0, true), "beside the panel");
    }

    /// Reaching for the gear in the top-right corner must not shut the sheet.
    ///
    /// The bug: the open region was the panel rectangle exactly, so a pointer that
    /// clipped the edge on its way to a corner control left the region, and the
    /// panel closed under the thing being aimed at.
    #[test]
    fn the_corner_controls_are_reachable() {
        let (w, h) = (540.0, 640.0);
        super::set_open_size(w, h);
        let edge = N.center_x + w / 2.0;
        assert!(N.is_hovered(edge - 12.0, 14.0, true), "on the gear");
        assert!(
            N.is_hovered(edge + 20.0, 14.0, true),
            "overshot the edge reaching for it"
        );
        assert!(
            !N.is_hovered(edge + 90.0, 14.0, true),
            "properly away from the panel"
        );
    }

    #[test]
    fn the_notch_itself_opens_it() {
        assert!(N.is_hovered(756.0, 4.0, false), "dead centre");
        assert!(
            N.is_hovered(700.0, 30.0, false),
            "inside, near the bottom edge"
        );
        assert!(
            N.is_hovered(756.0 - 90.0 - 45.0, 10.0, false),
            "just outside, in the slack"
        );
    }

    #[test]
    fn the_menu_bar_either_side_is_not_ours() {
        assert!(
            !N.is_hovered(200.0, 10.0, false),
            "far left is the Apple menu"
        );
        assert!(
            !N.is_hovered(1400.0, 10.0, false),
            "far right is the status items"
        );
    }
}
