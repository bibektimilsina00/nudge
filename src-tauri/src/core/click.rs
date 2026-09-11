//! Synthetic clicks, and watching for real ones.
//!
//! Two jobs, same OS API. Posting a click is what "let Nudge do it" means; reading
//! the physical button is how the overlay knows you have actually done the thing it
//! pointed at, so the ring can stay up until then rather than guessing.
//!
//! Posting events needs Accessibility permission. Reading button state does not.
use crate::core::capture::Point;
use crate::error::{Error, Result};
use core_graphics::event::{
    CGEvent, CGEventTapLocation, CGEventType, CGMouseButton, EventField,
};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use core_graphics::geometry::CGPoint;

// CoreGraphics exposes the post-event permission separately from Accessibility's
// general API, and unlike a click it never silently no-ops.
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGPreflightPostEventAccess() -> bool;
    fn CGRequestPostEventAccess() -> bool;
    fn CGEventSourceButtonState(state: i32, button: u32) -> bool;
}

/// Has the user allowed us to post input events?
pub fn may_click() -> bool {
    unsafe { CGPreflightPostEventAccess() }
}

/// Ask for permission. Shows the system prompt once; afterwards the user has to
/// grant it by hand in Privacy & Security, so callers must cope with a `false`.
pub fn request_click_permission() -> bool {
    unsafe { CGRequestPostEventAccess() }
}

/// Is the physical left button down right now?
///
/// `CombinedSessionState` reports the real hardware state -- including clicks we
/// posted ourselves, which is why the watcher ignores presses it caused.
pub fn left_button_down() -> bool {
    const LEFT: u32 = 0;
    unsafe { CGEventSourceButtonState(CGEventSourceStateID::CombinedSessionState as i32, LEFT) }
}

/// Click at a point in logical screen coordinates -- the same space the overlay
/// draws in, so a ring's position passes straight through.
///
/// Every failure here is reported rather than swallowed. Posting an event that
/// goes nowhere looks exactly like a working click that the app ignored, and
/// "nothing happened" is the least debuggable bug there is.
/// Put the pointer somewhere without pressing anything. This is what "hover" means
/// in auto mode, and inside an open menu it is the only safe thing to do.
pub fn move_to(at: Point) -> Result<()> {
    click_inner(at, 0)
}

pub fn click(at: Point, times: u8) -> Result<()> {
    click_inner(at, times.max(1))
}

fn click_inner(at: Point, times: u8) -> Result<()> {
    if !may_click() {
        return Err(Error::Click(
            "Clicking needs Accessibility. System Settings > Privacy & Security >              Accessibility, add Nudge, then quit and reopen it."
                .into(),
        ));
    }
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| Error::Click("couldn't create an event source".into()))?;
    let point = CGPoint::new(at.x, at.y);

    let post = |kind: CGEventType, click_state: i64| -> Result<()> {
        let event = CGEvent::new_mouse_event(source.clone(), kind, point, CGMouseButton::Left)
            .map_err(|_| Error::Click("couldn't build a click event".into()))?;
        if click_state > 0 {
            // Without this the event is a button state change, not a click, and
            // apps read the pair as the start and end of a drag. It is the single
            // difference between "the cursor twitched" and "the button was pressed".
            event.set_integer_value_field(EventField::MOUSE_EVENT_CLICK_STATE, click_state);
        }
        event.post(CGEventTapLocation::HID);
        Ok(())
    };

    // Move first. A button press arriving at a point the cursor was never at skips
    // every hover and tracking update, and plenty of controls only arm themselves
    // once the pointer is actually over them.
    post(CGEventType::MouseMoved, 0)?;
    std::thread::sleep(SETTLE);

    // The click *state* is what makes a double click double: the second press
    // carries state 2, and that is the field every app actually reads. Two
    // separate state-1 clicks in quick succession are two single clicks -- which
    // in Finder means "select, then select again", never "open".
    for n in 1..=times {
        post(CGEventType::LeftMouseDown, n as i64)?;
        // Real fingers are not instantaneous, and a zero-length press is dropped
        // or coalesced by some toolkits.
        std::thread::sleep(PRESS);
        post(CGEventType::LeftMouseUp, n as i64)?;
        if n < times {
            std::thread::sleep(GAP);
        }
    }
    Ok(())
}

/// Long enough for hover tracking to catch up, short enough to feel instant.
const SETTLE: std::time::Duration = std::time::Duration::from_millis(24);
/// How long the button stays down. Roughly a brisk human click.
const PRESS: std::time::Duration = std::time::Duration::from_millis(40);
/// Between the clicks of a double. Must stay well inside the system double-click
/// interval, whose default is 500ms and which the user can shorten.
const GAP: std::time::Duration = std::time::Duration::from_millis(70);
