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

    // Where it goes. Not where anyone assumed: the resize and the JPEG were
    // tuned carefully and are a tenth of it.
    let t = Instant::now();
    let full = nudge_lib::core::screen::capture::grab(1280).expect("grab");
    let whole = t.elapsed().as_secs_f32() * 1000.0;

    let t = Instant::now();
    let img = image::load_from_memory(&full.bytes).expect("decode");
    let mut out = Vec::new();
    img.resize(240, 240, image::imageops::FilterType::Triangle)
        .write_with_encoder(image::codecs::jpeg::JpegEncoder::new_with_quality(
            &mut std::io::Cursor::new(&mut out),
            60,
        ))
        .expect("encode");
    let reencode = t.elapsed().as_secs_f32() * 1000.0;
    println!("\n  resize + encode         {reencode:>7.0}ms");
    println!("  compositing             {:>7.0}ms  <- everything else", whole - reencode);

    let t = Instant::now();
    let f = nudge_lib::core::screen::facts::gather();
    println!("\n  facts::gather           {:>7.0}ms  ({:?})", t.elapsed().as_secs_f32() * 1000.0, f.app);

    if let Some((pid, app, _)) = nudge_lib::core::screen::privacy::frontmost_window() {
        let t = Instant::now();
        let n = nudge_lib::core::screen::ax::controls(pid).len();
        println!("  ax::controls            {:>7.0}ms  ({n} in {app})", t.elapsed().as_secs_f32() * 1000.0);
    }
}
