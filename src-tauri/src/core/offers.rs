//! Whether to ask about connecting something, and what they said last time.
//!
//! The offer bar itself is in `app::ui::connect`. This is the part that decides
//! it is a reasonable moment, which is almost all of the design: a prompt shown
//! at the wrong time is not a prompt with bad timing, it is an advert.
//!
//! ## The trigger
//!
//! Somebody asked for something Nudge could not do, and named what would have
//! done it. That is the only moment worth using, because it is the only one
//! where the examples on the bar land -- one of them is what they said thirty
//! seconds ago. Every other trigger is a guess dressed up as helpfulness.
//!
//! This is 3.2 with a face on it. *"ffmpeg is not on this Mac"* and *"I cannot
//! see your calendar"* are the same sentence about different things, and the
//! same rule applies to both: at the moment it matters, once.
//!
//! ## What is never done
//!
//! - **Offering because the environment suggests it.** `gh` is installed, so
//!   here is GitHub. That is inference rather than a request, it can fire on the
//!   first day, and it is how every application that nags you begins.
//! - **Interrupting.** Nothing is offered while an agent is working or while
//!   somebody is mid-sentence. Breaking off what they asked for to raise
//!   something else is the worst available timing.
//! - **Asking again after a no.** They decided.
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

/// A service worth offering, and what it would be good for.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Offer {
    pub name: String,
    /// A letter or two for the tile. Other companies' logos are a licensing
    /// question, and a coloured initial carries the same recognition at 38px.
    pub mark: String,
    pub tint: String,
    /// Dark text, for a light tile.
    pub dark: bool,
    /// Things somebody could ask for once it is connected. Written as requests
    /// in their words, never as capabilities in ours: "summarise the open PRs",
    /// not "read repository metadata".
    pub examples: Vec<String>,
    /// Applications or commands whose presence means this person plausibly uses
    /// the thing.
    ///
    /// The only grounds on which an offer is ever raised unprompted. Somebody
    /// with the Slack app on their Mac uses Slack; somebody with `gh` installed
    /// works on GitHub. It is weak evidence and it is the only evidence there is
    /// short of being asked -- which is why volunteering is held to a far
    /// stricter budget than answering.
    ///
    /// Empty means never volunteered. Calendar and Mail ship with macOS, so
    /// their presence says nothing at all about whether somebody wants them
    /// connected.
    #[serde(skip)]
    pub evidence: Vec<&'static str>,
}

/// The offers there are, for now written down rather than discovered.
///
/// This will come from somewhere else eventually -- what is installed, what is
/// configured, what the person keeps asking for and cannot have. Hard-coded here
/// because the question being answered today is what the bar looks like.
pub fn catalogue() -> Vec<Offer> {
    vec![
        Offer {
            name: "GitHub".into(),
            evidence: vec!["gh", "GitHub Desktop"],
            mark: "GH".into(),
            tint: "#e6e6e6".into(),
            dark: true,
            examples: vec![
                "What broke the build?".into(),
                "Summarise the open PRs".into(),
                "Find issues about this bug".into(),
                "Draft a release checklist".into(),
                "Who reviewed this last?".into(),
                "What changed since Friday?".into(),
                "Open a PR for this branch".into(),
                "Is CI green on main?".into(),
                "Find the commit that touched this".into(),
            ],
        },
        Offer {
            name: "Calendar".into(),
            evidence: vec![],
            mark: "31".into(),
            tint: "#1a73e8".into(),
            dark: false,
            examples: vec![
                "What does my day look like?".into(),
                "Move the standup to Thursday".into(),
                "Find an hour with Sara this week".into(),
                "Am I free on Friday afternoon?".into(),
                "When is my next meeting?".into(),
                "Block out two hours tomorrow".into(),
                "Cancel the three o'clock".into(),
                "What is on next Monday?".into(),
            ],
        },
        Offer {
            name: "Mail".into(),
            evidence: vec![],
            mark: "M".into(),
            tint: "#ea4335".into(),
            dark: false,
            examples: vec![
                "Anything urgent this morning?".into(),
                "Reply saying I will be late".into(),
                "Find the invoice from March".into(),
                "Draft a reply to the last one".into(),
                "Who has not answered me?".into(),
                "Unsubscribe me from this".into(),
                "Send that file to Sara".into(),
                "What did the client say?".into(),
            ],
        },
        Offer {
            name: "Slack".into(),
            evidence: vec!["Slack"],
            mark: "S".into(),
            tint: "#4a154b".into(),
            dark: false,
            examples: vec![
                "What did I miss in #general?".into(),
                "Tell the team I am running late".into(),
                "Find where we discussed pricing".into(),
                "Anyone mention me today?".into(),
                "Summarise the standup thread".into(),
                "Send this to the design channel".into(),
                "What is the link they shared?".into(),
            ],
        },
    ]
}

