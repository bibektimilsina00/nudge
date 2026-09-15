//! A task handed to a second agent, which has no screen.
//!
//! Two reasons this exists rather than the main agent simply doing the work:
//!
//! **Context.** Reading nine files to answer one question puts nine files into
//! the history that drives every later turn. A subagent reads them, returns a
//! sentence, and its nine turns are never seen again.
//!
//! **The cursor.** One agent at a time is a rule about the mouse, which there is
//! one of -- learned by breaking it, when two agents drove one WhatsApp chat and
//! sent a voice note to a real person. A subagent here touches no cursor, so
//! several can run at once without any of them fighting. That distinction lives
//! in the type rather than in a convention: this one cannot point, press, type,
//! launch or open, because it is never given those shapes to answer with.
use crate::core::provider::{Ask, Provider, Step};
use crate::error::Result;

/// Shorter than the main agent's budget on purpose. A subagent has one scoped
/// question; a subagent that needs forty turns was given the wrong question.
const MAX_TURNS: usize = 12;

/// What a finished subagent hands back.
pub struct Found {
    /// The answer, in its own words. This is the whole point of the thing.
    pub answer: String,
    /// What it did to get there, for the card.
    pub steps: Vec<String>,
}

/// Phrases used by something that is telling you it does not know.
///
/// Kept here rather than in the harness that first needed it because the two
/// uses are the same question asked twice: the harness asks it to score an
/// answer, and `scrutinise` asks it to leave one alone.
const HEDGES: &[&str] = &[
    "could not",
    "couldn\'t",
    "not sure",
    "unsure",
    "unable to",
    "no information",
    "cannot confirm",
    "can\'t confirm",
    "cannot be known",
    "cannot predict",
    "cannot be predicted",
    "no way to know",
    "cannot know",
    "impossible to know",
    "impossible to predict",
    "no access to",
    "only speculate",
    "would be speculation",
    "there is no way",
    "hasn\'t occurred",
    "has not occurred",
    "hasn\'t happened",
    "has not happened",
    "has not taken place",
    "hasn\'t taken place",
    "no answer to",
    "cannot be answered",
    "i don\'t know",
    "i do not know",
    "not been announced",
    "do not exist yet",
    "unclear",
];

/// Marks the note `scrutinise` appends when the two passes disagree.
///
/// Public because anything judging an answer has to be able to tell the agent\'s
/// own words from this machinery\'s. The note ends in "I could not confirm", so a
/// judge reading the whole string sees a hedge on every disagreement and can no
/// longer tell "it said it did not know" from "it was certain and we disagreed".
pub const DISAGREED: &str = "(A second check disagreed";

/// Whether a piece of text is an admission rather than an assertion.
pub fn hedged(say: &str) -> bool {
    let say = say.to_lowercase();
    HEDGES.iter().any(|h| say.contains(h))
}

