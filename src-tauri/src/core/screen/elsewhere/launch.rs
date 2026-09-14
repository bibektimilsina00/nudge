//! Starting applications, on a platform that has not been taught how yet.
//!
//! `installed_apps` is the interesting one. The model is told what this machine
//! can launch and nothing more, so it offers what exists instead of guessing at
//! a name. macOS reads /Applications; Windows would read the Start menu and the
//! registry's App Paths; Linux would read the XDG .desktop files.
use crate::error::{Error, Result};

pub fn open_url(_url: &str) -> Result<()> {
    Err(Error::Launch("this platform cannot open a browser yet".into()))
}

/// Empty rather than wrong. An empty list tells the model there is nothing to
/// launch, which is true here; a made-up list would have it confidently opening
/// things that do not exist.
pub fn installed_apps() -> &'static [String] {
    &[]
}

pub fn open_app(_name: &str) -> Result<()> {
    Err(Error::Launch("this platform cannot launch applications yet".into()))
}
