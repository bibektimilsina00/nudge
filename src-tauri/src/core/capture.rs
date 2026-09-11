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
    /// Screen size in logical points -- the space the overlay window draws in.
    pub logical: (f64, f64),
}

impl Shot {
    /// The whole coordinate pipeline, in one place.
    ///
    /// Physical pixels never appear here on purpose: `logical / sent` folds the Retina
    /// backing factor and our own downscale into a single ratio. Two separate
    /// conversions is how you ship a ring that is off by exactly 2x.
    pub fn to_overlay(&self, p: Point) -> Point {
        Point {
            x: p.x * self.logical.0 / self.sent.0 as f64,
            y: p.y * self.logical.1 / self.sent.1 as f64,
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
            .map(|i| i.resize_exact(16, 16, image::imageops::FilterType::Triangle).to_luma8().into_raw())
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

/// ponytail: shells out to `screencapture` (~200ms, zero crates). Swap for the
/// ScreenCaptureKit bindings only once that 200ms is measurably in the way --
/// and note it would also let us exclude Nudge's own overlay from the shot.
pub fn grab(max_edge: u32, logical: (f64, f64)) -> Result<Shot> {
    // Capture straight to JPEG: decoding a 6MP PNG back in was pure waste.
    let tmp = std::env::temp_dir().join("nudge-shot.jpg");
    let out = std::process::Command::new("screencapture")
        .args(["-x", "-o", "-t", "jpg"])
        .arg(&tmp)
        .output()?;
    if !out.status.success() {
        return Err(Error::Capture(String::from_utf8_lossy(&out.stderr).into()));
    }

    let img = image::open(&tmp)?;
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
    Ok(Shot { bytes, sent: (img.width(), img.height()), logical })
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
    use super::*;

    fn shot(sent: (u32, u32), logical: (f64, f64)) -> Shot {
        Shot { bytes: vec![], sent, logical }
    }

    #[test]
    fn retina_and_downscale_collapse_into_one_ratio() {
        // 3024x1964 Retina panel -> 1512x982 logical, downscaled to 1920 long edge.
        let s = shot((1920, 1247), (1512.0, 982.0));
        let c = s.to_overlay(Point { x: 960.0, y: 623.5 });
        assert!((c.x - 756.0).abs() < 1.0, "x drifted: {}", c.x);
        assert!((c.y - 491.0).abs() < 1.0, "y drifted: {}", c.y);
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
        let o = s.to_overlay(Point { x: 0.0, y: 0.0 });
        assert_eq!((o.x, o.y), (0.0, 0.0));
        let o = s.to_overlay(Point { x: 1920.0, y: 1080.0 });
        assert_eq!((o.x, o.y), (1512.0, 982.0));
    }
}
