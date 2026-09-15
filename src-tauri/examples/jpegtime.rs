//! Who should turn the frame into a JPEG: us, or the machine?
//!
//! The encode is now the largest part of taking a screenshot -- bigger than
//! compositing, and untouched by anything a running capture stream would fix.
//! We do it in Rust from RGB bytes we converted ourselves. macOS has a hardware
//! encoder, and ScreenCaptureKit already hands us a CGImage, so both the
//! conversion and the encode could belong to someone else.
use objc2_core_foundation::{CFDictionary, CFMutableData, CFString};
use objc2_image_io::CGImageDestination;
use std::time::Instant;

fn main() {
    let (_, _, id) = nudge_lib::core::screen::capture::active_display();

    // Ours: composite, convert to RGB, encode in Rust.
    let began = Instant::now();
    let img =
        nudge_lib::core::screen::fast::grab(id, 1280, std::process::id() as i32).expect("frame");
    let composited = began.elapsed().as_secs_f32() * 1000.0;
    let t = Instant::now();
    let mut bytes = Vec::new();
    img.write_with_encoder(image::codecs::jpeg::JpegEncoder::new_with_quality(
        &mut std::io::Cursor::new(&mut bytes),
        78,
    ))
    .expect("encode");
    println!(
        "  ours    composite+convert {composited:>6.0}ms   encode {:>6.0}ms   {:>4}KB",
        t.elapsed().as_secs_f32() * 1000.0,
        bytes.len() / 1024
    );

    // Theirs: hand the CGImage straight to ImageIO.
    let began = Instant::now();
    let cg =
        nudge_lib::core::screen::fast::frame(id, 1280, std::process::id() as i32).expect("frame");
    let composited = began.elapsed().as_secs_f32() * 1000.0;
    let t = Instant::now();
    let data = CFMutableData::new(None, 0).expect("data");
    let kind = CFString::from_static_str("public.jpeg");
    let dest = unsafe { CGImageDestination::with_data(&data, &kind, 1, None) }.expect("dest");
    let key = unsafe { objc2_image_io::kCGImageDestinationLossyCompressionQuality };
    let quality = objc2_core_foundation::CFNumber::new_f64(0.78);
    let props = CFDictionary::from_slices(
        &[key],
        &[quality.as_ref() as &objc2_core_foundation::CFType],
    );
    let props = props.as_opaque();
    unsafe { dest.add_image(&cg, Some(props)) };
    let ok = unsafe { dest.finalize() };
    println!(
        "  theirs  composite         {composited:>6.0}ms   encode {:>6.0}ms   {:>4}KB  ok={ok}",
        t.elapsed().as_secs_f32() * 1000.0,
        data.len() / 1024
    );
}
