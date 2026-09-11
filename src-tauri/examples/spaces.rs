//! Diagnostic: is Nudge's overlay actually on screen right now?
//!
//! Polls the window server once a second and reports whether a window owned by
//! Nudge is in the on-screen list, and what is in front of it.
fn main() {
    use core_foundation::base::{CFType, TCFType};
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::number::CFNumber;
    use core_foundation::string::CFString;
    use core_graphics::window::{
        copy_window_info, kCGNullWindowID, kCGWindowListOptionOnScreenOnly,
    };

    println!("polling for 60s -- switch an app to full screen now");
    let mut last = String::new();

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
                    floating.push(format!("{owner}@{layer}/a{alpha:.1}"));
                } else if front == "?" && layer == 0 {
                    front = owner;
                }
            }
        }

        let line = format!(
            "front={front:<18} floating: {}",
            if floating.is_empty() { "(none)".into() } else { floating.join("  ") }
        );
        if line != last {
            println!("{line}");
            last = line;
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    println!("done");
}
