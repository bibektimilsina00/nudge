//! The notch dock: a pill that lives in the notch and opens when you point at it.
//!
//! The panel is never resized to expand. It is always the full expanded size,
//! transparent, and the *content* grows -- so the open and close are a CSS
//! transition rather than a sequence of window resizes, which cannot be animated
//! and tear on a Retina display.
use objc2_app_kit::NSScreen;
use objc2_foundation::MainThreadMarker;

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

/// The open panel, in points. Deliberately a little larger than the CSS panel:
/// this is the region that *keeps* it open, and a few pixels of forgiveness at the
/// edge is the difference between a panel you can use and one that snaps shut as
/// you reach for a button. Kept generous rather than exact so the two numbers do
/// not have to be kept in lockstep with the stylesheet.
/// Big enough for the settings sheet, which is far taller than the home view.
/// This is the region that *keeps* the panel open, so it has to cover the tallest
/// thing the panel can become, not the smallest.
const OPEN: (f64, f64) = (570.0, 700.0);

/// Measure the real notch, or invent a plausible one.
///
/// `safeAreaInsets.top` is the notch height, and the two auxiliary areas are the
/// usable menu bar either side of it -- so what is left in the middle is the notch.
/// External displays have neither, and get a pill of the same size centred at the
/// top, which is where a notch would be if the machine had one.
pub fn measure() -> Notch {
    let fallback = Notch { center_x: 720.0, width: 180.0, height: 32.0 };
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
        Notch { center_x, width: gap, height: top }
    } else {
        // No notch: a pill where one would be.
        Notch { center_x, width: 180.0, height: 32.0 }
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
            OPEN
        } else {
            (self.width + SLACK * 2.0, self.height + 8.0)
        };
        y <= h && (x - self.center_x).abs() <= w / 2.0
    }
}

#[cfg(test)]
mod tests {
    use super::Notch;

    const N: Notch = Notch { center_x: 756.0, width: 180.0, height: 32.0 };

    #[test]
    fn the_notch_itself_opens_it() {
        assert!(N.is_hovered(756.0, 4.0, false), "dead centre");
        assert!(N.is_hovered(700.0, 30.0, false), "inside, near the bottom edge");
        assert!(N.is_hovered(756.0 - 90.0 - 45.0, 10.0, false), "just outside, in the slack");
    }

    #[test]
    fn the_menu_bar_either_side_is_not_ours() {
        assert!(!N.is_hovered(200.0, 10.0, false), "far left is the Apple menu");
        assert!(!N.is_hovered(1400.0, 10.0, false), "far right is the status items");
    }

    #[test]
    fn while_open_the_whole_panel_holds_it_open() {
        // The bug this exists for: reaching for a button inside the open panel left
        // the notch region and shut the thing being reached for.
        assert!(!N.is_hovered(756.0, 200.0, false), "closed: well below the notch");
        assert!(N.is_hovered(756.0, 200.0, true), "open: still on the panel");
        // The settings sheet is far taller than the home view, and the region has to
        // cover the tallest thing the panel can become -- not the shortest.
        assert!(N.is_hovered(756.0, 650.0, true), "open: deep in the settings sheet");
    }

    #[test]
    fn leaving_the_open_panel_closes_it() {
        assert!(!N.is_hovered(756.0, 800.0, true), "below even the settings sheet");
        assert!(!N.is_hovered(1200.0, 100.0, true), "beside the panel");
    }
}
