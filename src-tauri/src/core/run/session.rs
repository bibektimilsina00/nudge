//! The step loop. A goal is several nudges, and each one needs a *fresh* screenshot:
//! step 2's target usually lives inside a menu that step 1 opens, so it does not
//! exist in any earlier frame. That single fact rules out planning all the steps up
//! front, and it is the reason this is a loop rather than one call.
use crate::config::Config;
use crate::core::provider::{self, Ask, Provider, Step};
use crate::core::screen::capture::{self, Shot};
use crate::core::screen::{self, Look};
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

/// Longest to wait for Nudge's own voice to stop before reading the room.
///
/// Most sentences are done inside this, and the settle wait overlaps it, so it
/// usually costs nothing at all.
const HUSH_MAX: std::time::Duration = std::time::Duration::from_secs(4);

/// How long a look taken ahead of time still describes the screen.
///
/// Generous, because the thing it is racing is a transcription that normally
/// takes about a second -- and mean, because a screenshot is a claim about what
/// is in front of you right now. Past this we photograph again and pay for it.
const EARLY_MAX: std::time::Duration = std::time::Duration::from_secs(3);

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
    /// Where work happens, when it has been changed by voice.
    ///
    /// One setting was being asked to do two jobs: where generated files go,
    /// and which project Nudge may look at. Those want different answers -- a
    /// scratch folder for one, whatever you are working on for the other -- and
    /// switching meant editing a TOML file and restarting.
    ///
    /// Held here rather than written back to the config: a spoken "work in my
    /// nudge project" is for now, and a setting that silently rewrites itself is
    /// worse than one you have to change on purpose.
    moved: Mutex<Option<std::path::PathBuf>>,
    /// Where this turn's time is going. See [`crate::core::laps`].
    laps: Mutex<crate::core::laps::Laps>,
    /// A look taken before the turn that will use it. See [`Nudge::stash`].
    early: Mutex<Option<Look>>,
    /// Servers started once at launch and kept for the life of the process.
    ///
    /// A `OnceLock` because starting them is asynchronous and constructing this
    /// is not: `npx` may spend a minute fetching a server before it answers, and
    /// the hotkey has to work during that minute. Until it is filled there are no
    /// tools, which the prompt handles by saying nothing about tools.
    mcp: std::sync::OnceLock<crate::core::tools::mcp::Servers>,
    /// What has been allowed beyond the defaults.
    ///
    /// Owned here rather than managed separately by Tauri, because the menu bar
    /// flips it and the prompt reads it, and two copies of that would drift the
    /// first time somebody clicked -- leaving a tick saying one thing and a gate
    /// enforcing another, which is the worst possible failure for a thing whose
    /// entire job is being visible.
    pub reach: crate::core::reach::Reach,
}

impl Nudge {
    pub fn new(cfg: Config) -> Result<Self> {
        let provider = provider::build(&cfg)?;
        let reach = crate::core::reach::Reach::from_config(&cfg.reach);
        Ok(Self {
            reach,
            cfg,
            provider,
            session: Mutex::new(None),
            moved: Mutex::new(None),
            laps: Mutex::default(),
            early: Mutex::new(None),
            mcp: std::sync::OnceLock::new(),
        })
    }

    /// Start the configured MCP servers. Called once, off the startup path.
    pub async fn connect_tools(&self) {
        if self.cfg.mcp.is_empty() {
            return;
        }
        let servers = crate::core::tools::mcp::Servers::start(&self.cfg.mcp).await;
        // Losing the race means another caller already did it, which is fine and
        // is not worth an error -- the servers this one started are dropped, and
        // dropping them kills the children.
        let _ = self.mcp.set(servers);
    }

    /// What the connected servers can do, minus any that have been switched off.
    ///
    /// Filtered here rather than at the call, so a server switched off disappears
    /// from the prompt as well as from the dispatcher. Leaving it listed and
    /// refusing it later would be a worse kind of off: the model would keep
    /// reaching for a tool it is told it has.
    pub fn tools(&self) -> Vec<crate::core::tools::mcp::Tool> {
        self.mcp
            .get()
            .map(|s| self.reach.usable(s.tools()))
            .unwrap_or_default()
    }

