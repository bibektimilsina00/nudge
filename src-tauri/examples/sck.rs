//! Is ScreenCaptureKit actually faster than the deprecated call we use?
//!
//!     cargo run --example sck
//!
//! Measured before migrating, because the whole reason to migrate is a number,
//! and "it is the modern API" is not one. Compositing costs 1.53s today; this
//! has to beat that by enough to be worth a framework.
use objc2::rc::Retained;
use objc2::{AnyThread, Message};
use objc2_screen_capture_kit::{
    SCContentFilter, SCScreenshotManager, SCShareableContent, SCStreamConfiguration,
};
use std::sync::mpsc;
use std::time::Instant;

/// Wait for one of ScreenCaptureKit's completion handlers.
///
/// Everything here is asynchronous and we want a screenshot now, so each call is
/// turned back into a blocking one. Safe off the main thread: the handlers fire
/// on ScreenCaptureKit's own queue, not ours.
fn shareable() -> Option<Retained<SCShareableContent>> {
    let (tx, rx) = mpsc::channel();
    let handler = block2::RcBlock::new(
        move |content: *mut SCShareableContent, _err: *mut objc2_foundation::NSError| {
            let got = unsafe { content.as_ref() }.map(|c| c.retain());
            let _ = tx.send(got);
        },
    );
    unsafe { SCShareableContent::getShareableContentWithCompletionHandler(&handler) };
    rx.recv_timeout(std::time::Duration::from_secs(5))
        .ok()
        .flatten()
}

/// A ScreenCaptureKit image, as bytes we can hand to the JPEG encoder.
///
/// The pixels come back BGRA -- Core Graphics' native order on this platform --
/// so the red and blue channels are swapped on the way into an RGB image. A
/// picture that comes out looking cold and blue is this line.
fn pixels(img: &objc2_core_graphics::CGImage) -> (usize, usize, Vec<u8>) {
    use objc2_core_foundation::CFData;
    use objc2_core_graphics::{CGDataProvider, CGImage};

    let (w, h) = (CGImage::width(Some(img)), CGImage::height(Some(img)));
    let stride = CGImage::bytes_per_row(Some(img));
    let provider = CGImage::data_provider(Some(img)).expect("data provider");
    let data: objc2_core_foundation::CFRetained<CFData> =
        CGDataProvider::data(Some(&provider)).expect("pixel data");
    let raw = data.to_vec();

    let mut rgb = Vec::with_capacity(w * h * 3);
    for y in 0..h {
        let row = &raw[y * stride..];
        for x in 0..w {
            let p = &row[x * 4..x * 4 + 4];
            rgb.extend_from_slice(&[p[2], p[1], p[0]]);
        }
    }
    (w, h, rgb)
}

/// Mean of each channel, for comparing two renderings of the same screen.
fn channels(rgb: &[u8]) -> (f64, f64, f64) {
    let n = (rgb.len() / 3) as f64;
    let mut sum = (0f64, 0f64, 0f64);
    for p in rgb.chunks_exact(3) {
        sum.0 += p[0] as f64;
        sum.1 += p[1] as f64;
        sum.2 += p[2] as f64;
    }
    (sum.0 / n, sum.1 / n, sum.2 / n)
}

fn main() {
    let t = Instant::now();
    let Some(content) = shareable() else {
        eprintln!("no shareable content -- is Screen Recording granted to this binary?");
        std::process::exit(1);
    };
    println!(
        "  SCShareableContent        {:>7.0}ms",
        t.elapsed().as_secs_f32() * 1000.0
    );

    let displays = unsafe { content.displays() };
    let Some(display) = displays.iter().next() else {
        eprintln!("no displays");
        std::process::exit(1);
    };
    let (w, h) = (unsafe { display.width() }, unsafe { display.height() });
    println!("  display                    {w}x{h}");

    let empty = objc2_foundation::NSArray::new();
    let filter = unsafe {
        SCContentFilter::initWithDisplay_excludingWindows(
            SCContentFilter::alloc(),
            &display,
            &empty,
        )
    };
    let config = unsafe { SCStreamConfiguration::new() };
    unsafe {
        config.setWidth(w as usize);
        config.setHeight(h as usize);
    }

    // Scaled during the capture rather than after it. If ScreenCaptureKit will
    // hand back the size we actually send, the 232ms of resize and JPEG we have
    // been paying goes away too.
    unsafe {
        config.setWidth(1280);
        config.setHeight((1280.0 * h as f64 / w as f64) as usize);
    }

    for i in 0..4 {
        let t = Instant::now();
        let (tx, rx) = mpsc::channel();
        let handler = block2::RcBlock::new(
            move |img: *mut objc2_core_graphics::CGImage, _err: *mut objc2_foundation::NSError| {
                let got = unsafe { img.as_ref() }.map(pixels);
                let _ = tx.send(got);
            },
        );
        unsafe {
            SCScreenshotManager::captureImageWithFilter_configuration_completionHandler(
                &filter,
                &config,
                Some(&handler),
            )
        };
        let got = rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .ok()
            .flatten();
        println!(
            "  captureImage #{i}            {:>7.0}ms  {}",
            t.elapsed().as_secs_f32() * 1000.0,
            match &got {
                Some((w, h, rgb)) => {
                    // A black rectangle has the right dimensions too. The only
                    // proof that this is a picture of anything is that the
                    // pixels differ from each other.
                    let mean: f64 = rgb.iter().map(|b| *b as f64).sum::<f64>() / rgb.len() as f64;
                    let var: f64 = rgb.iter().map(|b| (*b as f64 - mean).powi(2)).sum::<f64>()
                        / rgb.len() as f64;
                    format!("{w}x{h}, mean {mean:.0}, sd {:.0}", var.sqrt())
                }
                None => "FAILED".into(),
            }
        );
        if i == 3 {
            // Is red actually red? A wrong channel order gives a picture with
            // identical brightness statistics and the wrong colours, so the only
            // honest check is against the path we already trust -- same screen,
            // same moment, per channel.
            if let Some((w, h, rgb)) = got {
                let mine = channels(&rgb);
                let theirs = {
                    let shot = nudge_lib::core::screen::capture::grab(1280).expect("grab");
                    let img = image::load_from_memory(&shot.bytes)
                        .expect("decode")
                        .to_rgb8();
                    channels(img.as_raw())
                };
                println!(
                    "\n  sck    r {:.0} g {:.0} b {:.0}   ({w}x{h})",
                    mine.0, mine.1, mine.2
                );
                println!(
                    "  cgwin  r {:.0} g {:.0} b {:.0}",
                    theirs.0, theirs.1, theirs.2
                );
                let off = (mine.0 - theirs.0).abs()
                    + (mine.1 - theirs.1).abs()
                    + (mine.2 - theirs.2).abs();
                println!(
                    "  channels {} (total difference {off:.1})",
                    if off < 12.0 {
                        "AGREE"
                    } else {
                        "DISAGREE -- check the byte order"
                    }
                );
            }
        }
    }
}
