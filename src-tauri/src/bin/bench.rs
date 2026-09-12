//! The accuracy harness: does the model land on the control it names?
//!
//! One agent run that works proves the loop holds together. It does not give a
//! hit rate, because a twenty-turn run hides twenty individual groundings and
//! the successful ones are invisible. This measures the groundings.
//!
//! Cases are saved screenshots with a known target, so the same twenty goals can
//! be re-scored against a new model or a changed prompt and compared honestly.
//!
//!     cargo run --bin bench -- record "click the settings gear"
//!     cargo run --bin bench
//!     NUDGE_MODEL=gemini-3-pro cargo run --bin bench
//!
//! `record` captures the screen, waits for you to click the right answer, and
//! writes the case. Recording the target by *clicking it* rather than typing
//! coordinates means the answer is where a person actually aimed.
use nudge_lib::config::Config;
use nudge_lib::core::provider;
use nudge_lib::core::screen::capture::{Point, Shot};
use nudge_lib::core::screen::{capture, click, facts};
use provider::{Ask, Step};
use std::io::Write;

/// How close counts as a hit, in image pixels.
///
/// A control people hit without thinking is about this big. The overlay's own
/// `HIT_RADIUS` is 70 points, but that is "did the user click the thing we rang"
/// -- deliberately generous. Scoring the model wants the stricter question.
const TOLERANCE: f64 = 28.0;

fn main() {
    let mut cfg = Config::load().unwrap_or_default();
    for (var, apply) in [
        ("NUDGE_PROVIDER", 0),
        ("NUDGE_MODEL", 1),
        ("NUDGE_MAX_EDGE", 2),
    ] {
        if let Ok(v) = std::env::var(var) {
            match apply {
                0 => cfg.provider = v,
                1 => cfg.model = Some(v),
                _ => {
                    if let Ok(e) = v.parse() {
                        cfg.max_edge = e;
                    }
                }
            }
        }
    }

    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("record") => record(&cfg, &args[1..].join(" ")),
        Some("list") => list(),
        None => score(&cfg),
        Some(other) => {
            eprintln!("unknown command {other:?} -- use `record <goal>`, `list`, or no argument");
            std::process::exit(2);
        }
    };
    if let Err(e) = result {
        eprintln!("FAIL {e}");
        std::process::exit(1);
    }
}

// ---------------------------------------------------------------------------

struct Case {
    name: String,
    goal: String,
    /// The right answer, in the saved image's pixel space.
    target: Point,
    image: std::path::PathBuf,
}

fn dir() -> std::path::PathBuf {
    // Relative to the repo, not the crate: the cases are data, not code.
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("bench")
}

fn load() -> std::io::Result<Vec<Case>> {
    let mut cases = Vec::new();
    let mut entries: Vec<_> = std::fs::read_dir(dir())?.filter_map(Result::ok).collect();
    entries.sort_by_key(|e| e.path());
    for e in entries {
        let path = e.path();
        if path.extension().and_then(|s| s.to_str()) != Some("txt") {
            continue;
        }
        let body = std::fs::read_to_string(&path)?;
        let mut goal = String::new();
        let (mut x, mut y) = (0.0, 0.0);
        for line in body.lines() {
            match line.split_once('=') {
                Some(("goal", v)) => goal = v.trim().to_string(),
                Some(("x", v)) => x = v.trim().parse().unwrap_or(0.0),
                Some(("y", v)) => y = v.trim().parse().unwrap_or(0.0),
                _ => {}
            }
        }
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        cases.push(Case {
            image: path.with_extension("jpg"),
            name,
            goal,
            target: Point { x, y },
        });
    }
    Ok(cases)
}

