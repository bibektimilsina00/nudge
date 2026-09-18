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
        drawn: false,
        using: &[],
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
        memory: String::new(),
        earlier: &[],
        skills: String::new(),
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
        provider::Step::Tour { parts, next, .. } => {
            println!("  tour:   {} parts", parts.len());
            // Everything into the picture's pixels, once.
            //
            // A part arrives with `at` in the picture and `size` in screen
            // points -- `to_global_size` converts the size on the way out
            // because the overlay draws in screen points. Mixing the two has
            // now cost two rounds of this: boxes 18% too big, which looks
            // exactly like a model drawing loose ones, and then a scorer
            // complaining that they hang off the screen.
            let shrink = shot.sent.0 as f64 / shot.logical.0;
            let parts: Vec<provider::Step> = parts
                .into_iter()
                .map(|p| match p {
                    provider::Step::Point {
                        at,
                        say,
                        act,
                        size,
                        control,
                    } => provider::Step::Point {
                        at,
                        say,
                        act,
                        size: size.map(|(w, h)| (w * shrink, h * shrink)),
                        control,
                    },
                    other => other,
                })
                .collect();
            // Every mark on one picture, numbered in the order they are spoken.
            //
            // A tour is judged by whether each box is round the thing its
            // sentence names, and that is not a judgement anybody can make from
            // a list of coordinates. Until now the only way to see a wrong box
            // was to watch a tour go past and catch it.
            let out = format!("tour-{name}.png");
            draw_tour(&shot, &parts, &out)?;
            println!("  drawn:  {out}");

            // Judged as well as drawn. The picture is for a person; this is the
            // half that can be run fifty times without one.
            let said =
                nudge_lib::core::teaching::score(&parts, (shot.sent.0 as f64, shot.sent.1 as f64));
            match said.is_empty() {
                true => println!("  score:  nothing wrong with the shape of it"),
                false => {
                    println!("  score:  {} complaints", said.len());
                    for c in &said {
                        match c.part {
                            Some(n) => println!("    part {n}: {}", c.about),
                            None => println!("    tour:   {}", c.about),
                        }
                    }
                }
            }
            for p in &parts {
                match p {
                    provider::Step::Point {
                        at,
                        size,
                        control,
                        say,
                        ..
                    } => println!(
                        "    {} ({:.0},{:.0}){} {say}",
                        match size {
                            Some(_) => "box ",
                            None => "ring",
                        },
                        at.x,
                        at.y,
                        control.as_deref().unwrap_or(""),
                    ),
                    other => println!("    --   {}", other.say()),
                }
            }
            match next {
                Some(offer) => println!("  next:   {offer}"),
                // A chapter that offers nothing is where the teaching stops.
                None => println!("  next:   (nothing offered)"),
            }
        }
        provider::Step::Unsure { .. } => println!("  unsure: control not on this screen"),
        // The probe reports the intent; it never actually opens anything.
        provider::Step::Launch { app, .. } => println!("  launch: {app}"),
        provider::Step::Open { url, .. } => println!("  open:   {url}"),
        provider::Step::Press { keys, .. } => println!("  press:  {keys}"),
        provider::Step::Run { command, .. } => println!("  run:    {command}"),
        provider::Step::Await { id, .. } => println!("  await:  {id}"),
        provider::Step::Mcp { tool, args, .. } => println!("  tool:   {tool} {args}"),
        provider::Step::Tools { server, .. } => {
            println!("  tools:  asking {server} what it offers")
        }
        provider::Step::Request { method, url, .. } => println!("  http:   {method} {url}"),
        provider::Step::Delegate { task, .. } => println!("  hand over: {task}"),
        provider::Step::Remember { about, note, .. } => println!("  note:   {about}: {note}"),
        provider::Step::Skill { name, .. } => println!("  skill:  {name}"),
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

/// Every part of a tour on one image: boxes, rings, and the order they come in.
fn draw_tour(
    shot: &capture::Shot,
    parts: &[provider::Step],
    out: &str,
) -> nudge_lib::error::Result<()> {
    // Both in the picture's pixels by the time they arrive here -- see the
    // conversion at the call site, and why it exists.
    let mut img = image::load_from_memory(&shot.bytes)?.to_rgb8();
    let (w, h) = (img.width() as i64, img.height() as i64);
    let mut dot = |x: i64, y: i64, c: [u8; 3]| {
        if (0..w).contains(&x) && (0..h).contains(&y) {
            img.put_pixel(x as u32, y as u32, image::Rgb(c));
        }
    };

    for (n, part) in parts.iter().enumerate() {
        let provider::Step::Point { at, size, .. } = part else {
            continue;
        };
        // The colour says which part, because two boxes that overlap are
        // otherwise one shape.
        let hue = [
            [255, 45, 85],
            [90, 200, 250],
            [255, 214, 10],
            [52, 199, 89],
            [191, 90, 242],
            [255, 149, 0],
        ][n % 6];

        match size {
            Some((bw, bh)) => {
                let (bw, bh) = (*bw, *bh);
                let (x0, y0) = ((at.x - bw / 2.0) as i64, (at.y - bh / 2.0) as i64);
                let (x1, y1) = ((at.x + bw / 2.0) as i64, (at.y + bh / 2.0) as i64);
                // Three pixels thick, so a box is visible against a busy window.
                for t in 0..3 {
                    for x in x0..=x1 {
                        dot(x, y0 + t, hue);
                        dot(x, y1 - t, hue);
                    }
                    for y in y0..=y1 {
                        dot(x0 + t, y, hue);
                        dot(x1 - t, y, hue);
                    }
                }
                // A filled square at the top-left corner, one per part, as a
                // stand-in for a number nobody wants to rasterise by hand.
                for i in 0..=(n as i64) {
                    for dy in 0..7 {
                        for dx in 0..7 {
                            dot(x0 + 4 + i * 9 + dx, y0 + 4 + dy, hue);
                        }
                    }
                }
            }
            None => {
                for d in 0..360 {
                    let a = f64::from(d).to_radians();
                    for r in 30..34 {
                        dot(
                            (at.x + f64::from(r) * a.cos()) as i64,
                            (at.y + f64::from(r) * a.sin()) as i64,
                            hue,
                        );
                    }
                }
            }
        }
    }
    img.save(out)?;
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
