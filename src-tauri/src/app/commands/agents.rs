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
        let yes = files::is_yes(&text);
        eprintln!("{}", pending.recorded(yes));
        carry_out(&app, pending, yes);
    }
    app.state::<Agents>().answer(id, text.clone());
    // The running session is what the next turn reads, so the answer has to land
    // there as well as in the agent's own record.
    app.state::<Nudge>()
        .note(format!("The user answered: {text}"));
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
fn carry_out(app: &AppHandle, pending: crate::core::reach::Pending, yes: bool) {
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
            // Remembered for the rest of the run, so a page of results does not
            // ask once per page. Scope comes with 1.3.
            app.state::<Nudge>().reach.allow_host(&host);
            app.state::<Nudge>()
                .note(format!("They agreed to {host}. Fetch {url} now."));
        }
    }
}
