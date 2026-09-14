//! Does the pointer come home after a click?
//!
//!     cargo run --example handback
//!
//! Clicks an empty patch of desktop and checks the cursor is where it started.
use nudge_lib::core::screen::{capture::Point, click};

fn main() {
    let before = click::cursor().expect("a cursor");
    println!("  before  ({:.0}, {:.0})", before.x, before.y);

    // Somewhere harmless and far away, so a failure to return is obvious.
    let target = Point { x: 60.0, y: 400.0 };
    let began = std::time::Instant::now();
    click::click(target, 1).expect("click");
    let took = began.elapsed();

    let after = click::cursor().expect("a cursor");
    let drift = ((after.x - before.x).powi(2) + (after.y - before.y).powi(2)).sqrt();
    println!("  clicked ({:.0}, {:.0}) in {:.0}ms", target.x, target.y, took.as_secs_f32() * 1000.0);
    println!("  after   ({:.0}, {:.0}) -- {drift:.1}px from where it started", after.x, after.y);
    println!("  {}", if drift < 2.0 { "came home" } else { "LEFT STRANDED" });
}
