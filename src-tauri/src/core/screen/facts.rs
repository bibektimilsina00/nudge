//! What the system knows, so the model does not have to guess it from pixels.
//!
//! A screenshot is a still picture, and some of the most important things about
//! a screen are not in it. "Is the video playing?" is the one that cost a whole
//! test run: the model clicked play, the video played, the next frame looked
//! identical to a paused one, so it clicked play again and stopped it -- thirty
//! times. Motion is not visible in a photograph.
//!
//! Everything here is public API, needs no permission, and names no application.
//! The rule it follows: before teaching a model to recognise a state, check
//! whether macOS will simply tell us.
#[cfg(target_os = "macos")]
use std::ffi::c_void;

/// Facts gathered immediately before the capture, and handed to the model
/// alongside it.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Facts {
    /// The application actually in front. The model reads this off the
    /// screenshot today and gets it wrong -- one run reported VS Code while a
    /// browser was frontmost, and then spent ten turns acting on that belief.
    pub app: Option<String>,
    /// Its window title. Often names the document, the page, or the track.
    pub title: Option<String>,
    /// Is sound coming out of this Mac right now? `None` means we cannot tell.
    ///
    /// Nudge speaks every step aloud through the same output device, so while it
    /// is talking the device is busy by definition and the answer is neither
    /// true nor false. Reporting `false` there would be a lie in the dangerous
    /// direction -- it is what makes an agent click play on something already
    /// playing -- and `true` is how one run heard itself say "let's sail
    /// straight to Queen" and declared the song playing over a blank page.
    pub audio: Option<bool>,
}

pub fn gather() -> Facts {
    let (app, title) = match super::privacy::frontmost() {
        Some((a, t)) => (Some(a), Some(t).filter(|t| !t.is_empty())),
        None => (None, None),
    };
    Facts {
        app,
        title,
        // Our own voice is not evidence of anything. While Nudge is talking the
        // device is busy by definition, so the honest answer is "cannot tell",
        // and the safe rendering of that is silence -- claiming audio would
        // finish tasks that never started.
        audio: (!crate::core::voice::speech::is_playing()).then(audio_playing),
    }
}

impl Facts {
    /// The block that goes into the prompt. Empty when there is nothing to say,
    /// so a machine that reports nothing does not get a heading with no content.
    pub fn brief(&self) -> String {
        let mut lines = Vec::new();
        if let Some(app) = &self.app {
            match &self.title {
                Some(t) => lines.push(format!("- Frontmost application: {app} -- \"{t}\"")),
                None => lines.push(format!("- Frontmost application: {app}")),
            }
        }
        // Unknown is left unsaid. A fact we are not sure of does not belong in a
        // list the model has been told to trust over its own eyes.
        if let Some(audio) = self.audio {
            lines.push(format!(
                "- Audio is {} right now",
                if audio { "PLAYING" } else { "silent" }
            ));
        }
        if lines.is_empty() {
            return String::new();
        }
        format!(
            "What the system reports, which is more reliable than the picture:\n{}\n\n",
            lines.join("\n")
        )
    }
}

// ---------------------------------------------------------------------------

/// Four-character codes, the way CoreAudio spells its constants.
#[cfg(target_os = "macos")]
const fn fourcc(s: &[u8; 4]) -> u32 {
    ((s[0] as u32) << 24) | ((s[1] as u32) << 16) | ((s[2] as u32) << 8) | (s[3] as u32)
}

#[cfg(target_os = "macos")]
#[repr(C)]
struct Address {
    selector: u32,
    scope: u32,
    element: u32,
}

#[cfg(target_os = "macos")]
#[link(name = "CoreAudio", kind = "framework")]
extern "C" {
    fn AudioObjectGetPropertyData(
        object: u32,
        address: *const Address,
        qualifier_size: u32,
        qualifier: *const c_void,
        size: *mut u32,
        out: *mut c_void,
    ) -> i32;
}

/// Is the default output device running?
///
/// Device-level on purpose. Asking "is *this app* playing" means naming apps,
/// and there is no end to that list; asking whether the Mac is making a sound
/// is one question that works for a browser, a music app, a video in Preview or
/// something nobody has written yet.
///
/// ponytail: the whole default output device, so another app's notification
/// chime reads as audio for the moment it lasts. Per-process attribution needs
/// a tap on the audio stream, which is a permission prompt and a background
/// thread -- not worth it until a false positive actually costs something.
#[cfg(target_os = "macos")]
fn audio_playing() -> bool {
    const SYSTEM_OBJECT: u32 = 1;
    let global = fourcc(b"glob");

    let mut device: u32 = 0;
    let mut size = std::mem::size_of::<u32>() as u32;
    let want_device = Address {
        selector: fourcc(b"dOut"),
        scope: global,
        element: 0,
    };
    let ok = unsafe {
        AudioObjectGetPropertyData(
            SYSTEM_OBJECT,
            &want_device,
            0,
            std::ptr::null(),
            &mut size,
            &mut device as *mut _ as *mut c_void,
        )
    };
    if ok != 0 || device == 0 {
        return false;
    }

    let mut running: u32 = 0;
    let mut size = std::mem::size_of::<u32>() as u32;
    let want_running = Address {
        selector: fourcc(b"gone"),
        scope: global,
        element: 0,
    };
    let ok = unsafe {
        AudioObjectGetPropertyData(
            device,
            &want_running,
            0,
            std::ptr::null(),
            &mut size,
            &mut running as *mut _ as *mut c_void,
        )
    };
    ok == 0 && running != 0
}

#[cfg(not(target_os = "macos"))]
fn audio_playing() -> bool {
    false
}

// CoreAudio's constants, and the Objective-C runtime that reads them.
#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    /// Wrong by one byte and CoreAudio silently answers a different question.
    #[test]
    fn the_four_character_codes_are_the_ones_coreaudio_means() {
        assert_eq!(
            fourcc(b"glob"),
            0x676C6F62,
            "kAudioObjectPropertyScopeGlobal"
        );
        assert_eq!(
            fourcc(b"dOut"),
            0x644F7574,
            "kAudioHardwarePropertyDefaultOutputDevice"
        );
        assert_eq!(
            fourcc(b"gone"),
            0x676F6E65,
            "kAudioDevicePropertyDeviceIsRunningSomewhere"
        );
    }

    #[test]
    fn the_brief_always_settles_the_question_a_picture_cannot() {
        let f = Facts {
            app: Some("Arc".into()),
            title: Some("Queen - Bohemian Rhapsody".into()),
            audio: Some(true),
        };
        let b = f.brief();
        assert!(b.contains("Arc -- \"Queen - Bohemian Rhapsody\""));
        assert!(b.contains("Audio is PLAYING"));

        // Silence is stated; unknown is not. "No news" must read as unknown, and
        // saying "silent" while Nudge itself is talking is the lie that makes an
        // agent click play on something already playing.
        let silent = Facts {
            audio: Some(false),
            ..Default::default()
        };
        assert!(silent.brief().contains("Audio is silent"));
        assert!(
            !Facts::default().brief().contains("Audio"),
            "unknown says nothing at all"
        );
    }

    /// Real call, real device. Asserts nothing about the answer -- the machine
    /// may or may not be making a sound -- only that the codes and struct layout
    /// are right enough not to fault. Prints it so
    /// `cargo test audio -- --nocapture` can be checked against a real sound.
    #[test]
    fn asking_the_system_does_not_blow_up() {
        eprintln!("audio_playing() = {}", audio_playing());
    }
}
