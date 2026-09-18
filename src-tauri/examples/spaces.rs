//! Diagnostic: is Nudge's overlay actually on screen right now?
//!
//! Polls the window server once a second and reports whether a window owned by
//! Nudge is in the on-screen list, and what is in front of it.
fn main() {
    use core_foundation::base::{CFType, TCFType};
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::number::CFNumber;
    use core_foundation::string::CFString;
    use core_graphics::geometry::CGRect;
    use core_graphics::window::{
        copy_window_info, kCGNullWindowID, kCGWindowListOptionOnScreenOnly,
    };

    println!("polling for 60s -- switch an app to full screen now");
    let mut last = String::new();
    let began = std::time::Instant::now();

    for _ in 0..60 {
        let mut front = String::from("?");
        // Every floating window, whoever owns it. Comparing ourselves against
        // another overlay app in the *same* Space, at the same moment, is worth
        // more than any amount of reasoning about window flags.
        let mut floating: Vec<String> = Vec::new();

        if let Some(windows) = copy_window_info(kCGWindowListOptionOnScreenOnly, kCGNullWindowID) {
            for item in windows.iter() {
                let d = unsafe { CFDictionary::<CFType, CFType>::wrap_under_get_rule(*item as _) };
                let get = |k: &str| d.find(CFString::new(k).as_CFType()).map(|v| v.clone());
                let owner = get("kCGWindowOwnerName")
                    .and_then(|v| v.downcast::<CFString>())
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                let layer = get("kCGWindowLayer")
                    .and_then(|v| v.downcast::<CFNumber>().and_then(|n| n.to_i64()))
                    .unwrap_or(-999);
                let alpha = get("kCGWindowAlpha")
                    .and_then(|v| v.downcast::<CFNumber>().and_then(|n| n.to_f64()))
                    .unwrap_or(-1.0);

                if layer > 0 {
                    // Size as well as owner. Three windows of ours on the list
                    // and one window of ours on the list are different answers,
                    // and which one survived is the whole question: the anchor is
                    // 1x1, the strip is 560 wide, the companion covers the desk.
                    let size = get("kCGWindowBounds")
                        .and_then(|v| v.downcast::<CFDictionary>())
                        .and_then(|d| CGRect::from_dict_representation(&d))
                        .map(|r| format!("{:.0}x{:.0}", r.size.width, r.size.height))
                        .unwrap_or_else(|| "?".into());
                    floating.push(format!("{owner}@{layer}/a{alpha:.1}/{size}"));
                } else if front == "?" && layer == 0 {
                    front = owner;
                }
            }
        }

        let line = format!(
            "front={front:<18} floating: {}",
            if floating.is_empty() {
                "(none)".into()
            } else {
                floating.join("  ")
            }
        );
        // Timestamped, and repeats kept. "Nothing of ours is on screen" is a
        // state with a duration, and printing only the transitions hides how
        // long it lasted -- which is the one number that says whether a window
        // was evicted or was mid-animation.
        let mark = if line == last { "" } else { "  <-- changed" };
        println!("{:5.1}s {line}{mark}", began.elapsed().as_secs_f32());
        last = line;
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    println!("done");
}
