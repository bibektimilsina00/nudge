//! Does macOS already know what is on the screen?
//!
//! The whole of Phase 5 in [SPEED.md] rests on one question nobody has asked:
//! when an app draws a button, does it also *say* there is a button, where it is,
//! and what it is called? If it does, finding it costs a function call rather
//! than a six second round trip to a vision model.
//!
//! The doubt is Electron. A native app is built from real controls and gets an
//! accessibility tree for free; an Electron app is a web page in a window, and
//! what it exposes depends on how much its authors cared. VS Code is in the
//! bench cases, so this is answerable rather than arguable.
//!
//!     cargo run --bin ax                      # what the frontmost app exposes
//!     cargo run --bin ax -- search            # only elements matching "search"
//!
//! Counts down first, so you can bring the app you want to the front.
use core_foundation::base::{CFType, CFTypeRef, TCFType};
use core_foundation::string::{CFString, CFStringRef};
use core_graphics::geometry::{CGPoint, CGSize};
use std::ffi::c_void;

type AXUIElementRef = CFTypeRef;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn AXUIElementCreateApplication(pid: i32) -> AXUIElementRef;
    fn AXUIElementCopyAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: *mut CFTypeRef,
    ) -> i32;
    fn AXUIElementSetMessagingTimeout(element: AXUIElementRef, seconds: f32) -> i32;
    fn AXUIElementSetAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: CFTypeRef,
    ) -> i32;
    fn AXValueGetValue(value: CFTypeRef, the_type: u32, out: *mut c_void) -> bool;
}

const K_AX_VALUE_CG_POINT: u32 = 1;
const K_AX_VALUE_CG_SIZE: u32 = 2;

/// One attribute of one element, or nothing.
///
/// Nothing is the common case and not an error: most elements have no title,
/// plenty have no position, and an app is free to answer every question with a
/// shrug. Distinguishing "no" from "failed to ask" is not useful here -- either
/// way we did not learn where the button is.
fn attr(el: AXUIElementRef, name: &str) -> Option<CFType> {
    let key = CFString::new(name);
    let mut out: CFTypeRef = std::ptr::null();
    let err = unsafe { AXUIElementCopyAttributeValue(el, key.as_concrete_TypeRef(), &mut out) };
    if err != 0 || out.is_null() {
        return None;
    }
    Some(unsafe { CFType::wrap_under_create_rule(out) })
}

fn text(el: AXUIElementRef, name: &str) -> Option<String> {
    let v = attr(el, name)?;
    v.downcast::<CFString>().map(|s| s.to_string())
}

fn frame(el: AXUIElementRef) -> Option<(CGPoint, CGSize)> {
    let (p, s) = (attr(el, "AXPosition")?, attr(el, "AXSize")?);
    let mut point = CGPoint::new(0.0, 0.0);
    let mut size = CGSize::new(0.0, 0.0);
    let ok = unsafe {
        AXValueGetValue(
            p.as_CFTypeRef(),
            K_AX_VALUE_CG_POINT,
            &mut point as *mut _ as *mut c_void,
        ) && AXValueGetValue(
            s.as_CFTypeRef(),
            K_AX_VALUE_CG_SIZE,
            &mut size as *mut _ as *mut c_void,
        )
    };
    ok.then_some((point, size))
}

/// The children of `el`, under whichever attribute holds them.
///
/// `AXChildren` is the general one. `AXWindows` is not always a subset of it --
/// several applications answer `AXChildren` with their menu bar alone and keep
/// the windows behind the specific name, which is how the first run of this
/// probe concluded that Chromium exposes nothing but a menu bar.
fn roots(el: AXUIElementRef) -> Vec<CFType> {
    let mut out = under(el, "AXChildren");
    // Usually the same windows again, and walking them twice counts every button
    // twice. Only worth asking when `AXChildren` admitted to no window at all.
    let has_window = out
        .iter()
        .any(|c| text(c.as_CFTypeRef(), "AXRole").as_deref() == Some("AXWindow"));
    if !has_window {
        out.extend(under(el, "AXWindows"));
    }
    out
}

fn children(el: AXUIElementRef) -> Vec<CFType> {
    under(el, "AXChildren")
}

