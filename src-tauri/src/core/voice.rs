//! Microphone capture. Speaking is the primary way to ask, so this runs on the
//! push-to-talk path: hold the hotkey, talk, release.
use crate::error::{Error, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc;
use std::sync::Arc;

/// What macOS thinks about us and the microphone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    /// Never asked. The next recording triggers the system prompt -- and returns
    /// silence while it is on screen, because the prompt does not block.
    Unasked,
    Granted,
    Denied,
}

/// Ask the OS, rather than inferring from the audio.
///
/// Inference cannot tell these apart: a denied microphone is fed to us as a
/// stream of zeros, not as an error, so "permission refused" and "you said
/// nothing in a very quiet room" look identical from the samples alone. Guessing
/// produced exactly the wrong advice -- telling someone to check their input
/// device when the mic was working and the grant was missing, or vice versa.
#[cfg(target_os = "macos")]
pub fn access() -> Access {
    use objc2_av_foundation::{AVAuthorizationStatus, AVCaptureDevice, AVMediaTypeAudio};
    let media = unsafe { AVMediaTypeAudio }.expect("AVMediaTypeAudio");
    match unsafe { AVCaptureDevice::authorizationStatusForMediaType(media) } {
        AVAuthorizationStatus::Authorized => Access::Granted,
        AVAuthorizationStatus::NotDetermined => Access::Unasked,
        _ => Access::Denied,
    }
}

#[cfg(not(target_os = "macos"))]
pub fn access() -> Access {
    Access::Granted
}

/// Once denied, macOS will never show the prompt again -- only this pane will do.
/// Opening it directly is the difference between a dead end and a fix.
pub fn open_privacy_settings() {
    let _ = std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone")
        .spawn();
}

/// Trigger the system prompt. Returns immediately -- the answer arrives whenever
/// the user gets round to it, so callers re-check `access()` rather than wait.
#[cfg(target_os = "macos")]
pub fn request_access() {
    use objc2_av_foundation::{AVCaptureDevice, AVMediaTypeAudio};
    let media = unsafe { AVMediaTypeAudio }.expect("AVMediaTypeAudio");
    let done = block2::StackBlock::new(|_granted: objc2::runtime::Bool| {});
    unsafe { AVCaptureDevice::requestAccessForMediaType_completionHandler(media, &done) };
}

#[cfg(not(target_os = "macos"))]
pub fn request_access() {}

/// A recording in progress. `cpal::Stream` is `!Send`, so it never leaves the
/// thread that built it -- the handle talks to that thread instead of owning it.
pub struct Recording {
    stop: Arc<AtomicBool>,
    done: mpsc::Receiver<Result<Vec<u8>>>,
    level: Arc<AtomicU32>,
}

pub fn start() -> Recording {
    let stop = Arc::new(AtomicBool::new(false));
    let (tx, done) = mpsc::channel();
    let (flag, level) = (stop.clone(), Arc::new(AtomicU32::new(0)));
    let meter = level.clone();

    std::thread::spawn(move || {
        let _ = tx.send(record(&flag, &meter));
    });

    Recording { stop, done, level }
}

impl Recording {
    /// Current loudness, 0.0 to 1.0, for the waveform on screen. Read from the
    /// audio callback itself -- a meter derived from the finished recording would
    /// only be able to animate after the fact.
    pub fn level(&self) -> f32 {
        f32::from_bits(self.level.load(Ordering::Relaxed))
    }

    /// Stops capture and returns a WAV. Blocks briefly while the stream drains.
    pub fn finish(self) -> Result<Vec<u8>> {
        self.stop.store(true, Ordering::Relaxed);
        self.done
            .recv()
            .map_err(|_| Error::Voice("recording thread died".into()))?
    }
}

/// Speech sits far below full scale, so raw RMS would leave the waveform almost
/// flat. Tuned by ear against the built-in microphone.
const METER_GAIN: f32 = 9.0;

