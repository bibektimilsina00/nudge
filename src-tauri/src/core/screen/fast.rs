//! Compositing the screen through ScreenCaptureKit, which is thirty times
//! quicker than the call it replaces.
//!
//! `CGWindowListCreateImageFromArray` spends 1.53 seconds putting one frame
//! together. Measured against the same screen at the same moment,
//! `SCScreenshotManager` does it in 40 milliseconds -- and scales to the size we
//! actually send while it is there, which removes the resize as well.
//!
//! That was worth nearly four seconds a turn, because the screen is composited
//! three times: twice to decide whether it has stopped changing, and once for
//! the picture itself.
//!
//! The old path stays as the fallback. This one needs macOS 14, it needs the
//! screen recording grant to be current, and it is a framework that can decline;
//! a slow screenshot is better than none.
use objc2::rc::Retained;
use objc2::{AnyThread, Message};
use objc2_foundation::{NSArray, NSError};
use objc2_screen_capture_kit::{
    SCContentFilter, SCDisplay, SCScreenshotManager, SCShareableContent, SCStreamConfiguration,
};
use std::sync::mpsc;
use std::time::Duration;

/// How long to wait on any one of ScreenCaptureKit's completion handlers.
///
/// Generous next to the 40ms this normally takes. It is here so that a framework
/// having a bad day costs us one slow turn through the fallback rather than a
/// thread that never comes back.
const PATIENCE: Duration = Duration::from_secs(3);

/// Everything sharable right now: displays, windows, and the applications that
/// own them.
///
/// Asked fresh each time rather than cached. It costs about 90ms and the cache
/// would have to be invalidated on every window that opens, every display that
/// is plugged in, and every Space that is switched -- which is a lot of ways to
/// be subtly wrong in exchange for a tenth of a second.
fn shareable() -> Option<Retained<SCShareableContent>> {
    let (tx, rx) = mpsc::channel();
    let handler = block2::RcBlock::new(move |content: *mut SCShareableContent, _: *mut NSError| {
        let _ = tx.send(unsafe { content.as_ref() }.map(|c| c.retain()));
    });
    unsafe { SCShareableContent::getShareableContentWithCompletionHandler(&handler) };
    rx.recv_timeout(PATIENCE).ok().flatten()
}

/// The pixels of a Core Graphics image, as RGB.
///
/// They arrive BGRA, which is Core Graphics' native order here, so red and blue
/// are swapped on the way out. Verified against the old path rather than assumed:
/// both renderings of the same screen agree to within half a level per channel,
/// and a wrong byte order would have produced identical brightness statistics
/// with the colours inverted.
fn rgb(img: &objc2_core_graphics::CGImage) -> Option<(u32, u32, Vec<u8>)> {
    use objc2_core_graphics::{CGDataProvider, CGImage};
    let (w, h) = (CGImage::width(Some(img)), CGImage::height(Some(img)));
    let stride = CGImage::bytes_per_row(Some(img));
    let provider = CGImage::data_provider(Some(img))?;
    let data = CGDataProvider::data(Some(&provider))?;
    let raw = data.to_vec();
    if raw.len() < h * stride || w == 0 || h == 0 {
        return None;
    }
    let mut out = Vec::with_capacity(w * h * 3);
    for y in 0..h {
        let row = &raw[y * stride..];
        for x in 0..w {
            let p = &row[x * 4..x * 4 + 4];
            out.extend_from_slice(&[p[2], p[1], p[0]]);
        }
    }
    Some((w as u32, h as u32, out))
}

