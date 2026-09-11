//! Architecture test: `core` must not know that Tauri exists.
//!
//! Lives outside `src/` because a scanner kept inside the directory it scans
//! matches its own needles -- and because this checks a boundary, not a function.
//!
//! The split is only worth having if it holds. A documented rule decays the first
//! time someone reaches for an `AppHandle` to emit a progress event from inside a
//! provider; this fails the build instead.
use std::path::PathBuf;

const FORBIDDEN: [&str; 2] = ["tauri", "crate::app"];

#[test]
fn core_stays_independent_of_the_gui_layer() {
    let mut walk = vec![PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/src/core"))];
    let mut checked = 0;

    while let Some(dir) = walk.pop() {
        for entry in std::fs::read_dir(&dir).expect("read core/") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                walk.push(path);
                continue;
            }
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            checked += 1;
            let src = std::fs::read_to_string(&path).expect("read source");
            for (n, line) in src.lines().enumerate() {
                if line.trim_start().starts_with("//") {
                    continue; // comments may discuss the rule
                }
                for needle in FORBIDDEN {
                    assert!(
                        !line.contains(needle),
                        "{}:{} reaches out of core ({needle}): {}",
                        path.display(),
                        n + 1,
                        line.trim(),
                    );
                }
            }
        }
    }

    // A scanner that silently walks nothing passes forever.
    assert!(checked >= 5, "only scanned {checked} files -- did core/ move?");
}
