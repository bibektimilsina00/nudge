//! Watching the user: the hotkey they hold, and the pointer they move.
//!
//! Both are polled rather than subscribed to, and for the same reason -- the
//! things worth knowing are not delivered as events. A bare modifier cannot be
//! registered as a shortcut, and a click-through window never receives
//! mousemove, so the OS has to be asked sixty times a second instead.
pub mod cursor;
pub mod hotkey;
pub mod inject;