    /// Every server that was configured, and how many tools it offers.
    ///
    /// For the menu bar, which has to say what a server can do before anybody can
    /// decide whether to let it. `None` means it has not connected yet, or did
    /// not start at all.
    pub fn tool_servers(&self) -> Vec<(String, Option<usize>)> {
        let connected = self.mcp.get();
        self.cfg
            .mcp
            .iter()
            .map(|spec| {
                let count = connected.map(|s| {
                    s.tools()
                        .iter()
                        .filter(|t| t.server == spec.name)
                        .count()
                });
                (spec.name.clone(), count)
            })
            .collect()
    }

    /// Run one, naming it the way the model does: `server/tool`.
    pub async fn run_tool(&self, tool: &str, args: &serde_json::Value) -> Result<String> {
        let Some(servers) = self.mcp.get() else {
            return Err(crate::error::Error::Config(
                "no tool servers are connected".into(),
            ));
        };
        let (server, name) = tool.split_once('/').ok_or_else(|| {
            crate::error::Error::Config(format!(
                "{tool:?} is not a tool name -- they look like server/tool"
            ))
        })?;
        // Checked here as well as by hiding it from the prompt, because a
        // history from before it was switched off still names it, and a model
        // repeating its last step must not get through.
        if !self.reach.server(server) {
            return Err(crate::error::Error::Config(format!(
                "the {server:?} tools have been switched off in the menu bar"
            )));
        }
        servers.call(server, name, args.clone()).await
    }

    /// Where commands run and files are written.
    pub fn workspace(&self) -> std::path::PathBuf {
        self.moved
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(|| self.cfg.workspace_dir())
    }

    /// Work somewhere else from now on.
    ///
    /// Refuses anything that is not a directory that exists. The boundary is
    /// still a boundary -- it has just moved, because the user said where to.
    pub fn move_to(&self, path: &str) -> Result<std::path::PathBuf> {
        let expanded = match path.trim().strip_prefix("~/") {
            Some(rest) => dirs::home_dir().unwrap_or_default().join(rest),
            None => std::path::PathBuf::from(path.trim()),
        };
        if !expanded.is_dir() {
            return Err(Error::Config(format!(
                "{} is not a folder I can find",
                expanded.display()
            )));
        }
        *self.moved.lock().unwrap() = Some(expanded.clone());
        Ok(expanded)
    }