fn list() -> nudge_lib::error::Result<()> {
    for c in load()? {
        println!(
            "{:<24} ({:>5.0},{:>5.0})  {}",
            c.name, c.target.x, c.target.y, c.goal
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------

/// Capture the screen, then wait for a click to mark the right answer.
fn record(cfg: &Config, goal: &str) -> nudge_lib::error::Result<()> {
    if goal.is_empty() {
        eprintln!("usage: bench record \"click the settings gear\"");
        std::process::exit(2);
    }
    print!("Set the screen up, then press Return to capture... ");
    std::io::stdout().flush().ok();
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).ok();

    let shot = capture::grab(cfg.max_edge)?;
    println!("captured {}x{}", shot.sent.0, shot.sent.1);
    println!("Now CLICK the control this goal means. Waiting...");

    // Wait for a press, then for the release, so the recorded point is where the
    // click finished rather than wherever the pointer was on the way there.
    while !click::left_button_down() {
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
    let at = click::cursor().unwrap_or(Point { x: 0.0, y: 0.0 });
    while click::left_button_down() {
        std::thread::sleep(std::time::Duration::from_millis(16));
    }

    // The click is in logical points; the case is stored in the image's space so
    // it stays valid whatever max_edge was used to capture it.
    let (lw, lh) = shot.logical;
    let target = Point {
        x: at.x * shot.sent.0 as f64 / lw,
        y: at.y * shot.sent.1 as f64 / lh,
    };

    std::fs::create_dir_all(dir())?;
    let n = load()?.len() + 1;
    let slug: String = goal
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .to_lowercase();
    let name = format!(
        "{n:02}-{}",
        slug.trim_matches('-').split("--").next().unwrap_or("case")
    );
    std::fs::write(dir().join(format!("{name}.jpg")), &shot.bytes)?;
    std::fs::write(
        dir().join(format!("{name}.txt")),
        format!("goal = {goal}\nx = {:.0}\ny = {:.0}\n", target.x, target.y),
    )?;
    println!("saved {name}  target ({:.0},{:.0})", target.x, target.y);
    Ok(())
}

// ---------------------------------------------------------------------------

fn score(cfg: &Config) -> nudge_lib::error::Result<()> {
    let cases = load()?;
    if cases.is_empty() {
        println!("No cases yet. Record some:\n    cargo run --bin bench -- record \"click the settings gear\"");
        return Ok(());
    }
    let provider = provider::build(cfg)?;
    let model = cfg.model.clone().unwrap_or_else(|| "<default>".into());
    println!(
        "{} ({model}) -- {} cases, tolerance {TOLERANCE:.0}px\n",
        provider.name(),
        cases.len()
    );

    let (mut hits, mut misses, mut refused, mut times) = (0, 0, 0, Vec::new());
    for c in &cases {
        let bytes = match std::fs::read(&c.image) {
            Ok(b) => b,
            Err(e) => {
                println!("  {:<24} SKIP  {e}", c.name);
                continue;
            }
        };
        let (w, h) = size(&bytes);
        let shot = Shot {
            bytes,
            sent: (w, h),
            // The case lives in image space, so this is the identity mapping.
            logical: (w as f64, h as f64),
            origin: (0.0, 0.0),
        };
        let ask = Ask {
            goal: &c.goal,
            done: &[],
            stalled: false,
            agent: false,
            // A saved screenshot has no live frontmost app or audio to report,
            // and inventing some would measure something other than grounding.
            facts: facts::Facts::default(),
        };

        let t0 = std::time::Instant::now();
        let step = tauri::async_runtime::block_on(provider.next_step(&shot, &ask));
        let took = t0.elapsed();
        times.push(took.as_millis() as u64);

        match step {
            Ok(Step::Point { at, .. }) => {
                let d = (at.x - c.target.x).hypot(at.y - c.target.y);
                if d <= TOLERANCE {
                    hits += 1;
                    println!(
                        "  {:<24} HIT   {d:>5.0}px  {:>5}ms",
                        c.name,
                        took.as_millis()
                    );
                } else {
                    misses += 1;
                    println!(
                        "  {:<24} MISS  {d:>5.0}px  {:>5}ms  said ({:.0},{:.0}) wanted ({:.0},{:.0})",
                        c.name, took.as_millis(), at.x, at.y, c.target.x, c.target.y
                    );
                }
            }
            // Not a miss and not a hit. A model that says "I cannot see it" is
            // behaving correctly on a screen that does not contain the control,
            // and scoring it as a wrong click would reward guessing.
            Ok(other) => {
                refused += 1;
                println!(
                    "  {:<24} ----  {:>5}ms  {other:?}",
                    c.name,
                    took.as_millis()
                );
            }
            Err(e) => {
                misses += 1;
                println!("  {:<24} ERR   {e}", c.name);
            }
        }
    }

    times.sort_unstable();
    let pct = |p: f64| {
        times
            .get(((times.len() as f64 - 1.0) * p) as usize)
            .copied()
            .unwrap_or(0)
    };
    let scored = hits + misses;
    println!(
        "\n{hits}/{scored} hits ({:.0}%){}  --  median {}ms, p95 {}ms",
        if scored == 0 {
            0.0
        } else {
            hits as f64 * 100.0 / scored as f64
        },
        if refused > 0 {
            format!(", {refused} not pointed at")
        } else {
            String::new()
        },
        pct(0.5),
        pct(0.95),
    );
    Ok(())
}

/// Image dimensions, without decoding the whole thing.
fn size(bytes: &[u8]) -> (u32, u32) {
    image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .ok()
        .and_then(|r| r.into_dimensions().ok())
        .unwrap_or((1, 1))
}