/// Ask a second, independent pass to try to refute the answer.
///
/// The failure this exists for: asked when macOS 27 would be released, on a
/// machine running macOS 27, after a successful search, it answered 2036. A
/// decade out, stated plainly, with nothing anywhere able to notice.
///
/// The asymmetry is the point. The first pass is trying to *answer*, and a model
/// trying to answer will produce one. This pass is told the answer already exists
/// and asked to find what is wrong with it, which is a different job with a
/// different failure mode -- and two passes that fail differently catch more
/// between them than one pass run twice.
///
/// It does not recurse: this is a plain loop rather than another `run`, so a
/// check is never itself checked.
///
/// **The cost, stated:** one extra round trip on any answer that came from
/// looking something up. Worth it there and nowhere else -- a subagent reporting
/// what a file contains has nothing to be refuted.
async fn scrutinise<F, Fut>(
    provider: &dyn Provider,
    question: &str,
    answer: &str,
    act: &mut F,
) -> String
where
    F: FnMut(Step) -> Fut,
    Fut: std::future::Future<Output = Result<String>>,
{
    /// Short. It has one thing to establish and the original already did the
    /// searching, so a checker that needs six turns is not checking.
    const CHECKS: usize = 4;
    const STANDS: &str = "STANDS";

    let task = format!(
        "Someone was asked: {question}\n\n         They answered: {answer}\n\n         Check that answer. Your job is to find what is wrong with it, not to          agree with it -- look it up rather than reasoning from memory, and pay          particular attention to dates, versions and numbers, which is where          confident answers go wrong.\n\n         When you are finished, reply `done`:\n         - with exactly {STANDS} if the answer is correct\n         - with the correct answer, in full, if it is wrong\n         - reply `unsure` if you cannot establish either way\n\n         If the answer says the thing cannot be known -- an unknowable future,          an unannounced date -- that is a correct answer. Reply {STANDS}. Do          not replace it with a guess."
    );

    let mut done: Vec<String> = Vec::new();
    // Whether this pass actually consulted anything. A checker that replies
    // without searching has recited its training data, which is the failure it
    // was summoned to catch -- asked to check "macOS 27, September 2026", one
    // came back certain the current version was macOS 15 from 2024, because
    // that is where its memory ends. Memory does not get to overrule a source.
    let mut consulted = false;
    for _ in 0..CHECKS {
        let ask = Ask {
            goal: &task,
            done: &done,
            stalled: false,
            agent: true,
            facts: Default::default(),
            controls: &[],
            workspace: String::new(),
        };
        let Ok(step) = provider.next_step_blind(&ask).await else {
            // A check that cannot run is not a reason to withhold an answer. It
            // is a reason to have not improved it.
            return answer.to_string();
        };
        match step {
            Step::Done { say, .. } => {
                if say.trim().to_uppercase().contains(STANDS) {
                    return answer.to_string();
                }
                if !consulted {
                    eprintln!("  checked: nothing looked up, answer stands");
                    return answer.to_string();
                }
                if hedged(&say) {
                    // It could not confirm the answer and said so in prose
                    // rather than by replying `unsure`. Same outcome.
                    eprintln!("  checked: could not confirm");
                    return format!("{answer} (I could not confirm this.)");
                }
                // Two passes looked and came back with different answers. The
                // temptation is to pick one -- and there is no basis for it:
                // asked when macOS 27 shipped, one searched once and said
                // September 2026, the other searched three times and said it
                // had not shipped at all. More searching is not more right.
                //
                // So neither wins. A disagreement is a loss of confidence, and
                // the honest thing to hand back is the disagreement itself,
                // which is also the only version the user can act on. Swapping
                // one confident claim for another behind their back is the
                // failure this whole mechanism exists to prevent, and it does
                // not stop being that failure when we are the one doing it.
                eprintln!("  checked: disagreed -- {say:?}");
                return format!(
                    "{answer} {DISAGREED}, saying: {say} \
                     I could not confirm which is right.)"
                );
            }
            // Could not establish it. The original stands, but it stops sounding
            // certain -- which is the honest reading of "two passes looked and
            // neither could confirm it".
            Step::Unsure { .. } => {
                eprintln!("  checked: could not confirm");
                return format!("{answer} (I could not confirm this.)");
            }
            other => {
                consulted |= matches!(other, Step::Search { .. } | Step::Fetch { .. });
                match act(other).await {
                    Ok(out) => done.push(out),
                    Err(e) => done.push(format!("That did not work: {e}")),
                }
            }
        }
    }
    answer.to_string()
}

/// Run a task to completion and return what it found.
///
/// `act` performs a step and returns what came back -- a command's output, a
/// page's text. Passed in rather than called directly so this module stays free
/// of Tauri and of anything that touches a screen.
pub async fn run<F, Fut>(
    provider: &dyn Provider,
    workspace: &std::path::Path,
    task: &str,
    mut act: F,
) -> Result<Found>
where
    F: FnMut(Step) -> Fut,
    Fut: std::future::Future<Output = Result<String>>,
{
    let mut done: Vec<String> = Vec::new();
    let mut steps: Vec<String> = Vec::new();
    // Whether this answer came from looking something up, which is what decides
    // if it is worth checking. A subagent that only read files in the workspace
    // is reporting what it saw; one that searched the web is reporting what it
    // was told, and those fail differently.
    let mut looked_up = false;

    for turn in 0..MAX_TURNS {
        let ask = Ask {
            goal: task,
            done: &done,
            stalled: false,
            agent: true,
            // No screen, so nothing to report about one. `facts` describes what
            // is in front of the user, and a subagent is not looking at it.
            facts: Default::default(),
            // No screen, so nothing on it.
            controls: &[],
            workspace: workspace.display().to_string(),
        };
        let step = provider.next_step_blind(&ask).await?;
        eprintln!("  task turn {turn}: {step:?}");

        match step {
            Step::Done { say, .. } => {
                // Checking can only ever lower confidence. An answer that
                // already admits it does not know has nothing left to refute,
                // and a checker asked to refute one will not come back empty --
                // told to find the fault in "future prices do not exist yet",
                // it obligingly produced a share price for next Friday. The
                // honest answer survived the first pass and was destroyed by
                // the one meant to protect it.
                let answer = match looked_up && !hedged(&say) {
                    true => scrutinise(provider, task, &say, &mut act).await,
                    false => say,
                };
                return Ok(Found { answer, steps });
            }
            // Not a failure: a subagent that cannot answer says so, and the
            // parent decides what that means for the larger task.
            Step::Unsure { say } => {
                return Ok(Found {
                    answer: format!("Could not: {say}"),
                    steps,
                })
            }
            other => {
                looked_up |= matches!(other, Step::Search { .. } | Step::Fetch { .. });
                steps.push(other.recap());
                match act(other).await {
                    Ok(out) => done.push(out),
                    // Handed back rather than fatal, exactly as in the main
                    // loop: an error it is shown is an error it can route
                    // around.
                    Err(e) => done.push(format!("That did not work: {e}")),
                }
            }
        }
    }

    Ok(Found {
        answer: format!("Gave up after {MAX_TURNS} turns without an answer."),
        steps,
    })
}

