//! Accuracy harness. Same providers, same coordinate math as the app -- it just
//! writes a PNG with a ring on it instead of an overlay, so you can eyeball
//! hit/miss and count
//!
//!     cargo run --bin probe -- "open the UV editor"
//!     NUDGE_PROVIDER=gemini NUDGE_MODEL=gemini-robotics-er-2-preview \
//!         cargo run --bin probe -- "open the UV editor"
//!
//! Run ~20 real goals, count the hits. That number decides the project.
use nudge_lib::config::Config;
use nudge_lib::core::provider;
use nudge_lib::core::screen::{capture, facts};

fn main() {
    let goal = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
    if goal.is_empty() {
        eprintln!("usage: probe \"what you want to do\"");
        std::process::exit(2);
    }

    let mut cfg = Config::load().unwrap_or_default();
    // Env overrides so comparing models is a prefix, not a config edit.
    if let Ok(p) = std::env::var("NUDGE_PROVIDER") {
        cfg.provider = p;
    }
    if let Ok(m) = std::env::var("NUDGE_MODEL") {
        cfg.model = Some(m);
    }
    if let Some(e) = std::env::var("NUDGE_MAX_EDGE")
        .ok()
        .and_then(|v| v.parse().ok())
    {
        cfg.max_edge = e;
    }

    if let Err(e) = run(cfg, &goal) {
        eprintln!("FAIL {e}");
        std::process::exit(1);
    }
}

fn run(cfg: Config, goal: &str) -> nudge_lib::error::Result<()> {
    let provider = provider::build(&cfg)?;
    let name = provider.name();
    let model = cfg.model.clone().unwrap_or_else(|| "<default>".into());

    // The probe compares models, so it works in image space and skips the
    // overlay mapping -- logical size is irrelevant here.
    // The probe captures too, so it refuses the same things the app does.
    if let Some((app, title)) = nudge_lib::core::screen::privacy::frontmost() {
        if let Some(reason) = nudge_lib::core::screen::privacy::blocked_by(&cfg, &app, &title) {
            println!("{name}: {reason} (frontmost: {app})");
            return Ok(());
        }
    }

    let t0 = std::time::Instant::now();
    let shot = capture::grab(cfg.max_edge)?;
    let captured = t0.elapsed();
    println!(
        "{name} ({model}): sent {}x{}, {} KB",
        shot.sent.0,
        shot.sent.1,
        shot.bytes.len() / 1024,
    );

    // The same list the app would get: whatever the frontmost application is
    // willing to say about its own controls.
    let t_ax = std::time::Instant::now();
    let controls = nudge_lib::core::screen::privacy::frontmost_window()
        .map(|(pid, _, _)| nudge_lib::core::screen::ax::controls(pid))
        .unwrap_or_default();
    eprintln!(
        "  tree: {} controls in {:.0}ms{}",
        controls.len(),
        t_ax.elapsed().as_secs_f32() * 1000.0,
        controls
            .first()
            .map(|c| format!(" -- e.g. {} {:?}", c.role, c.label))
            .unwrap_or_default()
    );
    let ask = provider::Ask {
        goal,
        done: &[],
        stalled: false,
        agent: false,
        workspace: cfg.workspace_dir().display().to_string(),
        facts: facts::gather(),
        controls: &controls,
        tools: &[],
        reach: String::new(),
        shell: false,
    };
    let t1 = std::time::Instant::now();
    let step = tauri::async_runtime::block_on(provider.next_step(&shot, &ask))?;
    println!(
        "  timing: capture {}ms, model {}ms",
        captured.as_millis(),
        t1.elapsed().as_millis(),
    );
    println!("  say:  {}", step.say());

    match step {
        provider::Step::Done { .. } => println!("  done:   goal already met"),
        provider::Step::Unsure { .. } => println!("  unsure: control not on this screen"),
        // The probe reports the intent; it never actually opens anything.
        provider::Step::Launch { app, .. } => println!("  launch: {app}"),
        provider::Step::Open { url, .. } => println!("  open:   {url}"),
        provider::Step::Press { keys, .. } => println!("  press:  {keys}"),
        provider::Step::Run { command, .. } => println!("  run:    {command}"),
        provider::Step::Mcp { tool, args, .. } => println!("  tool:   {tool} {args}"),
        provider::Step::Request { method, url, .. } => println!("  http:   {method} {url}"),
        provider::Step::Delegate { task, .. } => println!("  hand over: {task}"),
        provider::Step::Start { command, .. } => println!("  start:  {command}"),
        provider::Step::Output { id, .. } => println!("  output: {id}"),
        provider::Step::Kill { id, .. } => println!("  kill:   {id}"),
        provider::Step::Fetch { url, .. } => println!("  fetch:  {url}"),
        provider::Step::Search { query, .. } => println!("  search: {query}"),
        provider::Step::Task { task, .. } => println!("  task:   {task}"),
        provider::Step::Show { path, .. } => println!("  show:   {path}"),
        provider::Step::Workspace { path, .. } => println!("  cd:     {path}"),
        provider::Step::Read { path, from, .. } => println!("  read:   {path} @{from}"),
        provider::Step::Edit { path, .. } => println!("  edit:   {path}"),
        provider::Step::Plan { todos, .. } => println!("  plan:   {} steps", todos.len()),
        provider::Step::Write { path, content, .. } => {
            println!("  write:  {path} ({} bytes)", content.len())
        }
        provider::Step::Reply { .. } => println!("  reply:  (conversation, no action)"),
        provider::Step::Agent { title, .. } => {
            println!("  agent:  {title:?} (would run unattended)")
        }
        provider::Step::Question { question } => println!("  ask:    {question:?}"),
        provider::Step::Type { text, submit, .. } => {
            println!(
                "  type:   {text:?}{}",
                if submit { " + Return" } else { "" }
            )
        }
        provider::Step::Point { at, act, .. } => {
            let out = format!("hit-{name}.png");
            draw_ring(&shot.bytes, at, &out)?;
            println!("  point:  ({:.0}, {:.0}) {act:?} -> {out}", at.x, at.y);
        }
    }
    Ok(())
}

/// ponytail: hand-drawn circle rather than pulling in imageproc for one shape.
fn draw_ring(png: &[u8], p: capture::Point, out: &str) -> nudge_lib::error::Result<()> {
    const R: f64 = 34.0;
    const W: f64 = 3.0;
    let mut img = image::load_from_memory(png)?.to_rgb8();
    let (w, h) = (img.width() as i64, img.height() as i64);

    for dy in -(R as i64 + 4)..=(R as i64 + 4) {
        for dx in -(R as i64 + 4)..=(R as i64 + 4) {
            let (x, y) = (p.x as i64 + dx, p.y as i64 + dy);
            if x < 0 || y < 0 || x >= w || y >= h {
                continue;
            }
            let d = ((dx * dx + dy * dy) as f64).sqrt();
            if (d - R).abs() <= W {
                img.put_pixel(x as u32, y as u32, image::Rgb([255, 45, 85]));
            } else if (d - R).abs() <= W + 1.5 {
                img.put_pixel(x as u32, y as u32, image::Rgb([255, 255, 255]));
            }
        }
    }
    img.save(out)?;
    Ok(())
}
