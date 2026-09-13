//! Screenshot -> the exact image we hand a model, plus the numbers to map back.
use crate::error::{Error, Result};
use std::io::Cursor;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

use serde::Serialize;

pub struct Shot {
    /// JPEG bytes. Named for what it is, not for the format it used to be.
    pub bytes: Vec<u8>,
    /// Size of the image the model actually saw. Its coordinates live in this space.
    pub sent: (u32, u32),
    /// The captured display's size in logical points.
    pub logical: (f64, f64),
    /// The captured display's top-left corner, in global points.
    ///
    /// Zero on the main display, and the whole reason a second monitor works:
    /// a point in the image is only meaningful once you know which display it
    /// came from. This used to be assumed rather than recorded, so on any
    /// display but the first, every coordinate was wrong by the offset between
    /// them.
    pub origin: (f64, f64),
}

impl Shot {
    /// The whole coordinate pipeline, in one place: image pixels to global
    /// screen points, which is the space clicks are posted in.
    ///
    /// Physical pixels never appear here on purpose: `logical / sent` folds the
    /// Retina backing factor and our own downscale into a single ratio. Two
    /// separate conversions is how you ship a ring that is off by exactly 2x.
    ///
    /// `origin` is what makes it right on a second display, and adds nothing on
    /// the first.
    pub fn to_global(&self, p: Point) -> Point {
        Point {
            x: self.origin.0 + p.x * self.logical.0 / self.sent.0 as f64,
            y: self.origin.1 + p.y * self.logical.1 / self.sent.1 as f64,
        }
    }

    /// The inverse of [`Shot::to_global`].
    ///
    /// The accessibility tree answers in global screen points, because that is
    /// where controls actually are. Everything downstream of a step expects
    /// image coordinates and converts them back on the way out, so a control is
    /// converted *into* the picture rather than teaching the whole path about a
    /// second coordinate space. One conversion each way, and they are tested to
    /// agree.
    pub fn to_image(&self, p: Point) -> Point {
        Point {
            x: (p.x - self.origin.0) * self.sent.0 as f64 / self.logical.0,
            y: (p.y - self.origin.1) * self.sent.1 as f64 / self.logical.1,
        }
    }

    /// A tiny greyscale thumbnail, for "did anything actually happen?".
    ///
    /// Comparing full screenshots is hopeless -- a clock digit, an antialiased
    /// cursor, or a blinking caret all differ. At 16x16 luma those vanish, and a
    /// menu opening or a window appearing still moves it decisively. The overlay's
    /// own cat and ring are well under one pixel at this size, so Nudge does not
    /// mistake its own animation for progress.
    pub fn fingerprint(&self) -> Vec<u8> {
        image::load_from_memory(&self.bytes)
            .map(|i| {
                i.resize_exact(16, 16, image::imageops::FilterType::Triangle)
                    .to_luma8()
                    .into_raw()
            })
            .unwrap_or_default()
    }

    pub fn b64(&self) -> String {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(&self.bytes)
    }
}

/// What the model is sent. JPEG, not PNG.
///
/// Measured: the same screenshot is ~930KB as PNG and ~130KB as JPEG, and upload
/// was the dominant cost of a step. Lossless pixels buy nothing here -- the model
/// is looking for a button, not inspecting compression artefacts.
const QUALITY: u8 = 78;
pub const MIME: &str = "image/jpeg";

