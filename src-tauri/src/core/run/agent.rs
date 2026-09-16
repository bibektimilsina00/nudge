//! Tasks Nudge carries out on its own.
//!
//! An agent is the ordinary session loop with three things added: it drives
//! itself instead of waiting for the user to click, it has a budget, and it can
//! be stopped. Everything else -- capture, grounding, the privacy guard, the
//! settle waits, stall detection -- is reused exactly as the foreground path
//! uses it, because an agent doing those differently would be a second
//! implementation to keep in step.
//!
//! While one runs it owns the real cursor and keyboard. That is what "do it for
//! me" means, and it is why `stop` is checked between every step rather than at
//! the end: a stop that waits for the current model call is not a stop.
use crate::core::provider::Step;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Long enough for a real task -- open a browser, search, click a result, play --
/// and short enough that a model which never finishes gives up rather than
/// clicking forever. The foreground loop caps at 12 for the same reason; an
/// agent needs more room because nobody is watching it.
pub const MAX_STEPS: usize = 40;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum State {
    Running,
    /// Blocked on the user. `question` is shown with a field to answer it, and
    /// `choices` -- when there are any -- as buttons above it. Carried here
    /// rather than fetched by the card, so the question and the answers to it
    /// arrive together and cannot be a frame out of step.
    Waiting {
        question: String,
        #[serde(default)]
        choices: Vec<String>,
    },
    Done,
    Failed {
        why: String,
    },
    Stopped,
    /// The process ended while this was still going.
    ///
    /// Its own state rather than `Stopped`, because they are different facts and
    /// only one of them is a decision. Somebody who pressed Escape meant it;
    /// somebody whose laptop went to sleep did not, and offering to carry on is
    /// right for exactly one of those.
    ///
    /// Never written by the runtime. A live process cannot know it is about to
    /// die, so this is applied on the way *back in*: anything read from the
    /// ledger that claims to be running was running when the process ended, and
    /// saying otherwise would be the lie this used to avoid by dropping them.
    Interrupted,
}

/// A command an agent ran, and what came back.
///
/// Kept so it can be shown. An agent that runs things on your machine and leaves
/// no record of what it ran is not one anybody should install -- and the record
/// has to survive the run, because the moment you want it is after something
/// surprising happened.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Ran {
    pub command: String,
    pub output: String,
}

/// One line of an agent's plan.
///
/// Written by the agent and replaced wholesale each time, so there are no ids to
/// get wrong and no way for the list the model believes in to drift from the one
/// on screen.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Todo {
    pub text: String,
    pub status: Doing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Doing {
    Pending,
    Active,
    Done,
}

/// Milliseconds since the epoch. A wall clock, because these outlive the process.
fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Most a plan may hold. Past this it is not a plan, it is a model narrating.
pub const MAX_TODOS: usize = 20;

/// A file an agent produced.
///
/// Kept apart from the command log because it is a different kind of thing: the
/// log is what happened, an artifact is what you have now. Asked to make a
/// landing page, what you want afterwards is the page -- not a transcript of the
/// making of it.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Made {
    /// Absolute, so it can be opened. The UI shows only the last component.
    pub path: String,
    /// Which step wrote it, so "two steps ago" can be said rather than "at some
    /// point". Defaults for rows written before this was recorded.
    #[serde(default)]
    pub step: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Agent {
    pub id: u64,
    /// When it started, as milliseconds since the epoch.
    ///
    /// Needed to say *when* rather than only *what*: a list of past runs with no
    /// dates on it is a list nobody can find anything in. A wall clock rather
    /// than an `Instant`, because this outlives the process that made it.
    #[serde(default)]
    pub started: u64,
    pub goal: String,
    pub title: String,
    /// The latest step's sentence -- what the card shows while it works.
    pub status: String,
    pub step: usize,
    #[serde(flatten)]
    pub state: State,
    pub history: Vec<String>,
    /// Long enough to be worth its own window. Short tasks run in the notch.
    pub background: bool,
    /// Every command this agent ran, in order, with its output.
    pub ran: Vec<Ran>,
    /// Files it created or replaced, newest last, one entry per path.
    pub made: Vec<Made>,
    /// Coding agents this run has already handed work to.
    ///
    /// So a second job goes to the same conversation rather than to a stranger.
    /// Per run rather than per session: a new task is a new subject, and
    /// carrying the last one's context into it is how an agent ends up
    /// confidently answering a question nobody asked.
    #[serde(default)]
    pub handed: Vec<String>,
    /// What it plans to do, if it said. Empty for work short enough not to need
    /// a plan, which is most of it.
    pub plan: Vec<Todo>,
}

