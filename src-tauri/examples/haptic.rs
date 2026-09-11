//! Does the trackpad actuator respond to this process at all?
fn main() {
    use objc2_app_kit::{
        NSHapticFeedbackManager, NSHapticFeedbackPattern, NSHapticFeedbackPerformanceTime,
        NSHapticFeedbackPerformer,
    };
    let patterns = [
        ("Alignment", NSHapticFeedbackPattern::Alignment),
        ("LevelChange", NSHapticFeedbackPattern::LevelChange),
        ("Generic", NSHapticFeedbackPattern::Generic),
    ];
    for (name, pattern) in patterns {
        println!("{name} in 1s -- rest a finger on the trackpad");
        std::thread::sleep(std::time::Duration::from_secs(1));
        let performer = unsafe { NSHapticFeedbackManager::defaultPerformer() };
        // Three in quick succession: one tick is easy to miss.
        for _ in 0..3 {
            unsafe {
                performer.performFeedbackPattern_performanceTime(
                    pattern,
                    NSHapticFeedbackPerformanceTime::Now,
                )
            };
            std::thread::sleep(std::time::Duration::from_millis(180));
        }
    }
    println!("done");
}
