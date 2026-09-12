//! The step loop. A goal is several nudges, and each one needs a *fresh* screenshot:
//! step 2's target usually lives inside a menu that step 1 opens, so it does not
//! exist in any earlier frame. That single fact rules out planning all the steps up
//! front, and it is the reason this is a loop rather than one call.
use crate::config::Config;
use crate::core::provider::{self, Ask, Provider, Step};
use crate::core::screen::capture::{self, Shot};
use crate::core::screen::privacy;
use crate::error::{Error, Result};
use std::sync::Mutex;

/// Auto mode advances itself, so a model that keeps finding "one more step"
/// would click forever. Guide mode is capped by the user's patience; this is the
/// equivalent for the machine.
/// Guide mode is bounded by the user's patience, and twelve nudges is already
/// more than anyone will sit through.
const MAX_STEPS: usize = 12;

/// Longest to wait for the screen to stop moving before photographing it anyway.
///
/// Generous, because the thing it is waiting for is a web application booting,
/// and stingy compared with getting the answer wrong: a capture taken early
/// costs a whole turn and sends the model down the wrong path.
const SETTLE_MAX: std::time::Duration = std::time::Duration::from_secs(6);

pub struct Session {
    pub goal: String,
    /// What we have already told the user, fed back so the model advances.
    pub done: Vec<String>,
    /// The screen as it looked after the previous step.
    pub seen: Vec<u8>,
    /// Nudge is carrying this out itself, unwatched. Changes both the budget and
    /// what the model is allowed to answer -- see `provider::prompt`.
    pub agent: bool,
}

pub struct Nudge {
    pub cfg: Config,
    provider: Box<dyn Provider>,
    session: Mutex<Option<Session>>,
}

impl Nudge {
    pub fn new(cfg: Config) -> Result<Self> {
        let provider = provider::build(&cfg)?;
        Ok(Self {
            cfg,
            provider,
            session: Mutex::new(None),
        })
    }

    pub fn provider_name(&self) -> &'static str {
        self.provider.name()
    }

    pub fn begin(&self, goal: String) {
        self.open(goal, false);
    }

    /// Same session, run by the agent runtime rather than by the user's taps.
    pub fn begin_agent(&self, goal: String) {
        self.open(goal, true);
    }

    fn open(&self, goal: String, agent: bool) {
        *self.session.lock().unwrap() = Some(Session {
            goal,
            done: Vec::new(),
            seen: Vec::new(),
            agent,
        });
    }

    /// Ask the provider to search. Here rather than in the app layer because the
    /// provider is owned here and nothing else should reach past it.
    pub async fn search(&self, query: &str) -> Result<String> {
        self.provider.search(query).await
    }

    /// Run a scoped task on a second agent and return what it found.
    pub async fn task<F, Fut>(
        &self,
        task: &str,
        act: F,
    ) -> Result<crate::core::run::subagent::Found>
    where
        F: FnMut(Step) -> Fut,
        Fut: std::future::Future<Output = Result<String>>,
    {
        crate::core::run::subagent::run(self.provider.as_ref(), &self.cfg, task, act).await
    }

    /// The most recent thing recorded against this session.
    ///
    /// How a subagent's step reports its own output: `perform` writes what a
    /// command printed into the session, and this reads it straight back out,
    /// rather than every action growing a second way to return a value.
    pub fn last_note(&self) -> String {
        self.session
            .lock()
            .unwrap()
            .as_ref()
            .and_then(|s| s.done.last().cloned())
            .unwrap_or_default()
    }

    pub fn end(&self) {
        *self.session.lock().unwrap() = None;
    }

    /// What the current session is working towards, if anything. The agent
    /// runtime needs it to restart the loop under its own control.
    pub fn goal(&self) -> String {
        self.session
            .lock()
            .unwrap()
            .as_ref()
            .map(|s| s.goal.clone())
            .unwrap_or_default()
    }

    /// Fold an answer into the context, so the next turn knows what was said.
    pub fn note(&self, line: String) {
        if let Some(s) = self.session.lock().unwrap().as_mut() {
            s.done.push(line);
        }
    }

    pub fn active(&self) -> bool {
        self.session.lock().unwrap().is_some()
    }

    /// One nudge. Returns `None` when nothing is in flight, and a `Step` whose
    /// point is already in screen coordinates -- callers never see image space.
    ///
    /// Points come back in *global* screen coordinates -- see `Shot::to_global`,
    /// which is what makes a second display work. Which display gets captured is
    /// decided per call by where the pointer is, not fixed at startup.
    pub async fn step(&self) -> Result<Option<Step>> {
        // Snapshot and release: the lock must not be held across the await, and a
        // tokio Mutex would be a heavier fix than simply not needing one.
        let Some((goal, done, seen, agent)) = self
            .session
            .lock()
            .unwrap()
            .as_ref()
            .map(|s| (s.goal.clone(), s.done.clone(), s.seen.clone(), s.agent))
        else {
            return Ok(None);
        };

        // The agent has its own, larger budget, enforced by the runtime that can
        // actually show it. Applying the guide-mode cap here cut every agent off
        // at twelve turns regardless.
        if !agent && done.len() >= MAX_STEPS {
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

        // Let the screen finish becoming whatever the last step made it.
        //
        // Every caller wants this and none of them should have to remember it,
        // so it lives here rather than in the two loops. The per-action waits in
        // `Settle` still set a floor -- some things take a moment to *begin* --
        // and this covers the rest, which is unbounded and unguessable.
        capture::wait_until_still(SETTLE_MAX).await;

        // Before the capture, so the two describe the same moment.
        let facts = crate::core::screen::facts::gather();
        let shot: Shot = capture::grab(self.cfg.max_edge)?;
        let now = shot.fingerprint();
        // Only meaningful once something has been tried.
        let stalled = !done.is_empty() && capture::unchanged(&seen, &now);
        let ask = Ask {
            goal: &goal,
            done: &done,
            stalled,
            agent,
            facts,
        };
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
            session.done.push(step.recap());
        }
        session.seen = now;

        Ok(Some(step.map_point(|p| shot.to_global(p))))
    }
}