/// How long *not now* lasts.
///
/// Long enough that it does not feel like the same question tomorrow, short
/// enough that somebody who was busy gets a second chance at something they
/// might want. Two weeks is a guess and is the first number to change if the
/// bar turns out to be annoying.
const LATER_DAYS: u64 = 14;

/// How long between offers nobody asked for.
///
/// Far longer than the gap between answers to a request, and deliberately so. An
/// offer raised in reply to somebody hitting a wall is a useful answer; an offer
/// raised because they own an application is a guess, and a guess gets one
/// chance a week rather than one a day.
const VOLUNTEER_DAYS: u64 = 7;

/// Turns somebody has to have taken before anything is raised unprompted.
///
/// Not a delay, a sign of use. Somebody who has just installed this and is
/// finding out what it does should not be sold anything -- the first thing a new
/// person meets should be the thing they came for.
const SETTLED_IN: u64 = 5;

/// The least time between any two offers, whatever they are about.
///
/// The limit is attention, not relevance: a person who hits three walls in an
/// afternoon has three good reasons to be asked and one appetite for being
/// asked.
const QUIET_HOURS: u64 = 24;

const DAY_MS: u64 = 86_400_000;
const HOUR_MS: u64 = 3_600_000;

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Said {
    /// Never ask about this again.
    No,
    /// Not at the moment.
    Later,
    /// Connected, or on the way to it.
    Yes,
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Answer {
    pub said: Said,
    /// When, in milliseconds since the epoch.
    pub at: u64,
}

/// What has been asked, and what came back.
#[derive(Default)]
pub struct Offers {
    path: Option<PathBuf>,
    answers: Mutex<BTreeMap<String, Answer>>,
    /// When any offer was last put on screen. Not persisted: a quiet day is
    /// about this session's attention, and a restart is a new day's worth.
    last: Mutex<Option<u64>>,
    /// When one was last raised without being asked for. Persisted, because a
    /// weekly budget that resets whenever somebody quits is not weekly.
    volunteered: Mutex<Option<u64>>,
    /// Turns taken this session. See [`SETTLED_IN`].
    turns: std::sync::atomic::AtomicU64,
}

/// The whole file, which is the answers plus the one timestamp that has to
/// outlive the process.
#[derive(Default, serde::Serialize, serde::Deserialize)]
struct Written {
    #[serde(default)]
    volunteered: Option<u64>,
    #[serde(default, flatten)]
    answers: BTreeMap<String, Answer>,
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

impl Offers {
    pub fn load() -> Offers {
        let path = dirs::home_dir().map(|d| d.join(".config/nudge/offers.toml"));
        let written: Written = path
            .as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|t| toml::from_str(&t).ok())
            .unwrap_or_default();
        Offers {
            path,
            answers: Mutex::new(written.answers),
            last: Mutex::new(None),
            volunteered: Mutex::new(written.volunteered),
            turns: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Is now a reasonable moment to ask about this?
    ///
    /// `busy` is whether anything is running. Passed in rather than read here so
    /// that this stays a decision about offers and not a thing that reaches into
    /// the agent runtime.
    pub fn may_ask(&self, service: &str, busy: bool) -> bool {
        if busy {
            return false;
        }
        let now = now_ms();
        if let Some(last) = *self.last.lock().unwrap() {
            if now.saturating_sub(last) < QUIET_HOURS * HOUR_MS {
                return false;
            }
        }
        match self.answers.lock().unwrap().get(service) {
            // Asked and answered, in both directions: there is nothing to offer
            // somebody who already said yes.
            Some(a) if matches!(a.said, Said::No | Said::Yes) => false,
            Some(a) => now.saturating_sub(a.at) > LATER_DAYS * DAY_MS,
            None => true,
        }
    }

    /// Note that one has been put on screen.
    pub fn asked(&self) {
        *self.last.lock().unwrap() = Some(now_ms());
    }

    /// Somebody said something. Counted, not recorded.
    pub fn a_turn_happened(&self) {
        self.turns
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Is this a reasonable moment to raise something nobody asked about?
    ///
    /// Everything `may_ask` requires, and then three more things: they are
    /// actually using this, there is some evidence they use the service, and the
    /// last unprompted offer was a week ago. The extra weight is the whole
    /// difference between an offer and a nag.
    pub fn may_volunteer(&self, offer: &Offer, busy: bool, here: impl Fn(&str) -> bool) -> bool {
        if offer.evidence.is_empty() {
            return false;
        }
        if self.turns.load(std::sync::atomic::Ordering::Relaxed) < SETTLED_IN {
            return false;
        }
        if !offer.evidence.iter().any(|e| here(e)) {
            return false;
        }
        if let Some(when) = *self.volunteered.lock().unwrap() {
            if now_ms().saturating_sub(when) < VOLUNTEER_DAYS * DAY_MS {
                return false;
            }
        }
        self.may_ask(&offer.name, busy)
    }

    /// Note that one was raised unprompted.
    pub fn volunteered(&self) {
        *self.volunteered.lock().unwrap() = Some(now_ms());
        self.save();
    }

    /// Note what they said, and keep it.
    pub fn answered(&self, service: &str, said: Said) {
        self.answers
            .lock()
            .unwrap()
            .insert(service.to_string(), Answer { said, at: now_ms() });
        self.save();
        eprintln!("offer: {service} -> {said:?}");
    }

    fn save(&self) {
        let Some(path) = &self.path else { return };
        let written = Written {
            volunteered: *self.volunteered.lock().unwrap(),
            answers: self.answers.lock().unwrap().clone(),
        };
        let Ok(text) = toml::to_string(&written) else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Err(e) = std::fs::write(path, text) {
            eprintln!("offer: could not write {}: {e}", path.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Nothing here touches the real file.
    fn offers() -> Offers {
        Offers::default()
    }

    #[test]
    fn something_never_asked_about_may_be_asked_about() {
        assert!(offers().may_ask("Calendar", false));
    }

    #[test]
    fn nothing_is_offered_while_something_is_running() {
        assert!(!offers().may_ask("Calendar", true));
    }

    /// They decided. Asking again is not persistence, it is not listening.
    #[test]
    fn a_no_is_forever() {
        let o = offers();
        o.answered("Calendar", Said::No);
        assert!(!o.may_ask("Calendar", false));
    }

    /// And there is nothing to offer somebody who already connected it.
    #[test]
    fn a_yes_is_not_asked_again_either() {
        let o = offers();
        o.answered("Calendar", Said::Yes);
        assert!(!o.may_ask("Calendar", false));
    }

    #[test]
    fn a_later_holds_for_a_fortnight_and_then_lets_go() {
        let o = offers();
        o.answered("Calendar", Said::Later);
        assert!(!o.may_ask("Calendar", false));

        // Wound back past the fortnight.
        o.answers.lock().unwrap().insert(
            "Calendar".into(),
            Answer {
                said: Said::Later,
                at: now_ms() - (LATER_DAYS + 1) * DAY_MS,
            },
        );
        assert!(o.may_ask("Calendar", false));
    }

    /// Three walls in an afternoon are three good reasons and one appetite.
    #[test]
    fn only_one_offer_a_day_whatever_it_is_about() {
        let o = offers();
        assert!(o.may_ask("Calendar", false));
        o.asked();
        assert!(
            !o.may_ask("Mail", false),
            "a different service is still an interruption"
        );
    }

    fn github() -> Offer {
        catalogue()
            .into_iter()
            .find(|o| o.name == "GitHub")
            .unwrap()
    }

    /// Five turns of use before anything is raised unprompted. Somebody finding
    /// out what this does should meet the thing they came for, not an offer.
    #[test]
    fn nothing_is_volunteered_to_somebody_who_just_arrived() {
        let o = offers();
        assert!(!o.may_volunteer(&github(), false, |_| true));
        for _ in 0..5 {
            o.a_turn_happened();
        }
        assert!(o.may_volunteer(&github(), false, |_| true));
    }

    /// Owning the application is the only evidence there is, and without it
    /// there is nothing but a guess.
    #[test]
    fn nothing_is_volunteered_without_evidence() {
        let o = offers();
        for _ in 0..5 {
            o.a_turn_happened();
        }
        assert!(!o.may_volunteer(&github(), false, |_| false));
    }

    /// Calendar ships with macOS, so its presence says nothing about whether
    /// anybody wants it connected -- it has no evidence and is never raised.
    #[test]
    fn something_everybody_has_is_never_volunteered() {
        let o = offers();
        for _ in 0..5 {
            o.a_turn_happened();
        }
        let calendar = catalogue()
            .into_iter()
            .find(|c| c.name == "Calendar")
            .unwrap();
        assert!(calendar.evidence.is_empty());
        assert!(!o.may_volunteer(&calendar, false, |_| true));
    }

    #[test]
    fn one_unprompted_offer_a_week() {
        let o = offers();
        for _ in 0..5 {
            o.a_turn_happened();
        }
        assert!(o.may_volunteer(&github(), false, |_| true));
        o.volunteered();
        assert!(!o.may_volunteer(&github(), false, |_| true));
    }

    /// Everything that stops a requested offer stops an unprompted one too.
    #[test]
    fn a_no_stops_the_unprompted_kind_as_well() {
        let o = offers();
        for _ in 0..5 {
            o.a_turn_happened();
        }
        o.answered("GitHub", Said::No);
        assert!(!o.may_volunteer(&github(), false, |_| true));
    }

    #[test]
    fn one_service_being_refused_says_nothing_about_another() {
        let o = offers();
        o.answered("Calendar", Said::No);
        assert!(o.may_ask("Mail", false));
    }
}
