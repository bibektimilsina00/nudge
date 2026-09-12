//! The loop, as the frontend sees it: ask, act, and stop.
//!
//! `perform` is the one place an action actually happens. Both the foreground
//! path and the agent runtime go through it, and a subagent's steps do too --
//! which is what makes the shell allow-list, the workspace boundary and
//! ask-before-replacing hold everywhere rather than in whichever caller
//! remembered them.
use crate::app::state::{Background, Grants, Screen, Settle, Voice};
use crate::core::provider::{Act, Step};
use crate::core::run::agent::Agents;
use crate::core::run::session::Nudge;
use crate::core::screen::click;
use crate::core::screen::keyboard;
use crate::core::screen::launch;
use crate::core::tools::fetch;
use crate::core::tools::files;
use crate::core::tools::shell;
use crate::core::voice::speech;
use crate::error::Result;
use tauri::{AppHandle, Emitter, Manager};

#[tauri::command]
pub async fn start(goal: String, app: AppHandle) -> Result<Option<Step>> {
    app.state::<Nudge>().begin(goal);
    advance(app.clone()).await
}

/// A menu is drawn in well under this; it is the cost of not photographing the
/// screen mid-animation.
const AFTER_CLICK: std::time::Duration = std::time::Duration::from_millis(420);
/// Applications and web pages take their time, and there is nothing useful to see
/// until they are up.
/// A floor, not the wait. `capture::wait_until_still` covers the rest, so this
/// only has to be long enough for the *beginning* of a change to show up --
/// before that, the screen is still convincingly the old one.
const AFTER_OPEN: std::time::Duration = std::time::Duration::from_millis(500);

