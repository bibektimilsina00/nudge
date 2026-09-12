//! Diagnostic: record briefly and report levels. Prints statistics only.
fn main() {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use std::sync::{Arc, Mutex};

    let device = cpal::default_host()
        .default_input_device()
        .expect("no input device");
    let cfg = device.default_input_config().unwrap();
    println!("device: {:?}", device.name());
    println!(
        "config: {:?} {}ch {:?}",
        cfg.sample_format(),
        cfg.channels(),
        cfg.sample_rate()
    );

    let buf = Arc::new(Mutex::new(Vec::<f32>::new()));
    let sink = buf.clone();
    let ch = cfg.channels() as usize;
    let stream = device
        .build_input_stream(
            &cfg.config(),
            move |d: &[f32], _: &cpal::InputCallbackInfo| {
                sink.lock().unwrap().extend(d.iter().step_by(ch));
            },
            |e| eprintln!("stream error: {e}"),
            None,
        )
        .expect("build stream");
    stream.play().unwrap();
    std::thread::sleep(std::time::Duration::from_secs(2));
    drop(stream);

    let s = buf.lock().unwrap();
    let peak = s.iter().fold(0f32, |m, v| m.max(v.abs()));
    let rms = (s.iter().map(|v| v * v).sum::<f32>() / s.len().max(1) as f32).sqrt();
    let zeros = s.iter().filter(|v| **v == 0.0).count();
    println!(
        "samples={} peak={peak:.5} rms={rms:.5} exact_zeros={zeros}",
        s.len()
    );
}