impl Agent {
    /// How far along, 0.0 to 1.0.
    ///
    /// Steps against the budget, not an estimate of the task: there is no way to
    /// know how many clicks "play a song" takes until it is done, and a bar that
    /// invents a number is a lie that happens to move. It therefore creeps rather
    /// than predicts, and never goes backwards.
    pub fn progress(&self) -> f32 {
        if let State::Done = self.state {
            return 1.0;
        }
        // A plan measures the work; the step count only measures the budget. A
        // bar that creeps toward forty turns tells you how patient the runtime
        // is, not how close the task is.
        if !self.plan.is_empty() {
            let done = self.plan.iter().filter(|t| t.status == Doing::Done).count();
            return (done as f32 / self.plan.len() as f32).clamp(0.0, 0.97);
        }
        (self.step as f32 / MAX_STEPS as f32).clamp(0.0, 0.97)
    }

    pub fn finished(&self) -> bool {
        matches!(
            self.state,
            State::Done | State::Failed { .. } | State::Stopped | State::Interrupted
        )
    }

    /// Ended without reaching an answer, and not because anybody said so.
    pub fn interrupted(&self) -> bool {
        matches!(self.state, State::Interrupted)
    }
}

/// Every agent this session has seen, running or not.
pub struct Agents {
    /// The run a step is being carried out for, right now. Zero for none.
    ///
    /// Explicit rather than inferred. Whoever records what happened used to look
    /// for an agent that had not finished, which is wrong twice: with two runs
    /// it picks an arbitrary one, and an agent stopped mid-step is already
    /// finished by the time its tool call returns -- so the one record that
    /// most wants attributing, a refusal, is the one that loses it.
    doing: std::sync::atomic::AtomicU64,
    items: Mutex<Vec<Agent>>,
    next_id: AtomicU64,
    /// Set to stop whichever agent is running. One at a time, so one flag.
    abort: AtomicBool,
    /// Where finished runs are written, or nowhere.
    ///
    /// `None` in tests, and that is the whole reason it exists: the tests below
    /// finish agents, finishing an agent writes the history, and for one build
    /// they wrote runs called "t" into the real file in somebody's home
    /// directory. A test that can reach a person's data is a test that will.
    ledger: Option<PathBuf>,
}

impl Default for Agents {
    fn default() -> Self {
        Agents {
            doing: AtomicU64::default(),
            items: Mutex::default(),
            next_id: AtomicU64::default(),
            abort: AtomicBool::default(),
            ledger: dirs::home_dir().map(|d| d.join(".config/nudge/history.json")),
        }
    }
}

impl Agents {
    /// The running agent, if any. One at a time -- see [`Agents::start`].
    pub fn current(&self) -> Option<Agent> {
        self.items
            .lock()
            .unwrap()
            .iter()
            .find(|a| !a.finished())
            .cloned()
    }

    /// Start an agent, unless one is already running.
    ///
    /// Returns `None` when refused. Two agents mean two sets of clicks going to
    /// one cursor, and they do not take turns -- they interleave. It has already
    /// cost a real message: two agents drove one WhatsApp chat, one clicked send
    /// and the other clicked the same spot a moment later, by which time the
    /// button had become the microphone. A voice note went to a real person.
    ///
    /// Refusing here rather than at each call site, because the rule is a
    /// property of owning the cursor, not of any one way in.
    pub fn start(
        &self,
        goal: String,
        title: String,
        status: String,
        background: bool,
    ) -> Option<u64> {
        if self.running() {
            return None;
        }
        Some(self.start_now(goal, title, status, background))
    }

