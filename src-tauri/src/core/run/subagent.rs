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
use crate::core::provider::{recalled, Ask, Provider, Step};
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
///
/// **A missing phrase here scores a correct answer as a wrong one**, which is the
/// worse direction: it sends somebody looking for a bug in the product when the
/// bug is in the ruler. Two were missing on the first full run of the truth
/// suite, and both looked like model failures until the answers were read.
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
    // Said the other way round, which is how it actually came out: "I do not
    // have access to your personal notes". `no access to` does not match that
    // -- there is a "t" between the "no" and the space -- so a correct refusal
    // was scored as a confident answer.
    "have access to",
    "forecast years in advance",
    "cannot be forecast",
    "can't be forecast",
    "no single year",
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

/// Is this answer worth the price of a second pass?
///
/// Three conditions, and each one removes a slice of the bill:
///
/// - It came from looking something up. An answer read off a file has no source
///   to go back to.
/// - It is not already an admission. Nothing to refute, everything to lose --
///   asked to find the fault in "future prices do not exist yet", a checker
///   produced a share price for next Friday.
/// - **It states a specific.** The failure this exists for is a confidently
///   wrong number: 2036, macOS 15, $150,000. An answer with no digit in it is
///   not that failure, and checking it buys nothing at three times the price.
fn worth_checking(looked_up: bool, say: &str) -> bool {
    looked_up && !hedged(say) && say.chars().any(|c| c.is_ascii_digit())
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
    /// Short, and shortened again once the bill was counted: on the one full
    /// trace taken, a four-turn checker spent three searches and carried 7,059
    /// characters to second-guess an answer found with 428. Three turns is two
    /// searches and a verdict, which is as much as checking one claim deserves.
    const CHECKS: usize = 3;
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
            tools: &[],
            reach: String::new(),
            memory: String::new(),
            earlier: &[],
            skills: String::new(),
            shell: false,
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
// Eight, and each is a separate thing the caller already holds. Bundling them
// into a struct would move the list rather than shorten it, and give the struct
// a name that means "the arguments to run".
#[allow(clippy::too_many_arguments)]
pub async fn run<F, Fut>(
    provider: &dyn Provider,
    workspace: &std::path::Path,
    task: &str,
    tools: &[crate::core::tools::mcp::Tool],
    reach: &str,
    shell: bool,
    verify: bool,
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
    // Whether anything at all was consulted -- a command, a file, a page, a
    // search. Wider than `looked_up`, and asking a different question: that one
    // decides whether an answer is worth checking, this one decides whether the
    // answer came from anywhere but memory.
    let mut grounded = false;

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
            tools,
            reach: reach.to_string(),
            memory: String::new(),
            earlier: &[],
            skills: String::new(),
            shell,
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
                let answer = match verify && worth_checking(looked_up, &say) {
                    true => scrutinise(provider, task, &say, &mut act).await,
                    false => say,
                };
                // A subagent has no screen. If it also consulted nothing, there
                // was no source in the room: whatever it just said, it said from
                // memory, whether or not it marked it as such. Marked here
                // rather than trusted from the model for the same reason the
                // check in `scrutinise` is -- this can only ever add the
                // qualifier, never remove one the model asked for.
                let answer = match grounded || hedged(&answer) {
                    true => answer,
                    false => recalled(answer),
                };
                return Ok(Found { answer, steps });
            }
            // Not a failure: a subagent that cannot answer says so, and the
            // parent decides what that means for the larger task.
            Step::Unsure { say, .. } => {
                return Ok(Found {
                    answer: format!("Could not: {say}"),
                    steps,
                })
            }
            other => {
                looked_up |= matches!(other, Step::Search { .. } | Step::Fetch { .. });
                grounded |= other.consults();
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
    #[test]
    fn a_refusal_counts_however_it_is_phrased() {
        // All four were produced by the real model on the first full run of the
        // truth suite, and the first three were scored as confident answers
        // because the list below did not have their words in it.
        for said in [
            "I do not have access to your personal notes or files in this setup.",
            "exact future weather cannot be forecast years in advance",
            "There is no single year, but estimates cluster into ranges",
            "Tonight's draw numbers cannot be known before the draw happens.",
        ] {
            assert!(super::hedged(said), "not counted as a refusal: {said}");
        }
    }

    #[test]
    fn a_confident_answer_is_still_confident() {
        // And the widening must not swallow the failure it exists to catch.
        for said in [
            "Forecasts range by scenario: conservative estimates sit between $80,000 and $98,000.",
            "macOS 27 Golden Gate was released on September 14, 2026.",
            "The latest stable release of Python is 3.14.7.",
        ] {
            assert!(!super::hedged(said), "wrongly counted as a refusal: {said}");
        }
    }

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
            &[],
            "",
            false,
            false,
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
        let found = run(
            &p,
            std::path::Path::new("/tmp"),
            "x",
            &[],
            "",
            false,
            false,
            |_| async { Err(crate::error::Error::Click("rm is not allowed".into())) },
        )
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
        let found = run(
            &p,
            std::path::Path::new("/tmp"),
            "x",
            &[],
            "",
            false,
            false,
            |_| async { Ok(String::new()) },
        )
        .await
        .unwrap();
        assert!(found.answer.contains("Gave up"));
        assert_eq!(found.steps.len(), MAX_TURNS);
    }

    /// The cheapest of the three gates, and the one that removes the most: most
    /// of what a subagent says has no number in it.
    #[test]
    fn only_an_answer_stating_a_specific_is_worth_checking() {
        use super::worth_checking;
        assert!(worth_checking(
            true,
            "macOS 27 was released on 14 September 2026."
        ));
        // Nothing was looked up, so there is no source to go back to.
        assert!(!worth_checking(false, "macOS 27 was released in 2026."));
        // No specific to be wrong about.
        assert!(!worth_checking(
            true,
            "The page explains how the parser works."
        ));
        // Already an admission.
        assert!(!worth_checking(
            true,
            "I could not find a date for version 27."
        ));
    }

    #[test]
    fn the_recalled_marker_does_not_stack() {
        use crate::core::provider::recalled;
        let once = recalled("Paris.".into());
        assert_eq!(recalled(once.clone()), once);
        assert!(once.starts_with("From memory"));
    }

    /// The whole point of 1.3: two sentences that used to sound identical.
    #[test]
    fn a_recalled_fact_and_an_observed_action_do_not_sound_the_same() {
        use crate::core::provider::recalled;
        let observed = "I clicked Send.".to_string();
        let remembered = recalled("macOS 27 shipped in 2026.".into());
        assert_ne!(observed, remembered);
        assert!(!observed.starts_with("From memory"));
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
