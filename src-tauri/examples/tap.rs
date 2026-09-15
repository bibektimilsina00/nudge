//! Can a click land without taking the pointer?
//!
//!     cargo run --example tap -- <pid> "Label"
//!
//! `CGEventPost` hands an event to the system, which is why the cursor goes with
//! it. `CGEventPostToPid` hands it to one process instead. If an application
//! accepts that, the click happens where we said and the pointer stays where the
//! user left it -- which is the whole question, and it is a measurement rather
//! than an opinion.
use nudge_lib::core::screen::{ax, click};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (pid, label) = match args.as_slice() {
        [p, l, ..] => (p.parse::<i32>().expect("pid"), l.clone()),
        _ => {
            eprintln!("usage: tap <pid> \"Label\"");
            std::process::exit(2);
        }
    };

    let Some(c) = ax::controls(pid)
        .into_iter()
        .find(|c| c.label.eq_ignore_ascii_case(&label))
    else {
        eprintln!("no control called {label:?} in pid {pid}");
        std::process::exit(1);
    };
    println!(
        "  target  {} {:?} at ({:.0},{:.0})",
        c.role, c.label, c.at.0, c.at.1
    );

    let before = click::cursor();
    let began = std::time::Instant::now();
    tap(pid, c.at.0, c.at.1);
    let took = began.elapsed();
    std::thread::sleep(std::time::Duration::from_millis(400));
    let after = click::cursor();

    println!("  posted  in {:.0}ms", took.as_secs_f32() * 1000.0);
    println!("  pointer {before:?}");
    println!("       -> {after:?}");
    match (before, after) {
        (Some(a), Some(b)) if (a.x - b.x).abs() < 1.0 && (a.y - b.y).abs() < 1.0 => {
            println!("  the pointer did not move")
        }
        _ => println!("  THE POINTER MOVED"),
    }
}

#[cfg(target_os = "macos")]
fn tap(pid: i32, x: f64, y: f64) {
    use core_graphics::event::{CGEvent, CGEventType, CGMouseButton};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
    use core_graphics::geometry::CGPoint;

    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState).expect("source");
    let at = CGPoint::new(x, y);
    for kind in [CGEventType::LeftMouseDown, CGEventType::LeftMouseUp] {
        let e = CGEvent::new_mouse_event(source.clone(), kind, at, CGMouseButton::Left)
            .expect("mouse event");
        // To the process, not to the system. The system is what moves the cursor.
        e.post_to_pid(pid);
    }
}
