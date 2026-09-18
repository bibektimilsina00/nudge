//! Does the cross-platform path actually work?
//!
//!     cargo run --features portable --example portable
//!
//! Run on macOS, against the implementations written for everywhere else. That
//! is the whole point of the `portable` feature: Windows and Linux cannot be
//! compiled from here, let alone run, so the only way to know this code does
//! anything is to make this machine use it.
//!
//! It does not prove the port works. It proves the code is not nonsense, which
//! is a different and much lower bar -- and one that unverified code written
//! from memory routinely fails.
use nudge_lib::core::screen::{capture, click, launch, privacy};
use std::time::Instant;

fn main() {
    let t = Instant::now();
    match capture::grab(1280) {
        Ok(shot) => {
            let img = image::load_from_memory(&shot.bytes).expect("the JPEG does not decode");
            let grey = img.to_luma8();
            let mean =
                grey.as_raw().iter().map(|p| *p as f64).sum::<f64>() / grey.as_raw().len() as f64;
            let sd = (grey
                .as_raw()
                .iter()
                .map(|p| (*p as f64 - mean).powi(2))
                .sum::<f64>()
                / grey.as_raw().len() as f64)
                .sqrt();
            println!(
                "  capture     {:>6.0}ms   {}x{}, {}KB, sd {sd:.0}{}",
                t.elapsed().as_secs_f32() * 1000.0,
                shot.sent.0,
                shot.sent.1,
                shot.bytes.len() / 1024,
                if sd > 5.0 {
                    ""
                } else {
                    "   <- FLAT, captured nothing"
                }
            );
        }
        Err(e) => println!("  capture     FAILED: {e}"),
    }

    match click::cursor() {
        Some(p) => println!("  cursor      ({:.0}, {:.0})", p.x, p.y),
        None => println!("  cursor      FAILED"),
    }
    println!("  may_click   {}", click::may_click());
    println!(
        "  modifiers   {:?} (hold some while running to see this change)",
        click::modifiers_held()
    );
    println!("  button down {}", click::left_button_down());

    let apps = launch::installed_apps();
    println!(
        "  apps        {} found{}",
        apps.len(),
        match apps.first() {
            Some(a) => format!("  e.g. {a}"),
            None => "   (none -- this machine keeps them somewhere else)".into(),
        }
    );

    // The one that is not optional. `blocked_by` is handed what this returns, so
    // a platform where it answers nothing is a platform with no privacy guard.
    match privacy::frontmost_window() {
        Some((pid, app, title)) => println!("  frontmost   {app:?} (pid {pid}) -- {title:?}"),
        None => println!("  frontmost   FAILED -- the privacy guard would never fire"),
    }
}
