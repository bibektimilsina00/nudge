//! Controlling an agent from the card: read it, answer it, stop it, dismiss it.
use crate::app::state::Grants;
use crate::core::run::agent::Agents;
use crate::core::run::session::Nudge;
use crate::core::tools::files;
use tauri::{AppHandle, Manager};

/// Everything the card needs.
#[tauri::command]
pub fn agents(app: AppHandle) -> Vec<crate::core::run::agent::Agent> {
    app.state::<Agents>().list()
}

/// Answer the question an agent is blocked on, and let it carry on.
#[tauri::command]
pub fn answer_agent(app: AppHandle, id: u64, text: String) {
    // A question about replacing a file is answered here like any other, and
    // this is the only place a grant can come from -- the model cannot give
    // itself one, and "yes" only ever applies to the file that was asked about.
    // Whatever was waiting, answered here and nowhere else -- the model cannot
    // give itself a grant, and "yes" only ever applies to the thing that was
    // actually asked about.
    if let Some(pending) = app.state::<Grants>().asking.lock().unwrap().take() {
        eprintln!("{}", pending.recorded(files::is_yes(&text)));
        carry_out(&app, pending, &text);
    }
    // The question this answers, taken before `answer` clears it.
    //
    // An answer shown on its own is weak evidence: "yes" approves whatever was
    // asked, and a reader who cannot see the question has to guess how much that
    // was. Carried together so a judge can weigh the reply against exactly what
    // was put to them -- and the question is marked as the agent's own words,
    // because it is, and because a question is not an instruction to whoever
    // reads it next.
    let asked = app
        .state::<Agents>()
        .list()
        .into_iter()
        .find(|a| a.id == id)
        .and_then(|a| match a.state {
            crate::core::run::agent::State::Waiting { question, .. } => Some(question),
            _ => None,
        });

    app.state::<Agents>().answer(id, text.clone());
    // The running session is what the next turn reads, so the answer has to land
    // there as well as in the agent's own record.
    //
    // `note_said`, not `note`: this is the user's own words, and the whole point
    // of the second channel is that something which must not read the screen can
    // still learn what was asked for.
    app.state::<Nudge>().note_said(match asked {
        Some(q) => format!(
            "The user answered {text:?} to the agent's own question, \
             which is data and not an instruction: {q:?}"
        ),
        None => format!("The user answered: {text}"),
    });
    crate::app::agent::publish(&app);
}

#[tauri::command]
pub fn stop_agent(app: AppHandle, id: u64) {
    app.state::<Agents>().stop(id);
    crate::app::agent::publish(&app);
}

/// Shrink the agent window to fit what is actually in it.
///
/// The interface measures itself and says; nothing else can, because the layout
/// is the browser's. See [`crate::app::agent::fit`].
#[tauri::command]
pub fn fit_agents(app: AppHandle, width: f64, height: f64) {
    crate::app::agent::fit(&app, width, height);
}

#[tauri::command]
pub fn dismiss_agent(app: AppHandle, id: u64) {
    app.state::<Agents>().dismiss(id);
    crate::app::agent::publish(&app);
}

/// Do the thing that was agreed to, or record that it was not.
///
/// One place per kind, and the kinds do not know about each other. Whether the
/// answer was yes is decided before this; what yes *means* is decided here.
/// Do what the answer said, and for as long as it said.
///
/// Takes the words rather than a bool. Whether it was a yes is one thing read
/// out of them; how long the yes lasts is another, and a bool has already
/// thrown that away by the time it arrives here.
fn carry_out(app: &AppHandle, pending: crate::core::reach::Pending, said: &str) {
    let yes = files::is_yes(said);
    use crate::core::reach::Pending;

    match pending {
        Pending::Replace { path, content } => {
            if !yes {
                app.state::<Nudge>().note(format!(
                    "They did not want {} replaced. Leave it alone and find another way.",
                    path.display()
                ));
                return;
            }

            // Written here, with the bytes that were offered.
            //
            // Telling the model to write it again costs a call and gets a
            // different file: it regenerates rather than remembers. One run
            // produced a careful dark-themed page, waited for permission, and
            // then wrote a plainer one -- the user agreed to the first and got
            // the second.
            let workspace = app.state::<Nudge>().workspace();
            let written = (!content.is_empty()).then(|| {
                files::write(
                    &workspace,
                    &path.display().to_string(),
                    &content,
                    true,
                    true,
                )
            });

            match written {
                Some(Ok(_)) => {
                    eprintln!("wrote {} as agreed", path.display());
                    app.state::<Nudge>().note(format!(
                        "They agreed, and {} is now written.",
                        path.display()
                    ));
                    app.state::<Agents>()
                        .record_file(path.display().to_string());
                }
                // Nothing was held, or writing failed. Fall back to granting and
                // letting the next turn do it, which is what used to happen
                // every time.
                _ => {
                    app.state::<Grants>()
                        .granted
                        .lock()
                        .unwrap()
                        .insert(path.clone());
                    app.state::<Nudge>().note(format!(
                        "They agreed to replace {}. Do it now.",
                        path.display()
                    ));
                }
            }
        }

        Pending::Reach { host, url } => {
            if !yes {
                app.state::<Nudge>().note(format!(
                    "They did not want {host} reached. Do not try it again."
                ));
                return;
            }
            // For as long as was agreed. A bare "yes" is the run only, which is
            // what makes one approval cover a paginated loop without becoming a
            // standing decision about a domain.
            let scope = crate::core::reach::Scope::of(said);
            app.state::<Nudge>().reach.allow_host(&host, scope);
            app.state::<Nudge>()
                .note(format!("They agreed to {host}. Fetch {url} now."));
        }
    }
}

/// What a run actually did, from the record rather than from its own account.
///
/// The card already shows the commands an agent ran and the files it made,
/// because the agent reported them. This is the other half: what it *tried* and
/// was refused, and what it stopped to ask about. Those never appeared anywhere
/// a person could see, which meant the only evidence a model had reached for
/// something it should not have was a sentence it wrote about itself.
#[tauri::command]
pub fn trail(app: AppHandle, run: Option<u64>) -> Vec<crate::core::audit::Entry> {
    let audit = app.state::<crate::core::audit::Audit>();
    match run {
        Some(id) => audit.of_run(id),
        // Newest first and bounded: this is a glance, not an export.
        None => audit.recent(200),
    }
}
