//! The switches and the window plumbing. Nothing here decides anything; it
//! translates a click in the panel into a call and back.
use crate::app::state::{Docked, Voice};
use crate::core::run::session::Nudge;
use crate::core::tools::files;
use crate::error::Result;
use tauri::{AppHandle, Emitter, Manager};

/// Is the companion parked in the panel?
#[tauri::command]
pub fn docked(app: AppHandle) -> bool {
    app.state::<Docked>().0.on()
}

/// Park the companion in the panel, or release it onto the screen.
#[tauri::command]
pub fn set_docked(app: AppHandle, docked: bool) {
    app.state::<Docked>().0.set(docked);
    // Both windows draw the companion, so both need to know which of them owns it.
    app.emit("docked", docked).ok();

    // The panel window is not a panel: it is the notch, and the notch is the only
    // way back into Nudge.
    //
    // Releasing the companion used to hide this window, which read as "get the
    // settings sheet out of the way" and actually meant "delete the dock". The
    // pill went with it, docking again never called `show`, and the companion had
    // nowhere to sit -- so it vanished from the cursor and from the notch at
    // once, with nothing left to click.
    //
    // The sheet collapsing is the frontend's job and it already does it. This
    // window stays.
    if let Some(panel) = crate::app::ui::panel::window(&app) {
        let _ = panel.show();
        #[cfg(target_os = "macos")]
        crate::app::ui::native::float_everywhere(&panel);
    }
}

/// Which voice is in use: "off", "system" or "gemini".
#[tauri::command]
pub fn voice_mode(app: AppHandle) -> &'static str {
    match app.state::<Voice>().get() {
        crate::app::state::VoiceMode::Off => "off",
        crate::app::state::VoiceMode::System => "system",
        crate::app::state::VoiceMode::Gemini => "gemini",
    }
}

/// Change it. Unknown names are ignored rather than guessed at.
#[tauri::command]
pub fn set_voice_mode(app: AppHandle, mode: String) {
    let picked = match mode.as_str() {
        "off" => crate::app::state::VoiceMode::Off,
        "system" => crate::app::state::VoiceMode::System,
        "gemini" => crate::app::state::VoiceMode::Gemini,
        _ => return,
    };
    app.state::<Voice>().set(picked);
    // The menu bar shows the same setting and has to follow.
    app.emit("voice-mode", mode).ok();
}

/// Which character the companion is wearing.
#[tauri::command]
pub fn look(app: AppHandle) -> String {
    app.state::<crate::app::state::Look>().get()
}

/// Wear a different one.
///
/// Broadcast rather than addressed, because both windows draw the companion and
/// which of them owns it changes with docking -- the same reason `docked` is
/// emitted to everything.
#[tauri::command]
pub fn set_look(app: AppHandle, key: String) {
    app.state::<crate::app::state::Look>().set(&key);
    app.emit("look", key).ok();
}

/// The microphone recording will use.
#[tauri::command]
pub fn microphone() -> String {
    crate::core::voice::input_name().unwrap_or_else(|| "No input device".into())
}

/// Version, for the footer.
#[tauri::command]
pub fn version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}

#[tauri::command]
pub fn quit(app: AppHandle) {
    app.exit(0);
}

/// Tell the hover test how big the panel currently is.
#[tauri::command]
pub fn set_open_size(w: f64, h: f64) {
    crate::app::ui::notch::set_open_size(w, h);
}

/// The real notch's height in points, so the docked pill can match the hardware
/// exactly rather than guessing at it. Sync, so it runs on the main thread --
/// `measure` needs AppKit.
#[tauri::command]
pub fn notch_height() -> f64 {
    crate::app::ui::notch::measure().height
}

/// Open a file an agent made, in whatever app owns it.
///
/// Checked against the workspace rather than trusted: this is reachable from the
/// UI, and a path arriving from anywhere is a path that has to be proven.
#[tauri::command]
pub fn open_artifact(app: AppHandle, path: String) -> Result<()> {
    let workspace = app.state::<Nudge>().cfg.workspace_dir();
    let resolved = files::resolve(&workspace, &path, false)?;
    if !resolved.is_file() {
        return Err(crate::error::Error::Click(format!(
            "{} is not there any more",
            resolved.display()
        )));
    }
    std::process::Command::new("open").arg(&resolved).spawn()?;
    Ok(())
}

