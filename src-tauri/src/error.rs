//! One error type. Add a variant when a caller needs to *branch* on it, not before.
use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("screenshot failed: {0}")]
    Capture(String),
    #[error("{provider} gave no usable point: {detail}")]
    NoPoint { provider: &'static str, detail: String },
    #[error("config: {0}")]
    Config(String),
    #[error("voice: {0}")]
    Voice(String),
    #[error("{0}")]
    Launch(String),
    #[error("{0}")]
    Click(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    #[error(transparent)]
    Image(#[from] image::ImageError),
}

pub type Result<T> = std::result::Result<T, Error>;

// ponytail: Tauri commands need Serialize; the UI only ever shows the message.
impl Serialize for Error {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}
