//! What the system already knows about the controls on screen.
//!
//! A screenshot is a picture of a button. The accessibility tree is the button:
//! its name, its role, and the exact rectangle it occupies. Asking for it costs
//! a few hundred milliseconds and no tokens; working it out from pixels costs
//! several seconds and is right about two thirds of the time.
//!
//! This is the same rule the audio fact follows, applied to the harder half of
//! the problem: *before teaching a model to recognise something, check whether
//! macOS will simply tell us.*
//!
//! It does not replace the picture. Plenty is not exposed -- custom-drawn
//! interfaces, canvases, games, anything in a video -- and a closed menu has no
//! geometry at all until it opens. So this is a fast path over a floor, not a
//! replacement for one.
#[cfg(target_os = "macos")]
mod imp {
    use core_foundation::array::{
        CFArrayGetCount, CFArrayGetTypeID, CFArrayGetValueAtIndex, CFArrayRef,
    };
    use core_foundation::base::{CFGetTypeID, CFType, CFTypeRef, TCFType};
    use core_foundation::boolean::CFBoolean;
    use core_foundation::string::{CFString, CFStringRef};
    use core_graphics::geometry::{CGPoint, CGSize};
    use std::ffi::c_void;

    type Ref = CFTypeRef;

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> bool;
        fn AXUIElementCreateApplication(pid: i32) -> Ref;
        fn AXUIElementCopyAttributeValue(el: Ref, attr: CFStringRef, out: *mut Ref) -> i32;
        fn AXUIElementSetAttributeValue(el: Ref, attr: CFStringRef, value: Ref) -> i32;
        fn AXUIElementSetMessagingTimeout(el: Ref, seconds: f32) -> i32;
        fn AXValueGetValue(value: Ref, kind: u32, out: *mut c_void) -> bool;
    }

    const CG_POINT: u32 = 1;
    const CG_SIZE: u32 = 2;

    /// How long the whole walk may take.
    ///
    /// The entire point is that asking the system is instant. A tree that needs
    /// a second and a half to read has already lost to the screenshot it was
    /// meant to replace, so we take what we have and move on.
    const BUDGET: std::time::Duration = std::time::Duration::from_millis(900);
    const MAX_ELEMENTS: usize = 6_000;
    const MAX_DEPTH: usize = 60;

    /// Roles worth clicking. Everything else is layout, and counting a group or
    /// a splitter as a control would bury the real ones.
    const ACTIONABLE: [&str; 13] = [
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
        "AXDisclosureTriangle",
    ];

    /// A control the system says is there.
    #[derive(Debug, Clone, PartialEq)]
    pub struct Control {
        pub role: String,
        pub label: String,
        /// Centre, in global screen points -- the same space clicks are posted
        /// in, so this needs no conversion and no screenshot to be meaningful.
        pub at: (f64, f64),
        pub size: (f64, f64),
    }

    fn attr(el: Ref, name: &str) -> Option<CFType> {
        let key = CFString::new(name);
        let mut out: Ref = std::ptr::null();
        let err = unsafe { AXUIElementCopyAttributeValue(el, key.as_concrete_TypeRef(), &mut out) };
        (err == 0 && !out.is_null()).then(|| unsafe { CFType::wrap_under_create_rule(out) })
    }

    fn text(el: Ref, name: &str) -> Option<String> {
        attr(el, name)?
            .downcast::<CFString>()
            .map(|s| s.to_string())
            .filter(|s| !s.trim().is_empty())
    }

    /// Children under a named attribute, each retained.
    ///
    /// Retained rather than borrowed because the array owns them and the array
    /// is dropped on the way out of here. Borrowing crashed with a SIGTRAP.
    fn under(el: Ref, attribute: &str) -> Vec<CFType> {
        let Some(v) = attr(el, attribute) else {
            return Vec::new();
        };
        // An application may answer any attribute with anything. Reading a
        // CFString as a CFArray is the other half of that same crash.
        if unsafe { CFGetTypeID(v.as_CFTypeRef()) } != unsafe { CFArrayGetTypeID() } {
            return Vec::new();
        }
        let array = v.as_CFTypeRef() as CFArrayRef;
        (0..unsafe { CFArrayGetCount(array) })
            .filter_map(|i| {
                let p = unsafe { CFArrayGetValueAtIndex(array, i) } as Ref;
                (!p.is_null()).then(|| unsafe { CFType::wrap_under_get_rule(p) })
            })
            .collect()
    }

    /// Where to start walking.
    ///
    /// `AXWindows` is asked for separately because several applications answer
    /// `AXChildren` with their menu bar alone and keep the windows behind the
    /// specific name -- which had the first version of this reporting that
    /// Chromium exposes nothing but a menu bar.
    fn roots(app: Ref) -> Vec<CFType> {
        let mut out = under(app, "AXChildren");
        let has_window = out
            .iter()
            .any(|c| text(c.as_CFTypeRef(), "AXRole").as_deref() == Some("AXWindow"));
        if !has_window {
            out.extend(under(app, "AXWindows"));
        }
        out
    }

    fn own_label(el: Ref) -> Option<String> {
        ["AXTitle", "AXDescription", "AXValue", "AXHelp"]
            .iter()
            .find_map(|a| text(el, a))
    }

    /// The text inside an unnamed control.
    ///
    /// What is clickable and what is named are frequently different elements: a
    /// file row in Finder is the click target and has no title at all, while the
    /// filename sits in an `AXStaticText` inside it. Measured on one window --
    /// fifteen rows and fifteen cells, not one of them named, and thirty-two
    /// static texts holding every name on screen.
    ///
    /// Shallow on purpose. A row's name is a level or two down; deeper than that
    /// and a label becomes a transcript of the window.
    fn inner_text(el: Ref, depth: usize) -> String {
        if depth > 3 {
            return String::new();
        }
        let mut parts = Vec::new();
        for child in under(el, "AXChildren") {
            let c = child.as_CFTypeRef();
            match own_label(c) {
                Some(s) => parts.push(s),
                None => {
                    let deeper = inner_text(c, depth + 1);
                    if !deeper.is_empty() {
                        parts.push(deeper);
                    }
                }
            }
            if parts.len() >= 4 {
                break;
            }
        }
        parts.join(" ")
    }

    fn frame(el: Ref) -> Option<((f64, f64), (f64, f64))> {
        let (p, s) = (attr(el, "AXPosition")?, attr(el, "AXSize")?);
        let mut point = CGPoint::new(0.0, 0.0);
        let mut size = CGSize::new(0.0, 0.0);
        let ok = unsafe {
            AXValueGetValue(p.as_CFTypeRef(), CG_POINT, &mut point as *mut _ as *mut c_void)
                && AXValueGetValue(s.as_CFTypeRef(), CG_SIZE, &mut size as *mut _ as *mut c_void)
        };
        ok.then_some(((point.x, point.y), (size.width, size.height)))
    }

    fn walk(el: Ref, depth: usize, seen: &mut usize, out: &mut Vec<Control>, began: std::time::Instant) {
        if *seen >= MAX_ELEMENTS || depth > MAX_DEPTH || began.elapsed() > BUDGET {
            return;
        }
        *seen += 1;

        let role = text(el, "AXRole").unwrap_or_default();
        if ACTIONABLE.contains(&role.as_str()) {
            let label = own_label(el).unwrap_or_else(|| inner_text(el, 0));
            if let Some(c) = usable(role.clone(), label, frame(el)) {
                out.push(c);
            }
        }
        for child in under(el, "AXChildren") {
            walk(child.as_CFTypeRef(), depth + 1, seen, out, began);
        }
    }

    /// Everything the caller has to be able to rely on, in one place.
    pub(super) fn usable(
        role: String,
        label: String,
        frame: Option<((f64, f64), (f64, f64))>,
    ) -> Option<Control> {
        let label = label.split_whitespace().collect::<Vec<_>>().join(" ");
        if label.is_empty() {
            return None;
        }
        let (at, size) = frame?;
        // A closed menu reports (0, 982) and no size at all: its items exist but
        // have nowhere to be until the menu opens. Clicking the centre of a zero
        // sized rectangle is clicking the wrong thing, confidently.
        if size.0 < 2.0 || size.1 < 2.0 {
            return None;
        }
        Some(Control {
            role,
            label: label.chars().take(80).collect(),
            at: (at.0 + size.0 / 2.0, at.1 + size.1 / 2.0),
            size,
        })
    }

    /// The controls the given process is willing to describe.
    pub fn controls(pid: i32) -> Vec<Control> {
        if !unsafe { AXIsProcessTrusted() } {
            return Vec::new();
        }
        let app = unsafe { AXUIElementCreateApplication(pid) };
        if app.is_null() {
            return Vec::new();
        }
        let app = unsafe { CFType::wrap_under_create_rule(app) };
        let app = app.as_CFTypeRef();
        // Otherwise a busy application hangs us for its own idea of forever.
        unsafe { AXUIElementSetMessagingTimeout(app, 1.0) };

        // Chromium and Electron build no tree until an assistive client says it
        // wants one -- and building it is not free for them, which is why they
        // wait to be asked. Without this a browser offers a menu bar and nothing
        // else: no address bar, no tabs, no page.
        let yes = CFBoolean::true_value();
        for attribute in ["AXManualAccessibility", "AXEnhancedUserInterface"] {
            let key = CFString::new(attribute);
            unsafe { AXUIElementSetAttributeValue(app, key.as_concrete_TypeRef(), yes.as_CFTypeRef()) };
        }

        let began = std::time::Instant::now();
        let (mut seen, mut out) = (0usize, Vec::new());
        for root in roots(app) {
            walk(root.as_CFTypeRef(), 1, &mut seen, &mut out, began);
        }
        out
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    #[derive(Debug, Clone, PartialEq)]
    pub struct Control {
        pub role: String,
        pub label: String,
        pub at: (f64, f64),
        pub size: (f64, f64),
    }
    pub fn controls(_pid: i32) -> Vec<Control> {
        Vec::new()
    }
}

