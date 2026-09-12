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

    for turn in 0..MAX_TURNS {
        let ask = Ask {
            goal: task,
            done: &done,
            stalled: false,
            agent: true,
            // No screen, so nothing to report about one. `facts` describes what
            // is in front of the user, and a subagent is not looking at it.
            facts: Default::default(),
            workspace: workspace.display().to_string(),
        };
        let step = provider.next_step_blind(&ask).await?;
        eprintln!("  task turn {turn}: {step:?}");

        match step {
            Step::Done { say, .. } => return Ok(Found { answer: say, steps }),
            // Not a failure: a subagent that cannot answer says so, and the
            // parent decides what that means for the larger task.
            Step::Unsure { say } => {
                return Ok(Found {
                    answer: format!("Could not: {say}"),
                    steps,
                })
            }
            other => {
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
}
