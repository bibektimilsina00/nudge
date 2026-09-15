//! Transcription without the network.
//!
//! The hosted call is a second or more and it is the last thing on the critical
//! path that is purely waiting on someone else's server. macOS has had a
//! recogniser for years, and on this machine it reports `supportsOnDeviceRecognition`
//! -- so the words can exist without a round trip at all, and with the wifi off.
//!
//! It is a *fast path*, not a replacement. A separate permission has to be
//! granted, the recogniser can report itself unavailable, and a locale may have
//! no on-device model. Every one of those falls back to the hosted call, which
//! is slower and always works.
#[cfg(target_os = "macos")]
mod imp {
    use objc2::AnyThread;
    use objc2_foundation::{NSString, NSURL};
    use objc2_speech::{
        SFSpeechRecognitionResult, SFSpeechRecognizer, SFSpeechRecognizerAuthorizationStatus,
        SFSpeechURLRecognitionRequest,
    };
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    /// How long to give the recogniser before falling back.
    ///
    /// Tight on purpose. The whole reason to be here is that it is quicker than a
    /// round trip; waiting longer than one for it to fail would make this slower
    /// than doing nothing.
    const PATIENCE: Duration = Duration::from_millis(1500);

    /// Stop trying after it lets us down once.
    ///
    /// A fallback that runs after a timeout costs the timeout *and* the fallback,
    /// every time. One failure is enough to conclude this machine is not going to
    /// do it, and the hosted path is right there.
    static GIVEN_UP: AtomicBool = AtomicBool::new(false);

    /// What the grant is, for saying out loud at startup.
    pub fn status() -> &'static str {
        match unsafe { SFSpeechRecognizer::authorizationStatus() } {
            SFSpeechRecognizerAuthorizationStatus::Authorized => "granted",
            SFSpeechRecognizerAuthorizationStatus::Denied => "denied",
            SFSpeechRecognizerAuthorizationStatus::Restricted => "restricted",
            _ => "not asked yet",
        }
    }

    pub fn granted() -> bool {
        let status = unsafe { SFSpeechRecognizer::authorizationStatus() };
        status == SFSpeechRecognizerAuthorizationStatus::Authorized
    }

    /// Ask, once, at startup. The answer arrives whenever the user gets to it,
    /// so nothing waits on this -- `granted` is re-checked each time instead.
    pub fn request_access() {
        let status = unsafe { SFSpeechRecognizer::authorizationStatus() };
        if status != SFSpeechRecognizerAuthorizationStatus::NotDetermined {
            return;
        }
        let handler = block2::RcBlock::new(|s: SFSpeechRecognizerAuthorizationStatus| {
            eprintln!("nudge: speech recognition = {s:?}");
        });
        unsafe { SFSpeechRecognizer::requestAuthorization(&handler) };
    }

    /// Words from a WAV, entirely on this machine, or `None`.
    ///
    /// `None` for every way this declines -- no grant, no on-device model, a
    /// recogniser that is busy, a handler that never fires -- and the caller
    /// falls back. It never returns a guess.
    pub fn transcribe(wav: &[u8]) -> Option<String> {
        if GIVEN_UP.load(Ordering::Relaxed) || !granted() {
            return None;
        }
        let recognizer = unsafe { SFSpeechRecognizer::new() };
        if !unsafe { recognizer.isAvailable() }
            || !unsafe { recognizer.supportsOnDeviceRecognition() }
        {
            return None;
        }

        // The URL request wants a file. Writing a couple of hundred kilobytes to
        // the temp directory costs about a millisecond, against the buffer-based
        // API which wants the audio converted into AVAudioPCMBuffers first.
        let path = std::env::temp_dir().join("nudge-utterance.wav");
        std::fs::write(&path, wav).ok()?;
        let url = NSURL::fileURLWithPath(&NSString::from_str(path.to_str()?));
        let request = unsafe {
            SFSpeechURLRecognitionRequest::initWithURL(SFSpeechURLRecognitionRequest::alloc(), &url)
        };
        // Without this it is a round trip to Apple instead of one to Google,
        // which would be no gain at all.
        unsafe { request.setRequiresOnDeviceRecognition(true) };

        let (tx, rx) = mpsc::channel();
        let handler = block2::RcBlock::new(
            move |result: *mut SFSpeechRecognitionResult, _err: *mut objc2_foundation::NSError| {
                // Partial results arrive as it works. Only the final one is the
                // sentence; the rest are it changing its mind mid-word.
                let done = unsafe { result.as_ref() }
                    .filter(|r| unsafe { r.isFinal() })
                    .map(|r| unsafe { r.bestTranscription().formattedString() }.to_string());
                if let Some(text) = done {
                    let _ = tx.send(text);
                }
            },
        );
        unsafe { recognizer.recognitionTaskWithRequest_resultHandler(&request, &handler) };

        // Turning the runloop while we wait. The results are delivered through
        // it, and a worker thread does not have one running unless it asks --
        // which is why the first attempt at this returned nothing, silently.
        let deadline = Instant::now() + PATIENCE;
        let heard = loop {
            if let Ok(text) = rx.try_recv() {
                break Some(text);
            }
            if Instant::now() > deadline {
                break None;
            }
            objc2_core_foundation::CFRunLoop::run_in_mode(
                unsafe { objc2_core_foundation::kCFRunLoopDefaultMode },
                0.02,
                false,
            );
        };

        if let Some(text) = &heard {
            eprintln!("nudge: heard on this machine -- {text:?}");
        }
        if heard.is_none() {
            eprintln!(
                "nudge: on-device transcription timed out -- using the hosted one from now on"
            );
            GIVEN_UP.store(true, Ordering::Relaxed);
        }
        heard.filter(|t| !t.trim().is_empty())
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    pub fn granted() -> bool {
        false
    }
    pub fn status() -> &'static str {
        "unsupported"
    }
    pub fn request_access() {}
    pub fn transcribe(_wav: &[u8]) -> Option<String> {
        None
    }
}

pub use imp::*;