    pub fn provider_name(&self) -> &'static str {
        self.provider.name()
    }

    pub fn begin(&self, goal: String) {
        self.open(goal, false);
    }

    /// Same session, run by the agent runtime rather than by the user's taps.
    ///
    /// `carried` is what the turn that handed over had already done. Starting
    /// empty threw that away: a foreground search found the answer, the handover
    /// wiped the session, and the agent searched again for the same thing. What
    /// happened on screen the agent can see for itself; what a command printed
    /// or a search returned exists nowhere else.
    pub fn begin_agent(&self, goal: String, carried: Vec<String>) {
        self.open(goal, true);
        if let Some(s) = self.session.lock().unwrap().as_mut() {
            s.done = carried;
        }
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
        crate::core::run::subagent::run(
            self.provider.as_ref(),
            &self.workspace(),
            task,
            &self.tools(),
            &self.reach.prompt(),
            self.reach.has(crate::core::reach::Grant::Shell),
            self.cfg.verify,
            act,
        )
        .await
    }

    /// The most recent thing recorded against this session.
    ///
    /// How a subagent's step reports its own output: `perform` writes what a
    /// command printed into the session, and this reads it straight back out,
    /// rather than every action growing a second way to return a value.
    /// What this session has done so far, for handing on.
    pub fn history(&self) -> Vec<String> {
        self.session
            .lock()
            .unwrap()
            .as_ref()
            .map(|s| s.done.clone())
            .unwrap_or_default()
    }

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
    /// A turn began. Safe to call twice -- see [`crate::core::laps::Laps::start`],
    /// because a spoken turn is started by the hotkey and passes through
    /// `advance`, which believes it is starting one too.
    pub fn clock_in(&self) {
        self.laps.lock().unwrap().start();
    }

    /// That stage of the current turn is over.
    pub fn mark(&self, stage: &'static str) {
        self.laps.lock().unwrap().mark(stage);
    }

    /// Keep a look taken ahead of the turn that will use it.
    ///
    /// The picture does not depend on the words. Photographing the screen while
    /// the transcription is still in flight costs nothing extra and is finished
    /// well before the answer comes back -- so the first turn arrives at the
    /// model with its screenshot already taken.
    ///
    /// Only the first turn may use it, and only briefly: see [`Nudge::step`].
    pub fn stash(&self, look: Look) {
        *self.early.lock().unwrap() = Some(look);
    }

    /// The look taken ahead of this turn, if it may still be used.
    ///
    /// Empties the stash either way. A picture belonging to a turn that never
    /// happened must not be sitting there waiting for the next one, and the
    /// cheapest way to guarantee that is to take it out before deciding.
    fn take_early(&self, first_turn: bool) -> Option<Look> {
        self.early
            .lock()
            .unwrap()
            .take()
            .filter(|look| first_turn && look.taken.elapsed() < EARLY_MAX)
    }

    pub async fn step(&self) -> Result<Option<Step>> {
        // Printed however this returns, refusals included. A turn that ended
        // early has still spent its clock, and leaving it running would hand its
        // start time to the next turn and blame it for the wait.
        struct Report<'a>(&'a Nudge);
        impl Drop for Report<'_> {
            fn drop(&mut self) {
                if let Some(line) = self.0.laps.lock().unwrap().line() {
                    eprintln!("timing: {line}");
                }
            }
        }
        let _report = Report(self);

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

        // Nothing has happened yet, so there is nothing to wait for.
        //
        // Both waits below guard against the *previous* action: one lets the
        // screen finish becoming whatever that action made it, the other lets our
        // own voice finish describing it. On the first turn of a session there is
        // no previous action, and we were paying for both anyway -- two screen
        // composites and 120ms, then up to a second of listening to ourselves.
        //
        // ponytail: `done` is also filled by tools that never touch the screen --
        // a `read`, a `fetch`, a command's output -- so the turn after one of
        // those still settles for nothing. Knowing which steps actually move the
        // screen is the upgrade. Worth doing when `still` shows up large in the
        // timing line on a tool-heavy turn, and not before: guessing "inert" for
        // a step that did move the screen photographs it mid-change, which is the
        // most expensive class of bug this project has had.
        let first_turn = done.is_empty();

        // Let the screen finish becoming whatever the last step made it.
        //
        // Every caller wants this and none of them should have to remember it,
        // so it lives here rather than in the two loops. The per-action waits in
        // `Settle` still set a floor -- some things take a moment to *begin* --
        // and this covers the rest, which is unbounded and unguessable.
        if !first_turn {
            capture::wait_until_still(SETTLE_MAX).await;
        }
        self.mark("still");

        // Let our own voice finish before listening.
        //
        // `facts.audio` reports unknown while Nudge is speaking, because its own
        // voice comes out of the same device. That was right, and it made the
        // fact useless: the agent speaks every step, a sentence runs two or three
        // seconds, and that is exactly the window in which the next turn gathers
        // its facts. So the one turn that most needs to know whether the video
        // started -- the turn straight after clicking it -- was always told
        // "cannot tell", and went back to guessing from a still frame. Which is
        // how it clicked play on something already playing and stopped it.
        //
        // Capped, because a long sentence should not hold up the work; past this
        // the answer stays unknown, which is at least honest.
        // Not on the first turn, where the only thing playing is the
        // acknowledgement we speak to cover this very wait. Two features working
        // against each other: one says a short line so the pause feels shorter,
        // the other made the pause a second longer by waiting for the line to
        // finish. Each was right when it was written and neither author could
        // have seen the other -- they are months and three files apart.
        //
        // It now plays over the model call instead, which is the gap it was
        // written to fill.
        //
        // This used to cost the audio fact on a first turn -- we are still
        // talking through it, so the device is busy by definition and the honest
        // answer is "cannot tell". It does not any more, because the look is now
        // taken before we speak at all. The fact is only lost when that early
        // look was missing or too old and we photograph again here, talking.
        if !first_turn {
            let quiet = std::time::Instant::now();
            while crate::core::voice::speech::is_playing() && quiet.elapsed() < HUSH_MAX {
                tokio::time::sleep(std::time::Duration::from_millis(80)).await;
            }
        }
        self.mark("hush");

        // Taken while the transcription was still in flight, if it was taken at
        // all and still describes this screen.
        let early = self.take_early(first_turn);

        let Look {
            facts,
            shot,
            controls,
            ..
        } = match early {
            Some(look) => look,
            // `look` checks before it photographs, so a refusal happens while the
            // only thing that exists is a window title.
            None => screen::look(&self.cfg).map_err(|e| {
                // A refusal ends the session. The answer is no, and asking again
                // from the same screen would get the same no.
                if matches!(e, Error::Blocked(_)) {
                    self.end();
                }
                e
            })?,
        };
        let shot: Shot = shot;
        self.mark("shot");
        let now = shot.fingerprint();
        // Only meaningful once something has been tried.
        let stalled = !done.is_empty() && capture::unchanged(&seen, &now);
        let ask = Ask {
            goal: &goal,
            done: &done,
            stalled,
            agent,
            facts,
            controls: &controls,
            tools: &self.tools(),
            reach: self.reach.prompt(),
            shell: self.reach.has(crate::core::reach::Grant::Shell),
            workspace: self.workspace().display().to_string(),
        };
        // When the system has already named exactly the control that was asked
        // for, there is nothing for a model to work out. Six seconds of a turn
        // goes to looking at a picture to discover what we have been told.
        //
        // First turn only, which is what makes it safe to have no second opinion:
        // it can fire once and then the model has the rest of the task. A fast
        // path that could fire repeatedly could also loop, and nothing here would
        // notice.
        let shortcut = first_turn
            .then(|| crate::core::screen::ax::obvious(&goal, &controls))
            .flatten();

        let step = match shortcut {
            Some(c) => {
                eprintln!("  obvious: {} {:?} -- not asking the model", c.role, c.label);
                Step::Point {
                    control: Some(c.label.clone()),
                    at: shot.to_image(crate::core::screen::capture::Point {
                        x: c.at.0,
                        y: c.at.1,
                    }),
                    say: format!("Clicking {}.", c.label),
                    act: crate::core::provider::Act::Click,
                }
            }
            None => self.provider.next_step(&shot, &ask).await?,
        };
        self.mark("brain");

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

