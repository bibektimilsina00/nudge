//! One error type. Add a variant when a caller needs to *branch* on it, not before.
use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("screenshot failed: {0}")]
    Capture(String),
    #[error("{provider} gave no usable point: {detail}")]
    NoPoint {
        provider: &'static str,
        detail: String,
    },
    #[error("config: {0}")]
    Config(String),
    #[error("voice: {0}")]
    Voice(String),
    #[error("{0}")]
    Launch(String),
    #[error("{0}")]
    Click(String),
    #[error("{0}")]
    Blocked(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Redacted where it is turned into text, rather than at each of the dozen
    /// places that do so. `reqwest` puts the whole URL in its message and the
    /// provider's URL carries the API key -- so one rate limit used to put a
    /// credential in the log, on the screen, and in whatever got pasted into a
    /// bug report. Fixing it here covers every path at once, including the ones
    /// written next year.
    #[error("{}", crate::core::tools::secret::redact(&.0.to_string()))]
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

/// What to tell a person when something went wrong doing what they asked.
///
/// Nobody using Nudge knows that `reqwest` exists, or `osascript`, or which of
/// them just failed. A message passed up unedited is a bug report about a program
/// they did not know was running, and it tells them nothing they can act on.
///
/// So the sentence is about **their** task. The original is not thrown away -- it
/// goes to the log and to the model, which is the audience it was written for.
///
/// Most of Nudge's own errors are already sentences a person can read; those come
/// through unchanged. What gets rewritten is the machinery underneath.
pub fn plainly(goal: &str, e: &Error) -> String {
    let about = match goal.trim().is_empty() {
        true => String::new(),
        // Their words, not ours -- it is how they know which request failed when
        // two are in flight.
        false => format!(" while trying to {}", goal.trim().trim_end_matches('.')),
    };
    match e {
        // Ours, and already written for a person.
        Error::Click(said) | Error::Blocked(said) => said.clone(),
        Error::Config(said) => said.clone(),

        Error::Http(_) => {
            let said = e.to_string();
            match said.contains("Too Many Requests") || said.contains("429") {
                true => format!("The model provider is rate limiting me{about}."),
                false => format!("I could not reach the network{about}."),
            }
        }
        Error::Io(io) => match io.kind() {
            std::io::ErrorKind::NotFound => format!("That file is not there{about}."),
            std::io::ErrorKind::PermissionDenied => {
                format!("I am not allowed to read that{about}.")
            }
            _ => format!("Something on this Mac would not cooperate{about}."),
        },
        Error::Capture(_) => format!("I could not see the screen{about}."),
        Error::Voice(said) => said.clone(),
        Error::Launch(said) => said.clone(),
        Error::NoPoint { .. } => format!("I could not find that on the screen{about}."),
        Error::Image(_) => format!("I could not read that picture{about}."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The point of the module: a person hears about their task, not about a
    /// crate they have never heard of.
    #[test]
    fn a_machine_failure_is_described_as_a_task_failure() {
        let io = Error::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "no such file",
        ));
        let said = plainly("open my shopping list", &io);
        assert_eq!(
            said,
            "That file is not there while trying to open my shopping list."
        );
        assert!(!said.contains("os error"));
    }

    /// Ours are already sentences. Rewriting them would lose the part that says
    /// what to do about it.
    #[test]
    fn our_own_refusals_come_through_untouched() {
        let refusal = Error::Click("ffmpeg is not on this Mac".into());
        assert_eq!(
            plainly("convert my video", &refusal),
            "ffmpeg is not on this Mac"
        );
    }

    #[test]
    fn with_no_goal_it_does_not_invent_one() {
        let e = Error::Capture("no display".into());
        assert_eq!(plainly("", &e), "I could not see the screen.");
    }
}