#[tauri::command]
pub async fn advance(app: AppHandle) -> Result<Option<Step>> {
    // Belt and braces. The hotkey already refuses while an agent runs, but this
    // is the one funnel every foreground action goes through, and an agent owns
    // the cursor for its whole run. The agent's own loop does not come through
    // here -- it calls `Nudge::step` directly.
    if app.state::<Agents>().running() {
        return Ok(None);
    }
    // And not twice at once. Two of these ran concurrently in one session -- two
    // model calls, two identical answers, and only the agent registry stopping
    // the second from spawning. Had they been `point` steps instead, both would
    // have clicked.
    static BUSY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if BUSY.swap(true, std::sync::atomic::Ordering::SeqCst) {
        // Say so. Dropping it silently is the same mistake as the agent refusing
        // a second goal without a word -- the user spoke, nothing happened, and
        // the natural response is to say it again, which is how one WhatsApp
        // chat ended up with three agents in it.
        eprintln!("foreground: already thinking, ignoring");
        speak(&app, "One moment, still working on the last one.");
        return Ok(None);
    }
    struct Done;
    impl Drop for Done {
        fn drop(&mut self) {
            BUSY.store(false, std::sync::atomic::Ordering::SeqCst);
        }
    }
    // Released however this returns, including on the `?` below.
    let _done = Done;
    app.emit("status", "thinking").ok();

    // Let whatever we just did finish happening before photographing the result.
    if let Some(wait) = app.state::<Settle>().remaining() {
        tokio::time::sleep(wait).await;
    }

    let began = std::time::Instant::now();
    let step = app.state::<Nudge>().step().await?;
    // Logged like an agent turn. The trace used to begin at the agent, so a
    // foreground step that launched something looked like an app the agent found
    // already open -- it appeared on screen and nothing said why.
    if let Some(s) = &step {
        eprintln!("foreground ({:.1}s): {s:?}", began.elapsed().as_secs_f32());
    }

    // Handing over is not an action, so it alone is neither spoken nor performed.
    //
    // Everything else IS performed here, including when an agent is about to
    // take over. Skipping it was tried and was worse: this turn decided to open
    // WhatsApp Web -- correctly -- the step was discarded, and the agent started
    // from a blank desktop and spent six turns finding its way back to the same
    // answer. A first step already taken is the best thing an agent can inherit.
    let handing_over = matches!(step, Some(Step::Agent { .. }));

    if let Some(step) = &step {
        // Said before acting: launching an app or moving the pointer changes the
        // screen, and an explanation arriving after that is just noise.
        //
        // A handover says nothing. Its sentence is a plan made before looking at
        // anything, and the agent's own first turn follows two seconds later --
        // so "opening WhatsApp Web" got announced and then contradicted by "who
        // should I message?".
        if !handing_over {
            speak(&app, step.say());
        }
        if !handing_over {
            let done = match perform(&app, step) {
                Ok(()) => perform_async(&app, step).await,
                Err(e) => Err(e),
            };
            if let Err(e) = done {
                app.emit("error", e.to_string()).ok();
            }
        }
    }

    // Finishing ends the session; being unsure does not -- open the right app and
    // tap the hotkey again and the same goal carries on.
    if matches!(&step, Some(Step::Done { .. } | Step::Reply { .. })) {
        app.state::<Nudge>().end();
    }

    // Handing the goal to an agent, in this order and not the other one.
    //
    // The agent opens its own session on a worker thread. Spawning first and
    // ending after is a race that this end() usually wins but sometimes does
    // not -- and when it lost, it wiped the session the agent had just opened,
    // so the agent's first look found nothing in flight and reported Done
    // immediately. A full progress bar, a cheerful title, and no work done.
    if let Some(Step::Agent {
        title,
        say,
        background,
    }) = &step
    {
        let goal = app.state::<Nudge>().goal();
        let carried = app.state::<Nudge>().history();
        app.state::<Nudge>().end();
        crate::app::agent::spawn(&app, goal, title.clone(), say.clone(), *background, carried);
        return Ok(step);
    }

    // Hand the goal over. The step itself was not performed -- see `handing_over`.
    //
    // Nothing drives the foreground loop except the user: the click watcher
    // advances a `Point`, and a tap of the hotkey advances anything. A `Launch`
    // produces no click, so a session that launched something simply sat there,
    // and every task in one test run did exactly one turn: look at the desktop,
    // open the app, stop. An app launcher, not an agent.
    //
    // `Point` is deliberately not here. That is guide mode -- the user asked
    // where something is, and the ring waits for them.
    // Named by what ENDS a turn, not by what continues one.
    //
    // The old list named the four steps that hand over, and every tool added
    // afterwards was missing from it -- a foreground `search` found the answer,
    // wrote it into the session, and stopped, because nothing drove the turn
    // that would have said it out loud. Listing the terminal cases instead means
    // the next tool is handled the day it is written.
    //
    // `Point` is terminal here on purpose: that is guide mode, where the ring
    // waits for the user to do it.
    let carries_on = !matches!(
        &step,
        None | Some(
            Step::Done { .. }
                | Step::Reply { .. }
                | Step::Unsure { .. }
                | Step::Question { .. }
                | Step::Point { .. }
                | Step::Agent { .. }
        )
    );
    if let Some(s) = step.as_ref().filter(|_| carries_on) {
        let goal = app.state::<Nudge>().goal();
        if !goal.is_empty() {
            // Carried over, not discarded -- see `Nudge::begin_agent`.
            let carried = app.state::<Nudge>().history();
            app.state::<Nudge>().end();
            crate::app::agent::spawn(
                &app,
                goal.clone(),
                goal,
                s.say().to_string(),
                false,
                carried,
            );
        }
    }

    // Acting uses global points; drawing uses the overlay's own. Converted here,
    // at the one place a step leaves Rust for the UI, so everything behind this
    // line stays in a single coordinate space.
    let step = step.map(|s| {
        let screen = app.state::<Screen>();
        s.map_point(|p| screen.to_overlay(p))
    });
    Ok(step)
}

