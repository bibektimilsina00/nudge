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
    use super::{usable, Control};

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

#[cfg(target_os = "windows")]
mod imp {
    //! The same question, asked of UI Automation.
    //!
    //! Windows' answer to the accessibility tree, and the reason this port is
    //! worth making: the fast path on macOS is not a macOS idea, it is "ask the
    //! system where the button is", and Windows has been able to answer that
    //! since Vista.
    //!
    //! **Written without being run.** It cannot be compiled from the machine it
    //! was written on, let alone tested, so treat every line as a first draft.
    //! The parts that decide what a control *is* -- [`super::usable`], the
    //! matching in [`super::obvious`] -- are shared with macOS and tested there;
    //! what is unproven is only the walk that produces them.
    use super::{usable, Control};
    use uiautomation::controls::ControlType;
    use uiautomation::{UIAutomation, UIElement, UITreeWalker};

    /// The same limits as the macOS walk, for the same reasons: the point is to
    /// be quicker than a screenshot, and a tree that takes a second has already
    /// lost to the thing it was replacing.
    const BUDGET: std::time::Duration = std::time::Duration::from_millis(900);
    const MAX_ELEMENTS: usize = 6_000;
    const MAX_DEPTH: usize = 60;

    /// Control types worth clicking. The rest is layout -- panes, groups,
    /// separators -- and counting those would bury the real ones.
    fn actionable(kind: ControlType) -> Option<&'static str> {
        Some(match kind {
            ControlType::Button => "Button",
            ControlType::SplitButton => "Button",
            ControlType::MenuItem => "MenuItem",
            ControlType::CheckBox => "CheckBox",
            ControlType::RadioButton => "RadioButton",
            ControlType::ComboBox => "PopUpButton",
            ControlType::Edit => "TextField",
            ControlType::Document => "TextArea",
            ControlType::Hyperlink => "Link",
            ControlType::TabItem => "Tab",
            ControlType::ListItem => "Row",
            ControlType::TreeItem => "Row",
            ControlType::DataItem => "Row",
            _ => return None,
        })
    }

    /// What the element calls itself.
    ///
    /// `Name` is the label a screen reader would read, which is the one a person
    /// would say out loud. `HelpText` is the tooltip, and stands in for the many
    /// toolbar buttons that are an icon with no visible text.
    fn own_label(el: &UIElement) -> Option<String> {
        for got in [el.get_name(), el.get_help_text()] {
            if let Ok(s) = got {
                if !s.trim().is_empty() {
                    return Some(s);
                }
            }
        }
        None
    }

    /// The text inside an unnamed control.
    ///
    /// Same problem as macOS, where a Finder row is the click target and the
    /// filename lives in a static text inside it. A Windows list item is
    /// usually named directly, but a custom one often is not.
    fn inner_text(walker: &UITreeWalker, el: &UIElement, depth: usize) -> String {
        if depth > 3 {
            return String::new();
        }
        let mut parts: Vec<String> = Vec::new();
        let mut child = walker.get_first_child(el).ok();
        while let Some(c) = child {
            match own_label(&c) {
                Some(s) => parts.push(s),
                None => {
                    let deeper = inner_text(walker, &c, depth + 1);
                    if !deeper.is_empty() {
                        parts.push(deeper);
                    }
                }
            }
            if parts.len() >= 4 {
                break;
            }
            child = walker.get_next_sibling(&c).ok();
        }
        parts.join(" ")
    }

    fn walk(
        walker: &UITreeWalker,
        el: &UIElement,
        depth: usize,
        seen: &mut usize,
        out: &mut Vec<Control>,
        began: std::time::Instant,
    ) {
        if *seen >= MAX_ELEMENTS || depth > MAX_DEPTH || began.elapsed() > BUDGET {
            return;
        }
        *seen += 1;

        if let Some(role) = el.get_control_type().ok().and_then(actionable) {
            let label = own_label(el).unwrap_or_else(|| inner_text(walker, el, 0));
            // Screen pixels, which is the space clicks are posted in -- the same
            // promise the macOS side makes, so nothing downstream has to know
            // which platform it came from.
            let frame = el.get_bounding_rectangle().ok().map(|r| {
                (
                    (r.get_left() as f64, r.get_top() as f64),
                    (
                        (r.get_right() - r.get_left()) as f64,
                        (r.get_bottom() - r.get_top()) as f64,
                    ),
                )
            });
            if let Some(c) = usable(role.to_string(), label, frame) {
                out.push(c);
            }
        }

        let mut child = walker.get_first_child(el).ok();
        while let Some(c) = child {
            walk(walker, &c, depth + 1, seen, out, began);
            if began.elapsed() > BUDGET {
                return;
            }
            child = walker.get_next_sibling(&c).ok();
        }
    }

    pub fn controls(pid: i32) -> Vec<Control> {
        let Ok(automation) = UIAutomation::new() else {
            return Vec::new();
        };
        let Ok(root) = automation.get_root_element() else {
            return Vec::new();
        };
        // The control view, not the raw one: the raw tree includes every
        // implementation detail the framework happens to have, and is enormous.
        let Ok(walker) = automation.get_control_view_walker() else {
            return Vec::new();
        };

        let began = std::time::Instant::now();
        let (mut seen, mut out) = (0usize, Vec::new());

        // This process's top-level windows, the same starting point as `roots`
        // on macOS. An application with no window exposes nothing, which reads
        // exactly like an application that exposes nothing -- a distinction that
        // cost an afternoon on the other platform.
        let mut window = walker.get_first_child(&root).ok();
        while let Some(w) = window {
            if w.get_process_id().map(|p| p as i32) == Ok(pid) {
                walk(&walker, &w, 1, &mut seen, &mut out, began);
            }
            if began.elapsed() > BUDGET {
                break;
            }
            window = walker.get_next_sibling(&w).ok();
        }
        out
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod imp {
    /// Nothing, honestly. A platform with no tree falls back to the vision
    /// model, which is slower and less accurate and works.
    pub fn controls(_pid: i32) -> Vec<super::Control> {
        Vec::new()
    }
}

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

/// Everything the caller has to be able to rely on, in one place.
pub fn usable(
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


pub use imp::*;

/// The one control a goal obviously means, when there is one.
///
/// The model costs about six seconds of an eight second turn. When someone says
/// "click the Send button" and the system is already telling us there is exactly
/// one thing called Send, asking a model to look at a picture and work that out
/// is six seconds spent confirming something we were told.
///
/// The bar is deliberately high, because there is nothing behind this. A model
/// that grounds badly still described what it saw and can be second-guessed by
/// the next turn; a wrong match here clicks something with no one disagreeing.
/// So all of these have to hold:
///
/// - the goal opens with an explicit instruction to click something, so that
///   "how do I send this" and "is the send button greyed out" are not clicks
/// - a control's whole label appears in the goal, on word boundaries
/// - exactly one control qualifies
///
/// Anything else returns `None` and the model decides, which is the normal path
/// and always available. This only ever removes a wait, never a judgement.
pub fn obvious<'a>(goal: &str, controls: &'a [Control]) -> Option<&'a Control> {
    let goal = normalise(goal);
    // Only an instruction to press something. Everything else -- a question, a
    // description, a multi-step task -- is for the model.
    // Words that can only mean "act on that thing on the screen".
    const PLAIN: [&str; 6] = ["click ", "press ", "tap ", "hit ", "choose ", "select "];
    // And words that usually mean that, but sometimes mean something else
    // entirely. "Open Safari" is a request to launch an application, and if
    // Safari happens to be frontmost there is a menu called Safari to click --
    // so these carry an extra condition below.
    const LOOSE: [&str; 4] = ["open ", "go to ", "switch to ", "show me "];

    let plain = PLAIN.iter().find_map(|v| goal.strip_prefix(v));
    let loose = LOOSE.iter().find_map(|v| goal.strip_prefix(v));
    let rest = plain.or(loose)?;

    // Naming a control is not always asking for it. "The thing next to View"
    // names View in order to point somewhere else, and matching on the name
    // alone clicks View with complete confidence. Found by testing the matcher
    // against phrasings it was never built to handle, which is what that kind of
    // test is for -- it was live, and it was the exact failure this path is
    // dangerous for: wrong, fast, and with nothing watching.
    //
    // Anything that positions the target relative to something else is a
    // description rather than a name, and descriptions are what the model is for.
    const RELATIVE: [&str; 14] = [
        "next to", "beside", "left of", "right of", "above", "below", "under",
        "over", "after", "before", "near", "other", "second", "third",
    ];
    if RELATIVE
        .iter()
        .any(|r| rest.split(' ').any(|w| w == *r) || rest.contains(&format!("{r} ")))
    {
        return None;
    }

    let hits: Vec<(&Control, String)> = controls
        .iter()
        .filter_map(|c| {
            let label = normalise(&c.label);
            // Two, because "OK" and "No" are real buttons and the shortest
            // things anyone says. One character is a letter in a word.
            (label.len() >= 2 && contains_phrase(rest, &label)).then_some((c, label))
        })
        .collect();

    // The most specific wins, but only when it is genuinely more specific.
    //
    // "click send later" matches both "Send" and "Send Later", and treating that
    // as ambiguous would punish someone for being precise -- the second contains
    // the first, so it is the same control named more fully. Whereas "click send
    // or cancel" matches two unrelated labels, and there we really do not know.
    let most = hits.iter().map(|(_, label)| label.len()).max()?;
    let mut best = hits.iter().filter(|(_, label)| label.len() == most);
    let longest = best.next()?;
    // Two controls equally well named -- most often the same label twice, which
    // is what a list of rows looks like. Nothing here can tell them apart.
    if best.next().is_some() {
        return None;
    }
    let all_within = hits
        .iter()
        .all(|(_, label)| contains_phrase(&longest.1, label));
    if !all_within {
        return None;
    }

    // The extra condition for the looser verbs. A menu bar item is named after
    // the application as often as not -- Safari, Finder, Chrome -- so "open
    // Safari" lands on one whether or not a menu was ever wanted. Saying "menu"
    // settles it, and not saying it leaves this for the model, which can see
    // that the request was to launch something.
    let menu_bar = longest.0.role.contains("MenuBar");
    if plain.is_none() && menu_bar && !contains_phrase(rest, "menu") {
        return None;
    }

    Some(longest.0)
}

