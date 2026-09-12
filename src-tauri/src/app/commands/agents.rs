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
    if let Some(path) = app.state::<Grants>().asking.lock().unwrap().take() {
        let yes = files::is_yes(&text);
        eprintln!(
            "replace {}: {}",
            path.display(),
            if yes { "granted" } else { "refused" }
        );
        if yes {
            app.state::<Grants>()
                .granted
                .lock()
                .unwrap()
                .insert(path.clone());
            // Said plainly in the history, because the next turn has to know to
            // try the write again -- "they answered: yes" on its own does not
            // say what to do with it.
            app.state::<Nudge>().note(format!(
                "They agreed to replace {}. Write it again now.",
                path.display()
            ));
        } else {
            app.state::<Nudge>().note(format!(
                "They did not want {} replaced. Leave it alone and find another way.",
                path.display()
            ));
        }
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

#[tauri::command]
pub fn dismiss_agent(app: AppHandle, id: u64) {
    app.state::<Agents>().dismiss(id);
    crate::app::agent::publish(&app);
}