fn record(stop: &AtomicBool, meter: &Arc<AtomicU32>) -> Result<Vec<u8>> {
    let device = cpal::default_host()
        .default_input_device()
        .ok_or_else(|| Error::Voice("no microphone".into()))?;
    let cfg = device
        .default_input_config()
        .map_err(|e| Error::Voice(e.to_string()))?;
    let rate = cfg.sample_rate();
    let channels = cfg.channels() as usize;

    let samples = Arc::new(std::sync::Mutex::new(Vec::<f32>::new()));
    let sink = samples.clone();
    let meter = meter.clone();

    // ponytail: takes channel 0 and keeps the device's own sample rate -- no mixdown,
    // no resampler. Speech recognisers accept whatever the mic gives; adding a
    // resampling crate to reach a "nicer" 16kHz would be pure ceremony.
    let stream = device
        .build_input_stream(
            &cfg.config(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                // RMS, not peak: it tracks perceived loudness, so the waveform
                // follows the voice instead of twitching on every consonant.
                let sum: f32 = data.iter().map(|s| s * s).sum();
                let rms = (sum / data.len().max(1) as f32).sqrt();
                meter.store((rms * METER_GAIN).min(1.0).to_bits(), Ordering::Relaxed);

                if let Ok(mut buf) = sink.lock() {
                    buf.extend(data.iter().step_by(channels));
                }
            },
            |e| eprintln!("nudge: audio stream error: {e}"),
            None,
        )
        .map_err(|e| Error::Voice(e.to_string()))?;

    stream.play().map_err(|e| Error::Voice(e.to_string()))?;
    while !stop.load(Ordering::Relaxed) {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    drop(stream); // flushes the last callbacks before we read the buffer

    let mut buf = samples.lock().unwrap().clone();
    check_and_normalise(&mut buf, rate)?;
    to_wav(&buf, rate)
}

/// Below this there is no signal at all -- not "quiet", *nothing*.
///
/// Measured rather than guessed: an untouched MacBook microphone in a silent room
/// reads a peak around 0.05, so an earlier threshold of 0.005 was rejecting real
/// audio as silence. A denied microphone is fed to us as exact zeros, so the only
/// job left for this number is to separate "no signal" from "any signal", and
/// quietness is the recogniser's problem, not ours -- `examples/mic.rs` prints the
/// levels if this ever needs re-checking on other hardware.
const SILENCE: f32 = 0.0008;
/// Shorter than this is a slipped key, not a sentence.
const MIN_SECONDS: f32 = 0.35;
/// Normalise to just under full scale rather than exactly 1.0, to leave headroom.
const TARGET_PEAK: f32 = 0.95;

/// The built-in microphone records well below full scale, and transcription quality
/// falls off a cliff on a quiet signal -- so level it before sending.
///
/// The silence check earns its place separately: when microphone permission is
/// denied, macOS does not fail the stream, it feeds it **zeros**. Without this,
/// that is indistinguishable from a working mic in a quiet room, and the only
/// symptom is a transcript that is confidently wrong.
fn check_and_normalise(samples: &mut [f32], rate: u32) -> Result<()> {
    if (samples.len() as f32) < MIN_SECONDS * rate as f32 {
        return Err(Error::Voice("hold the key a moment longer".into()));
    }
    let peak = samples.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    if peak < SILENCE {
        // A working microphone always has a noise floor, so this is not a quiet
        // room -- it is no microphone. Deciding *why* is the OS's job, not ours.
        return Err(Error::Voice(match access() {
            Access::Denied => {
                open_privacy_settings();
                "Microphone access is off for Nudge -- I have opened the settings".into()
            }
            Access::Unasked => {
                request_access();
                "Allow microphone access, then hold the key again".into()
            }
            // Permission is fine, so the signal is genuinely absent: usually the
            // wrong input selected, or a headset that is connected but muted.
            Access::Granted => format!(
                "No sound reached the microphone (peak {peak:.4}) -- check the input \
                 device in System Settings > Sound"
            ),
        }));
    }
    let gain = TARGET_PEAK / peak;
    for s in samples.iter_mut() {
        *s *= gain;
    }
    Ok(())
}

fn to_wav(samples: &[f32], rate: u32) -> Result<Vec<u8>> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut out = std::io::Cursor::new(Vec::new());
    {
        let mut w = hound::WavWriter::new(&mut out, spec)
            .map_err(|e| Error::Voice(e.to_string()))?;
        for &s in samples {
            w.write_sample((s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
                .map_err(|e| Error::Voice(e.to_string()))?;
        }
        w.finalize().map_err(|e| Error::Voice(e.to_string()))?;
    }
    Ok(out.into_inner())
}

#[cfg(test)]
mod tests {
    use super::{check_and_normalise, to_wav, MIN_SECONDS};

    #[test]
    fn writes_a_readable_wav_with_a_real_header() {
        let wav = to_wav(&[0.0, 0.5, -0.5, 1.0], 44_100).unwrap();
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        let r = hound::WavReader::new(std::io::Cursor::new(&wav)).unwrap();
        assert_eq!(r.spec().sample_rate, 44_100);
        assert_eq!(r.spec().channels, 1);
        assert_eq!(r.len(), 4);
    }

    fn seconds(n: f32, rate: u32) -> usize {
        (n * rate as f32) as usize + 1
    }

    #[test]
    fn quiet_speech_is_levelled_up() {
        let rate = 16_000;
        let mut buf = vec![0.02f32; seconds(1.0, rate)];
        buf[0] = 0.05; // peak
        check_and_normalise(&mut buf, rate).unwrap();
        assert!((buf[0] - 0.95).abs() < 1e-5, "peak should hit target, got {}", buf[0]);
    }

    #[test]
    fn a_denied_microphone_is_reported_not_transcribed() {
        // macOS feeds zeros rather than failing when permission is denied, so
        // silence has to be caught here or it reaches the recogniser as "audio".
        let rate = 16_000;
        let mut buf = vec![0.0f32; seconds(1.0, rate)];
        assert!(check_and_normalise(&mut buf, rate).is_err());
    }

    #[test]
    fn a_slipped_key_is_not_a_sentence() {
        let rate = 16_000;
        let mut buf = vec![0.5f32; seconds(MIN_SECONDS / 2.0, rate)];
        assert!(check_and_normalise(&mut buf, rate).is_err());
    }

    #[test]
    fn clamps_instead_of_wrapping_to_the_opposite_sign() {
        // Without the clamp, 2.0 overflows i16 and a loud sample becomes a loud
        // click of the wrong polarity.
        let wav = to_wav(&[2.0, -2.0], 16_000).unwrap();
        let s: Vec<i16> = hound::WavReader::new(std::io::Cursor::new(&wav))
            .unwrap()
            .into_samples::<i16>()
            .map(|s| s.unwrap())
            .collect();
        assert_eq!(s, vec![i16::MAX, -i16::MAX]);
    }
}