/// Let the panel size itself to its content.
#[tauri::command]
pub fn fit_panel(app: AppHandle, height: f64) {
    crate::app::ui::panel::fit(&app, height);
}

/// Every skill installed, for the skills window.
#[tauri::command]
pub fn skills() -> Vec<crate::core::skills::Skill> {
    crate::core::skills::installed()
}

/// Open the skills folder in Finder, making it if it is not there yet.
///
/// This is the whole of "load a skill": drop a folder in. No importer, no
/// archive format, no registry -- the unit is a directory with a `SKILL.md`, which
/// is a thing people already have and already know how to copy.
#[tauri::command]
pub fn open_skills_folder() -> Result<()> {
    let Some(dir) = crate::core::skills::folder() else {
        return Err(crate::error::Error::Config("no home directory".into()));
    };
    std::fs::create_dir_all(&dir)?;
    // A folder somebody opens for the first time and finds empty teaches nothing.
    let example = dir.join("example-skill/SKILL.md");
    if crate::core::skills::installed().is_empty() && !example.exists() {
        if let Some(parent) = example.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&example, EXAMPLE)?;
    }
    std::process::Command::new("open").arg(&dir).spawn()?;
    Ok(())
}

/// Written once, into an empty folder, so the format is obvious from the folder
/// rather than from documentation nobody opened.
const EXAMPLE: &str = "---\n\
    name: Example skill\n\
    description: Delete this folder once you have written a real one.\n\
    ---\n\n\
    Write the steps here, the way you would tell a capable person who cannot see\n\
    your screen. Nudge reads the name and the description above on every turn, and\n\
    only reads this part when it decides this is the skill you meant.\n\n\
    A skill is just a folder with a SKILL.md in it, which is the same shape Claude\n\
    Code and other agents use -- so one you already have will work here.\n";

/// The offer being made right now, if any.
///
/// Asked by the connect window when it loads, because a window that missed the
/// event that opened it would come up empty -- and the event is emitted before
/// anybody can be sure the webview is listening.
#[tauri::command]
pub fn pending_offer(app: AppHandle) -> Option<crate::app::ui::connect::Offer> {
    app.state::<crate::app::state::Offering>().current()
}

/// What they said.
///
/// Nothing is remembered yet. *No* should mean never and *Not now* should mean
/// this week, and both want somewhere to write it down -- which is the same
/// question as when to ask in the first place, still open on purpose.
#[tauri::command]
pub fn answer_offer(app: AppHandle, service: String, said: String) {
    use crate::core::offers::Said;
    let said = match said.as_str() {
        "yes" => Said::Yes,
        "no" => Said::No,
        // Anything unrecognised is the cautious answer, not the permanent one.
        _ => Said::Later,
    };
    app.state::<crate::core::offers::Offers>().answered(&service, said);
    app.state::<crate::app::state::Offering>().clear();
    crate::app::ui::connect::hide(&app);
}

/// A screenshot to attach, with Nudge itself out of the way.
///
/// The panel is over the very thing they are trying to show, so it steps out
/// before the shutter and comes back after. The pause is for the compositor
/// rather than for show -- ordering a window out and grabbing in the same breath
/// catches it still on screen.
///
/// Smaller than the model's capture on purpose. This is going into a JSON body
/// over somebody's connection, and a bug is legible at 1600px.
#[tauri::command]
pub async fn shot_for_report(app: AppHandle) -> std::result::Result<String, String> {
    let panel = crate::app::ui::panel::window(&app);
    if let Some(win) = panel.clone() {
        let _ = win.hide();
    }
    tokio::time::sleep(std::time::Duration::from_millis(220)).await;

    let shot = tokio::task::spawn_blocking(|| crate::core::screen::capture::grab(1600))
        .await
        .map_err(|e| e.to_string())?;

    if let Some(win) = panel {
        let _ = win.show();
        #[cfg(target_os = "macos")]
        crate::app::ui::native::float_everywhere(&win);
    }

    let shot = shot.map_err(|e| e.to_string())?;
    Ok(as_data_url(&shot.bytes))
}