fn under(el: AXUIElementRef, attribute: &str) -> Vec<CFType> {
    use core_foundation::array::{CFArrayGetCount, CFArrayGetTypeID, CFArrayGetValueAtIndex, CFArrayRef};
    use core_foundation::base::CFGetTypeID;
    let Some(v) = attr(el, attribute) else {
        return Vec::new();
    };
    // An app is free to answer "AXChildren" with anything it likes, and one that
    // answers with a string would have us read a CFString as an array. Ask what
    // it is before believing it: the first version of this trusted the name and
    // crashed with a SIGTRAP.
    if unsafe { CFGetTypeID(v.as_CFTypeRef()) } != unsafe { CFArrayGetTypeID() } {
        return Vec::new();
    }
    let array = v.as_CFTypeRef() as CFArrayRef;
    let n = unsafe { CFArrayGetCount(array) };
    // Retained one by one, not borrowed. The array owns these, and the array is
    // `v` -- which is dropped on the way out of this function, taking the
    // children with it. That was the other half of the same crash.
    (0..n)
        .filter_map(|i| {
            let p = unsafe { CFArrayGetValueAtIndex(array, i) } as CFTypeRef;
            (!p.is_null()).then(|| unsafe { CFType::wrap_under_get_rule(p) })
        })
        .collect()
}

/// What a control has to have before it could stand in for a vision model:
/// something to match a spoken goal against, and somewhere to click.
struct Found {
    role: String,
    label: String,
    at: CGPoint,
    size: CGSize,
    depth: usize,
}

/// Roles worth clicking. Everything else is layout -- groups, splitters, static
/// text -- and counting those as "exposed" would flatter the result.
const ACTIONABLE: [&str; 12] = [
    "AXButton",
    "AXMenuItem",
    "AXMenuBarItem",
    "AXCheckBox",
    "AXRadioButton",
    "AXPopUpButton",
    "AXTextField",
    "AXTextArea",
    "AXLink",
    "AXTab",
    "AXRow",
    "AXCell",
];

/// How long the walk may take before we call it: an answer that needs a minute
/// is not an answer, because the point of asking the system is that it is
/// instant. If a tree cannot be read in a second it cannot replace a model call.
const BUDGET: std::time::Duration = std::time::Duration::from_millis(1500);

// Every role encountered, and whether it had anything to call it by. Only for
// the `--roles` histogram -- threading a second accumulator through a recursive
// walk to answer a diagnostic question is not worth the signature.
thread_local! {
    static SEEN_ROLES: std::cell::RefCell<Vec<(String, bool)>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn walk(
    el: AXUIElementRef,
    depth: usize,
    seen: &mut usize,
    out: &mut Vec<Found>,
    began: std::time::Instant,
) {
    const MAX_ELEMENTS: usize = 20_000;
    const MAX_DEPTH: usize = 60;
    if *seen >= MAX_ELEMENTS || depth > MAX_DEPTH || began.elapsed() > BUDGET {
        return;
    }
    *seen += 1;

    let role = text(el, "AXRole").unwrap_or_default();
    // In order of how a person would name the thing out loud.
    let label = ["AXTitle", "AXDescription", "AXValue", "AXHelp"]
        .iter()
        .find_map(|a| text(el, a))
        .unwrap_or_default();
    SEEN_ROLES.with(|r| r.borrow_mut().push((role.clone(), !label.trim().is_empty())));
    if ACTIONABLE.contains(&role.as_str()) && !label.trim().is_empty() {
        if let Some((at, size)) = frame(el) {
            out.push(Found {
                role: role.clone(),
                label: label.chars().take(60).collect(),
                at,
                size,
                depth,
            });
        }
    }
    for child in children(el) {
        walk(child.as_CFTypeRef(), depth + 1, seen, out, began);
    }
}

fn frontmost_pid() -> Option<(i32, String)> {
    use core_foundation::base::CFType;
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::number::CFNumber;
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
        let pid = get(&dict, "kCGWindowOwnerPID")
            .and_then(|v| v.downcast::<CFNumber>())
            .and_then(|n| n.to_i64())? as i32;
        let name = get(&dict, "kCGWindowOwnerName")
            .and_then(|v| v.downcast::<CFString>())
            .map(|s| s.to_string())
            .unwrap_or_default();
        return Some((pid, name));
    }
    None
}

