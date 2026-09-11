//! The step loop. A goal is several nudges, and each one needs a *fresh* screenshot:
//! step 2's target usually lives inside a menu that step 1 opens, so it does not
//! exist in any earlier frame. That single fact rules out planning all the steps up
//! front, and it is the reason this is a loop rather than one call.
use crate::core::capture::{self, Shot};
use crate::config::Config;
use crate::error::{Error, Result};
use crate::core::privacy;
use crate::core::provider::{self, Ask, Provider, Step};
use std::sync::Mutex;

/// Auto mode advances itself, so a model that keeps finding "one more step"
/// would click forever. Guide mode is capped by the user's patience; this is the
/// equivalent for the machine.
const MAX_STEPS: usize = 12;

pub struct Session {
    pub goal: String,
    /// What we have already told the user, fed back so the model advances.
    pub done: Vec<String>,
    /// The screen as it looked after the previous step.
    pub seen: Vec<u8>,
}

pub struct Nudge {
    pub cfg: Config,
    provider: Box<dyn Provider>,
    session: Mutex<Option<Session>>,
}

impl Nudge {
    pub fn new(cfg: Config) -> Result<Self> {
        let provider = provider::build(&cfg)?;
        Ok(Self { cfg, provider, session: Mutex::new(None) })
    }

    pub fn provider_name(&self) -> &'static str {
        self.provider.name()
    }

    pub fn begin(&self, goal: String) {
        *self.session.lock().unwrap() =
            Some(Session { goal, done: Vec::new(), seen: Vec::new() });
    }

    pub fn end(&self) {
        *self.session.lock().unwrap() = None;
    }

    pub fn active(&self) -> bool {
        self.session.lock().unwrap().is_some()
    }

    /// One nudge. Returns `None` when nothing is in flight, and a `Step` whose
    /// point is already in overlay coordinates -- callers never see image space.
    ///
    /// `logical` is the screen in points; see `Shot::to_overlay`.
    pub async fn step(&self, logical: (f64, f64)) -> Result<Option<Step>> {
        // Snapshot and release: the lock must not be held across the await, and a
        // tokio Mutex would be a heavier fix than simply not needing one.
        let Some((goal, done, seen)) = self
            .session
            .lock()
            .unwrap()
            .as_ref()
            .map(|s| (s.goal.clone(), s.done.clone(), s.seen.clone()))
        else {
            return Ok(None);
        };

        if done.len() >= MAX_STEPS {
            self.end();
            return Ok(Some(Step::Unsure {
                say: format!("Stopping after {MAX_STEPS} steps -- this isn't converging."),
            }));
        }

        // Before the capture. There is no un-sending a screenshot, so the check
        // has to happen while the only thing that exists is a window title.
        if let Some((app, title)) = privacy::frontmost() {
            if let Some(reason) = privacy::blocked_by(&self.cfg, &app, &title) {
                self.end();
                return Err(Error::Blocked(reason));
            }
        }

        let shot: Shot = capture::grab(self.cfg.max_edge, logical)?;
        let now = shot.fingerprint();
        // Only meaningful once something has been tried.
        let stalled = !done.is_empty() && capture::unchanged(&seen, &now);
        let ask = Ask { goal: &goal, done: &done, stalled };
        let step = self.provider.next_step(&shot, &ask).await?;

        let mut guard = self.session.lock().unwrap();
        let Some(session) = guard.as_mut() else {
            return Ok(None); // cancelled while the model was thinking
        };
        // Only a real instruction counts as progress. Recording "I can't see it" as a
        // completed step would make the next call think the user had done it.
        if matches!(
            step,
            Step::Point { .. } | Step::Launch { .. } | Step::Open { .. } | Step::Type { .. }
        ) {
            session.done.push(step.say().to_string());
        }
        session.seen = now;

        Ok(Some(step.map_point(|p| shot.to_overlay(p))))
    }
}