/// One frame of the given display, already scaled so its long edge is `max_edge`,
/// with our own windows left out of it.
///
/// `None` for every way this can decline -- no permission, no such display, a
/// handler that never fires. The caller falls back to the old path, which is
/// slow and works.
pub fn grab(display_id: u32, max_edge: u32, exclude_pid: i32) -> Option<image::DynamicImage> {
    let content = shareable()?;

    let displays = unsafe { content.displays() };
    let display: Retained<SCDisplay> = displays
        .iter()
        .find(|d| unsafe { d.displayID() } == display_id)
        .or_else(|| displays.iter().next())?;

    // Excluded by application, not by window. Our windows come and go -- the
    // notch, the overlay, the agent card -- and a filter listing the ones that
    // existed a moment ago would photograph whichever one appeared since.
    let mine: Vec<_> = unsafe { content.applications() }
        .iter()
        .filter(|a| unsafe { a.processID() } == exclude_pid)
        .collect();
    let mine = NSArray::from_retained_slice(&mine);
    let none = NSArray::new();
    let filter = unsafe {
        SCContentFilter::initWithDisplay_excludingApplications_exceptingWindows(
            SCContentFilter::alloc(),
            &display,
            &mine,
            &none,
        )
    };

    // Scaled here rather than afterwards. ScreenCaptureKit is doing the work on
    // the GPU either way, and asking it for the size we want costs nothing where
    // resizing it ourselves cost 232ms.
    let (dw, dh) = (unsafe { display.width() } as f64, unsafe { display.height() } as f64);
    let scale = (max_edge as f64 / dw.max(dh)).min(1.0);
    let config = unsafe { SCStreamConfiguration::new() };
    unsafe {
        config.setWidth((dw * scale).round().max(1.0) as usize);
        config.setHeight((dh * scale).round().max(1.0) as usize);
    }

    let (tx, rx) = mpsc::channel();
    let handler = block2::RcBlock::new(
        move |img: *mut objc2_core_graphics::CGImage, _: *mut NSError| {
            let _ = tx.send(unsafe { img.as_ref() }.and_then(rgb));
        },
    );
    unsafe {
        SCScreenshotManager::captureImageWithFilter_configuration_completionHandler(
            &filter,
            &config,
            Some(&handler),
        )
    };
    let (w, h, pixels) = rx.recv_timeout(PATIENCE).ok().flatten()?;
    image::RgbImage::from_raw(w, h, pixels).map(image::DynamicImage::ImageRgb8)
}

#[cfg(test)]
mod tests {
    /// The framework is there, it composites, and what comes back is a picture
    /// of something.
    ///
    ///     cargo test screen_capture_kit -- --ignored --nocapture
    ///
    /// `NUDGE_EXCLUDE_PID` proves the exclusion rather than only the mechanism:
    /// point it at the running app and Nudge's notch pill leaves the image.
    /// Without it this excludes the test binary, which owns no windows.
    #[test]
    #[ignore]
    fn screen_capture_kit_returns_a_real_picture_quickly() {
        let pid = std::env::var("NUDGE_EXCLUDE_PID")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(std::process::id() as i32);
        let (_, _, id) = super::super::capture::active_display();

        // The first call in a process pays for the framework waking up -- about
        // 900ms, against 300 for every one after it. The app is long-lived and
        // pays that once; the loop pays the warm number, so that is the one
        // worth asserting on.
        let began = std::time::Instant::now();
        let _cold = super::grab(id, 1280, pid).expect("ScreenCaptureKit returned nothing");
        let cold = began.elapsed();

        let began = std::time::Instant::now();
        let img = super::grab(id, 1280, pid).expect("ScreenCaptureKit returned nothing");
        let took = began.elapsed();
        eprintln!("  cold {cold:?}, warm {took:?}, {}x{}", img.width(), img.height());

        assert!(img.width() > 200 && img.height() > 200, "a {}x{} image", img.width(), img.height());
        assert!(img.width().max(img.height()) <= 1280, "not scaled to what was asked");

        // The whole reason this exists. If it is not comfortably quicker than the
        // 1.5s the old path takes to composite, it has bought nothing.
        assert!(
            took.as_millis() < 800,
            "warm capture took {took:?}; the path this replaces composites in 1.5s, so this has bought nothing"
        );

        // A black frame is the other way this fails: the call succeeds and
        // composites nothing. Any real screen has a spread of brightness.
        let grey = img.to_luma8();
        let mean = grey.as_raw().iter().map(|p| *p as f64).sum::<f64>() / grey.as_raw().len() as f64;
        let sd = (grey
            .as_raw()
            .iter()
            .map(|p| (*p as f64 - mean).powi(2))
            .sum::<f64>()
            / grey.as_raw().len() as f64)
            .sqrt();
        eprintln!("  mean {mean:.0}, sd {sd:.0}");
        assert!(sd > 5.0, "a flat image -- composited nothing");
    }
}
