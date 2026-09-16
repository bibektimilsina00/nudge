//! Being the person at a supervised terminal.
//!
//! The loop: watch what a delegated tool prints, notice when it has stopped to
//! ask something, decide what can be decided, and put the rest to whoever is
//! actually here.
//!
//! Everything it decides *with* lives elsewhere and is tested without a
//! terminal: [`asked`](crate::core::asked) recognises the shape of a question,
//! [`answering`](crate::core::answering) decides what may be answered without a
//! person. This is the part that has to run against a real process, so it is
//! kept to the timing and the plumbing.
use crate::app::state::{Background, Grants};
use crate::core::answering::{self, Answer};
use crate::core::asked;
use crate::core::reach::Pending;
use crate::core::run::agent::Agents;
use crate::core::run::session::Nudge;
use tauri::{AppHandle, Manager};

/// How long output must be still before it counts as waiting.
///
/// Long enough that a tool pausing mid-sentence is not mistaken for one asking a
/// question; short enough that somebody watching would not have noticed the
/// delay. A tool that prints a prompt and then prints more is not waiting, and
/// this is the only thing standing between those two cases.
const SETTLED: std::time::Duration = std::time::Duration::from_millis(900);

/// How often to look.
const BEAT: std::time::Duration = std::time::Duration::from_millis(300);

/// Watch a job until it ends, answering what can be answered.
///
/// Spawned and left to it. The agent that started the job carries on and can
/// `await` it like any other; this runs beside that.
pub fn over(app: &AppHandle, job: u64, task: String) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut seen = String::new();
        let mut still = std::time::Instant::now();
        let mut last_asked: Option<String> = None;

        loop {
            if app.state::<Agents>().stopping() {
                eprintln!("supervisor: job {job} -- the run was stopped");
                return;
            }

            let (text, alive) = match app.state::<Background>().peek(job) {
                Some(pair) => pair,
                // Gone: stopped, or cleaned up when its run ended.
                None => return,
            };
            if !alive {
                eprintln!("supervisor: job {job} finished");
                return;
            }

            if text.len() != seen.len() {
                // Something new. Whatever it was about to be asked, it is still
                // typing.
                seen = text;
                still = std::time::Instant::now();
                tokio::time::sleep(BEAT).await;
                continue;
            }

            if still.elapsed() < SETTLED {
                tokio::time::sleep(BEAT).await;
                continue;
            }

            let Some(question) = asked::waiting(&seen) else {
                // Quiet, but not asking. Thinking, or downloading, or wedged --
                // and telling those apart is the stall watch's job, not this.
                tokio::time::sleep(BEAT).await;
                continue;
            };

            // The same question twice means the answer did not take. Saying it
            // again would be a loop, and a tool re-asking is a tool that did not
            // accept what it was given.
            if last_asked.as_deref() == Some(question.line.as_str()) {
                eprintln!("supervisor: job {job} asked the same thing twice -- handing it over");
                put_to_the_person(&app, job, &question);
                return;
            }
            last_asked = Some(question.line.clone());

            answer(&app, job, &question, &task).await;
            // Give it a moment to act on the answer before looking again.
            tokio::time::sleep(SETTLED).await;
            still = std::time::Instant::now();
        }
    });
}

/// Decide one question and act on it.
async fn answer(app: &AppHandle, job: u64, question: &asked::Asked, task: &str) {
    use crate::core::audit::Outcome;

    // The judge, on facts rather than on what the tool said -- see
    // `core::answering::as_facts`. Only asked when the shape is one that could
    // be answered at all; an unrecognised prompt is a person's either way, and
    // a model call to be told so is a model call wasted.
    let agreed = match question.shape {
        asked::Shape::Unknown => None,
        _ => crate::app::commands::seconded(app, &answering::as_facts(question, task)).await,
    };

    match answering::decide(question, agreed) {
        Answer::Say { text, why } => {
            match app.state::<Background>().answer(job, &text) {
                Ok(()) => eprintln!(
                    "supervisor: job {job} asked {:?} -- said {:?} ({why})",
                    question.line, text
                ),
                Err(e) => eprintln!("supervisor: could not answer job {job}: {e}"),
            }
            crate::app::commands::note_supervision(
                app,
                job,
                question,
                Outcome::Did {
                    detail: format!("answered {text:?}: {why}"),
                },
            );
        }
        Answer::Ask => put_to_the_person(app, job, question),
    }
}

/// Hand it over, with the tool's own words and its own options.
fn put_to_the_person(app: &AppHandle, job: u64, question: &asked::Asked) {
    use crate::core::audit::Outcome;

    let options = match &question.shape {
        asked::Shape::Pick { options } => options.clone(),
        _ => Vec::new(),
    };
    let pending = Pending::Supervising {
        job,
        line: question.line.clone(),
        options,
    };
    let put = pending.question();
    let choices = pending.choices();

    crate::app::commands::note_supervision(
        app,
        job,
        question,
        Outcome::Asked {
            question: put.clone(),
        },
    );

    app.state::<Grants>()
        .asking
        .lock()
        .unwrap()
        .replace(pending);
    if app.state::<Agents>().asking(put.clone(), choices) {
        crate::app::commands::speak(app, &put);
        let _ = tauri::Emitter::emit(app, "status", "asking");
        crate::app::agent::publish(app);
        return;
    }
    // Nobody to ask. Better to say so than to leave a tool waiting for ever on
    // an answer that is never coming.
    app.state::<Grants>().asking.lock().unwrap().take();
    eprintln!("supervisor: job {job} is asking and nobody is here -- stopping it");
    let _ = app.state::<Background>().stop(job);
    app.state::<Nudge>().note(format!(
        "Job {job} stopped to ask something and there was nobody to ask, so I stopped it."
    ));
}