/// Lowercased, with the decoration people do not say out loud removed.
///
/// Labels carry their keyboard shortcut -- "Search (⇧⌘F)" -- and an ellipsis for
/// anything that opens a dialog. Nobody says either, so neither should have to
/// match.
fn normalise(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut depth = 0usize;
    for ch in s.chars() {
        match ch {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            _ if depth > 0 => {}
            c if c.is_alphanumeric() => out.extend(c.to_lowercase()),
            _ => out.push(' '),
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Does `phrase` appear in `text` as whole words?
///
/// Word boundaries, not a substring: "ok" must not match "bookmark", and "tab"
/// must not match "table".
fn contains_phrase(text: &str, phrase: &str) -> bool {
    if phrase.is_empty() {
        return false;
    }
    let words: Vec<&str> = text.split(' ').collect();
    let target: Vec<&str> = phrase.split(' ').collect();
    words.windows(target.len()).any(|w| w == target.as_slice())
}

#[cfg(test)]
mod matching {
    use super::{obvious, Control};

    fn c(label: &str) -> Control {
        Control {
            role: "AXButton".into(),
            label: label.into(),
            at: (10.0, 10.0),
            size: (20.0, 20.0),
        }
    }

    #[test]
    fn one_obvious_control_skips_the_model_entirely() {
        let controls = [c("Send"), c("Cancel"), c("Attach file")];
        assert_eq!(obvious("click Send", &controls).unwrap().label, "Send");
        assert_eq!(obvious("Click the Send button", &controls).unwrap().label, "Send");
        assert_eq!(obvious("press attach file", &controls).unwrap().label, "Attach file");

        // The decoration nobody says out loud.
        let shortcuts = [c("Search (⇧⌘F)"), c("Explorer (⇧⌘E)")];
        assert_eq!(
            obvious("click the search icon in the sidebar", &shortcuts).unwrap().label,
            "Search (⇧⌘F)"
        );
    }

    /// Every one of these has to fall through to the model. A fast path that is
    /// sometimes wrong is worse than no fast path, because nothing behind it
    /// disagrees.
    /// The same intention said a different way is the same intention. "Open the
    /// View menu" was taking fifteen times as long as "click the View menu" for
    /// no reason anyone could have explained to a user.
    #[test]
    fn the_looser_verbs_mean_the_same_thing() {
        let menus = [
            Control { role: "AXMenuBarItem".into(), label: "View".into(), at: (10.0, 10.0), size: (40.0, 30.0) },
            Control { role: "AXMenuBarItem".into(), label: "Safari".into(), at: (60.0, 10.0), size: (50.0, 30.0) },
        ];
        for said in [
            "open the View menu",
            "go to the View menu",
            "switch to the View menu",
            "show me the View menu",
        ] {
            assert_eq!(obvious(said, &menus).unwrap().label, "View", "{said:?}");
        }

        // And the reason those verbs needed a condition the others do not. A
        // menu bar item is named after its application as often as not, so this
        // reads as a request to launch Safari and lands on a menu instead.
        assert!(
            obvious("open Safari", &menus).is_none(),
            "\"open Safari\" is a request to launch an application"
        );
        // Saying menu settles it.
        assert_eq!(obvious("open the Safari menu", &menus).unwrap().label, "Safari");
        // And the plain verbs were never ambiguous: nobody clicks an application.
        assert_eq!(obvious("click Safari", &menus).unwrap().label, "Safari");

        // The condition is about menu bars, not about everything.
        let button = [Control { role: "AXButton".into(), label: "Safari".into(), at: (10.0, 10.0), size: (40.0, 30.0) }];
        assert_eq!(obvious("open Safari", &button).unwrap().label, "Safari");
    }

    #[test]
    fn anything_less_than_obvious_goes_to_the_model() {
        let controls = [c("Send"), c("Send Later"), c("OK")];

        // "Send Later" does not appear in "click send", so this is not ambiguous
        // -- only one label is actually present in what was said.
        assert_eq!(obvious("click send", &controls).unwrap().label, "Send");
        // Being more precise must not be punished: "Send" is inside "Send Later",
        // so they are the same control named more fully, not two candidates.
        assert_eq!(obvious("click send later", &controls).unwrap().label, "Send Later");
        // Whereas two unrelated labels genuinely leave us not knowing.
        assert!(obvious("click send or cancel", &[c("Send"), c("Cancel")]).is_none());
        // Short labels are real buttons and must work.
        assert_eq!(obvious("click OK", &[c("OK"), c("Cancel")]).unwrap().label, "OK");
        // But two controls with the same name is the case nothing can resolve --
        // which is what a list of rows looks like.
        assert!(obvious("click ok", &[c("OK"), c("OK")]).is_none());
        assert!(obvious("how do I send this", &controls).is_none(), "a question, not an instruction");
        assert!(obvious("the send button is greyed out", &controls).is_none(), "a description");
        assert!(obvious("open the file menu and send", &controls).is_none(), "does not open with a click");
        assert!(obvious("click something else entirely", &controls).is_none(), "nothing matches");

        // Naming a control in order to point somewhere else. Each of these
        // matched a real control and would have clicked it.
        let menus = [c("View"), c("Edit")];
        for said in [
            "click the thing next to View",
            "click the button beside View",
            "click the item below View",
            "click the second View",
            "click the other View",
            "click the one after View",
        ] {
            assert!(obvious(said, &menus).is_none(), "{said:?} should have gone to the model");
        }
        assert!(obvious("click ok", &[]).is_none(), "nothing exposed at all");

        // Word boundaries. "ok" inside "bookmark" is not a button called OK.
        assert!(obvious("click bookmarks", &[c("OK"), c("Bookmarks")]).unwrap().label == "Bookmarks");
        assert!(obvious("click tables", &[c("Tab")]).is_none(), "tab is not inside tables");

        // Too short to mean anything on its own.
        assert!(obvious("click a", &[c("A")]).is_none());
    }
}

// Not gated to a platform: `usable` is the shared rule about what counts as a
// control, and it has to mean the same thing wherever the tree came from.
#[cfg(test)]
mod tests {
    use super::usable;

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