/// Every on-screen window that is not ours, newest first.
///
/// Nudge is in every screenshot it takes: the companion follows the cursor and
/// the notch pill sits at the top. Feeding a model a picture of Nudge and asking
/// it what to click invites it to click Nudge -- one run stopped mid-task to
/// close "that mysterious black void of a window".
///
/// Returns `None` when the list cannot be read, so the caller falls back to a
/// plain capture rather than a blank screen.
#[cfg(target_os = "macos")]
fn other_windows(ours: i64) -> Option<core_foundation::array::CFArray> {
    use core_foundation::array::CFArray;
    use core_foundation::base::{CFType, TCFType};
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::number::CFNumber;
    use core_foundation::string::CFString;
    use core_graphics::window::{
        copy_window_info, kCGNullWindowID, kCGWindowListOptionOnScreenOnly,
    };

    // Desktop elements included on purpose. Excluding them drops the wallpaper
    // and the desktop icons, which are windows too -- so a screen with nothing
    // else open composited to solid black, and the model dutifully reported "a
    // completely black screen with no windows open". Hidden most of the time
    // because some window is usually covering it.
    let windows = copy_window_info(kCGWindowListOptionOnScreenOnly, kCGNullWindowID)?;

    let mut keep: Vec<*const std::ffi::c_void> = Vec::new();
    for item in windows.iter() {
        let dict = unsafe { CFDictionary::<CFType, CFType>::wrap_under_get_rule(*item as _) };
        let get = |key: &str| -> Option<i64> {
            dict.find(CFString::new(key).as_CFType())
                .and_then(|v| v.downcast::<CFNumber>())
                .and_then(|n| n.to_i64())
        };
        if get("kCGWindowOwnerPID") == Some(ours) {
            continue;
        }
        if let Some(id) = get("kCGWindowNumber") {
            keep.push(id as usize as *const std::ffi::c_void);
        }
    }
    (!keep.is_empty()).then(|| CFArray::from_copyable(&keep))
}

/// The display the pointer is on: its top-left and size, in global points.
///
/// Whichever screen someone is looking at is the one they mean, and the pointer
/// is the best available proxy for that. Falls back to the main display, which
/// is what the whole app assumed before.
#[cfg(target_os = "macos")]
pub fn active_display() -> ((f64, f64), (f64, f64)) {
    use core_graphics::display::CGDisplay;

    let main = CGDisplay::main();
    let fallback = {
        let b = main.bounds();
        ((b.origin.x, b.origin.y), (b.size.width, b.size.height))
    };
    let Some(at) = super::click::cursor() else {
        return fallback;
    };
    let Ok(ids) = CGDisplay::active_displays() else {
        return fallback;
    };
    for id in ids {
        let b = CGDisplay::new(id).bounds();
        let inside = at.x >= b.origin.x
            && at.x < b.origin.x + b.size.width
            && at.y >= b.origin.y
            && at.y < b.origin.y + b.size.height;
        if inside {
            return ((b.origin.x, b.origin.y), (b.size.width, b.size.height));
        }
    }
    fallback
}

#[cfg(not(target_os = "macos"))]
pub fn active_display() -> ((f64, f64), (f64, f64)) {
    ((0.0, 0.0), (1512.0, 982.0))
}

/// Capture the screen without Nudge in it.
///
/// ponytail: `CGWindowListCreateImageFromArray`, which Apple deprecated in macOS
/// 14 in favour of ScreenCaptureKit. Chosen because it is one call against a
/// crate already in the tree, where SCK is a new dependency and an async
/// completion handler for a synchronous need. Verified working on this machine;
/// when it stops, `objc2-screen-capture-kit` and `SCContentFilter`'s
/// `excludingWindows` do the same job.
///
/// Falls back to the whole screen if the window list is unreadable: a shot with
/// Nudge in it beats no shot at all.
#[cfg(target_os = "macos")]
fn capture_png(
    exclude_pid: i64,
    origin: (f64, f64),
    size: (f64, f64),
) -> Option<image::DynamicImage> {
    use core_graphics::display::CGDisplay;
    use core_graphics::geometry::{CGPoint, CGRect, CGSize};
    use core_graphics::window::kCGWindowImageDefault;

    let windows = other_windows(exclude_pid)?;
    // One display, not every window everywhere. Compositing the union of two
    // monitors gives one wide image that then downscales to half the detail on
    // each, and coordinates in it belong to no screen in particular.
    let rect = CGRect::new(
        &CGPoint::new(origin.0, origin.1),
        &CGSize::new(size.0, size.1),
    );
    let img = CGDisplay::screenshot_from_windows(rect, windows, kCGWindowImageDefault)?;

    // CGImage rows are padded to a stride; copying row by row is what keeps a
    // non-multiple-of-16 width from shearing the picture diagonally.
    let (w, h) = (img.width() as u32, img.height() as u32);
    let stride = img.bytes_per_row();
    let data = img.data();
    let src = data.bytes();
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h as usize {
        let row = &src[y * stride..y * stride + w as usize * 4];
        // CGImage is BGRA; image wants RGBA.
        for px in row.chunks_exact(4) {
            rgba.extend_from_slice(&[px[2], px[1], px[0], px[3]]);
        }
    }
    image::RgbaImage::from_raw(w, h, rgba).map(image::DynamicImage::ImageRgba8)
}

