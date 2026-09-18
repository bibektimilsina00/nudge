//! Where a model call goes, and what pays for it.
//!
//! Two answers, and the app has to work under both. Somebody who has set their
//! own key talks to Google directly: their quota, their bill, nothing of ours in
//! the path. Everybody else talks to our server with the session they already
//! have from signing in, and the server spends its own key on their behalf --
//! see `server/app/routes/think.py`.
//!
//! This exists because the difference between those two is a URL and a header,
//! and every caller that reaches Gemini would otherwise have to know about both.
//! Four of them do -- the provider, the transcriber, the voice and the search --
//! and the third one to grow its own copy of this logic is the one that gets it
//! subtly wrong.
//!
//! The choice is made per call rather than at startup. Signing out, signing in,
//! and pasting a key into Settings all change the answer, and none of them
//! should need a restart.
use crate::config::Config;

pub enum Relay {
    /// Their key, straight to Google.
    Direct(String),
    /// Our key, their account, through our server.
    Borrowed(String),
}

/// What Google calls its endpoint. Ours mirrors the model name in the path so
/// the two are one substitution apart.
const GOOGLE: &str = "https://generativelanguage.googleapis.com/v1beta/models";

impl Relay {
    /// A key wins over an account.
    ///
    /// Anybody who went to the trouble of pasting one in meant to use it, and
    /// silently routing them through our server instead would be spending our
    /// quota to ignore their instruction.
    pub fn choose(cfg: &Config) -> Option<Self> {
        match cfg.key("GEMINI_API_KEY") {
            Some(key) => Some(Relay::Direct(key)),
            None => crate::core::account::token().map(Relay::Borrowed),
        }
    }

    /// Why there is nothing to call, in words for a person.
    pub fn missing() -> String {
        "Sign in to use Nudge's model, or set your own Gemini key in Settings.".into()
    }

    pub fn url(&self, model: &str) -> String {
        match self {
            Relay::Direct(_) => format!("{GOOGLE}/{model}:generateContent"),
            Relay::Borrowed(_) => format!("{}/api/think/{model}", crate::core::account::api()),
        }
    }

    /// The one header that differs.
    pub fn authorise(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match self {
            Relay::Direct(key) => req.header("x-goog-api-key", key),
            Relay::Borrowed(token) => req.bearer_auth(token),
        }
    }

    /// For the startup line, and for Settings to say which one is in use.
    pub fn describe(&self) -> &'static str {
        match self {
            Relay::Direct(_) => "your own key",
            Relay::Borrowed(_) => "your Nudge account",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_key(key: &str) -> Config {
        Config {
            api_key: Some(key.into()),
            ..Default::default()
        }
    }

    #[test]
    fn a_key_goes_straight_to_google_and_an_account_goes_through_us() {
        let mine = Relay::Direct("k".into());
        assert!(mine.url("gemini-3.5-flash").starts_with(GOOGLE));
        let ours = Relay::Borrowed("t".into());
        assert!(ours
            .url("gemini-3.5-flash")
            .ends_with("/api/think/gemini-3.5-flash"));
    }

    /// The instruction, not our preference: somebody who pasted a key meant it.
    #[test]
    fn a_key_wins_over_an_account() {
        assert!(matches!(
            Relay::choose(&with_key("k")),
            Some(Relay::Direct(_))
        ));
    }
}