/// End the session. `silence` decides whether the voice stops with it.
///
/// These look like one action and are not. Escape means "stop, I am taking over",
/// and must cut the voice off mid-word. The overlay clearing itself after a reply
/// means only that the bubble has been on screen long enough -- killing the
/// sentence there is how a long answer ends up chopped in half, which is exactly
/// what it sounded like.
#[tauri::command]
pub fn cancel(app: AppHandle, silence: bool) {
    if silence {
        speech::hush();
        // Escape is the key people already hit when a machine starts doing
        // something they did not expect. It has to stop the agent too.
        for a in app.state::<Agents>().list() {
            if !a.finished() {
                app.state::<Agents>().stop(a.id);
            }
        }
        crate::app::agent::publish(&app);
    }
    // Escape ends everything. A timed dismissal ends only the foreground: the
    // overlay clearing itself after a sentence has been on screen long enough
    // must not reach into a session an agent is driving.
    //
    // The bug: the first YouTube run opened the page, then the overlay's own
    // read-timer fired `cancel(false)`, which ended the session under the agent.
    // Its next look found nothing in flight and it reported Done, mid-task.
    if silence || !app.state::<Agents>().running() {
        app.state::<Nudge>().end();
    }
}

/// The steps that have to await something.
///
/// Split out rather than making the whole of `perform` async: every other action
/// is a system call that returns, and an `async` on all of them would be a
/// keyword with nothing behind it.
pub(crate) async fn perform_async(app: &AppHandle, step: &Step) -> Result<()> {
    if let Step::Task { task, .. } = step {
        // The subagent's own steps are performed through the very same code the
        // main loop uses, so a rule enforced there -- the shell allow-list, the
        // workspace boundary, asking before replacing -- holds for it too. A
        // second path would be a second place for a safety rule to be missing.
        let app2 = app.clone();
        let found = app
            .state::<Nudge>()
            .task(task, move |s| {
                let app = app2.clone();
                async move {
                    perform(&app, &s)?;
                    Box::pin(perform_async(&app, &s)).await?;
                    Ok(app.state::<Nudge>().last_note())
                }
            })
            .await?;

        eprintln!("task finished: {}", found.answer);
        let agents = app.state::<Agents>();
        for s in &found.steps {
            agents.record_run(format!("task · {s}"), String::new());
        }
        app.state::<Nudge>()
            .note(format!("A task agent reports:\n{}", found.answer));
        crate::app::agent::publish(app);
    }
    if let Step::Search { query, .. } = step {
        let found = app.state::<Nudge>().search(query).await?;
        eprintln!("searched {query:?} ({} chars)", found.len());
        app.state::<Nudge>()
            .note(format!("Searched for {query:?}:\n{found}"));
        app.state::<Agents>()
            .record_run(format!("search {query:?}"), found);
        crate::app::agent::publish(app);
    }
    if let Step::Fetch { url, .. } = step {
        let text = fetch::read(url).await?;
        eprintln!("fetched {url} ({} chars)", text.len());
        app.state::<Nudge>()
            .note(format!("Read {url}, which says:\n{text}"));
        app.state::<Agents>()
            .record_run(format!("fetch {url}"), text);
        crate::app::agent::publish(app);
    }
    Ok(())
}

/// Ask whether to replace a file, and hold the task until the answer comes.
///
/// Returns the error to raise when there is no agent to hold -- the foreground
/// has nowhere to wait, so there it stays a refusal.
fn ask_to_replace(app: &AppHandle, path: &std::path::Path, content: String) -> Result<()> {
    app.state::<Grants>()
        .asking
        .lock()
        .unwrap()
        .replace((path.to_path_buf(), content));
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string());
    let question = format!("{name} already exists. Shall I replace it?");

    if app.state::<Agents>().ask(question.clone()) {
        // Spoken here because this question is not a `Step` and never passes
        // through the agent loop's own asking path.
        speak(app, &question);
        app.emit("status", "asking").ok();
        crate::app::agent::publish(app);
        return Ok(());
    }
    Err(crate::error::Error::Click(format!(
        "{} already exists, and replacing it needs saying so out loud.",
        path.display()
    )))
}