/// Bytes to something an `<img src>` and a JSON body can both carry.
fn as_data_url(bytes: &[u8]) -> String {
    use base64::Engine;
    format!(
        "data:image/jpeg;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )
}

/// Send a bug report or a feature request.
///
/// Errors come back as a sentence rather than a code, because the only reader is
/// somebody who just tried to tell you something is broken and deserves to know
/// whether it arrived.
#[tauri::command]
pub async fn send_report(
    app: AppHandle,
    kind: String,
    text: String,
    image: Option<String>,
) -> std::result::Result<(), String> {
    use crate::core::report::{worth_sending, Kind, Report, MOST_IMAGE};

    let Some(kind) = Kind::parse(&kind) else {
        return Err("Unknown report type.".into());
    };
    if !worth_sending(&text) {
        return Err("Write a line about it first.".into());
    }
    if let Some(img) = &image {
        if img.len() > MOST_IMAGE {
            return Err("That image is too big to send. Try a smaller one.".into());
        }
    }

    let url = {
        let nudge = app.state::<crate::core::run::session::Nudge>();
        let url = nudge.cfg.report_url.clone().unwrap_or_default();
        if url.trim().is_empty() {
            return Err(
                "Reporting is not set up yet: add report_url to ~/.config/nudge/config.toml".into(),
            );
        }
        url
    };

    let report = Report {
        kind,
        text: text.trim().to_string(),
        image,
        version: app.package_info().version.to_string(),
        os: os_line(),
    };

    report.post(&url).await.map_err(|e| {
        // The URL can carry a token, and an error from the HTTP layer prints
        // whatever it was given. Same reason the provider errors are redacted.
        crate::core::tools::secret::redact(&format!("Could not send it: {e}"))
    })
}

/// Something a maintainer can act on, in one line.
fn os_line() -> String {
    #[cfg(target_os = "macos")]
    {
        let out = std::process::Command::new("/usr/bin/sw_vers")
            .arg("-productVersion")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok());
        match out {
            Some(v) if !v.trim().is_empty() => format!("macOS {}", v.trim()),
            _ => "macOS".into(),
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        std::env::consts::OS.to_string()
    }
}

/// One of the things Nudge may or may not be allowed to do.
#[derive(serde::Serialize)]
pub struct Allowed {
    pub key: String,
    pub label: String,
    pub about: String,
    pub on: bool,
}

/// What Nudge is allowed to do right now.
///
/// The same three grants the menu bar shows, from the same state, so the two
/// cannot disagree. The panel is where somebody goes looking for a setting; the
/// menu bar is where somebody flips one mid-run. Neither is the source of truth --
/// `Nudge::reach` is.
#[tauri::command]
pub fn reach(app: AppHandle) -> Vec<Allowed> {
    let nudge = app.state::<crate::core::run::session::Nudge>();
    crate::core::reach::Grant::ALL
        .iter()
        .map(|g| Allowed {
            key: g.key().into(),
            label: g.menu().into(),
            about: g.told().into(),
            on: nudge.reach.has(*g),
        })
        .collect()
}

#[tauri::command]
pub fn set_reach(app: AppHandle, key: String, on: bool) {
    let Some(g) = crate::core::reach::Grant::ALL.iter().find(|g| g.key() == key) else {
        return;
    };
    app.state::<crate::core::run::session::Nudge>()
        .reach
        .set(*g, on);
    // The menu bar shows the same three checkboxes and would otherwise keep
    // showing the old answer until something else rebuilt it.
    let hotkey = app
        .state::<crate::core::run::session::Nudge>()
        .cfg
        .hotkey
        .clone();
    crate::app::ui::tray::refresh(&app, &hotkey);
}

/// One option in a picker, and what choosing it means.
///
/// The cost or the catch travels with the option rather than sitting in a
/// paragraph above the group. "Gemini" and "Gemini — a round trip and a charge per
/// step" are different choices, and only one of them can be made honestly.
#[derive(serde::Serialize)]
pub struct Pick {
    pub key: String,
    pub label: String,
    pub about: String,
}

fn pick(key: &str, label: &str, about: &str) -> Pick {
    Pick {
        key: key.into(),
        label: label.into(),
        about: about.into(),
    }
}

/// Which model is answering, and how hard it is thinking.
#[derive(serde::Serialize)]
pub struct Brain {
    pub provider: String,
    pub model: String,
    pub think: String,
    pub workspace: String,
    pub providers: Vec<Pick>,
    pub thinks: Vec<Pick>,
}

/// The "bring your own model" settings, read-only.
///
/// Shown rather than edited: the config file is the feature, and a panel that
/// half-edits it would be a second place to look with a subset of the answers.
/// Seeing which model is answering is the part people actually need at a glance --
/// "why is this slow" and "why did that cost money" both start here.
#[tauri::command]
pub fn brain(app: AppHandle) -> Brain {
    let nudge = app.state::<crate::core::run::session::Nudge>();
    let now = nudge.tuning();
    Brain {
        provider: now.provider.clone(),
        model: nudge
            .cfg
            .model
            .clone()
            .unwrap_or_else(|| "its default model".to_string()),
        think: now.think.clone().unwrap_or_else(|| "default".into()),
        workspace: nudge
            .cfg
            .workspace
            .clone()
            .unwrap_or_else(|| "everywhere — set a workspace".into()),
        // Written as the trade being made, because that is the only thing that
        // distinguishes them to somebody who has not read the code.
        providers: vec![
            pick("ollama", "Ollama", "On your machine. Free, private, slower."),
            pick("gemini", "Gemini", "Fast and cheap. A round trip per step."),
            pick("anthropic", "Anthropic", "Strongest at reading a screen. Dearest."),
        ],
        // The single most expensive line in the config, so it says so. Thinking
        // bills at the output rate -- five times input -- which outweighs the
        // choice of model.
        thinks: vec![
            pick("low", "Brief", "No thinking tokens at all. Cheapest by far."),
            pick("medium", "Considered", "Hundreds to thousands of extra tokens a step."),
            pick("default", "The model's own", "Whatever it does when not told."),
        ],
    }
}

/// Change which model answers, or how hard it thinks.
///
/// Either argument alone: the picker that was touched sends its value and the
/// other keeps whatever it had, so two controls do not have to agree about what
/// the other is showing.
#[tauri::command]
pub fn retune(
    app: AppHandle,
    provider: Option<String>,
    think: Option<String>,
) -> std::result::Result<(), String> {
    let nudge = app.state::<crate::core::run::session::Nudge>();
    let mut want = nudge.tuning();
    if let Some(p) = provider {
        want.provider = p;
    }
    if let Some(t) = think {
        // "default" is the absence of the setting, not a value for it.
        want.think = (t != "default").then_some(t);
    }
    nudge.retune(want).map_err(|e| {
        crate::core::tools::secret::redact(&crate::error::plainly("change the model", &e))
    })
}

/// A tool server, as the interface shows it.
#[derive(serde::Serialize)]
pub struct Server {
    /// Whatever it was named in the config, which may be anything.
    pub name: String,
    /// What is actually running, so the name does not have to carry the meaning.
    pub about: String,
    /// Connected and offering this many, or why not.
    pub said: String,
    pub failed: bool,
}

/// The tool servers, and whether they arrived.
///
/// A count rather than a tick, which is the same reasoning the menu bar uses: a
/// server described only by its name is something you have to trust, while one
/// that says it brought fourteen tools is something you can weigh.
///
/// The name alone was not enough though. These are named in the config by whoever
/// wrote it -- "files" is a perfectly reasonable thing to call a server and tells
/// a reader nothing at all -- so what is actually running is carried beside it.
/// Whether a server connected is worth showing precisely because it can fail on a
/// credential, and then nothing works and nothing says why.
#[tauri::command]
pub fn servers(app: AppHandle) -> Vec<Server> {
    use crate::core::run::session::ServerState;
    let nudge = app.state::<crate::core::run::session::Nudge>();
    let specs = nudge.cfg.mcp.clone();
    nudge
        .tool_servers()
        .into_iter()
        .map(|(name, state)| {
            let about = specs
                .iter()
                .find(|s| s.name == name)
                .map(running_what)
                .unwrap_or_default();
            let failed = matches!(state, ServerState::Failed(_));
            let said = match state {
                ServerState::Starting => "connecting…".to_string(),
                ServerState::Ready(0) => "nothing offered".into(),
                ServerState::Ready(1) => "1 tool".into(),
                ServerState::Ready(n) => format!("{n} tools"),
                ServerState::Failed(why) => why,
            };
            Server {
                name,
                about,
                said,
                failed,
            }
        })
        .collect()
}

/// One line describing what a server actually is.
///
/// The published package name where there is one, because that is the part
/// somebody can look up; `npx -y` is scaffolding and says nothing. Any path
/// arguments come after it, since "which folder" is the whole question with a
/// filesystem server and the difference between fine and alarming.
fn running_what(spec: &crate::core::tools::mcp::Spec) -> String {
    let package = spec
        .args
        .iter()
        .find(|a| !a.starts_with('-') && !a.starts_with('/'))
        .cloned()
        .unwrap_or_else(|| spec.command.clone());
    let paths: Vec<&str> = spec
        .args
        .iter()
        .filter(|a| a.starts_with('/'))
        .map(|s| s.as_str())
        .collect();
    if paths.is_empty() {
        package
    } else {
        format!("{package} · {}", paths.join(", "))
    }
}

/// Every grant macOS controls, and whether Nudge has it.
///
/// Read fresh each time rather than cached. The whole point of the page that
/// shows these is that somebody leaves, ticks a box in System Settings, and comes
/// back -- a cached answer would still say "not granted" over a grant that is
/// already working.
#[tauri::command]
pub fn permits() -> Vec<crate::core::permits::Permit> {
    crate::core::permits::all()
}

/// Ask the system for one.
///
/// Reports whether a prompt was still possible. It is not, once something has been
/// refused -- macOS asks exactly once -- so a `false` here is what tells the
/// interface to stop offering Allow and offer the Settings pane instead.
#[tauri::command]
pub fn ask_permit(key: String) -> bool {
    crate::core::permits::ask(&key)
}

/// Open the exact pane for one.
#[tauri::command]
pub fn open_permit(key: String) {
    crate::core::permits::open_settings(&key);
}

/// One shortcut, as the settings page shows it.
#[derive(serde::Serialize)]
pub struct Shortcut {
    pub id: String,
    pub name: String,
    pub about: String,
    /// The keys, already split for drawing as separate caps.
    pub keys: Vec<String>,
    /// Whether it can be rebound, and why not when it cannot.
    pub fixed: Option<String>,
}

/// Turn a stored accelerator into caps somebody can read.
///
/// `Ctrl+Shift+Space` is how the OS wants it and not how anybody reads it. Split
/// and spelled out, because a row of one-character symbols is a puzzle on a
/// keyboard where half of them are not printed.
fn as_caps(hotkey: &str) -> Vec<String> {
    if crate::app::input::hotkey::is_bare_modifier(hotkey) {
        return vec!["⌃ control".into()];
    }
    hotkey
        .split('+')
        .map(|part| match part.trim().to_ascii_lowercase().as_str() {
            "ctrl" | "control" => "⌃ control".to_string(),
            "shift" => "⇧ shift".into(),
            "alt" | "option" => "⌥ option".into(),
            "cmd" | "command" | "super" | "meta" => "⌘ command".into(),
            "space" => "space".into(),
            other => other.to_string(),
        })
        .collect()
}

/// Nudge's shortcuts, as they stand.
///
/// Two, and saying so is the point. The temptation with a page like this is to
/// pad it out to look substantial -- a row for every key the app happens to read
/// -- but a list of shortcuts is a thing people scan for the one they want, and
/// every invented entry makes that slower.
#[tauri::command]
pub fn shortcuts(app: AppHandle) -> Vec<Shortcut> {
    let now = app.state::<crate::app::state::Hotkey>().get();
    let bare = crate::app::input::hotkey::is_bare_modifier(&now);
    vec![
        Shortcut {
            id: "talk".into(),
            name: "Talk to Nudge".into(),
            about: if bare {
                "Hold it and speak. Release to send.".into()
            } else {
                "Hold it and speak. Release to send.".to_string()
            },
            keys: as_caps(&now),
            fixed: None,
        },
        Shortcut {
            id: "next".into(),
            name: "Next step".into(),
            about: "Tap the same key instead of holding it.".into(),
            keys: as_caps(&now),
            // Deliberately the same key, not an unset one. One key doing both jobs
            // is the design -- nothing new to learn, and the tap is there when you
            // are mid-sequence and just want the next nudge.
            fixed: Some("Same key as Talk — tap instead of hold.".into()),
        },
        Shortcut {
            id: "stop".into(),
            name: "Stop".into(),
            about: "Stops whatever Nudge is doing to your machine.".into(),
            keys: vec!["esc".into()],
            fixed: Some("Escape is the key people already hit. It stays.".into()),
        },
    ]
}

/// Rebind the summon key.
///
/// Bound before it is stored, so a shortcut the OS refuses -- already taken by
/// something else is the usual reason -- leaves the working one in place and says
/// why, rather than leaving Nudge with no way in.
#[tauri::command]
pub fn set_shortcut(app: AppHandle, keys: String) -> std::result::Result<(), String> {
    let keys = keys.trim().to_string();
    if keys.is_empty() {
        return Err("That is not a shortcut.".into());
    }
    let previous = app.state::<crate::app::state::Hotkey>().get();
    crate::app::input::hotkey::bind(&app, &keys).map_err(|e| {
        // Put back what was working before saying anything.
        let _ = crate::app::input::hotkey::bind(&app, &previous);
        format!("macOS would not take that one: {e}")
    })?;
    app.state::<crate::app::state::Hotkey>().set(&keys);
    // The menu bar prints the shortcut in its first line.
    crate::app::ui::tray::refresh(&app, &keys);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::as_caps;

    #[test]
    fn a_bare_modifier_reads_as_the_key_itself() {
        // The default, and the one that is not an accelerator at all: the OS has
        // no notion of "Control on its own", so it never reaches the plugin and
        // must still be drawable.
        assert_eq!(as_caps("ctrl"), vec!["⌃ control"]);
        assert_eq!(as_caps("control"), vec!["⌃ control"]);
    }

    #[test]
    fn an_accelerator_is_split_into_caps_people_can_read() {
        assert_eq!(
            as_caps("Ctrl+Shift+Space"),
            vec!["⌃ control", "⇧ shift", "space"]
        );
        assert_eq!(as_caps("Cmd+Alt+K"), vec!["⌘ command", "⌥ option", "k"]);
    }

    #[test]
    fn an_unknown_key_is_shown_rather_than_dropped() {
        // Silently losing a key would draw a shortcut that is not the shortcut,
        // which is worse than showing something unpolished.
        assert_eq!(as_caps("Ctrl+F13"), vec!["⌃ control", "f13"]);
    }
}

/// How agents are set up, for the page that shows it.
#[derive(serde::Serialize)]
pub struct AgentSetup {
    pub workspace: String,
    pub suggesting: bool,
    /// The step budget, shown rather than set -- see the page for why.
    pub steps: usize,
}

#[tauri::command]
pub fn agent_setup(app: AppHandle) -> AgentSetup {
    AgentSetup {
        workspace: app
            .state::<crate::core::run::session::Nudge>()
            .workspace()
            .display()
            .to_string(),
        suggesting: app.state::<crate::app::state::Suggesting>().0.on(),
        steps: crate::core::run::agent::MAX_STEPS,
    }
}

#[tauri::command]
pub fn set_suggesting(app: AppHandle, on: bool) {
    app.state::<crate::app::state::Suggesting>().0.set(on);
}

/// Choose the folder agents work in.
///
/// macOS's own folder chooser through `osascript`, rather than a dependency for
/// one dialog. It blocks until somebody answers, so it runs off the async runtime;
/// an empty return is a cancel, which is not an error and must not read as one.
#[tauri::command]
pub async fn pick_workspace(app: AppHandle) -> std::result::Result<Option<String>, String> {
    let chosen = tokio::task::spawn_blocking(|| {
        std::process::Command::new("/usr/bin/osascript")
            .arg("-e")
            .arg(
                "POSIX path of (choose folder with prompt \"Where should Nudge work?                  Everything it runs and writes stays inside.\")",
            )
            .output()
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    let path = String::from_utf8_lossy(&chosen.stdout).trim().to_string();
    if path.is_empty() {
        return Ok(None);
    }
    // Trailing slash from `POSIX path of`, which would otherwise show up in every
    // path printed back to somebody.
    let path = path.trim_end_matches('/').to_string();
    // Through the same check a spoken "work in my nudge project" goes through.
    // The boundary is still a boundary -- it has just moved, because somebody
    // said where to.
    app.state::<crate::core::run::session::Nudge>()
        .move_to(&path)
        .map_err(|e| e.to_string())?;
    Ok(Some(path))
}
