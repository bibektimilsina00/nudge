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
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Long enough for a real task -- open a browser, search, click a result, play --
/// and short enough that a model which never finishes gives up rather than
/// clicking forever. The foreground loop caps at 12 for the same reason; an
/// agent needs more room because nobody is watching it.
pub const MAX_STEPS: usize = 40;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum State {
    Running,
    /// Blocked on the user. `question` is shown with a field to answer it.
    Waiting {
        question: String,
    },
    Done,
    Failed {
        why: String,
    },
    Stopped,
}

/// A command an agent ran, and what came back.
///
/// Kept so it can be shown. An agent that runs things on your machine and leaves
/// no record of what it ran is not one anybody should install -- and the record
/// has to survive the run, because the moment you want it is after something
/// surprising happened.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Ran {
    pub command: String,
    pub output: String,
}

/// One line of an agent's plan.
///
/// Written by the agent and replaced wholesale each time, so there are no ids to
/// get wrong and no way for the list the model believes in to drift from the one
/// on screen.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Todo {
    pub text: String,
    pub status: Doing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Doing {
    Pending,
    Active,
    Done,
}

/// Most a plan may hold. Past this it is not a plan, it is a model narrating.
pub const MAX_TODOS: usize = 20;

/// A file an agent produced.
///
/// Kept apart from the command log because it is a different kind of thing: the
/// log is what happened, an artifact is what you have now. Asked to make a
/// landing page, what you want afterwards is the page -- not a transcript of the
/// making of it.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Made {
    /// Absolute, so it can be opened. The UI shows only the last component.
    pub path: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Agent {
    pub id: u64,
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
            State::Done | State::Failed { .. } | State::Stopped
        )
    }
}

/// Every agent this session has seen, running or not.
#[derive(Default)]
pub struct Agents {
    items: Mutex<Vec<Agent>>,
    next_id: AtomicU64,
    /// Set to stop whichever agent is running. One at a time, so one flag.
    abort: AtomicBool,
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

    fn start_now(&self, goal: String, title: String, status: String, background: bool) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed) + 1;
        self.abort.store(false, Ordering::Relaxed);
        self.items.lock().unwrap().push(Agent {
            id,
            goal,
            title,
            status,
            step: 0,
            state: State::Running,
            history: Vec::new(),
            background,
            ran: Vec::new(),
            made: Vec::new(),
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

    pub fn running(&self) -> bool {
        self.items.lock().unwrap().iter().any(|a| !a.finished())
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

    pub fn set_state(&self, id: u64, state: State) {
        self.edit(id, |a| a.state = state);
    }

    /// Answer a question and let the loop continue.
    pub fn answer(&self, id: u64, text: String) {
        self.edit(id, |a| {
            if let State::Waiting { question } = a.state.clone() {
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
                State::Waiting { question } => Some((a.id, question.clone())),
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
    pub fn record_file(&self, path: String) {
        if let Some(a) = self
            .items
            .lock()
            .unwrap()
            .iter_mut()
            .find(|a| !a.finished())
        {
            if !a.made.iter().any(|m| m.path == path) {
                a.made.push(Made { path });
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
        self.edit_running(|a| a.state = State::Waiting { question })
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

    fn agents() -> Agents {
        Agents::default()
    }

    /// The card is for background work only. Ordinary tasks are over in seconds
    /// and report through the notch; giving each one a floating window was a
    /// progress bar for something already finished.
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
            },
        );
        assert!(a.running(), "waiting on the user is still an open task");
        a.set_state(id, State::Done);
        assert!(!a.running());
    }
}
