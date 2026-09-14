//! Does Nudge pick the control you meant?
//!
//! The screenshot bench scores where a click lands, which was the right question
//! when finding a button meant guessing at pixels. It mostly does not any more:
//! the system says where the controls are, and the job is choosing the right one
//! from a list. That is a different question and this asks it.
//!
//!     cargo run --bin picks -- record "click the search icon" = Search
//!     cargo run --bin picks -- record "click the thing next to View" = none
//!     cargo run --bin picks
//!
//! A case is a control list, a goal, and the label that should win -- or `none`
//! when nothing should, because the phrasing is ambiguous or the request is not
//! a request to click at all. No screenshot, no network, no waiting: recording
//! one takes a second and scoring every case takes milliseconds. That is the
//! point. The old bench has ten cases because each one costs a screen you have
//! to arrange and a target you have to click; these cost a sentence.
//!
//! **The number that matters is wrong picks.** A refusal costs four seconds and
//! the model gets it right. A wrong pick clicks something nobody asked for, with
//! nothing watching, and the first one of those was found by accident.
use nudge_lib::core::screen::{ax, privacy};
use std::io::Write;

/// Beside the repository, not beside whatever directory you happened to run
/// from. `make picks` runs inside `src-tauri`, and a relative path there finds
/// an empty folder and reports, cheerfully, that there is nothing to score.
fn dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("picks")
}

struct Case {
    name: String,
    goal: String,
    /// `None` means: this must fall through to the model.
    expect: Option<String>,
    controls: Vec<ax::Control>,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("record") => record(&args[1..].join(" ")),
        None => score(),
        Some(other) => {
            eprintln!("unknown command {other:?} -- use `record \"<goal>\" = <label|none>`, or no argument");
            std::process::exit(2);
        }
    };
    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

/// `record "click the search icon" = Search (⇧⌘F)`
///
/// Takes the frontmost application's controls as they are right now. Bring the
/// app you want to the front first -- an application with no window exposes a
/// menu bar and nothing else, which reads exactly like one that exposes nothing.
fn record(spec: &str) -> std::io::Result<()> {
    let Some((goal, expect)) = spec.rsplit_once('=') else {
        eprintln!("usage: record \"<goal>\" = <the label that should win, or none>");
        std::process::exit(2);
    };
    let (goal, expect) = (goal.trim(), expect.trim());

    for n in (1..=3).rev() {
        eprint!("\rbring the app to the front... {n} ");
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    eprintln!("\r                                  ");

    let Some((pid, app, _)) = privacy::frontmost_window() else {
        eprintln!("could not tell what is frontmost");
        std::process::exit(1);
    };
    let controls = ax::controls(pid);
    if controls.is_empty() {
        eprintln!("{app} exposed nothing -- does it have a window open?");
        std::process::exit(1);
    }
    // A case whose answer is not in its own control list can never pass, and
    // would look like a matcher bug forever.
    if !expect.eq_ignore_ascii_case("none")
        && !controls.iter().any(|c| c.label.eq_ignore_ascii_case(expect))
    {
        eprintln!("{app} has no control called {expect:?}. It exposes:");
        for c in controls.iter().take(40) {
            eprintln!("    {}", c.label);
        }
        std::process::exit(1);
    }

    std::fs::create_dir_all(dir())?;
    let slug: String = goal
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    let slug = slug.trim_matches('-').replace("--", "-");
    let path = dir().join(format!("{:03}-{slug}.txt", count() + 1));

    let mut f = std::fs::File::create(&path)?;
    writeln!(f, "app = {app}")?;
    writeln!(f, "goal = {goal}")?;
    writeln!(f, "expect = {expect}")?;
    writeln!(f, "--")?;
    for c in &controls {
        writeln!(
            f,
            "{} | {} | {:.0} {:.0} | {:.0} {:.0}",
            c.role, c.label, c.at.0, c.at.1, c.size.0, c.size.1
        )?;
    }
    println!("wrote {} ({} controls from {app})", path.display(), controls.len());
    Ok(())
}

fn count() -> usize {
    std::fs::read_dir(dir())
        .map(|d| d.flatten().filter(|e| e.path().extension().is_some_and(|x| x == "txt")).count())
        .unwrap_or(0)
}

fn load() -> std::io::Result<Vec<Case>> {
    let Ok(found) = std::fs::read_dir(dir()) else {
        return Ok(Vec::new());
    };
    let mut paths: Vec<_> = found
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "txt"))
        .collect();
    paths.sort();

    let mut cases = Vec::new();
    for path in paths {
        let text = std::fs::read_to_string(&path)?;
        let (head, body) = text.split_once("\n--\n").unwrap_or((&text, ""));
        let mut goal = String::new();
        let mut expect = None;
        for line in head.lines() {
            // Trimmed on both sides. The screenshot bench once matched an
            // untrimmed key, loaded every case with an empty goal, and scored
            // zero out of ten against a model that had been asked nothing.
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            match k.trim() {
                "goal" => goal = v.trim().to_string(),
                "expect" => {
                    expect = match v.trim() {
                        "none" | "" => None,
                        label => Some(label.to_string()),
                    }
                }
                _ => {}
            }
        }
        let controls = body
            .lines()
            .filter_map(|l| {
                let f: Vec<&str> = l.split('|').map(str::trim).collect();
                let [role, label, at, size] = f[..] else {
                    return None;
                };
                let pair = |s: &str| -> Option<(f64, f64)> {
                    let (a, b) = s.split_once(' ')?;
                    Some((a.parse().ok()?, b.parse().ok()?))
                };
                Some(ax::Control {
                    role: role.to_string(),
                    label: label.to_string(),
                    at: pair(at)?,
                    size: pair(size)?,
                })
            })
            .collect();
        cases.push(Case {
            name: path.file_stem().unwrap_or_default().to_string_lossy().into(),
            goal,
            expect,
            controls,
        });
    }
    Ok(cases)
}

fn score() -> std::io::Result<()> {
    let cases = load()?;
    if cases.is_empty() {
        println!(
            "No cases yet. Record some:\n    \
             cargo run --bin picks -- record \"click the search icon\" = Search\n    \
             cargo run --bin picks -- record \"click the thing next to View\" = none"
        );
        return Ok(());
    }

    let (mut right, mut wrong, mut shy) = (0, 0, 0);
    for case in &cases {
        let got = ax::obvious(&case.goal, &case.controls).map(|c| c.label.clone());
        let verdict = match (&case.expect, &got) {
            (Some(want), Some(had)) if want.eq_ignore_ascii_case(had) => {
                right += 1;
                "ok".to_string()
            }
            // The only outcome that is actually dangerous.
            (_, Some(had)) => {
                wrong += 1;
                format!("WRONG -- clicked {had:?}")
            }
            (None, None) => {
                right += 1;
                "ok (fell through, as it should)".to_string()
            }
            (Some(want), None) => {
                shy += 1;
                format!("missed -- the model would handle it; wanted {want:?}")
            }
        };
        println!("  {:<44} {verdict}", case.name);
    }

    let total = cases.len();
    println!(
        "\n  {right}/{total} right, {wrong} WRONG, {shy} fell through unnecessarily\n\
         \n  A fall-through costs about four seconds and the model gets it right.\n  \
         A wrong pick clicks something nobody asked for, with nothing watching.\n  \
         They are not the same kind of mistake and should never be averaged."
    );
    if wrong > 0 {
        std::process::exit(1);
    }
    Ok(())
}