#[cfg(not(target_os = "macos"))]
fn capture_png(
    _exclude_pid: i64,
    _origin: (f64, f64),
    _size: (f64, f64),
) -> Option<image::DynamicImage> {
    None
}

/// Capture whichever display the pointer is on.
///
/// The geometry is read per capture rather than once at startup. It used to be
/// stored at launch from the primary monitor, so moving to a second screen --
/// or changing resolution, or unplugging a monitor -- left every coordinate
/// silently wrong until Nudge was restarted.
pub fn grab(max_edge: u32) -> Result<Shot> {
    let (origin, logical) = active_display();
    let img = match capture_png(std::process::id() as i64, origin, logical) {
        Some(img) => img,
        None => {
            // Fall back to the shell tool, which cannot exclude our windows but
            // always works.
            let tmp = std::env::temp_dir().join("nudge-shot.jpg");
            let out = std::process::Command::new("screencapture")
                .args(["-x", "-o", "-t", "jpg"])
                .arg(&tmp)
                .output()?;
            if !out.status.success() {
                return Err(Error::Capture(String::from_utf8_lossy(&out.stderr).into()));
            }
            image::open(&tmp)?
        }
    };
    let img = if img.width().max(img.height()) > max_edge {
        // Triangle, not Lanczos3: a third of the cost, and the difference is
        // invisible to something looking for a button.
        img.resize(max_edge, max_edge, image::imageops::FilterType::Triangle)
    } else {
        img
    };

    let mut bytes = Vec::new();
    img.write_with_encoder(image::codecs::jpeg::JpegEncoder::new_with_quality(
        &mut Cursor::new(&mut bytes),
        QUALITY,
    ))?;
    Ok(Shot {
        bytes,
        sent: (img.width(), img.height()),
        logical,
        origin,
    })
}

#[cfg(test)]
mod capture_tests {
    /// The wallpaper is a window, and leaving it out is how a screen with
    /// nothing open composites to solid black -- one run reported "a completely
    /// black screen with no windows or applications open", which was true of the
    /// picture and not of the screen.
    ///
    /// Checks the list contains something on a layer below the normal one, which
    /// is where the desktop and its icons live.
    #[test]
    fn the_desktop_itself_is_not_excluded() {
        use core_foundation::base::{CFType, TCFType};
        use core_foundation::dictionary::CFDictionary;
        use core_foundation::number::CFNumber;
        use core_foundation::string::CFString;
        use core_graphics::window::{
            copy_window_info, kCGNullWindowID, kCGWindowListOptionOnScreenOnly,
        };

        let windows = copy_window_info(kCGWindowListOptionOnScreenOnly, kCGNullWindowID)
            .expect("the window list should be readable");
        let below = windows.iter().any(|item| {
            let dict = unsafe { CFDictionary::<CFType, CFType>::wrap_under_get_rule(*item as _) };
            dict.find(CFString::new("kCGWindowLayer").as_CFType())
                .and_then(|v| v.downcast::<CFNumber>())
                .and_then(|n| n.to_i64())
                .is_some_and(|layer| layer < 0)
        });
        assert!(
            below,
            "nothing below the normal layer -- the desktop is missing"
        );
    }