/// Carry out one step. Shared by the foreground loop and the agent runtime, so
/// the two cannot drift into behaving differently.
pub(crate) fn perform(app: &AppHandle, step: &Step) -> Result<()> {
    match step {
        Step::Launch { app: name, .. } => {
            launch::open_app(name)?;
            app.state::<Settle>().after(AFTER_OPEN);
        }
        Step::Open { url, .. } => {
            launch::open_url(url)?;
            app.state::<Settle>().after(AFTER_OPEN);
        }
        Step::Type { text, submit, .. } => {
            keyboard::type_text(text, *submit)?;
            app.state::<Settle>().after(AFTER_CLICK);
        }
        // No advance() call here on purpose. Moving the pointer and pressing it
        // are both things the watchers already notice, so Nudge's own clicks
        // travel the same path a person's do and the two cannot drift.
        Step::Point { at, act, .. } => {
            match act {
                Act::Hover => click::move_to(*at),
                Act::DoubleClick => click::click(*at, 2),
                Act::Click => click::click(*at, 1),
            }?;
            app.state::<Settle>().after(AFTER_CLICK);
        }
        Step::Write { path, content, .. } => {
            let workspace = app.state::<Nudge>().workspace();
            let grants = app.state::<Grants>();
            let target = files::resolve(&workspace, path)?;
            let permitted = grants.granted.lock().unwrap().contains(&target);

            match files::write(&workspace, path, content, permitted)? {
                // Asked on the model's behalf; the task waits for the answer.
                files::Wrote::NeedsPermission { path } => {
                    return ask_to_replace(app, &path, content.clone())
                }
                files::Wrote::Done { path, backup } => {
                    // Spent: agreeing once is not agreeing forever.
                    grants.granted.lock().unwrap().remove(&path);
                    let note = match &backup {
                        Some(b) => format!(
                            "Wrote {} (the previous version is at {})",
                            path.display(),
                            b.display()
                        ),
                        None => format!("Wrote {}", path.display()),
                    };
                    eprintln!("{note}");
                    app.state::<Nudge>().note(note.clone());
                    let agents = app.state::<Agents>();
                    agents.record_run(format!("write {}", path.display()), note);
                    agents.record_file(path.display().to_string());
                    crate::app::agent::publish(app);
                }
            }
        }
        // Fetching has to wait for the network, so it happens in `perform_async`
        // rather than here.
        Step::Fetch { .. } => {}
        Step::Plan { todos, .. } => {
            use crate::core::run::agent::{Doing, Todo};
            let items: Vec<Todo> = todos
                .iter()
                .map(|(text, status)| Todo {
                    text: text.clone(),
                    status: match status.as_str() {
                        "active" | "in_progress" | "doing" => Doing::Active,
                        "done" | "completed" => Doing::Done,
                        _ => Doing::Pending,
                    },
                })
                .collect();
            app.state::<Agents>()
                .set_plan(items)
                .map_err(crate::error::Error::Click)?;
            crate::app::agent::publish(app);
        }
        Step::Workspace { path, .. } => {
            let moved = app.state::<Nudge>().move_to(path)?;
            eprintln!("workspace is now {}", moved.display());
            app.state::<Nudge>()
                .note(format!("Working in {} from now on.", moved.display()));
        }
        Step::Show { path, .. } => {
            // The same boundary as writing: a path from a model is a path that
            // has to be proven, and `open` on a file is a real action.
            let workspace = app.state::<Nudge>().workspace();
            let target = files::resolve(&workspace, path)?;
            if !target.is_file() {
                return Err(crate::error::Error::Click(format!(
                    "{} is not there to show",
                    target.display()
                )));
            }
            std::process::Command::new("open").arg(&target).spawn()?;
            app.state::<Settle>().after(AFTER_OPEN);
            app.state::<Agents>()
                .record_file(target.display().to_string());
            crate::app::agent::publish(app);
        }
        Step::Read {
            path, from, lines, ..
        } => {
            let text = files::read(&app.state::<Nudge>().workspace(), path, *from, *lines)?;
            eprintln!("read {path} @{from} ({} chars)", text.len());
            app.state::<Nudge>().note(format!("Read {path}:\n{text}"));
        }
        Step::Edit { path, old, new, .. } => {
            let workspace = app.state::<Nudge>().workspace();
            let grants = app.state::<Grants>();
            let target = files::resolve(&workspace, path)?;
            let permitted = grants.granted.lock().unwrap().contains(&target);

            match files::edit(&workspace, path, old, new, permitted)? {
                files::Wrote::NeedsPermission { path } => {
                    return ask_to_replace(app, &path, String::new())
                }
                files::Wrote::Done { path, backup } => {
                    grants.granted.lock().unwrap().remove(&path);
                    let note = match &backup {
                        Some(b) => format!(
                            "Edited {} (the previous version is at {})",
                            path.display(),
                            b.display()
                        ),
                        None => format!("Edited {}", path.display()),
                    };
                    eprintln!("{note}");
                    app.state::<Nudge>().note(note.clone());
                    let agents = app.state::<Agents>();
                    agents.record_run(format!("edit {}", path.display()), note);
                    agents.record_file(path.display().to_string());
                    crate::app::agent::publish(app);
                }
            }
        }
        Step::Start { command, .. } => {
            let workspace = app.state::<Nudge>().workspace();
            let id = app.state::<Background>().start(&workspace, command)?;
            let note = format!("Started `{command}` as {id}. Read its output with output.");
            eprintln!("{note}");
            app.state::<Nudge>().note(note.clone());
            app.state::<Agents>()
                .record_run(format!("start {command}"), note);
            super::super::agent::publish(app);
        }
        Step::Output { id, .. } => {
            let (text, alive) = app.state::<Background>().read(*id)?;
            let status = if alive { "still running" } else { "finished" };
            eprintln!("output of {id} ({status}, {} chars)", text.len());
            app.state::<Nudge>().note(format!(
                "Process {id} is {status}, and has printed:\n{}",
                if text.trim().is_empty() {
                    "(nothing yet)"
                } else {
                    &text
                }
            ));
        }
        Step::Kill { id, .. } => {
            app.state::<Background>().stop(*id)?;
            eprintln!("stopped {id}");
            app.state::<Nudge>().note(format!("Stopped process {id}."));
        }
        Step::Run { command, .. } => {
            // The output is the point, so it goes into the session's history
            // where the next turn reads it -- the same channel a screenshot uses
            // to report what happened.
            let out = shell::run(&app.state::<Nudge>().workspace(), command)?;
            eprintln!("$ {command}\n{out}");
            app.state::<Nudge>()
                .note(format!("Ran `{command}`, which printed:\n{out}"));
            // Also kept where a person can read it afterwards, not only where the
            // model can.
            app.state::<Agents>()
                .record_run(command.clone(), out.clone());
            crate::app::agent::publish(app);
        }
        Step::Press { keys, .. } => {
            keyboard::shortcut(keys)?;
            app.state::<Settle>().after(AFTER_CLICK);
        }
        _ => {}
    }
    Ok(())
}

/// Start the voice without blocking on it, and clear the indicator when the
/// audio actually stops rather than after a guessed duration.
pub(crate) fn speak(app: &AppHandle, line: &str) {
    app.emit("status", "speaking").ok();
    let mut cfg = app.state::<Nudge>().cfg.clone();
    app.state::<Voice>().get().apply(&mut cfg);
    let line = line.to_string();
    let done = app.clone();
    tauri::async_runtime::spawn(async move {
        speech::speak(&cfg, &line).await;
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
            if !speech::is_playing() {
                break;
            }
        }
        done.emit("status", "idle").ok();
    });
}
