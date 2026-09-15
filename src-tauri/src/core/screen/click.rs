//! Synthetic clicks, and watching for real ones.
//!
//! Two jobs, same OS API. Posting a click is what "let Nudge do it" means; reading
//! the physical button is how the overlay knows you have actually done the thing it
//! pointed at, so the ring can stay up until then rather than guessing.
//!
//! Posting events needs Accessibility permission. Reading button state does not.
use crate::core::screen::capture::Point;
use crate::error::{Error, Result};
use core_graphics::event::{CGEvent, CGEventTapLocation, CGEventType, CGMouseButton, EventField};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use core_graphics::geometry::CGPoint;

// CoreGraphics exposes the post-event permission separately from Accessibility's
// general API, and unlike a click it never silently no-ops.
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGPreflightPostEventAccess() -> bool;
    fn CGRequestPostEventAccess() -> bool;
    fn CGEventSourceButtonState(state: i32, button: u32) -> bool;
    fn CGEventSourceSecondsSinceLastEventType(state: i32, event: u32) -> f64;
    fn CGEventSourceFlagsState(state: i32) -> u64;
    fn CGEventSourceKeyState(state: i32, key: u16) -> bool;
}

// CGEventType values we care about.
const KEY_DOWN: u32 = 10;
const MOUSE_MOVED: u32 = 5;
const LEFT_DOWN: u32 = 1;
const RIGHT_DOWN: u32 = 3;
const SCROLL: u32 = 22;

fn since(event: u32) -> f64 {
    const COMBINED: i32 = 0;
    unsafe { CGEventSourceSecondsSinceLastEventType(COMBINED, event) }
}

/// Where the pointer is, in logical screen points.
///
/// The app gets this from Tauri; the bench harness has no Tauri, and needs it to
/// record where a human clicked when authoring a test case.
pub fn cursor() -> Option<Point> {
    let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState).ok()?;
    let p = CGEvent::new(source).ok()?.location();
    Some(Point { x: p.x, y: p.y })
}

/// Is Escape held right now?
///
/// Polled, not listened for. Escape used to be a `keydown` handler in the
/// overlay webview, which stopped working the moment the overlay became
/// permanently non-focusable -- a window that never takes focus never sees a
/// key. Reading the hardware state needs no focus and no permission.
pub fn escape_down() -> bool {
    const COMBINED: i32 = 0;
    /// `kVK_Escape`.
    const ESCAPE: u16 = 53;
    unsafe { CGEventSourceKeyState(COMBINED, ESCAPE) }
}

/// Is Control -- and nothing but Control -- held right now?
///
/// Push-to-talk is a bare modifier, which no global-shortcut API can register:
/// they all want a key code. Reading the flags needs no permission and the
/// pointer loop already runs sixty times a second, so it costs one getter.
///
/// "Nothing but" matters. Control is half of a dozen real shortcuts -- ctrl+arrow
/// switches Spaces, ctrl+click is a right click -- and a hold that fired on those
/// too would record constantly. Requiring it alone leaves the combinations to the
/// OS. Device-dependent bits are masked off: macOS sets both the generic Control
/// bit and a left/right one, and we do not care which key it was.
pub fn control_alone() -> bool {
    const COMBINED: i32 = 0;
    // CGEventFlags: the modifiers, minus the device-side left/right bits.
    const CONTROL: u64 = 0x0004_0000;
    const MODIFIERS: u64 = 0x00FF_0000;
    let flags = unsafe { CGEventSourceFlagsState(COMBINED) } & MODIFIERS;
    flags == CONTROL
}