#[cfg(test)]
mod tests {
    use super::*;

    fn nudge() -> Nudge {
        let mut cfg = Config::default();
        cfg.provider = "ollama".into(); // never called; just needs to build
        Nudge::new(cfg).unwrap()
    }

    /// Three ways an early picture stops being the truth, and the one way it is.
    #[test]
    fn an_early_look_is_used_once_and_only_while_it_is_still_true() {
        let n = nudge();
        let look = || Look {
            facts: Default::default(),
            controls: Vec::new(),
            shot: Shot {
                bytes: vec![1],
                sent: (1, 1),
                logical: (1.0, 1.0),
                origin: (0.0, 0.0),
            },
            taken: std::time::Instant::now(),
        };

        n.stash(look());
        assert!(n.take_early(true).is_some(), "a fresh look on a first turn is the whole point");
        assert!(n.take_early(true).is_none(), "the same picture cannot answer two turns");

        // Not the first turn: something has happened since, and this is a
        // photograph of before it happened.
        n.stash(look());
        assert!(n.take_early(false).is_none());
        assert!(
            n.take_early(true).is_none(),
            "a look we refused must not be left waiting for the next turn to accept it"
        );

        // Old enough that the screen may have moved on without us.
        let stale = std::time::Instant::now()
            .checked_sub(EARLY_MAX * 2)
            .expect("this machine has been up for a few seconds");
        n.stash(Look { taken: stale, ..look() });
        assert!(n.take_early(true).is_none());
    }

    /// The boundary still holds after it moves -- it is the user who says where,
    /// and a folder that does not exist is not somewhere to work.
    #[test]
    fn the_workspace_moves_only_to_somewhere_real() {
        let n = nudge();
        let home = dirs::home_dir().unwrap();
        assert_eq!(n.workspace(), home, "defaults to home when unset");

        let tmp = std::env::temp_dir();
        let moved = n.move_to(tmp.to_str().unwrap()).expect("a real folder");
        assert_eq!(moved, tmp);
        assert_eq!(n.workspace(), tmp, "and stays moved");

        assert!(n.move_to("/nowhere/at/all").is_err());
        assert_eq!(n.workspace(), tmp, "a refusal leaves it where it was");

        // `~` is how people say it out loud, so it has to be understood.
        assert!(n.move_to("~/").is_ok());
    }
}
