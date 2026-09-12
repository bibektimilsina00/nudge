//! The loop, and who is running it.
//!
//! One goal is several steps, and each needs a *fresh* screenshot: step two's
//! target usually lives inside a menu step one opens, so it exists in no earlier
//! frame. That single fact rules out planning everything up front and is why
//! this is a loop rather than one call.
//!
//! Three files because there are three answers to "who is driving":
//!
//! - [`session`] -- the step itself. Look, decide, hand back what to do.
//! - [`agent`] -- the record of an unattended run: its plan, its commands, its
//!   files, and the rule that only one may own the cursor.
//! - [`subagent`] -- a scoped job on a second agent with no screen, which is
//!   exactly why several of those can run at once and agents cannot.
pub mod agent;
pub mod session;
pub mod subagent;
