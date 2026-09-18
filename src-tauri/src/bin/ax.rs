//! What does macOS already know about the controls on screen?
//!
//! A front end for [`nudge_lib::core::screen::ax`], which is where the work is.
//! This exists because that question has to be asked of real applications -- a
//! native one, a browser, an Electron app -- and the answers differ enough that
//! guessing was never going to do it.
//!
//!     cargo run --features appkit --bin ax               # the frontmost application
//!     cargo run --features appkit --bin ax -- search     # only controls matching "search"
//!     cargo run --features appkit --bin ax -- --pid 7193 # a particular one, no countdown
//!
//! Behind `appkit` because it is CoreGraphics and the accessibility tree all the
//! way down, and a Linux build has neither.
//!
//! Note what it cannot tell you. An application with no window open exposes a
//! menu bar and nothing else, which reads exactly like an application that
//! exposes nothing at all. Two rounds of that were misread here as an Electron
//! limitation before `osascript -e 'tell application "System Events" to get
//! count of windows of process "Code"'` settled it.
use nudge_lib::core::screen::ax;

fn frontmost_pid() -> Option<i32> {
    use core_foundation::base::{CFType, TCFType};
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::number::CFNumber;
    use core_foundation::string::CFString;
    use core_graphics::window::{
        copy_window_info, kCGNullWindowID, kCGWindowListExcludeDesktopElements,
        kCGWindowListOptionOnScreenOnly,
    };
    let windows = copy_window_info(
        kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements,
        kCGNullWindowID,
    )?;
    let get = |d: &CFDictionary<CFType, CFType>, key: &str| -> Option<CFType> {
        d.find(CFString::new(key).as_CFType()).map(|v| v.clone())
    };
    for item in windows.iter() {
        let dict = unsafe { CFDictionary::<CFType, CFType>::wrap_under_get_rule(*item as _) };
        let layer = get(&dict, "kCGWindowLayer")
            .and_then(|v| v.downcast::<CFNumber>())
            .and_then(|n| n.to_i64())
            .unwrap_or(1);
        if layer != 0 {
            continue; // menu bars, docks, our own overlay
        }
        return get(&dict, "kCGWindowOwnerPID")
            .and_then(|v| v.downcast::<CFNumber>())
            .and_then(|n| n.to_i64())
            .map(|p| p as i32);
    }
    None
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let asked = args
        .iter()
        .position(|a| a == "--pid")
        .and_then(|i| args.get(i + 1))
        .and_then(|p| p.parse::<i32>().ok());
    let needle = args
        .iter()
        .filter(|a| a.as_str() != "--pid" && a.parse::<i32>().is_err())
        .cloned()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();

    if asked.is_none() {
        for n in (1..=4).rev() {
            eprint!("\rbring the app you want to the front... {n} ");
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
        eprintln!("\r                                        ");
    }

    let Some(pid) = asked.or_else(frontmost_pid) else {
        eprintln!("could not tell what is frontmost");
        std::process::exit(1);
    };

    // `--press "Label"` asks the application to do it, instead of sending the
    // mouse. Watch the pointer while this runs: it should not move at all.
    if let Some(i) = args.iter().position(|a| a == "--press") {
        let label = args.get(i + 1).cloned().unwrap_or_default();
        let before = nudge_lib::core::screen::click::cursor();
        let began = std::time::Instant::now();
        let ok = ax::press(pid, &label);
        let after = nudge_lib::core::screen::click::cursor();
        println!(
            "  press {label:?}: {} in {:.0}ms",
            if ok { "done" } else { "REFUSED" },
            began.elapsed().as_secs_f32() * 1000.0
        );
        println!("  pointer {before:?} -> {after:?}");
        return;
    }

    let began = std::time::Instant::now();
    let found = ax::controls(pid);
    println!(
        "pid {pid}: {} controls in {:.0}ms",
        found.len(),
        began.elapsed().as_secs_f32() * 1000.0
    );

    // The question that matters: would this have skipped the model? Say it the
    // way you would say it out loud.
    //
    //     cargo run --bin ax -- --pid 123 click the search icon
    if needle.starts_with("click ") || needle.starts_with("press ") {
        match ax::obvious(&needle, &found) {
            Some(c) => println!(
                "  obvious: {} {:?} at ({:.0},{:.0}) -- the model would be skipped\n",
                c.role, c.label, c.at.0, c.at.1
            ),
            None => println!("  not obvious -- this would go to the model\n"),
        }
    }

    let shown: Vec<&ax::Control> = found
        .iter()
        .filter(|c| needle.is_empty() || c.label.to_lowercase().contains(&needle))
        .collect();
    if !needle.is_empty() {
        println!("{} matching {needle:?}", shown.len());
    }
    println!();
    for c in shown.iter().take(60) {
        println!(
            "  {:<22} {:<46} at ({:.0},{:.0}) {:.0}x{:.0}",
            c.role, c.label, c.at.0, c.at.1, c.size.0, c.size.1
        );
    }
    if shown.len() > 60 {
        println!("  ... and {} more", shown.len() - 60);
    }
}
