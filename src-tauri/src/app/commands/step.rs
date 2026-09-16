//! The loop, as the frontend sees it: ask, act, and stop.
//!
//! `perform` is the one place an action actually happens. Both the foreground
//! path and the agent runtime go through it, and a subagent's steps do too --
//! which is what makes the shell allow-list, the workspace boundary and
//! ask-before-replacing hold everywhere rather than in whichever caller
//! remembered them.
use crate::app::state::{permits, Background, Grants, Screen, Settle, Voice};
use crate::core::audit::Outcome;
use crate::core::provider::{Act, Step};
use crate::core::reach::Grant;
use crate::core::risk::Risk;
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
    // Where a turn begins when it came from a click. A spoken one was already
    // started by the hotkey, several stages ago, and this leaves that alone.
    app.state::<Nudge>().clock_in();
    app.emit("status", "thinking").ok();

    // Let whatever we just did finish happening before photographing the result.
    if let Some(wait) = app.state::<Settle>().remaining() {
        tokio::time::sleep(wait).await;
    }
    app.state::<Nudge>().mark("settle");

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
            // Judged here as well as in an agent run.
            //
            // This was agent-only, on the reasoning that a foreground step is
            // something the person asked for a second ago and is watching
            // happen. That reasoning did not survive contact: asked to write a
            // file holding a line count, the foreground wrote the number it read
            // off a stale terminal -- 63, when the file had 313 lines. They were
            // watching, and had no way to know the number came from the screen
            // rather than the file. Being present is not the same as being able
            // to check.
            match reviewed(&app, step).await {
                Some(stopped) => {
                    // The person sees the judge's own words, which were written
                    // for them. The model gets the sentence that teaches it
                    // nothing -- being in the room does not make the retry loop
                    // safe, and a reason handed back is a reason to try around.
                    eprintln!("step stopped by the reviewer");
                    app.emit("error", stopped).ok();
                }
                None => {
                    let done = match perform(&app, step) {
                        Ok(()) => perform_async(&app, step).await,
                        Err(e) => Err(e),
                    };
                    if let Err(e) = done {
                        // Two audiences, two sentences. The person gets what it
                        // means for what they asked; the log keeps the original,
                        // which is the one worth having when somebody looks.
                        let goal = app.state::<Nudge>().goal();
                        eprintln!("step failed: {e}");
                        app.emit("error", crate::error::plainly(&goal, &e)).ok();
                    }
                }
            }
        }
    }

    // It could not, and said what would have let it.
    //
    // Here rather than anywhere else because this is where a turn ends and
    // nothing is about to happen -- the one moment an offer is not an
    // interruption. Checked against the catalogue so a model that names
    // something imaginary is simply ignored.
    if let Some(Step::Unsure {
        needed: Some(want), ..
    }) = &step
    {
        offer_if_it_is_a_good_moment(&app, want);
    } else if matches!(&step, Some(Step::Done { .. } | Step::Reply { .. })) {
        // Nothing was asked for, and the turn is over. The only other moment
        // worth raising something in -- and held to a far stricter budget,
        // because this one is a guess rather than an answer.
        volunteer_if_it_is_ever_a_good_moment(&app);
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
    if let Step::Skill { name, .. } = step {
        match crate::core::skills::open(name) {
            Some(how) => {
                eprintln!("skill {name:?} -> {} chars", how.len());
                app.state::<Nudge>()
                    .note(format!("The {name:?} skill says:\n{how}"));
            }
            // Named rather than ignored, and with the list, because the usual
            // cause is a name half-remembered from the summary line.
            None => {
                let have = crate::core::skills::installed()
                    .iter()
                    .map(|s| s.name.clone())
                    .collect::<Vec<_>>()
                    .join(", ");
                app.state::<Nudge>().note(match have.is_empty() {
                    true => format!("There is no skill called {name:?}; none are installed."),
                    false => format!("There is no skill called {name:?}. There is: {have}."),
                });
            }
        }
    }
    if let Step::Remember { about, note, .. } = step {
        let said = app.state::<Nudge>().memory.learn(about, note);
        app.state::<Nudge>().note(said);
    }
    if let Step::Delegate { task, named, .. } = step {
        let (_, _, form) = crate::core::tools::running::choose(named.as_deref())?;
        let command = crate::core::tools::running::command_for(form, task);
        // The chosen agent is in the log, where somebody debugging this needs it,
        // and nowhere a user will meet it.
        eprintln!("delegating: {command}");
        let id = app
            .state::<Background>()
            .start(&app.state::<Nudge>().workspace(), &command)?;
        app.state::<Nudge>().note(format!(
            "The job was handed over and is running as {id}. Read what it has \
             printed with output, and report what was done rather than who did it."
        ));
        app.state::<Agents>()
            .record_run(format!("working: {task}"), String::new());
        crate::app::agent::publish(app);
    }
    if let Step::Request {
        method,
        url,
        headers,
        body,
        ..
    } = step
    {
        let said = fetch::request(
            method,
            url,
            headers,
            body.as_deref(),
            permits(app, Grant::Http).allowed(),
        )
        .await?;
        eprintln!("{method} {url} -> {} chars", said.len());
        app.state::<Nudge>()
            .note(format!("{method} {url} answered:\n{said}"));
        app.state::<Agents>()
            .record_run(format!("{method} {url}"), said);
        crate::app::agent::publish(app);
    }
    if let Step::Mcp { tool, args, .. } = step {
        // Which files it named, before it runs, because afterwards a replaced
        // file cannot tell you what it used to be.
        let workspace = app.state::<Nudge>().workspace();
        let touched = crate::core::tools::files::named(&workspace, args);

        let said = app.state::<Nudge>().run_tool(tool, args).await?;
        eprintln!("mcp {tool} -> {} chars", said.len());

        // A tool server is a second way to write a file, and it used to be the
        // one with no record: Nudge's own writes are gated, diffed and logged,
        // and a call to `files/write_file` was none of those. So the same
        // question gets the same answer whichever route it came in by.
        //
        // The arguments are never recorded. They carry the file's new contents,
        // and a log that copies them is the leak this log exists to avoid -- so
        // the entry is the tool's name and the paths it named, nothing else.
        let changed: Vec<String> = touched
            .iter()
            .filter_map(|p| {
                crate::core::tools::files::changed(p).map(|d| format!("{} ({d})", p.display()))
            })
            .collect();
        let named = match touched.is_empty() {
            true => tool.clone(),
            false => format!(
                "{tool} on {}",
                touched
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        };
        record(
            app,
            "tool",
            &named,
            Outcome::Did {
                detail: match changed.is_empty() {
                    true => format!("{} chars back", said.len()),
                    false => changed.join(", "),
                },
            },
            // External, whatever it is called. The name of somebody else's tool
            // is a claim, not evidence -- see `core::risk`.
            Risk::External,
        );

        // And the model is told what actually changed rather than only what the
        // tool said about itself -- one run read a terminal, decided the file
        // was already written, and reported success having done nothing.
        let mut note = format!("Ran {tool}, which said:\n{said}");
        if !changed.is_empty() {
            note.push_str(&format!("\n\n(git says: {})", changed.join(", ")));
        }
        app.state::<Nudge>().note(note);
        app.state::<Agents>()
            .record_run(format!("tool {tool}"), said);
        for p in &touched {
            app.state::<Agents>().record_file(p.display().to_string());
        }
        crate::app::agent::publish(app);
    }
    if let Step::Fetch { url, .. } = step {
        if let Some(pause) = ask_before_reaching(app, url) {
            return pause;
        }
        let text = match fetch::read(url).await {
            Ok(text) => {
                // The host and a measurement. Never the page -- that is the
                // whole rule, and a fetched page is the largest untrusted thing
                // this program handles.
                record(
                    app,
                    "fetch",
                    url,
                    Outcome::Did {
                        detail: format!("{} chars", text.len()),
                    },
                    Risk::Egress,
                );
                text
            }
            Err(e) => {
                record(
                    app,
                    "fetch",
                    url,
                    Outcome::Refused { why: e.to_string() },
                    Risk::Egress,
                );
                return Err(e);
            }
        };
        eprintln!("fetched {url} ({} chars)", text.len());
        app.state::<Nudge>()
            .note(format!("Read {url}, which says:\n{text}"));
        app.state::<Agents>()
            .record_run(format!("fetch {url}"), text);
        crate::app::agent::publish(app);
    }
    Ok(())
}

/// Write down one thing that happened, against the run it happened in.
///
/// A free function so a call site is one line and cannot forget the run id --
/// the entries worth having are the ones nobody remembered to add by hand.
/// Write down what happened, and what class of thing it was.
///
/// There is no version of this that omits the risk. A record that *can* omit it
/// is one that will, at whichever call site somebody adds next -- and the class
/// is the only thing that can honestly be said about a tool this repository has
/// never heard of.
fn record(app: &AppHandle, kind: &str, said: &str, outcome: Outcome, risk: Risk) {
    record_by(app, kind, said, outcome, risk, None);
}

/// The same, naming the grant that allowed it.
fn record_by(
    app: &AppHandle,
    kind: &str,
    said: &str,
    outcome: Outcome,
    risk: Risk,
    rule: Option<String>,
) {
    use crate::core::audit::{Audit, Entry};
    let entry = Entry::new(kind, said, outcome)
        .at_risk(risk)
        .allowed_by(rule);
    // Attributed to the run that is actually acting, which the runtime says
    // rather than this guessing. Guessing looked for an agent that had not
    // finished and lost exactly the records worth keeping: an agent stopped
    // mid-step is already finished by the time its refusal comes back.
    let entry = match app.state::<Agents>().doing() {
        Some(id) => entry.during(id),
        None => entry,
    };
    app.state::<Audit>().note(entry);
}

/// Should a person be asked before this host is reached?
///
/// **Only while an agent is running**, and that is the whole of the rule.
///
/// A foreground fetch is the person's own question, seconds after they asked it,
/// with a URL that came from what they said. There is nothing there to protect
/// them from, there is often nobody to ask -- the card only exists during a run
/// -- and a question in front of every one of them would be worse than the
/// screenshot fetching exists to avoid.
///
/// An agent run is the opposite on all three counts. Nobody is watching, the URL
/// may have come from something on screen rather than from the person, and there
/// is always a card. That is where the exfiltration risk actually lives.
///
/// Asked per host and remembered for the run, so a page of results does not ask
/// once per page. Returns `Some` only when the turn should stop and wait.
fn ask_before_reaching(app: &AppHandle, url: &str) -> Option<Result<()>> {
    let host = crate::core::tools::fetch::host_of(url)?;

    // Already agreed to this run, or nobody to ask because nothing is running.
    if app.state::<Nudge>().reach.host_allowed(&host) || !app.state::<Agents>().running() {
        return None;
    }

    let pending = crate::core::reach::Pending::Reach {
        host,
        url: url.to_string(),
    };
    record(
        app,
        "fetch",
        url,
        Outcome::Asked {
            question: pending.question(),
        },
        Risk::Egress,
    );
    Some(put_to_the_person(app, pending).map(|_| ()))
}

/// Put a connect offer on screen, if this is a reasonable moment for one.
///
/// Every reason not to is in `core::offers`; the only thing decided here is what
/// counts as busy, which needs the app to answer.
fn offer_if_it_is_a_good_moment(app: &AppHandle, want: &str) {
    let Some(offer) = crate::core::offers::catalogue()
        .into_iter()
        .find(|o| o.name.eq_ignore_ascii_case(want.trim()))
    else {
        eprintln!("offer: no service called {want:?} -- ignoring");
        return;
    };
    // Busy means an agent is working, and nothing else.
    //
    // It also asked whether a session was open, which is wrong in exactly the
    // case this exists for: `unsure` deliberately leaves the session open so the
    // same goal can be picked up again, so the one moment an offer is wanted was
    // the one moment it read as "mid-task" and said nothing. An open session is a
    // thing that can be resumed, not a person in the middle of something.
    let busy = app.state::<Agents>().running();
    let offers = app.state::<crate::core::offers::Offers>();
    if !offers.may_ask(&offer.name, busy) {
        eprintln!("offer: not asking about {} right now", offer.name);
        return;
    }
    offers.asked();
    app.state::<crate::app::state::Offering>()
        .set(offer.clone());
    crate::app::ui::connect::ask(app, &offer);
    eprintln!("offer: asking about {}", offer.name);
}

/// Raise something nobody asked about, on the rare occasion that is defensible.
///
/// Everything that decides is in `core::offers`; what is supplied here is the two
/// things it cannot know -- whether anything is running, and what is installed on
/// this machine.
fn volunteer_if_it_is_ever_a_good_moment(app: &AppHandle) {
    let offers = app.state::<crate::core::offers::Offers>();
    let busy = app.state::<Agents>().running();
    let here = |name: &str| {
        crate::core::tools::present::installed(name)
            || crate::core::screen::launch::installed_apps()
                .iter()
                .any(|a| a.eq_ignore_ascii_case(name))
    };
    // Switched off, nothing is raised unprompted. Asking is untouched -- this is
    // about Nudge starting the conversation, not about it being able to answer.
    if !app.state::<crate::app::state::Suggesting>().0.on() {
        return;
    }
    let Some(offer) = crate::core::offers::catalogue()
        .into_iter()
        .find(|o| offers.may_volunteer(o, busy, here))
    else {
        return;
    };
    offers.asked();
    offers.volunteered();
    app.state::<crate::app::state::Offering>()
        .set(offer.clone());
    crate::app::ui::connect::ask(app, &offer);
    eprintln!("offer: raising {} unprompted", offer.name);
}

/// A write landed: say what changed, tell everyone, write it down.
///
/// One place rather than two near-identical copies at the write and edit sites,
/// which is what let them drift apart: neither recorded anything in the audit,
/// so the log that exists to answer "what did it do to my files" was the one
/// log with no file changes in it.
///
/// `verb` is what the person reads ("Wrote", "Edited"); `kind` is what the
/// record is filed under ("write", "edit").
fn landed(
    app: &AppHandle,
    verb: &str,
    kind: &str,
    path: &std::path::Path,
    backup: Option<&std::path::Path>,
) {
    // Asked once. It spawns git, and the person and the record want the same
    // answer -- asking twice would be two subprocesses to disagree with.
    let stat = crate::core::tools::files::changed(path);

    // The diff first, because it is the part that is checked rather than
    // claimed, and a sentence is read left to right.
    let mut note = match &stat {
        Some(stat) => format!("{verb} {} ({stat})", path.display()),
        None => format!("{verb} {}", path.display()),
    };
    if let Some(b) = backup {
        note.push_str(&format!(" (the previous version is at {})", b.display()));
    }
    eprintln!("{note}");
    app.state::<Nudge>().note(note.clone());

    // The path and how much of it moved. Never the content -- a file's contents
    // are the whole reason this log is not allowed to copy what it sees.
    record(
        app,
        kind,
        &path.display().to_string(),
        Outcome::Did {
            detail: stat.unwrap_or_else(|| "untracked".into()),
        },
        Risk::WriteLocal,
    );

    let agents = app.state::<Agents>();
    agents.record_run(format!("{kind} {}", path.display()), note);
    agents.record_file(path.display().to_string());
    crate::app::agent::publish(app);
}

/// Does this step name a file this run wrote?
///
/// The list of files a run made is background; this is the thing worth saying.
/// "setup.py exists" and "the command you are judging runs a file this task
/// wrote one step ago" are different claims, and only the second is worth
/// interrupting anybody over.
///
/// Looks at what the step *says*, not at what it does: the command line for a
/// shell step, the arguments for a tool call. Nothing here opens a file.
fn ours(app: &AppHandle, step: &Step) -> Option<String> {
    use crate::core::provenance::{names_ours, Ours};

    let said = match step {
        Step::Run { command, .. } | Step::Start { command, .. } => command.clone(),
        Step::Delegate { task, .. } => task.clone(),
        // The values, not the keys -- a path arrives as an argument's value.
        Step::Mcp { args, .. } => args.to_string(),
        Step::Read { path, .. } | Step::Show { path, .. } => path.clone(),
        _ => return None,
    };

    let agents = app.state::<Agents>();
    let id = agents.doing()?;
    let made: Vec<Ours> = agents
        .list()
        .into_iter()
        .find(|a| a.id == id)
        .map(|a| {
            a.made
                .iter()
                .map(|m| Ours {
                    path: m.path.clone(),
                    step: m.step,
                })
                .collect()
        })
        .unwrap_or_default();
    let now = agents.list().iter().find(|a| a.id == id).map(|a| a.step)?;
    names_ours(&said, &made, now)
}

/// Put one proposed action to a judge that never saw the screen.
///
/// `None` means carry on: the step has no consequences worth judging, or this
/// provider does not review. Anything else is the judge's own sentence, written
/// for a person.
///
/// Consulted for foreground steps and agent steps alike. It was agent-only at
/// first, on the reasoning that a foreground step is watched as it happens --
/// which did not survive being tested. What differs between the two is only who
/// reads the reason: a person can be shown it, an agent is handed
/// [`judge::REFUSED`] instead, because a reason given to the thing that proposed
/// the action is a reason to try around it.
pub async fn reviewed(app: &AppHandle, step: &Step) -> Option<String> {
    use crate::core::judge::{self, Verdict};

    let risk = crate::core::risk::of(step);
    if !risk.consequential() {
        return None;
    }
    if !app.state::<crate::app::state::Reviewing>().0.on() {
        return None;
    }
    let nudge = app.state::<Nudge>();
    let provider = nudge.answering();
    if !provider.aside() {
        // Not reviewing is not a failure. Nudge behaves exactly as it did before
        // there was a judge, which is the honest thing for a defence that can
        // only ever tighten -- see `Provider::reviews`.
        return None;
    }

    let world = judge::World {
        workspace: nudge.workspace().display().to_string(),
        granted: nudge
            .reach
            .granted()
            .into_iter()
            .map(|g| g.told().to_string())
            .collect(),
        // Where things could be sent. One git call, against a model call that
        // takes two and a half seconds -- not worth caching, and a cache would
        // have to notice the workspace moving.
        remotes: crate::core::tools::files::remotes(&nudge.workspace()),
        // Named, never opened. A script the user asked for is ordinary work;
        // running one the agent wrote for reasons of its own is not, and the
        // effects of a file cannot be read off the command that runs it.
        made: app
            .state::<Agents>()
            .doing()
            .map(|id| app.state::<Agents>().files_of(id))
            .unwrap_or_default(),
        // The observation, when there is one: this action names one of them.
        note: ours(app, step),
    };

    // Written down whether or not the judge minds. The record is what somebody
    // reads afterwards to understand a run, and "it ran a file it had written
    // itself" is one of the few facts that changes how everything else reads.
    if let Some(note) = &world.note {
        record(
            app,
            "wrote-then-ran",
            &judge::shown(step),
            Outcome::Did {
                detail: note.clone(),
            },
            risk,
        );
    }

    let asked = judge::prompt(&nudge.said(), &world, step, risk);
    let began = std::time::Instant::now();
    let verdict = match provider.ask_aside(&asked).await {
        Ok(reply) => judge::read(&reply),
        // It promised to review and could not. That is a question, not a pass:
        // the alternative is a defence that disappears whenever the network does.
        Err(e) => Verdict::unreachable(format!("the reviewer could not be reached: {e}")),
    };

    if verdict.agreed() {
        // Said out loud. This costs a model call, and a charge nobody can see
        // is a charge somebody finds on a bill -- the rest of this log prints
        // every turn and every command, so one line for a step that was checked
        // is the same density.
        eprintln!(
            "reviewer: agreed in {:.1}s -- {}",
            began.elapsed().as_secs_f32(),
            judge::shown(step)
        );
        return None;
    }

    eprintln!(
        "reviewer: {} in {:.1}s -- {}",
        match verdict.broken() {
            true => "unreachable",
            false => "stopped a step",
        },
        began.elapsed().as_secs_f32(),
        verdict.why()
    );
    record(
        app,
        "reviewer",
        &judge::shown(step),
        Outcome::Refused {
            why: verdict.why().to_string(),
        },
        risk,
    );

    Some(verdict.why().to_string())
}

/// Ask whether to replace a file, and hold the task until the answer comes.
///
/// Returns the error to raise when there is no agent to hold -- the foreground
/// has nowhere to wait, so there it stays a refusal.
fn ask_to_replace(app: &AppHandle, path: &std::path::Path, content: String) -> Result<()> {
    put_to_the_person(
        app,
        crate::core::reach::Pending::Replace {
            path: path.to_path_buf(),
            content,
        },
    )
    .map_err(|_| {
        crate::error::Error::Click(format!(
            "{} already exists, and replacing it needs saying so out loud.",
            path.display()
        ))
    })
}

/// Put a pending thing to the person and pause on it.
///
/// One path for every kind of question, so that "yes" means the same thing
/// whichever gate asked. It used to exist only for file replacement and was
/// shaped like it; the next question that needed a person would have arrived as
/// a second copy.
///
/// Fails when there is nobody to ask -- no agent is running, so there is no card
/// to put it on and nothing to resume afterwards. The caller turns that into a
/// sentence about its own case.
fn put_to_the_person(app: &AppHandle, pending: crate::core::reach::Pending) -> Result<()> {
    let question = pending.question();
    let choices = pending.choices();
    app.state::<Grants>()
        .asking
        .lock()
        .unwrap()
        .replace(pending);

    if app.state::<Agents>().asking(question.clone(), choices) {
        // Spoken here because this question is not a `Step` and never passes
        // through the agent loop's own asking path.
        speak(app, &question);
        app.emit("status", "asking").ok();
        crate::app::agent::publish(app);
        return Ok(());
    }
    // Nobody to ask: clear the slot rather than leaving a question standing that
    // no card will ever show, or the next run inherits somebody else's.
    app.state::<Grants>().asking.lock().unwrap().take();
    Err(crate::error::Error::Click("there is nobody to ask".into()))
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
        Step::Point {
            at, act, control, ..
        } => {
            // Ask the application to press it, rather than sending the pointer
            // to where it is and clicking.
            //
            // The pointer is the user's. Taking it away to press a button they
            // can see perfectly well is the rudest thing this program does, and
            // for anything the system has named it is also unnecessary --
            // measured at 157ms, with the cursor not moving by a pixel.
            //
            // Only a plain click. A double click is not two presses, and a hover
            // is a request to put the pointer somewhere, which is the one case
            // where moving it is the entire point.
            let asked = matches!(act, Act::Click)
                && control.as_deref().is_some_and(|label| {
                    let done = crate::core::screen::privacy::frontmost_window()
                        .is_some_and(|(pid, _, _)| crate::core::screen::ax::press(pid, label));
                    // Said out loud, because the two failures look identical from
                    // outside: a press that is refused falls back to a click, and
                    // a press that succeeds while doing nothing does not.
                    eprintln!(
                        "  {} {label:?}",
                        if done {
                            "pressed"
                        } else {
                            "press refused, clicking"
                        }
                    );
                    done
                });

            // Plenty of controls decline to be pressed, and a refusal is silent,
            // so the click is still there underneath.
            if !asked {
                match act {
                    Act::Hover => click::move_to(*at),
                    Act::DoubleClick => click::click(*at, 2),
                    Act::Click => click::click(*at, 1),
                }?;
            }
            app.state::<Settle>().after(AFTER_CLICK);
        }
        Step::Write { path, content, .. } => {
            let workspace = app.state::<Nudge>().workspace();
            let grants = app.state::<Grants>();
            let target = files::resolve(&workspace, path, permits(app, Grant::Files).allowed())?;
            let permitted = grants.granted.lock().unwrap().contains(&target);

            match files::write(
                &workspace,
                path,
                content,
                permitted,
                permits(app, Grant::Files).allowed(),
            )? {
                // Asked on the model's behalf; the task waits for the answer.
                files::Wrote::NeedsPermission { path } => {
                    return ask_to_replace(app, &path, content.clone())
                }
                files::Wrote::Done { path, backup } => {
                    // Spent: agreeing once is not agreeing forever.
                    grants.granted.lock().unwrap().remove(&path);
                    landed(app, "Wrote", "write", &path, backup.as_deref());
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
            let target = files::resolve(&workspace, path, permits(app, Grant::Files).allowed())?;
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
            let workspace = app.state::<Nudge>().workspace();
            let anywhere = permits(app, Grant::Files).allowed();
            // Resolved first either way, so a document outside the workspace is
            // refused by the same rule as a text file outside it. Being a PDF is
            // not a way round the boundary.
            let target = files::resolve(&workspace, path, anywhere)?;

            let text = match crate::core::tools::paper::is_paper(&target) {
                // Whole, and not by line. A PDF has pages rather than lines, and
                // `from`/`lines` mean nothing in one -- honouring them would be
                // inventing a coordinate system the document does not have.
                true => crate::core::tools::paper::read(&target)?,
                false => files::read(&workspace, path, *from, *lines, anywhere)?,
            };
            eprintln!("read {path} @{from} ({} chars)", text.len());
            app.state::<Nudge>().note(format!("Read {path}:\n{text}"));
        }
        Step::Edit { path, old, new, .. } => {
            let workspace = app.state::<Nudge>().workspace();
            let grants = app.state::<Grants>();
            let target = files::resolve(&workspace, path, permits(app, Grant::Files).allowed())?;
            let permitted = grants.granted.lock().unwrap().contains(&target);

            match files::edit(
                &workspace,
                path,
                old,
                new,
                permitted,
                permits(app, Grant::Files).allowed(),
            )? {
                files::Wrote::NeedsPermission { path } => {
                    return ask_to_replace(app, &path, String::new())
                }
                files::Wrote::Done { path, backup } => {
                    grants.granted.lock().unwrap().remove(&path);
                    landed(app, "Edited", "edit", &path, backup.as_deref());
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
        Step::Await { id, .. } => {
            // Long, because the point is to cover the whole wait in one turn.
            // Capped anyway by `MAX_LIFETIME` inside `running`, which kills a job
            // that never finishes -- so this is how long an *agent* waits, not
            // how long a process is allowed to live.
            const PATIENCE: std::time::Duration = std::time::Duration::from_secs(10 * 60);
            let agents = app.state::<Agents>();
            let p = app
                .state::<Background>()
                // Checked while waiting. Without it Escape would do nothing for
                // ten minutes, which is indistinguishable from a hung app.
                .settle(*id, PATIENCE, || agents.stopping())?;

            let status = match (p.alive, p.code) {
                (true, _) => "still going after ten minutes".to_string(),
                (false, Some(0)) => "finished successfully".to_string(),
                (false, Some(c)) => format!("failed with code {c}"),
                (false, None) => "stopped".to_string(),
            };
            eprintln!("waited for {id}: {status}");
            app.state::<Nudge>().note(format!(
                "Waited for process {id}. It {status}. Its last output:\n{}",
                match p.fresh.trim().is_empty() {
                    true => "(nothing)",
                    false => &p.fresh,
                }
            ));
            app.state::<Agents>()
                .record_run(format!("waited for {id}"), status);
            crate::app::agent::publish(app);
        }
        Step::Output { id, .. } => {
            // Waits rather than returning nothing. A turn costs a model call and
            // several seconds, so polling an agent that takes two minutes would
            // burn thirty of them saying "still nothing".
            const PATIENCE: std::time::Duration = std::time::Duration::from_secs(20);
            let p = app.state::<Background>().read(*id, PATIENCE)?;

            let status = match (p.alive, p.code) {
                (true, _) => "still running".to_string(),
                (false, Some(0)) => "finished successfully".to_string(),
                (false, Some(c)) => format!("failed with code {c}"),
                (false, None) => "stopped".to_string(),
            };
            eprintln!("output of {id} ({status}, {} new chars)", p.fresh.len());
            app.state::<Nudge>().note(format!(
                "Process {id} is {status}. New output since last time:\n{}",
                if p.fresh.trim().is_empty() {
                    "(nothing new)"
                } else {
                    &p.fresh
                }
            ));
            if !p.fresh.trim().is_empty() {
                app.state::<Agents>()
                    .record_run(format!("output {id}"), p.fresh.clone());
                super::super::agent::publish(app);
            }
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
            let gate = permits(app, Grant::Shell);
            let out = match shell::run(&app.state::<Nudge>().workspace(), command, gate.allowed()) {
                Ok(out) => {
                    // The command and how much it printed. Never the output --
                    // a command's output is a file's contents by another route.
                    record_by(
                        app,
                        "shell",
                        command,
                        Outcome::Did {
                            detail: format!("{} chars", out.len()),
                        },
                        Risk::Exec,
                        gate.rule.clone(),
                    );
                    out
                }
                Err(e) => {
                    // The refusals are the entries worth having. This is the
                    // record of a model trying something it was not allowed to,
                    // which is the thing nobody could see before.
                    record(
                        app,
                        "shell",
                        command,
                        Outcome::Refused { why: e.to_string() },
                        Risk::Exec,
                    );
                    return Err(e);
                }
            };
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