    /// Put a worked example on the card, so it can be looked at and designed.
    ///
    /// The card is normally invisible: it appears only while an agent is running,
    /// which makes changing how it looks a matter of starting a real task, doing
    /// the work fast enough to see it, and losing it before the change is right.
    ///
    /// This seeds a **real agent in the real list**, so what gets styled is the
    /// rendering path that actually runs -- not a preview mode that drifts from
    /// it. Every part is filled: a plan part-done, files made, commands run, so
    /// nothing is left unexercised because it happened to be empty.
    ///
    /// Behind `NUDGE_CARD` and off otherwise, for the reason `NUDGE_SAY` is: an
    /// environment variable lasts exactly as long as the process somebody started
    /// on purpose, where a setting is turned on once and forgotten.
    ///
    /// **While it is on, no real agent can start.** One at a time is a rule about
    /// the mouse, and this one never finishes, so it holds the slot. That is not
    /// worked around: a special case in `running` to ignore this agent would put a
    /// branch in the real path to serve a styling mode, and the real path is where
    /// two agents fighting over one cursor gets decided. Quit and start again
    /// without the variable.
    pub fn demonstrate(&self) -> u64 {
        // Four, because four tiles is what the corner has to survive and one tile
        // proves nothing about how they stack.
        //
        // **Four cannot actually happen today.** `start` refuses a second agent
        // while one is live, because there is one cursor and two agents driving it
        // sent a voice note to a real person once. These are seeded past that
        // check on purpose: the layout should be right before the limit is ever
        // lifted, and finding out then would mean finding out in front of somebody.
        //
        // Every one of them is filled in. The first version filled only the last,
        // so opening any of the others showed a card with a sentence and nothing
        // else -- which is not what the card looks like, and is the wrong thing to
        // design against.
        let mut last = 0;
        for (title, status, waiting) in [
            ("Renaming photos", "Reading the dates off 240 files.", false),
            (
                "Drafting the email",
                "Which address should this go to?",
                true,
            ),
            ("Running the tests", "Waiting on the build.", false),
            (
                "Tidying downloads",
                "Moving the last few files into place.",
                false,
            ),
        ] {
            let id = self.start_now(
                format!("demonstration: {title}"),
                title.into(),
                status.into(),
                true,
            );
            self.edit(id, |a| {
                a.step = 4;
                if waiting {
                    a.state = State::Waiting {
                        question: "Which address should this go to?".into(),
                        choices: Vec::new(),
                    };
                }
                a.plan = vec![
                    Todo { text: "Look at what is there".into(), status: Doing::Done },
                    Todo { text: "Work out what goes where".into(), status: Doing::Done },
                    Todo { text: "Move them into place".into(), status: Doing::Active },
                    Todo { text: "Report what changed".into(), status: Doing::Pending },
                ];
                a.made = vec![
                    Made { path: "/Users/you/Downloads/Invoices/march.pdf".into(), step: 2 },
                    Made { path: "/Users/you/Downloads/report.html".into(), step: 2 },
                ];
                a.ran = vec![
                    Ran {
                        command: "ls -la ~/Downloads".into(),
                        output: "total 248\ndrwxr-xr-x  14 you  staff  448 Sep 15 12:04 .\n-rw-r--r--   1 you  staff  81kB march.pdf".into(),
                    },
                    Ran {
                        command: "file ~/Downloads/*".into(),
                        output: "march.pdf: PDF document, version 1.7\nnotes.png: PNG image data, 1284 x 812".into(),
                    },
                ];
                a.history = vec![
                    "Looked at the Downloads folder".into(),
                    "Sorted 12 files by kind".into(),
                    "Made a folder called Invoices".into(),
                    "Moved march.pdf into Invoices".into(),
                ];
            });
            last = id;
        }
        last
    }