    /// `CGWindowListCreateImageFromArray` is deprecated, and a deprecated API on
    /// a new macOS is a coin toss -- it may return null, or a black frame, or
    /// work. Only a real call answers that, so this takes one and writes it out.
    ///
    /// Ignored by default: it photographs whatever is on screen.
    ///
    ///     cargo test excluding_our_own_windows -- --ignored --nocapture
    #[test]
    #[ignore]
    fn excluding_our_own_windows_still_captures_a_screen() {
        // NUDGE_EXCLUDE_PID lets this prove the exclusion rather than only the
        // mechanism: run it against the live app's pid and Nudge's notch pill
        // disappears from the image. Without it, it excludes this test binary,
        // which owns no windows.
        let pid = std::env::var("NUDGE_EXCLUDE_PID")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(std::process::id() as i64);
        let (origin, size) = super::active_display();
        let img =
            super::capture_png(pid, origin, size).expect("the deprecated path returned nothing");
        let (w, h) = (img.width(), img.height());
        assert!(w > 200 && h > 200, "got a {w}x{h} image");

        // A black frame is the other way this fails: the call succeeds and
        // composites nothing. Any real screen has a spread of brightness.
        let grey = img.to_luma8();
        let (min, max) = grey
            .iter()
            .fold((255u8, 0u8), |(lo, hi), &p| (lo.min(p), hi.max(p)));
        assert!(
            max - min > 40,
            "flat image, {min}..{max} -- composited nothing"
        );

        let out = std::env::temp_dir().join("nudge-excluded.jpg");
        img.save(&out).unwrap();
        eprintln!("{w}x{h}, luma {min}..{max} -> {}", out.display());
    }
}

/// Wait until the screen stops changing, then let the caller photograph it.
///
/// A fixed wait cannot be right. 2.2 seconds is far too long after a click and
/// nowhere near enough for a web app that boots in the browser -- WhatsApp Web
/// was captured on its loading screen, so the agent reported a QR code to
/// someone whose chats were already on screen. It was not looking at an old
/// screenshot; it was given a new one taken too early.
///
/// So ask the screen instead of guessing. Tiny frames, compared until two agree:
/// at 16x16 luma a spinner or a caret does not register, but a page finishing
/// its load moves it decisively.
///
/// `max` is a real limit, not a target. Some screens never go still -- a playing
/// video, a progress bar -- and waiting for those to settle would wait forever.
pub async fn wait_until_still(max: std::time::Duration) {
    /// Small enough to be cheap, large enough that the fingerprint still means
    /// something. The cost here is compositing the screen, not the resizing.
    const PEEK: u32 = 240;
    const GAP: std::time::Duration = std::time::Duration::from_millis(120);

    let began = std::time::Instant::now();
    let mut last: Option<Vec<u8>> = None;
    while began.elapsed() < max {
        let Ok(shot) = grab(PEEK) else { return };
        let now = shot.fingerprint();
        if last.as_ref().is_some_and(|before| unchanged(before, &now)) {
            return;
        }
        last = Some(now);
        tokio::time::sleep(GAP).await;
    }
}

/// Mean absolute difference, on a 0-255 scale. Tuned to survive a caret and a
/// clock, but not a menu.
const STILL: u32 = 4;

pub fn unchanged(before: &[u8], after: &[u8]) -> bool {
    if before.is_empty() || before.len() != after.len() {
        return false;
    }
    let total: u32 = before
        .iter()
        .zip(after)
        .map(|(a, b)| a.abs_diff(*b) as u32)
        .sum();
    total / before.len() as u32 <= STILL
}