pub use imp::*;

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::imp::usable;

    /// Everything downstream assumes a control can be clicked. These are the
    /// ways one cannot.
    #[test]
    fn a_control_without_a_name_or_a_place_is_not_a_control() {
        let frame = Some(((10.0, 20.0), (40.0, 30.0)));

        assert!(usable("AXButton".into(), String::new(), frame).is_none(), "no name");
        assert!(usable("AXButton".into(), "   ".into(), frame).is_none(), "blank name");
        assert!(usable("AXButton".into(), "Send".into(), None).is_none(), "nowhere");

        // A closed menu's items report a zero sized rectangle. Clicking the
        // centre of one is clicking somewhere else entirely, with confidence.
        assert!(
            usable("AXMenuItem".into(), "New Window".into(), Some(((0.0, 982.0), (0.0, 0.0)))).is_none(),
            "a closed menu has nowhere to click"
        );
    }

    #[test]
    fn a_control_is_named_and_clicked_in_the_middle() {
        let c = usable(
            "AXButton".into(),
            "  Send\n  Message ".into(),
            Some(((10.0, 20.0), (40.0, 30.0))),
        )
        .expect("a real button");
        assert_eq!(c.at, (30.0, 35.0), "clicks land in the middle, not the corner");
        assert_eq!(c.label, "Send Message", "labels are one line of ordinary spacing");
    }
}
