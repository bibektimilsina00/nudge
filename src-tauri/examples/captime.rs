//! What does looking at the screen actually cost?
//!
//!     cargo run --example captime
//!
//! Written because the estimate in SPEED.md was 0.3-0.5s and the real number is
//! nearly four times that -- and because the stillness check, which nobody had
//! ever timed, turned out to cost more than the screenshot it protects.
use std::time::Instant;

fn median(mut runs: Vec<f32>) -> f32 {
    runs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    runs[runs.len() / 2]
}

fn main() {
    for edge in [240u32, 640, 1280, 1920] {
        let runs: Vec<f32> = (0..3)
            .map(|_| {
                let t = Instant::now();
                let _ = nudge_lib::core::screen::capture::grab(edge).expect("grab");
                t.elapsed().as_secs_f32() * 1000.0
            })
            .collect();
        println!("  grab({edge:<4})            {:>7.0}ms", median(runs));
    }

    // The bytes have to survive the trip. `fingerprint` decodes them on every
    // turn to decide whether the screen has stopped changing, so a JPEG that is
    // the right size and unreadable would break the loop quietly.
    let shot = nudge_lib::core::screen::capture::grab(1280).expect("grab");
    let decoded = image::load_from_memory(&shot.bytes).expect("the JPEG does not decode");
    let grey = decoded.to_luma8();
    let mean = grey.as_raw().iter().map(|p| *p as f64).sum::<f64>() / grey.as_raw().len() as f64;
    let sd = (grey.as_raw().iter().map(|p| (*p as f64 - mean).powi(2)).sum::<f64>()
        / grey.as_raw().len() as f64)
        .sqrt();
    println!(
        "\n  {}x{}, {}KB, mean {mean:.0}, sd {sd:.0}{}",
        decoded.width(),
        decoded.height(),
        shot.bytes.len() / 1024,
        if sd > 5.0 { "" } else { "   <- FLAT, composited nothing" }
    );
    assert_eq!((decoded.width(), decoded.height()), shot.sent, "what we sent is not what we said we sent");

    let t = Instant::now();
    let f = nudge_lib::core::screen::facts::gather();
    println!("\n  facts::gather           {:>7.0}ms  ({:?})", t.elapsed().as_secs_f32() * 1000.0, f.app);

    if let Some((pid, app, _)) = nudge_lib::core::screen::privacy::frontmost_window() {
        let t = Instant::now();
        let n = nudge_lib::core::screen::ax::controls(pid).len();
        println!("  ax::controls            {:>7.0}ms  ({n} in {app})", t.elapsed().as_secs_f32() * 1000.0);
    }
}