#[cfg(test)]
mod tests {
    /// The two conversions have to agree, or a control lands one display over.
    ///
    /// Written against the case that actually breaks: a second monitor, where
    /// `origin` is not zero, and a Retina downscale, where the ratio is not one.
    #[test]
    fn a_point_survives_the_round_trip_between_the_screen_and_the_picture() {
        let shot = Shot {
            bytes: Vec::new(),
            sent: (1280, 800),
            logical: (1512.0, 945.0),
            origin: (1512.0, -200.0),
        };
        for p in [
            Point { x: 0.0, y: 0.0 },
            Point { x: 640.0, y: 400.0 },
            Point { x: 1280.0, y: 800.0 },
        ] {
            let there_and_back = shot.to_image(shot.to_global(p));
            assert!(
                (there_and_back.x - p.x).abs() < 0.001 && (there_and_back.y - p.y).abs() < 0.001,
                "{p:?} came back as {there_and_back:?}"
            );
        }

        // And the direction that matters: a control at the display's own origin
        // is the top-left of the picture, not the top-left of the desktop.
        let at_origin = shot.to_image(Point { x: 1512.0, y: -200.0 });
        assert_eq!((at_origin.x, at_origin.y), (0.0, 0.0));
    }

    use super::*;

    fn shot(sent: (u32, u32), logical: (f64, f64)) -> Shot {
        at(sent, logical, (0.0, 0.0))
    }

    fn at(sent: (u32, u32), logical: (f64, f64), origin: (f64, f64)) -> Shot {
        Shot {
            bytes: vec![],
            sent,
            logical,
            origin,
        }
    }

    #[test]
    fn retina_and_downscale_collapse_into_one_ratio() {
        // 3024x1964 Retina panel -> 1512x982 logical, downscaled to 1920 long edge.
        let s = shot((1920, 1247), (1512.0, 982.0));
        let c = s.to_global(Point { x: 960.0, y: 623.5 });
        assert!((c.x - 756.0).abs() < 1.0, "x drifted: {}", c.x);
        assert!((c.y - 491.0).abs() < 1.0, "y drifted: {}", c.y);
    }

    /// The bug this exists for: the display a shot came from was assumed to be
    /// the main one, so on a second monitor every coordinate was out by the gap
    /// between them -- clicks landing on the wrong screen entirely.
    #[test]
    fn a_second_display_is_offset_by_its_own_origin() {
        // A 1920x1080 monitor sitting to the right of a 1512-point main display.
        let s = at((1920, 1080), (1920.0, 1080.0), (1512.0, 0.0));
        let c = s.to_global(Point { x: 0.0, y: 0.0 });
        assert_eq!((c.x, c.y), (1512.0, 0.0), "its top-left is not the world's");

        let c = s.to_global(Point { x: 960.0, y: 540.0 });
        assert_eq!((c.x, c.y), (2472.0, 540.0), "middle of the second screen");

        // And a display above or to the left has a negative origin, which must
        // pass straight through rather than being clamped.
        let s = at((1000, 1000), (1000.0, 1000.0), (-1000.0, -200.0));
        let c = s.to_global(Point { x: 500.0, y: 500.0 });
        assert_eq!((c.x, c.y), (-500.0, 300.0));
    }

    #[test]
    fn a_still_screen_reads_as_unchanged_despite_small_noise() {
        let before = vec![100u8; 256];
        let mut after = before.clone();
        after[0] = 180; // one pixel of caret or clock
        assert!(unchanged(&before, &after));
    }

    #[test]
    fn an_opened_menu_reads_as_changed() {
        let before = vec![100u8; 256];
        let mut after = before.clone();
        // A menu covers a real fraction of the screen, not one pixel.
        after[..60].fill(20);
        assert!(!unchanged(&before, &after));
        assert!(!unchanged(&[], &after), "no baseline is not 'unchanged'");
    }

    #[test]
    fn corners_stay_corners() {
        let s = shot((1920, 1080), (1512.0, 982.0));
        let o = s.to_global(Point { x: 0.0, y: 0.0 });
        assert_eq!((o.x, o.y), (0.0, 0.0));
        let o = s.to_global(Point {
            x: 1920.0,
            y: 1080.0,
        });
        assert_eq!((o.x, o.y), (1512.0, 982.0));
    }
}
