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
//!     NUDGE_THINK=low cargo run --bin bench
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
        ("NUDGE_THINK", 3),
    ] {
        if let Ok(v) = std::env::var(var) {
            match apply {
                0 => cfg.provider = v,
                1 => cfg.model = Some(v),
                2 => {
                    if let Ok(e) = v.parse() {
                        cfg.max_edge = e;
                    }
                }
                _ => cfg.think = Some(v),
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
    ///
    /// A point for something small, a box for something wide. A click anywhere
    /// inside the address bar opens the address bar, so scoring that by distance
    /// from wherever the recorder happened to click marks a perfectly good
    /// answer wrong -- one run called a click 171px along a 600px-wide bar a
    /// miss.
    target: Target,
    image: std::path::PathBuf,
}

enum Target {
    /// Somewhere near here, within `TOLERANCE`.
    Spot(Point),
    /// Anywhere in here.
    Box { x1: f64, y1: f64, x2: f64, y2: f64 },
}

impl Target {
    /// How far outside the answer this point is. Zero means correct.
    fn miss_by(&self, p: Point) -> f64 {
        match self {
            Target::Spot(t) => ((p.x - t.x).hypot(p.y - t.y) - TOLERANCE).max(0.0),
            Target::Box { x1, y1, x2, y2 } => {
                let dx = (x1 - p.x).max(0.0).max(p.x - x2);
                let dy = (y1 - p.y).max(0.0).max(p.y - y2);
                dx.max(0.0).hypot(dy.max(0.0))
            }
        }
    }

    fn describe(&self) -> String {
        match self {
            Target::Spot(t) => format!("({:.0},{:.0})", t.x, t.y),
            Target::Box { x1, y1, x2, y2 } => {
                format!("the box ({x1:.0},{y1:.0})-({x2:.0},{y2:.0})")
            }
        }
    }
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
        let (mut x, mut y): (f64, f64) = (0.0, 0.0);
        let (mut x2, mut y2): (Option<f64>, Option<f64>) = (None, None);
        for line in body.lines() {
            // Both sides trimmed. `goal = x` splits into "goal " and " x", and
            // matching on the untrimmed key silently matched nothing -- so every
            // case ran with an empty goal and a target of (0,0), and the whole
            // first run scored 0/10 against a model that had been asked nothing.
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match (key.trim(), value.trim()) {
                ("goal", v) => goal = v.to_string(),
                ("x", v) => x = v.parse().unwrap_or(0.0),
                ("y", v) => y = v.parse().unwrap_or(0.0),
                ("x2", v) => x2 = v.parse().ok(),
                ("y2", v) => y2 = v.parse().ok(),
                _ => {}
            }
        }
        // A case that did not parse is worse than no case: it scores a miss
        // against a target of nothing.
        assert!(
            !goal.is_empty() && (x, y) != (0.0, 0.0),
            "{} did not parse -- goal {goal:?}, target ({x},{y})",
            path.display()
        );
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        // Two corners means a box; one point means a spot.
        let target = match (x2, y2) {
            (Some(bx), Some(by)) => {
                // A drag along a bar is horizontal, so it records almost no
                // height -- and a perfectly good answer a few pixels lower would
                // be scored a miss against a box one pixel tall. A thin drag
                // means "anywhere along this, at about this height", so any side
                // narrower than the tolerance grows to it, centred.
                let widen = |lo: f64, hi: f64| {
                    let (lo, hi) = (lo.min(hi), lo.max(hi));
                    if hi - lo >= TOLERANCE {
                        (lo, hi)
                    } else {
                        let mid = (lo + hi) / 2.0;
                        (mid - TOLERANCE / 2.0, mid + TOLERANCE / 2.0)
                    }
                };
                let (x1, x2) = widen(x, bx);
                let (y1, y2) = widen(y, by);
                Target::Box { x1, y1, x2, y2 }
            }
            _ => Target::Spot(Point { x, y }),
        };
        cases.push(Case {
            image: path.with_extension("jpg"),
            name,
            goal,
            target,
        });
    }
    Ok(cases)
}

fn list() -> nudge_lib::error::Result<()> {
    for c in load()? {
        println!("{:<40} {:<28} {}", c.name, c.target.describe(), c.goal);
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
    println!("Goal: {goal}");
    print!("Press Return, then bring the right window forward. ");
    std::io::stdout().flush().ok();
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).ok();

    // A countdown, because the capture used to happen the instant Return was
    // pressed -- which photographed the terminal the command was typed into,
    // rather than the application the case is about.
    for n in (1..=5).rev() {
        print!("\rCapturing in {n}... ");
        std::io::stdout().flush().ok();
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    println!("\rcapturing now          ");

    let shot = capture::grab(cfg.max_edge)?;
    println!("captured {}x{}", shot.sent.0, shot.sent.1);
    println!("Now mark the answer:");
    println!("  CLICK a small control, or DRAG across a wide one (a bar, a row).");

    // Where the press started and where it ended. A click gives the same point
    // twice; a drag gives two corners, which is how a wide control gets recorded
    // as the shape it actually is.
    while !click::left_button_down() {
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
    let from = click::cursor().unwrap_or(Point { x: 0.0, y: 0.0 });
    let mut to = from;
    while click::left_button_down() {
        if let Some(p) = click::cursor() {
            to = p;
        }
        std::thread::sleep(std::time::Duration::from_millis(16));
    }

    // Anything smaller than this was a click with a shaky hand, not a drag.
    const DRAGGED: f64 = 12.0;

    // The click is in logical points; the case is stored in the image's space so
    // it stays valid whatever max_edge was used to capture it.
    // Global point -> this display -> the image. The middle step is the one that
    // is easy to forget: a capture is of one display now, not of the whole desk,
    // so a click has to lose that display's origin before it means anything in
    // the picture. Zero on the main screen, which is exactly why it would have
    // gone unnoticed until someone recorded a case on a second monitor.
    let (lw, lh) = shot.logical;
    let (ox, oy) = shot.origin;
    let into_image = |p: Point| Point {
        x: (p.x - ox) * shot.sent.0 as f64 / lw,
        y: (p.y - oy) * shot.sent.1 as f64 / lh,
    };
    let (a, b) = (into_image(from), into_image(to));
    let dragged = (b.x - a.x).abs() > DRAGGED || (b.y - a.y).abs() > DRAGGED;

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
    let body = if dragged {
        format!(
            "goal = {goal}\nx = {:.0}\ny = {:.0}\nx2 = {:.0}\ny2 = {:.0}\n",
            a.x, a.y, b.x, b.y
        )
    } else {
        format!("goal = {goal}\nx = {:.0}\ny = {:.0}\n", a.x, a.y)
    };
    std::fs::write(dir().join(format!("{name}.txt")), body)?;
    println!(
        "saved {name}  {}",
        if dragged {
            format!(
                "box ({:.0},{:.0})-({:.0},{:.0})",
                a.x.min(b.x),
                a.y.min(b.y),
                a.x.max(b.x),
                a.y.max(b.y)
            )
        } else {
            // Says how far it moved, so a drag that did not take is diagnosable
            // rather than silently a point.
            format!(
                "point ({:.0},{:.0}) -- moved {:.0}px, under the {DRAGGED:.0}px a box needs",
                a.x,
                a.y,
                (b.x - a.x).abs().max((b.y - a.y).abs())
            )
        }
    );
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
        "{} ({model}{}) -- {} cases, tolerance {TOLERANCE:.0}px\n",
        provider.name(),
        match &cfg.think {
            Some(n) => format!(", thinking {n}"),
            None => ", thinking default".into(),
        },
        cases.len()
    );

    let (mut hits, mut misses, mut refused, mut times) = (0, 0, 0, Vec::new());
    for c in &cases {
        let bytes = match std::fs::read(&c.image) {
            Ok(b) => b,
            Err(e) => {
                println!("  {:<40} SKIP  {e}", c.name);
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
            // A saved case is a picture of a screen that is long gone, so there
            // is no tree to ask. This harness scores grounding from pixels, which
            // is exactly the thing the control list is meant to make unnecessary --
            // so a good score here and a good score live are different claims.
            controls: &[],
            tools: &[],
            reach: String::new(),
            shell: false,
            memory: String::new(),
            earlier: &[],
            goal: &c.goal,
            done: &[],
            stalled: false,
            agent: false,
            workspace: cfg.workspace_dir().display().to_string(),
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
                let off = c.target.miss_by(at);
                if off == 0.0 {
                    hits += 1;
                    println!("  {:<40} HIT           {:>5}ms", c.name, took.as_millis());
                } else {
                    misses += 1;
                    println!(
                        "  {:<40} MISS {off:>5.0}px  {:>5}ms  said ({:.0},{:.0}) wanted {}",
                        c.name,
                        took.as_millis(),
                        at.x,
                        at.y,
                        c.target.describe()
                    );
                }
            }
            // Not a miss and not a hit. A model that says "I cannot see it" is
            // behaving correctly on a screen that does not contain the control,
            // and scoring it as a wrong click would reward guessing.
            Ok(other) => {
                refused += 1;
                println!(
                    "  {:<40} ----          {:>5}ms  {other:?}",
                    c.name,
                    took.as_millis()
                );
            }
            Err(e) => {
                misses += 1;
                println!("  {:<40} ERR   {e}", c.name);
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