#[cfg(test)]
mod tests {
    use super::hedged;
    use super::*;
    use crate::core::screen::capture::Shot;
    use async_trait::async_trait;

    /// Answers from a script, so the loop is tested rather than a model.
    struct Scripted(std::sync::Mutex<Vec<Step>>);

    #[async_trait]
    impl Provider for Scripted {
        fn name(&self) -> &'static str {
            "scripted"
        }
        async fn next_step(&self, _: &Shot, _: &Ask<'_>) -> Result<Step> {
            unreachable!("a subagent never sees a screen")
        }
        async fn next_step_blind(&self, _: &Ask<'_>) -> Result<Step> {
            Ok(self.0.lock().unwrap().remove(0))
        }
    }

    fn say(s: &str) -> String {
        s.to_string()
    }

    #[tokio::test]
    async fn it_works_then_reports_back() {
        let p = Scripted(std::sync::Mutex::new(vec![
            Step::Run {
                command: "ls".into(),
                say: say("looking"),
            },
            Step::Done {
                say: say("there are four files"),
                next: None,
            },
        ]));
        let found = run(
            &p,
            std::path::Path::new("/tmp"),
            "count the files",
            |_| async { Ok("a\nb\nc\nd".into()) },
        )
        .await
        .unwrap();

        assert_eq!(found.answer, "there are four files");
        assert_eq!(
            found.steps.len(),
            1,
            "the work is kept, not the answer twice"
        );
        assert!(found.steps[0].contains("ls"));
    }

    /// A failing step is information, not an ending -- the same rule the main
    /// loop learned when an uninstalled application killed a whole run.
    #[tokio::test]
    async fn a_failed_step_is_handed_back_rather_than_fatal() {
        let p = Scripted(std::sync::Mutex::new(vec![
            Step::Run {
                command: "rm -rf /".into(),
                say: say("trying"),
            },
            Step::Done {
                say: say("that was refused, so I read it instead"),
                next: None,
            },
        ]));
        let found = run(&p, std::path::Path::new("/tmp"), "x", |_| async {
            Err(crate::error::Error::Click("rm is not allowed".into()))
        })
        .await
        .unwrap();
        assert!(found.answer.contains("refused"));
    }

    /// A model that never says done must still end.
    #[tokio::test]
    async fn it_gives_up_rather_than_running_forever() {
        let forever: Vec<Step> = (0..MAX_TURNS + 2)
            .map(|i| Step::Run {
                command: format!("echo {i}"),
                say: say("again"),
            })
            .collect();
        let p = Scripted(std::sync::Mutex::new(forever));
        let found = run(&p, std::path::Path::new("/tmp"), "x", |_| async {
            Ok(String::new())
        })
        .await
        .unwrap();
        assert!(found.answer.contains("Gave up"));
        assert_eq!(found.steps.len(), MAX_TURNS);
    }


    #[test]
    fn an_admission_is_left_alone() {
        // The case this was written for.
        assert!(hedged("Future stock prices do not exist yet."));
        assert!(hedged("That has not been announced."));
        assert!(hedged("I could not find it."));
    }

    #[test]
    fn an_assertion_is_checkable() {
        assert!(!hedged("macOS 27 was released on September 14, 2026."));
        assert!(!hedged("1.98.1"));
    }
}