fn main() {
    if !unsafe { AXIsProcessTrusted() } {
        eprintln!(
            "Not trusted for accessibility. This binary needs its own grant --\n\
             System Settings > Privacy & Security > Accessibility, then add\n\
             the `ax` binary or the terminal running it."
        );
        std::process::exit(1);
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    // `--pid 123` skips the countdown entirely, which is how this gets pointed at
    // six applications in a row without a human alt-tabbing for each one.
    let asked_pid: Option<i32> = args
        .iter()
        .position(|a| a == "--pid")
        .and_then(|i| args.get(i + 1))
        .and_then(|p| p.parse().ok());
    let needle = args
        .iter()
        .filter(|a| a.as_str() != "--pid")
        .filter(|a| a.parse::<i32>().is_err())
        .cloned()
        .collect::<Vec<_>>()
        .join(" ");

    for n in (1..=4).rev() {
        if asked_pid.is_some() {
            break;
        }
        eprint!("\rbring the app you want to the front... {n} ");
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    eprintln!("\r                                        ");

    let Some((pid, name)) = asked_pid
        .map(|p| (p, format!("pid {p}")))
        .or_else(frontmost_pid)
    else {
        eprintln!("could not tell what is frontmost");
        std::process::exit(1);
    };

    let app = unsafe { AXUIElementCreateApplication(pid) };
    if app.is_null() {
        eprintln!("{name} (pid {pid}) exposes no accessibility element at all");
        std::process::exit(1);
    }
    // An app that is busy will otherwise hang us for its own idea of forever.
    unsafe { AXUIElementSetMessagingTimeout(app, 2.0) };

    // Chromium and Electron keep their accessibility tree switched off until
    // something asks for it -- building it is not free, and nothing needs it
    // until a screen reader turns up. These two attributes are how a screen
    // reader announces itself. Without them a browser exposes its menu bar and
    // nothing else: no address bar, no tabs, no page.
    //
    // Not free for them either. This asks a large application to maintain a
    // parallel tree of its entire UI for as long as we are watching, which is
    // the real cost of this approach and belongs in the decision.
    if !std::env::args().any(|a| a == "--no-enable") {
        use core_foundation::boolean::CFBoolean;
        let yes = CFBoolean::true_value();
        for attribute in ["AXManualAccessibility", "AXEnhancedUserInterface"] {
            let key = CFString::new(attribute);
            let err = unsafe {
                AXUIElementSetAttributeValue(app, key.as_concrete_TypeRef(), yes.as_CFTypeRef())
            };
            eprintln!("  set {attribute} -> {err}");
        }
        // It builds the tree asynchronously; walking immediately finds the old one.
        std::thread::sleep(std::time::Duration::from_millis(600));
    }

    // What the application itself hangs off. An app with no AXWindows is either
    // hiding its content or has not built it yet, and that is worth seeing
    // before drawing conclusions from an empty search.
    eprintln!(
        "  AXChildren {} · AXWindows {}",
        under(app, "AXChildren").len(),
        under(app, "AXWindows").len()
    );
    let roles: Vec<String> = roots(app)
        .iter()
        .map(|c| {
            let r = text(c.as_CFTypeRef(), "AXRole").unwrap_or_default();
            let n = children(c.as_CFTypeRef()).len();
            format!("{r}({n})")
        })
        .collect();
    eprintln!("  top level: {}", roles.join(" "));

    let began = std::time::Instant::now();
    let (mut seen, mut found) = (0usize, Vec::new());
    for root in roots(app) {
        walk(root.as_CFTypeRef(), 1, &mut seen, &mut found, began);
    }
    let took = began.elapsed();

    println!("{name} (pid {pid})");
    println!(
        "  walked {seen} elements in {:.0}ms{}, {} of them clickable and named",
        took.as_secs_f32() * 1000.0,
        if took > BUDGET { " (ran out of time)" } else { "" },
        found.len()
    );

    if args.iter().any(|a| a == "--roles") {
        let mut counts: std::collections::BTreeMap<String, (usize, usize)> = Default::default();
        SEEN_ROLES.with(|r| {
            for (role, named) in r.borrow().iter() {
                let e = counts.entry(role.clone()).or_default();
                e.0 += 1;
                e.1 += usize::from(*named);
            }
        });
        let mut rows: Vec<_> = counts.into_iter().collect();
        rows.sort_by_key(|(_, (n, _))| std::cmp::Reverse(*n));
        println!("\n  role                        total   named");
        for (role, (n, named)) in rows.iter().take(25) {
            println!("  {role:<26} {n:>6}  {named:>6}");
        }
        return;
    }

    let shown: Vec<&Found> = match needle.is_empty() {
        true => found.iter().collect(),
        false => found
            .iter()
            .filter(|f| {
                f.label.to_lowercase().contains(&needle.to_lowercase())
                    || f.role.to_lowercase().contains(&needle.to_lowercase())
            })
            .collect(),
    };
    if !needle.is_empty() {
        println!("  {} matching {needle:?}\n", shown.len());
    } else {
        println!();
    }
    for f in shown.iter().take(60) {
        println!(
            "  {:<16} {:<44} at ({:.0},{:.0}) {:.0}x{:.0}  depth {}",
            f.role, f.label, f.at.x, f.at.y, f.size.width, f.size.height, f.depth
        );
    }
    if shown.len() > 60 {
        println!("  ... and {} more", shown.len() - 60);
    }
}
