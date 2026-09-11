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
    Waiting { question: String },
    Done,
    Failed { why: String },
    Stopped,
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
}

impl Agent {
    /// How far along, 0.0 to 1.0.
    ///
    /// Steps against the budget, not an estimate of the task: there is no way to
    /// know how many clicks "play a song" takes until it is done, and a bar that
    /// invents a number is a lie that happens to move. It therefore creeps rather
    /// than predicts, and never goes backwards.
    pub fn progress(&self) -> f32 {
        match self.state {
            State::Done => 1.0,
            _ => (self.step as f32 / MAX_STEPS as f32).clamp(0.0, 0.97),
        }
    }

    pub fn finished(&self) -> bool {
        matches!(self.state, State::Done | State::Failed { .. } | State::Stopped)
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
    pub fn start(&self, goal: String, title: String, status: String) -> u64 {
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
        });
        id
    }

    pub fn list(&self) -> Vec<Agent> {
        self.items.lock().unwrap().clone()
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

    pub fn stop(&self, id: u64) {
        self.abort.store(true, Ordering::Relaxed);
        self.edit(id, |a| {
            if !a.finished() {
                a.state = State::Stopped;
            }
        });
    }

    pub fn dismiss(&self, id: u64) {
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
        Step::Done { .. } => Some(State::Done),
        Step::Question { question } => Some(State::Waiting { question: question.clone() }),
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

    #[test]
    fn progress_creeps_forward_and_never_goes_back() {
        let a = agents();
        let id = a.start("g".into(), "t".into(), "s".into());
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
        let id = a.start("g".into(), "t".into(), "s".into());
        assert!(!a.stopping());
        a.stop(id);
        assert!(a.stopping(), "the loop has to see this between steps");
        assert_eq!(a.list()[0].state, State::Stopped);
    }

    #[test]
    fn a_question_blocks_until_it_is_answered() {
        let a = agents();
        let id = a.start("play a song".into(), "Playing".into(), "…".into());
        a.set_state(id, State::Waiting { question: "Which song?".into() });
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
        let id = a.start("g".into(), "t".into(), "s".into());
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
        let id = a.start("g".into(), "t".into(), "s".into());
        assert!(a.running());
        a.set_state(id, State::Waiting { question: "?".into() });
        assert!(a.running(), "waiting on the user is still an open task");
        a.set_state(id, State::Done);
        assert!(!a.running());
    }
}
