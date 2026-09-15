//! Telling the people who made this that something is wrong, or missing.
//!
//! **The whole design is that it takes one sentence and no account.** Every step
//! between noticing a bug and the bug being written down loses a share of the
//! people who noticed it: opening a browser loses some, signing in loses more,
//! finding the right repository and pressing New Issue loses most of the rest.
//! What survives that gauntlet is the reports of people already invested enough to
//! run it, which is the opposite of who you most want to hear from.
//!
//! So there is no browser, no account and no template. A box, a Send, and -- the
//! part people cannot do themselves -- the version and the OS filled in for them,
//! because "it broke" from an unknown build is a report nobody can act on and
//! nobody thinks to include.
//!
//! What is deliberately *not* collected: the screen, unless they attach it on
//! purpose; anything about what they were doing; any identifier. A report says
//! what somebody chose to type and nothing they did not choose.
use crate::error::Result;
use serde::{Deserialize, Serialize};

/// Which of the two buttons they pressed.
///
/// One endpoint rather than two, because they differ by a word. Whoever reads
/// them wants them in one place anyway, and a second URL to configure is a second
/// thing to get wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Bug,
    Idea,
}

impl Kind {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "bug" => Some(Self::Bug),
            "idea" => Some(Self::Idea),
            _ => None,
        }
    }
}

/// What gets posted.
#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub kind: Kind,
    pub text: String,
    /// A data URL, if they attached a picture. Absent otherwise rather than null,
    /// so the common report stays small on the wire.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    /// Filled in for them. Nobody remembers their build number and every report
    /// is worse without it.
    pub version: String,
    pub os: String,
}

/// Is this worth sending?
///
/// Only that there is something in it. The temptation is a minimum length, and it
/// is worth resisting: "crashes on launch" is five words and the most useful
/// report you will get all week, while a counter that rejects it teaches people
/// that the form is a chore. Whitespace alone is the one thing that is certainly
/// an accident.
pub fn worth_sending(text: &str) -> bool {
    !text.trim().is_empty()
}

/// Keep an attachment small enough to post.
///
/// A full-screen grab as a data URL runs to megabytes, and the person who
/// attached it is watching a spinner while it uploads. This is the ceiling, not a
/// target -- the screenshot path already asks for a smaller capture.
pub const MOST_IMAGE: usize = 6 * 1024 * 1024;

impl Report {
    /// Post it.
    ///
    /// The endpoint is whatever the config says. There is no fallback to some
    /// service of ours, because there is no service of ours -- and a Send that
    /// quietly went nowhere would be worse than a Send that says it is not set up.
    pub async fn post(&self, url: &str) -> Result<()> {
        let res = reqwest::Client::new()
            .post(url)
            .json(self)
            .timeout(std::time::Duration::from_secs(20))
            .send()
            .await?
            .error_for_status()?;
        drop(res);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whitespace_is_not_a_report() {
        assert!(!worth_sending(""));
        assert!(!worth_sending("   \n\t "));
    }

    #[test]
    fn a_short_report_is_still_a_report() {
        // The rule this is guarding: no minimum length. A five-word bug report is
        // often the best one, and rejecting it trains people not to bother.
        assert!(worth_sending("crashes on launch"));
        assert!(worth_sending("x"));
    }

    #[test]
    fn kinds_round_trip_the_names_the_interface_sends() {
        assert_eq!(Kind::parse("bug"), Some(Kind::Bug));
        assert_eq!(Kind::parse("idea"), Some(Kind::Idea));
        assert_eq!(Kind::parse("Bug"), None);
        assert_eq!(Kind::parse(""), None);
    }

    /// The whole path, against something that really answers.
    ///
    /// Ignored because it needs a listener. Run one and then the test:
    ///
    /// ```text
    /// python3 -c "import http.server as h;
    /// exec('class H(h.BaseHTTPRequestHandler):\n def do_POST(s):
    ///  print(s.rfile.read(int(s.headers[chr(67)+\'ontent-Length\'])))
    ///  s.send_response(200); s.end_headers()')
    /// h.HTTPServer(('127.0.0.1',8787),H).serve_forever()"
    /// cargo test --manifest-path src-tauri/Cargo.toml -- --ignored posts_for_real
    /// ```
    #[tokio::test]
    #[ignore = "needs a listener on 127.0.0.1:8787"]
    async fn posts_for_real() {
        let r = Report {
            kind: Kind::Bug,
            text: "the notch pill flickers in Mission Control".into(),
            image: Some("data:image/jpeg;base64,/9j/4AAQSkZJRg==".into()),
            version: "0.1.0".into(),
            os: "macOS 27.0".into(),
        };
        r.post("http://127.0.0.1:8787/report").await.unwrap();
    }

    /// A destination that answers with an error is a failure, not a success.
    ///
    /// The one outcome worth engineering away is a Send that looks like it worked
    /// and went nowhere, and `error_for_status` is the line that prevents it.
    #[tokio::test]
    #[ignore = "needs a listener on 127.0.0.1:8787"]
    async fn a_refusal_is_not_a_send() {
        let r = Report {
            kind: Kind::Idea,
            text: "x".into(),
            image: None,
            version: "0.1.0".into(),
            os: "macOS".into(),
        };
        assert!(r.post("http://127.0.0.1:8787/nope").await.is_err());
    }

    #[test]
    fn an_attachment_is_left_out_rather_than_sent_as_null() {
        let r = Report {
            kind: Kind::Bug,
            text: "it broke".into(),
            image: None,
            version: "0.1.0".into(),
            os: "macOS".into(),
        };
        let json = serde_json::to_string(&r).unwrap();
        assert!(!json.contains("image"), "{json}");
        assert!(json.contains(r#""kind":"bug""#), "{json}");
    }
}