    fn start_now(&self, goal: String, title: String, status: String, background: bool) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed) + 1;
        self.abort.store(false, Ordering::Relaxed);
        self.items.lock().unwrap().push(Agent {
            id,
            started: now_ms(),
            goal,
            title,
            status,
            step: 0,
            state: State::Running,
            history: Vec::new(),
            background,
            ran: Vec::new(),
            made: Vec::new(),
            handed: Vec::new(),
            plan: Vec::new(),
        });
        id
    }

    pub fn list(&self) -> Vec<Agent> {
        self.items.lock().unwrap().clone()
    }

    /// Is anything running that earns a window? The card is for background work
    /// only; a foreground task reports through the notch like every other state.
    pub fn showing(&self) -> bool {
        self.items
            .lock()
            .unwrap()
            .iter()
            .any(|a| a.background && !a.finished())
    }

    /// Mark which run is acting, for the record. Zero clears it.
    pub fn now_doing(&self, id: u64) {
        self.doing.store(id, Ordering::Relaxed);
    }

    /// The run a step is being carried out for, if any.
    pub fn doing(&self) -> Option<u64> {
        match self.doing.load(Ordering::Relaxed) {
            0 => None,
            id => Some(id),
        }
    }

    pub fn running(&self) -> bool {
        self.items.lock().unwrap().iter().any(|a| !a.finished())
    }

    /// How long an interrupted run keeps offering to carry on.
    ///
    /// Long enough to cover quitting and reopening, a crash, and a laptop that
    /// slept. Not long enough to still be asking about Tuesday -- an offer that
    /// arrives on every launch until dismissed is not an offer, it is nagging,
    /// and the run is still in the Agents tab with everything it did.
    ///
    /// **The card mirrors this number** -- see the `live` filter in `Agent.tsx`.
    /// Two places, because one is TypeScript, and both say so.
    pub const STILL_NEWS: u64 = 60 * 60 * 1000;

    /// Is there a run worth offering to pick back up?
    pub fn resumable(&self) -> bool {
        let now = now_ms();
        self.items.lock().unwrap().iter().any(|a| {
            a.interrupted() && a.started > 0 && now.saturating_sub(a.started) < Self::STILL_NEWS
        })
    }

    /// Edit one in place. Unknown ids are ignored rather than panicking -- a card
    /// dismissed while its agent was mid-step would otherwise take the app down.
    fn edit(&self, id: u64, f: impl FnOnce(&mut Agent)) {
        if let Some(a) = self.items.lock().unwrap().iter_mut().find(|a| a.id == id) {
            f(a);
        }
    }

    pub fn advanced(&self, id: u64, status: String) {
        self.edit(id, |a| {
            a.step += 1;
            a.history.push(status.clone());
            a.status = status;
        });
    }

    /// Read the past back in.
    ///
    /// Anything the file claims is still running was running when the process
    /// ended, and nothing has been running since -- so it comes back as
    /// [`State::Interrupted`]. That is the honest label, and it is applied here
    /// rather than on the way out because a live process cannot know it is about
    /// to be killed.
    pub fn remember(&self) {
        let Some(path) = &self.ledger else { return };
        let Ok(text) = std::fs::read_to_string(path) else {
            return;
        };
        let Ok(past): Result<Vec<Agent>, _> = serde_json::from_str(&text) else {
            eprintln!("agents: could not read the history at {}", path.display());
            return;
        };
        let kept: Vec<Agent> = past
            .into_iter()
            .map(|mut a| {
                if !a.finished() {
                    a.state = State::Interrupted;
                }
                a
            })
            .collect();
        if kept.is_empty() {
            return;
        }
        // Ids continue past the highest one seen, so a remembered run and a new
        // one can never collide -- which would show one card and lose the other.
        let highest = kept.iter().map(|a| a.id).max().unwrap_or(0);
        self.next_id.fetch_max(highest, Ordering::Relaxed);
        eprintln!(
            "agents: {} runs remembered, {} interrupted",
            kept.len(),
            kept.iter().filter(|a| a.interrupted()).count()
        );
        self.items.lock().unwrap().extend(kept);
    }

    /// Save where things have got to.
    ///
    /// Called once a turn rather than on every edit. Edits happen several times
    /// a step -- a status line, a plan tick, a file recorded -- and rewriting the
    /// ledger for each would be a lot of writes for a file whose only reader
    /// starts up after this process has died.
    ///
    /// Once a turn is enough because that is the unit somebody resumes from: a
    /// run killed mid-turn comes back having lost that turn, which is honest,
    /// and the alternative is writing after every field change to save a few
    /// seconds of work nobody was watching.
    pub fn checkpoint(&self) {
        self.write_down();
    }

    /// Write them down -- the finished ones, and whatever is still going.
    ///
    /// The unfinished ones used to be left out, on the grounds that a run which
    /// did not finish has nothing to report. True of what it *achieved* and
    /// false of what it was *for*: the goal, the plan and the steps taken are
    /// exactly what somebody needs to decide whether to carry on. They come back
    /// as [`State::Interrupted`], never as running.
    ///
    /// Bounded, newest kept: this is a record somebody scrolls, not an archive,
    /// and a file that grows for the life of a machine is a bug with a slow fuse.
    fn write_down(&self) {
        /// Past this, the oldest go.
        const KEEP: usize = 60;
        let Some(path) = &self.ledger else { return };
        let past: Vec<Agent> = {
            let items = self.items.lock().unwrap();
            let mut kept: Vec<Agent> = items.to_vec();
            if kept.len() > KEEP {
                kept.drain(..kept.len() - KEEP);
            }
            kept
        };
        let Ok(text) = serde_json::to_string(&past) else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Err(e) = std::fs::write(path, text) {
            eprintln!("agents: could not write the history: {e}");
        }
    }

    /// Stopped is final. Nothing moves an agent out of it.
    ///
    /// The failure this is for: Escape marked an agent stopped while a model call
    /// was already in flight, the call came back `Done`, and the loop wrote that
    /// over the top -- so a task somebody had just cancelled announced *"I have
    /// submitted the task"* and was recorded as finished. Two seconds of
    /// in-flight work is enough for that to happen, which means it is not a race
    /// worth being careful about; it is one the type has to refuse.
    pub fn set_state(&self, id: u64, state: State) {
        self.edit(id, |a| {
            if !matches!(a.state, State::Stopped) {
                a.state = state;
            }
        });
        // Written at the moment a run ends rather than on the way out. There is
        // no on-the-way-out to rely on: quitting from the menu bar, a crash and a
        // reboot all end the process without asking, and a history that only
        // survives a polite exit is a history that is missing the interesting runs.
        if self
            .items
            .lock()
            .unwrap()
            .iter()
            .any(|a| a.id == id && a.finished())
        {
            self.write_down();
        }
    }

    /// Answer a question and let the loop continue.
    pub fn answer(&self, id: u64, text: String) {
        self.edit(id, |a| {
            if let State::Waiting { question, .. } = a.state.clone() {
                // Both halves go into the record: a later turn needs to know what
                // was asked as well as what was said back.
                a.history.push(format!("Asked: {question}"));
                a.history.push(format!("They answered: {text}"));
                a.status = text;
                a.state = State::Running;
            }
        });
    }

    /// The agent blocked on the user, and what it asked. One at a time, so the
    /// answer never has to be aimed at a particular agent.
    pub fn waiting(&self) -> Option<(u64, String)> {
        self.items
            .lock()
            .unwrap()
            .iter()
            .find_map(|a| match &a.state {
                State::Waiting { question, .. } => Some((a.id, question.clone())),
                _ => None,
            })
    }

    /// Record a command against whichever agent is running.
    ///
    /// No id is passed because the caller does not have one: `perform` is shared
    /// with the foreground path and knows nothing about agents. It does not need
    /// to -- only one agent may own the cursor at a time, so "the running one" is
    /// unambiguous. A command run outside an agent is simply not recorded, which
    /// is right: there is no card to show it on.
    pub fn record_run(&self, command: String, output: String) {
        if let Some(a) = self
            .items
            .lock()
            .unwrap()
            .iter_mut()
            .find(|a| !a.finished())
        {
            a.ran.push(Ran { command, output });
        }
    }

    /// Record a file the running agent produced.
    ///
    /// One entry per path: writing the same file three times while iterating is
    /// one artifact, not three. The command log still has every write.
    /// The files one run has made, newest last.
    ///
    /// Names only. Whoever asks is deciding whether running one of them follows
    /// from what was asked, and that question is not answered by its contents --
    /// it is answered by nobody having asked for it.
    pub fn files_of(&self, run: u64) -> Vec<String> {
        self.items
            .lock()
            .unwrap()
            .iter()
            .find(|a| a.id == run)
            .map(|a| {
                a.made
                    .iter()
                    .map(|m| {
                        std::path::Path::new(&m.path)
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| m.path.clone())
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Has this run already given work to this tool?
    pub fn handed_to(&self, tool: &str) -> bool {
        self.items
            .lock()
            .unwrap()
            .iter()
            .find(|a| !a.finished())
            .map(|a| a.handed.iter().any(|h| h == tool))
            .unwrap_or(false)
    }

    /// Remember that it did.
    pub fn note_handed(&self, tool: &str) {
        if let Some(a) = self
            .items
            .lock()
            .unwrap()
            .iter_mut()
            .find(|a| !a.finished())
        {
            if !a.handed.iter().any(|h| h == tool) {
                a.handed.push(tool.to_string());
            }
        }
    }

    pub fn record_file(&self, path: String) {
        if let Some(a) = self
            .items
            .lock()
            .unwrap()
            .iter_mut()
            .find(|a| !a.finished())
        {
            if !a.made.iter().any(|m| m.path == path) {
                let step = a.step;
                a.made.push(Made { path, step });
            }
        }
    }

    /// Replace the running agent's plan.
    ///
    /// Refuses more than one active item rather than quietly fixing it: the
    /// model is told, and its next turn matches what is on screen. Silently
    /// normalising would leave it believing something the user cannot see.
    pub fn set_plan(&self, items: Vec<Todo>) -> std::result::Result<(), String> {
        if items.len() > MAX_TODOS {
            return Err(format!(
                "a plan of {} is too long; {MAX_TODOS} at most",
                items.len()
            ));
        }
        if items.iter().any(|t| t.text.trim().is_empty()) {
            return Err("every step needs saying".into());
        }
        let active = items.iter().filter(|t| t.status == Doing::Active).count();
        if active > 1 {
            return Err(format!(
                "{active} steps marked as in progress -- only one thing is happening at a time"
            ));
        }
        if let Some(a) = self
            .items
            .lock()
            .unwrap()
            .iter_mut()
            .find(|a| !a.finished())
        {
            a.plan = items;
        }
        Ok(())
    }

    /// Put the running agent's question to the user, on its behalf.
    ///
    /// Returns false when nothing is running, so the caller can fall back.
    ///
    /// The runtime asks rather than telling the model to ask. Told "this file
    /// exists, ask whether to replace it", one run ignored the instruction
    /// twice, tried `rm` to get around it, and then failed -- while the user,
    /// who would have said yes in a second, was never asked anything. A
    /// permission that depends on the model choosing to request it is not a
    /// permission system.
    pub fn ask(&self, question: String) -> bool {
        self.asking(question, Vec::new())
    }

    /// Ask, offering these as one-tap answers.
    ///
    /// A question with no choices is the open kind -- "which song?" -- and gets
    /// a field. A question with them is a decision somebody is being asked to
    /// make, and the answers should not have to be typed correctly to count.
    pub fn asking(&self, question: String, choices: Vec<String>) -> bool {
        self.edit_running(|a| a.state = State::Waiting { question, choices })
    }

    fn edit_running(&self, f: impl FnOnce(&mut Agent)) -> bool {
        match self
            .items
            .lock()
            .unwrap()
            .iter_mut()
            .find(|a| !a.finished())
        {
            Some(a) => {
                f(a);
                true
            }
            None => false,
        }
    }

    pub fn stop(&self, id: u64) {
        self.abort.store(true, Ordering::Relaxed);
        self.edit(id, |a| {
            if !a.finished() {
                a.state = State::Stopped;
            }
        });
    }

    /// Take it off the list.
    ///
    /// Stops it first if it is still going. Removing a running agent's record
    /// orphans the loop driving it: every write -- its step count, its state --
    /// lands nowhere, so a question can never register as asked and the thing
    /// asks again, and again. One run put the same question four times because
    /// the card had been dismissed after the first.
    pub fn dismiss(&self, id: u64) {
        let running = self
            .items
            .lock()
            .unwrap()
            .iter()
            .any(|a| a.id == id && !a.finished());
        if running {
            self.stop(id);
        }
        self.items.lock().unwrap().retain(|a| a.id != id);
    }

    pub fn stopping(&self) -> bool {
        self.abort.load(Ordering::Relaxed)
    }

    /// What the running agent is waiting to be told, if anything.
    pub fn pending_answer(&self, id: u64) -> Option<String> {
        let items = self.items.lock().unwrap();
        let agent = items.iter().find(|a| a.id == id)?;
        match &agent.state {
            State::Running => None,
            _ => Some(agent.status.clone()),
        }
    }
}

/// Whether this step should end the run, and why.
pub fn outcome(step: &Step) -> Option<State> {
    match step {
        // Finished, but with something worth asking. The task is over either way
        // -- the wait is an offer, not a dependency -- so the runtime gives it a
        // short window and then finishes on its own.
        Step::Done {
            next: Some(question),
            ..
        } => Some(State::Waiting {
            question: question.clone(),
            // The model's own questions are open ones -- "which song?" -- and
            // there is nothing to offer as a button. Choices come from a gate
            // that knows what it is asking, not from free text.
            choices: Vec::new(),
        }),
        Step::Done { .. } => Some(State::Done),
        // Replying inside an agent is talking to nobody. The prompt says not to,
        // and one run said it twelve times in a row -- each reply performed
        // nothing, changed nothing, and the loop had no reason to stop. Treated
        // as the end: whatever it wanted to say is spoken, and the user can
        // answer with a new instruction.
        Step::Reply { .. } => Some(State::Done),
        Step::Question { question } => Some(State::Waiting {
            question: question.clone(),
            choices: Vec::new(),
        }),
        // Unsure is not fatal on its own -- the screen may simply not be ready --
        // but the stall detector already tells the model when nothing changed, so
        // a second unsure in a row means genuinely stuck.
        _ => None,
    }
}

pub type Shared = Arc<Agents>;

#[cfg(test)]
mod tests {
    use super::*;

    /// Nothing here touches the real history file. See `Agents::ledger`.
    fn agents() -> Agents {
        Agents {
            ledger: None,
            ..Default::default()
        }
    }

    /// The card is for background work only. Ordinary tasks are over in seconds
    /// and report through the notch; giving each one a floating window was a
    /// progress bar for something already finished.
    /// A run that was going when the process died comes back as interrupted,
    /// and never as running.
    #[test]
    fn what_was_in_flight_comes_back_honestly() {
        let path = std::env::temp_dir().join(format!("nudge-resume-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&path);

        {
            let a = Agents {
                ledger: Some(path.clone()),
                ..Default::default()
            };
            let id = a.start_now("tidy the folder".into(), "Tidying".into(), "".into(), false);
            a.edit(id, |x| {
                x.step = 7;
                x.history = vec!["Ran `ls`".into()];
            });
            a.start_now("a finished one".into(), "Done".into(), "".into(), false);
            a.set_state(2, State::Done);
        }

        let back = Agents {
            ledger: Some(path.clone()),
            ..Default::default()
        };
        back.remember();
        let all = back.list();

        let live = all.iter().find(|a| a.goal == "tidy the folder").unwrap();
        assert_eq!(
            live.state,
            State::Interrupted,
            "claimed to still be running"
        );
        assert!(live.interrupted() && live.finished());
        // And what it needs to carry on survived.
        assert_eq!(live.step, 7);
        assert_eq!(live.history, vec!["Ran `ls`".to_string()]);

        // A run that genuinely finished is untouched.
        let done = all.iter().find(|a| a.goal == "a finished one").unwrap();
        assert_eq!(done.state, State::Done);
        assert!(!done.interrupted());

        let _ = std::fs::remove_file(&path);
    }

    /// Stopping is a decision. It must not come back as something to resume.
    #[test]
    fn a_run_somebody_stopped_stays_stopped() {
        let path = std::env::temp_dir().join(format!("nudge-stopped-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&path);
        {
            let a = Agents {
                ledger: Some(path.clone()),
                ..Default::default()
            };
            let id = a.start_now("something".into(), "Something".into(), "".into(), false);
            a.set_state(id, State::Stopped);
        }
        let back = Agents {
            ledger: Some(path.clone()),
            ..Default::default()
        };
        back.remember();
        assert_eq!(back.list()[0].state, State::Stopped);
        assert!(!back.list()[0].interrupted());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn only_background_work_earns_a_window() {
        let a = agents();
        let quick = a
            .start("play a song".into(), "Playing".into(), "…".into(), false)
            .unwrap();
        assert!(a.running(), "it is running");
        assert!(!a.showing(), "but it does not need a window");

        // Started directly: this test is about which agents get a window, not the
        // one-at-a-time rule, which `start` enforces and has its own test.
        a.start_now(
            "upload the footage".into(),
            "Uploading".into(),
            "…".into(),
            true,
        );
        assert!(a.showing(), "a long job does");

        a.set_state(quick, State::Done);
        assert!(a.showing(), "the long one is still going");
    }

    /// A question has to hold the task open until it is answered. Answering must
    /// resume the same agent rather than starting a new one -- the whole point is
    /// that the work so far is not thrown away.
    /// Two agents mean two sets of clicks going to one cursor, and they do not
    /// take turns -- they interleave. This cost a real message: two agents drove
    /// one WhatsApp chat, one clicked send and the other clicked the same spot a
    /// moment later, by which time the button had become the microphone.
    #[test]
    fn only_one_agent_may_own_the_cursor() {
        let a = agents();
        let first = a
            .start(
                "say hi to sara".into(),
                "Messaging".into(),
                "…".into(),
                false,
            )
            .expect("nothing running");

        assert!(
            a.start("play a song".into(), "Playing".into(), "…".into(), false)
                .is_none(),
            "a second agent must be refused, not queued behind the first"
        );
        assert_eq!(
            a.current().map(|x| x.id),
            Some(first),
            "the first still has it"
        );
        assert_eq!(a.list().len(), 1, "and the refused one left no trace");

        // Once it is over, the next one is free to start.
        a.set_state(first, State::Done);
        assert!(a
            .start("play a song".into(), "Playing".into(), "…".into(), false)
            .is_some());
    }

    /// A plan measures the work. The step count only measures the budget -- a
    /// bar creeping toward forty turns says how patient the runtime is, not how
    /// close the task is.
    #[test]
    fn progress_follows_the_plan_when_there_is_one() {
        let a = agents();
        let id = a
            .start("build a page".into(), "Building".into(), "…".into(), false)
            .unwrap();

        let plan = |states: &[Doing]| -> Vec<Todo> {
            states
                .iter()
                .enumerate()
                .map(|(i, s)| Todo {
                    text: format!("step {i}"),
                    status: *s,
                })
                .collect()
        };
        let of = |a: &Agents| {
            a.list()
                .into_iter()
                .find(|x| x.id == id)
                .unwrap()
                .progress()
        };

        a.set_plan(plan(&[
            Doing::Active,
            Doing::Pending,
            Doing::Pending,
            Doing::Pending,
        ]))
        .unwrap();
        assert_eq!(of(&a), 0.0, "nothing finished yet");

        a.set_plan(plan(&[
            Doing::Done,
            Doing::Done,
            Doing::Active,
            Doing::Pending,
        ]))
        .unwrap();
        assert_eq!(of(&a), 0.5, "half the steps, half the bar");

        // Never quite full until it says so, or a finished-looking bar on an
        // unfinished task reads as a hang.
        a.set_plan(plan(&[Doing::Done, Doing::Done, Doing::Done, Doing::Done]))
            .unwrap();
        assert!(of(&a) < 1.0 && of(&a) > 0.9);
        a.set_state(id, State::Done);
        assert_eq!(of(&a), 1.0);
    }

    /// Refused rather than quietly corrected: the model is told, so its next
    /// turn matches what the user can see.
    #[test]
    fn a_plan_can_only_have_one_thing_happening() {
        let a = agents();
        a.start("x".into(), "x".into(), "…".into(), false).unwrap();

        let two = vec![
            Todo {
                text: "one".into(),
                status: Doing::Active,
            },
            Todo {
                text: "two".into(),
                status: Doing::Active,
            },
        ];
        let why = a.set_plan(two).unwrap_err();
        assert!(why.contains("2 steps marked as in progress"), "got: {why}");

        assert!(a
            .set_plan(vec![Todo {
                text: "  ".into(),
                status: Doing::Pending
            }])
            .is_err());
        let long: Vec<Todo> = (0..MAX_TODOS + 1)
            .map(|i| Todo {
                text: format!("{i}"),
                status: Doing::Pending,
            })
            .collect();
        assert!(a.set_plan(long).is_err(), "a plan that long is narration");
    }

    /// Dismissing a running agent used to leave its loop driving a record that
    /// no longer existed -- every write dropped silently, so the question it had
    /// asked never registered and it asked again, four times over.
    #[test]
    fn dismissing_a_running_agent_stops_it_first() {
        let a = agents();
        let id = a
            .start("something long".into(), "Working".into(), "…".into(), false)
            .unwrap();
        assert!(a.running());

        a.dismiss(id);
        assert!(a.list().is_empty(), "gone from the list");
        assert!(a.stopping(), "and told to stop, not just forgotten");
    }

    /// An offer is not a dependency. Most tasks end with nothing worth asking,
    /// and an agent that asks every time makes the question mean nothing.
    #[test]
    fn a_finished_task_only_waits_when_it_has_something_to_offer() {
        let plain = Step::Done {
            say: "Playing it now.".into(),
            next: None,
        };
        assert_eq!(
            outcome(&plain),
            Some(State::Done),
            "nothing to ask, so done"
        );

        let offering = Step::Done {
            say: "Project created.".into(),
            next: Some("Want me to import your clips?".into()),
        };
        assert!(
            matches!(outcome(&offering), Some(State::Waiting { .. })),
            "holds the line for an answer"
        );
    }

    #[test]
    fn a_question_holds_the_task_until_it_is_answered() {
        let a = agents();
        let id = a
            .start(
                "message someone".into(),
                "Messaging".into(),
                "…".into(),
                false,
            )
            .unwrap();
        assert!(a.waiting().is_none(), "nothing asked yet");

        a.set_state(
            id,
            State::Waiting {
                question: "Who should I send it to?".into(),
                choices: Vec::new(),
            },
        );
        assert_eq!(a.waiting(), Some((id, "Who should I send it to?".into())));

        a.answer(id, "Sara".into());
        assert!(a.waiting().is_none(), "answered, so no longer blocked");
        assert!(a.running(), "and carrying on rather than finished");

        // Both halves are kept: a later turn needs the question as well as the
        // answer to make sense of what was agreed.
        let rec = a.list().into_iter().find(|x| x.id == id).unwrap();
        assert!(rec
            .history
            .iter()
            .any(|h| h.contains("Who should I send it to?")));
        assert!(rec.history.iter().any(|h| h.contains("Sara")));
    }

    #[test]
    fn progress_creeps_forward_and_never_goes_back() {
        let a = agents();
        let id = a.start("g".into(), "t".into(), "s".into(), false).unwrap();
        let mut last = 0.0;
        for _ in 0..MAX_STEPS * 2 {
            a.advanced(id, "step".into());
            let now = a.list()[0].progress();
            assert!(now >= last, "progress went backwards: {last} -> {now}");
            assert!(now <= 1.0);
            last = now;
        }
        // Never claims completion from step count alone -- only finishing does.
        assert!(last < 1.0);
        a.set_state(id, State::Done);
        assert_eq!(a.list()[0].progress(), 1.0);
    }

    #[test]
    fn stopping_is_visible_immediately_not_at_the_end() {
        let a = agents();
        let id = a.start("g".into(), "t".into(), "s".into(), false).unwrap();
        assert!(!a.stopping());
        a.stop(id);
        assert!(a.stopping(), "the loop has to see this between steps");
        assert_eq!(a.list()[0].state, State::Stopped);
    }

    /// Cancelling has to mean cancelled, including against work already running.
    #[test]
    fn nothing_moves_an_agent_out_of_stopped() {
        let a = agents();
        let id = a.start("g".into(), "t".into(), "s".into(), false).unwrap();
        a.stop(id);
        // The model call that was in flight when Escape was pressed comes back.
        a.set_state(id, State::Done);
        assert_eq!(
            a.list()[0].state,
            State::Stopped,
            "a cancelled task reported success"
        );
    }

    #[test]
    fn a_question_blocks_until_it_is_answered() {
        let a = agents();
        let id = a
            .start("play a song".into(), "Playing".into(), "…".into(), false)
            .unwrap();
        a.set_state(
            id,
            State::Waiting {
                question: "Which song?".into(),
                choices: Vec::new(),
            },
        );
        assert!(a.pending_answer(id).is_some());

        a.answer(id, "Bohemian Rhapsody".into());
        assert_eq!(a.list()[0].state, State::Running);
        // The question and the answer both survive, because the next turn needs
        // to know what was asked as well as what came back.
        let history = a.list()[0].history.join(" ");
        assert!(history.contains("Which song?"));
        assert!(history.contains("Bohemian Rhapsody"));
    }

    #[test]
    fn acting_on_an_agent_that_is_gone_is_a_no_op() {
        // A card dismissed while its agent was mid-step used to be a panic.
        let a = agents();
        let id = a.start("g".into(), "t".into(), "s".into(), false).unwrap();
        a.dismiss(id);
        a.advanced(id, "still going".into());
        a.answer(id, "hello".into());
        a.stop(id);
        a.dismiss(id);
        assert!(a.list().is_empty());
    }

    #[test]
    fn only_a_finished_agent_stops_counting_as_running() {
        let a = agents();
        let id = a.start("g".into(), "t".into(), "s".into(), false).unwrap();
        assert!(a.running());
        a.set_state(
            id,
            State::Waiting {
                question: "?".into(),
                choices: Vec::new(),
            },
        );
        assert!(a.running(), "waiting on the user is still an open task");
        a.set_state(id, State::Done);
        assert!(!a.running());
    }
}
