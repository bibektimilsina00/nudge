//! Can this machine transcribe without the network, and how fast?
//!
//!     say -o /tmp/utter.aiff "click the view menu"
//!     cargo run --example ear
//!
//! The transcription stage is a round trip to Gemini taking a second or more,
//! and it is the last thing on the critical path that is purely waiting on
//! someone else's server. macOS has had a recogniser for years. The question is
//! whether it is accurate enough and quick enough to replace a hosted model, and
//! that is a measurement, not an opinion.
use objc2::AnyThread;
use objc2_foundation::{NSString, NSURL};
use objc2_speech::{
    SFSpeechRecognitionResult, SFSpeechRecognizer, SFSpeechRecognizerAuthorizationStatus,
    SFSpeechURLRecognitionRequest,
};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// Wait for a callback while keeping the main runloop turning.
///
/// Both of these are asynchronous and both need the runloop: the permission
/// prompt is a window and cannot be drawn without one, and the recogniser
/// delivers its results through it. A command line tool has no runloop unless it
/// makes one, which is why the first attempt at this exited silently having
/// asked for nothing.
fn wait<T>(rx: &mpsc::Receiver<T>, patience: Duration) -> Option<T> {
    let deadline = Instant::now() + patience;
    loop {
        if let Ok(v) = rx.try_recv() {
            return Some(v);
        }
        if Instant::now() > deadline {
            return None;
        }
        unsafe {
            objc2_core_foundation::CFRunLoopRunInMode(
                objc2_core_foundation::kCFRunLoopDefaultMode,
                0.05,
                false,
            )
        };
    }
}

fn main() {
    // A separate grant from the microphone: one is permission to hear you, this
    // is permission to work out what you said.
    let probe = unsafe { SFSpeechRecognizer::new() };
    println!("  locale recogniser exists");
    println!("  available:           {}", unsafe { probe.isAvailable() });
    println!("  supports on-device:  {}", unsafe { probe.supportsOnDeviceRecognition() });

    let status = unsafe { SFSpeechRecognizer::authorizationStatus() };
    println!("  authorisation:       {status:?}");
    if status != SFSpeechRecognizerAuthorizationStatus::Authorized {
        println!("  not authorised yet ({status:?}) -- asking");
        let (tx, rx) = mpsc::channel();
        let handler = block2::RcBlock::new(move |s: SFSpeechRecognizerAuthorizationStatus| {
            let _ = tx.send(s);
        });
        unsafe { SFSpeechRecognizer::requestAuthorization(&handler) };
        match wait(&rx, Duration::from_secs(60)) {
            Some(s) => println!("  answered: {s:?}"),
            None => {
                println!("  no answer -- run again once the prompt is dealt with");
                return;
            }
        }
    }

    let recognizer = unsafe { SFSpeechRecognizer::new() };
    println!("  available: {}", unsafe { recognizer.isAvailable() });
    println!(
        "  supports on-device: {}",
        unsafe { recognizer.supportsOnDeviceRecognition() }
    );

    let path = NSString::from_str("/tmp/utter.aiff");
    let url = unsafe { NSURL::fileURLWithPath(&path) };
    let request = unsafe {
        SFSpeechURLRecognitionRequest::initWithURL(SFSpeechURLRecognitionRequest::alloc(), &url)
    };
    // The whole point. Without this it is a round trip to Apple instead of a
    // round trip to Google, which would be no gain at all.
    unsafe { request.setRequiresOnDeviceRecognition(true) };

    let began = Instant::now();
    let (tx, rx) = mpsc::channel();
    let handler = block2::RcBlock::new(
        move |result: *mut SFSpeechRecognitionResult, err: *mut objc2_foundation::NSError| {
            let said = unsafe { result.as_ref() }.map(|r| {
                let t = unsafe { r.bestTranscription() };
                (unsafe { t.formattedString() }.to_string(), unsafe { r.isFinal() })
            });
            let failed = unsafe { err.as_ref() }.map(|e| e.localizedDescription().to_string());
            let _ = tx.send((said, failed));
        },
    );
    unsafe { recognizer.recognitionTaskWithRequest_resultHandler(&request, &handler) };

    // Partial results arrive as it goes; the one that matters says it is final.
    loop {
        match wait(&rx, Duration::from_secs(20)) {
            Some((Some((text, final_)), _)) => {
                println!(
                    "  {:>7.0}ms  {}{text:?}",
                    began.elapsed().as_secs_f32() * 1000.0,
                    if final_ { "FINAL " } else { "partial " }
                );
                if final_ {
                    return;
                }
            }
            Some((None, Some(e))) => {
                println!("  failed after {:.0}ms: {e}", began.elapsed().as_secs_f32() * 1000.0);
                return;
            }
            _ => {
                println!("  nothing came back");
                return;
            }
        }
    }
}
