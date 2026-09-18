//! What you drew while you were talking.
//!
//! Circling a thing and saying "what is this" is how people point at something
//! on a screen to each other, and it answers a question words are bad at: *that
//! one*, of the nine identical rows. The gesture is free -- the hotkey is already
//! held, the pointer is already under the hand -- and it costs one sentence
//! instead of three.
//!
//! **The mark is drawn twice.** The overlay draws it for the person, in colour,
//! fading as it goes, because a line that stays is a line to clean up. This
//! module draws it again into the screenshot, because the capture deliberately
//! leaves our own windows out -- so what the person sees and what the model sees
//! are two different paintings of the same gesture, and only this one has to
//! survive being compressed.
use super::capture::{Point, Shot};
use std::sync::Mutex;

/// A point on the screen, and when it was passed through.
type Dab = (Point, std::time::Instant);

static STROKES: Mutex<Vec<Dab>> = Mutex::new(Vec::new());

/// Longer than this between two points and the pen came up: two marks, not one
/// line joining them across the screen.
const LIFT: std::time::Duration = std::time::Duration::from_millis(220);

/// How wide the mark is drawn into the screenshot, in image pixels.
///
/// Fatter than it looks on screen. The picture the model sees is scaled down to
/// about 1280 across and then JPEG-compressed, and a one-pixel line survives
/// neither -- it turns into a grey suggestion nobody can follow.
const NIB: i64 = 7;

/// Nudge's pink. The mark has to be findable in a screenshot of an application
/// that may be any colour at all, and this is the one colour the rest of the
/// interface already uses for "look here".
const COLOUR: [u8; 3] = [0xFF, 0x2D, 0x55];

/// Remember where the pointer went. Global screen points.
pub fn add(at: Point) {
    let mut ink = STROKES.lock().unwrap();
    // Standing still is not drawing. Without this, holding the key without
    // moving piles up thousands of identical points and any mark that follows
    // is a blob with a tail.
    if let Some((last, _)) = ink.last() {
        if (last.x - at.x).abs() < 1.0 && (last.y - at.y).abs() < 1.0 {
            return;
        }
    }
    ink.push((at, std::time::Instant::now()));
}

/// Forget it. Called when a turn ends, and when the next one starts listening.
pub fn clear() {
    STROKES.lock().unwrap().clear();
}

pub fn drawn() -> bool {
    !STROKES.lock().unwrap().is_empty()
}

/// The marks, split where the pen came up.
fn marks() -> Vec<Vec<Point>> {
    let ink = STROKES.lock().unwrap();
    let mut out: Vec<Vec<Point>> = Vec::new();
    let mut last: Option<std::time::Instant> = None;
    for (at, when) in ink.iter() {
        // `is_none_or` would say this in one line and is newer than this
        // crate's MSRV.
        let lifted = match last {
            None => true,
            Some(prev) => when.duration_since(prev) > LIFT,
        };
        match lifted {
            true => out.push(vec![*at]),
            false => out.last_mut().expect("a mark is open").push(*at),
        }
        last = Some(*when);
    }
    out
}

/// Draw the marks into the picture the model is about to be shown.
///
/// Decode, draw, encode. The capture itself is left alone: on macOS it comes
/// back already encoded by the hardware, and taking a slower screenshot for the
/// sake of the drawing would make every turn pay for a gesture used in a few.
/// This way the cost lands on the turn that drew -- about seventy milliseconds --
/// and only that one.
pub fn burn(shot: &mut Shot) {
    let marks = marks();
    if marks.is_empty() {
        return;
    }
    let Ok(img) = image::load_from_memory(&shot.bytes) else {
        return;
    };
    let mut img = img.to_rgb8();

    for mark in &marks {
        for pair in mark.windows(2) {
            let a = shot.to_image(pair[0]);
            let b = shot.to_image(pair[1]);
            line(&mut img, a, b);
        }
        // A single dab is a tap, not a stroke, and would otherwise draw nothing.
        if mark.len() == 1 {
            let a = shot.to_image(mark[0]);
            line(&mut img, a, a);
        }
    }

    let mut bytes = Vec::new();
    let encoded = img.write_with_encoder(image::codecs::jpeg::JpegEncoder::new_with_quality(
        &mut std::io::Cursor::new(&mut bytes),
        super::capture::QUALITY,
    ));
    if encoded.is_ok() {
        shot.bytes = bytes;
    }
}

/// One segment, as a fat line.
///
/// Bresenham with a square nib rather than a drawing crate: it is twenty lines,
/// the shape only has to be followable by a model looking for what was circled,
/// and a dependency for this would be a dependency to keep up to date forever.
fn line(img: &mut image::RgbImage, a: Point, b: Point) {
    let (w, h) = (img.width() as i64, img.height() as i64);
    let (mut x0, mut y0) = (a.x as i64, a.y as i64);
    let (x1, y1) = (b.x as i64, b.y as i64);
    let (dx, dy) = ((x1 - x0).abs(), -(y1 - y0).abs());
    let (sx, sy) = (if x0 < x1 { 1 } else { -1 }, if y0 < y1 { 1 } else { -1 });
    let mut err = dx + dy;

    loop {
        for ox in -NIB / 2..=NIB / 2 {
            for oy in -NIB / 2..=NIB / 2 {
                let (px, py) = (x0 + ox, y0 + oy);
                if (0..w).contains(&px) && (0..h).contains(&py) {
                    img.put_pixel(px as u32, py as u32, image::Rgb(COLOUR));
                }
            }
        }
        if x0 == x1 && y0 == y1 {
            return;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(x: f64, y: f64) -> Point {
        Point { x, y }
    }

    #[test]
    fn standing_still_is_not_drawing() {
        clear();
        add(at(10.0, 10.0));
        add(at(10.4, 10.4));
        add(at(40.0, 40.0));
        assert_eq!(marks().into_iter().map(|m| m.len()).sum::<usize>(), 2);
    }

    /// A pen that came up and went down again drew two marks. Joined into one,
    /// the picture gets a line straight across whatever is between them --
    /// which is exactly the part of the screen being asked about.
    #[test]
    fn a_pause_between_marks_is_two_marks() {
        clear();
        add(at(0.0, 0.0));
        add(at(5.0, 5.0));
        std::thread::sleep(LIFT + std::time::Duration::from_millis(30));
        add(at(80.0, 80.0));
        add(at(85.0, 85.0));
        let marks = marks();
        assert_eq!(marks.len(), 2, "{marks:?}");
        assert_eq!(marks[0].len(), 2);
    }

    #[test]
    fn a_line_lands_where_it_was_drawn() {
        let mut img = image::RgbImage::new(40, 40);
        line(&mut img, at(5.0, 20.0), at(35.0, 20.0));
        assert_eq!(img.get_pixel(20, 20).0, COLOUR, "the middle of the line");
        assert_eq!(img.get_pixel(20, 2).0, [0, 0, 0], "well away from it");
    }
}