/// Has macOS hidden the pointer because the user is typing?
///
/// There is no public way to ask whether the cursor is visible.
/// `CGCursorIsDrawnInFramebuffer` sounds like it and is not -- measured, it
/// reports whether the cursor is composited in software and never changed once
/// across a session. The private SkyLight calls that do answer are not something
/// to ship in a loop that runs sixty times a second.
///
/// But the rule macOS follows is simple and observable from public events: typing
/// hides the pointer, moving it brings it back. So whichever happened *more
/// recently* is the answer -- no thresholds, no timers, and it stays hidden for as
/// long as the real cursor does rather than for some duration we invented.
pub fn pointer_hidden() -> bool {
    let typed = since(KEY_DOWN);
    let pointed = since(MOUSE_MOVED)
        .min(since(LEFT_DOWN))
        .min(since(RIGHT_DOWN))
        .min(since(SCROLL));
    typed < pointed
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

/// The pointer, out of sight for as long as this lives.
///
/// The cursor has to travel to the control and back, and that journey is the
/// only part of a click anyone sees. Hidden, what is left is the hand in the
/// overlay arriving and pressing -- which is the thing actually happening.
///
/// A guard rather than two calls, because the failure here is severe and silent:
/// a pointer hidden and never restored is invisible until something else on the
/// machine happens to show it, and the usual response to that is a forced
/// restart. `Drop` runs on the way out however this returns, including while
/// unwinding from a panic.
///
/// `CGDisplayHideCursor` is also counted rather than boolean, so an unbalanced
/// hide leaks permanently -- which is why [`show_the_pointer`] runs at startup.
struct Hidden;

impl Hidden {
    fn now() -> Option<Self> {
        use core_graphics::display::CGDisplay;
        CGDisplay::main().hide_cursor().ok().map(|_| Hidden)
    }
}

impl Drop for Hidden {
    fn drop(&mut self) {
        use core_graphics::display::CGDisplay;
        let _ = CGDisplay::main().show_cursor();
    }
}

/// Undo any hide left over from a previous run.
///
/// The counter survives the process that incremented it. If Nudge was killed
/// outright -- not panicked, killed -- its guard never ran, and the pointer is
/// still invisible on a machine where nothing else is going to fix it. Called at
/// startup, several times, because the count is not readable and over-showing is
/// harmless.
pub fn show_the_pointer() {
    use core_graphics::display::CGDisplay;
    for _ in 0..4 {
        let _ = CGDisplay::main().show_cursor();
    }
}

/// When Nudge last clicked something itself.
///
/// The same problem as `OUR_ESCAPE` one file over, and the same fix. A click
/// anywhere outside the agent card closes it -- and an agent spends its whole
/// life clicking things outside the agent card, so its own work closed its own
/// card, over and over, while somebody was reading it.
///
/// Zero means never.
static OUR_CLICK: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn since_start_ms() -> u64 {
    static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    START
        .get_or_init(std::time::Instant::now)
        .elapsed()
        .as_millis() as u64
}

/// Was the click that just landed one of ours?
///
/// Shorter than the Escape window: clicks come in streams and a long window
/// would swallow a real one arriving straight after. Long enough that a poll at
/// sixty hertz cannot miss the one we made.
pub fn we_clicked() -> bool {
    const WINDOW_MS: u64 = 400;
    let last = OUR_CLICK.load(std::sync::atomic::Ordering::Relaxed);
    last != 0 && since_start_ms().saturating_sub(last) < WINDOW_MS
}

pub fn click(at: Point, times: u8) -> Result<()> {
    OUR_CLICK.store(
        since_start_ms().max(1),
        std::sync::atomic::Ordering::Relaxed,
    );
    // Out of sight for the trip. Dropped at the end of this function, whatever
    // happens in the middle.
    let _hidden = Hidden::now();

    // Where the user left it, before we borrow it.
    let theirs = cursor();
    let result = click_inner(at, times.max(1));

    // And hand it straight back.
    //
    // The pointer belongs to the person sitting there. It has to move, because a
    // click is delivered to whatever is under it and no amount of asking politely
    // changes that -- `AXPress` works only for menus, and posting the event to a
    // single process leaves the cursor alone but does not land the click. Both
    // measured before settling for this.
    //
    // So it moves, and it comes back. What is left is a flicker instead of a
    // pointer abandoned on the far side of the screen, halfway through whatever
    // its owner was doing.
    if let Some(home) = theirs {
        // Warped, not posted. A warp moves the cursor without generating an
        // event, so nothing downstream sees a second click or a stray drag.
        unsafe { core_graphics::display::CGWarpMouseCursorPosition(CGPoint::new(home.x, home.y)) };
        // Otherwise the next real movement snaps back to where the click was:
        // the hardware and the cursor stay associated, and warping alone does not
        // tell the system the mouse is somewhere new.
        unsafe { core_graphics::display::CGAssociateMouseAndMouseCursorPosition(1) };
    }
    result
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
