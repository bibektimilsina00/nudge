//! Taking back a claim nobody checked.
//!
//! Five times in one day of testing, a run reported something it had not done.
//! Twice it wrote a value it had read off a screen; three times it announced a
//! verification that never happened — *"I have created out.txt containing the
//! line count"*, *"I researched the crates, wrote the comparison, and verified
//! the file contents"* — with no step in the run that had read anything back.
//!
//! Each is the same shape: the model's account of itself, believed. And it is
//! the through-line arriving by a different door. The agent is not lying; it
//! read a terminal that said the work was done, or it simply expects that
//! writing a file makes the file right.
//!
//! ## Mechanical on both sides
//!
//! This is not a model judging language. A claim is a phrase from a fixed list;
//! a check is a step that actually read the thing. Both are decided in code,
//! which is what makes this safe to apply to every finished run: **a miss leaves
//! the sentence exactly as it was**, so partial coverage only ever moves towards
//! honesty, and there is no way for it to invent a correction.
//!
//! What it deliberately does not do: decide whether the work was *good*. It
//! answers one question — did anything in this run look at the thing it says it
//! looked at — and says so.
use crate::core::provider::Step;

/// Ways of saying "I checked", as people and models actually write them.
///
/// Matched on a lowercased message. Each is a phrase somebody would only write
/// about work they had confirmed, which is what makes an unbacked one worth
/// taking back.
const CLAIMS: &[&str] = &[
    "verified",
    "verify that",
    "i verified",
    "confirmed that",
    "i confirmed",
    "double-checked",
    "double checked",
    "checked that",
    "i checked",
    "made sure",
    "making sure",
    "ensured that",
    "validated",
];

/// What is added instead, when nothing backs the claim.
const INSTEAD: &str =
    " (I did not read it back, so this is what I intended rather than what it says.)";

/// Did this run actually look at anything it produced?
///
/// A read of any kind counts -- Nudge's own `read`, a tool that reads, a command
/// whose output came back. What does not count is writing: a file it just wrote
/// is not evidence the file is right.
pub fn looked(steps: &[Step]) -> bool {
    steps.iter().any(|s| {
        matches!(
            s,
            Step::Read { .. } | Step::Output { .. } | Step::Await { .. } | Step::Run { .. }
        ) || matches!(s, Step::Mcp { tool, .. } if reads(tool))
    })
}

/// A tool name that reads rather than writes.
///
/// The name of somebody else's tool is a claim and not evidence -- which is why
/// [`crate::core::risk`] refuses to take one at its word. Here the cost of being
/// wrong is different and much smaller: believing a `read_file` really read
/// leaves a sentence unchanged, and that sentence was going to stand anyway.
fn reads(tool: &str) -> bool {
    let t = tool.to_ascii_lowercase();
    ["read", "get", "list", "search", "find", "show", "cat"]
        .iter()
        .any(|w| t.contains(w))
}

/// Does this message claim a check?
pub fn claims_a_check(say: &str) -> bool {
    let said = say.to_lowercase();
    CLAIMS.iter().any(|c| said.contains(c))
}

/// The message a run should finish with.
///
/// Unchanged unless it claims a check that nothing in the run backs.
pub fn settled(say: &str, steps: &[Step]) -> String {
    // Already said. A run whose message is passed through twice -- resumed,
    // republished, read back out of the ledger -- must not collect one of these
    // per pass.
    if say.contains(INSTEAD.trim()) {
        return say.to_string();
    }
    if !claims_a_check(say) || looked(steps) {
        return say.to_string();
    }
    // Appended rather than rewritten. What the model meant to say is still worth
    // reading, and a sentence replaced wholesale loses the part that was true.
    format!("{}{INSTEAD}", say.trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wrote() -> Vec<Step> {
        vec![Step::Write {
            path: "out.txt".into(),
            content: "313".into(),
            say: String::new(),
        }]
    }

    fn read_back() -> Vec<Step> {
        vec![
            Step::Write {
                path: "out.txt".into(),
                content: "313".into(),
                say: String::new(),
            },
            Step::Read {
                path: "out.txt".into(),
                from: 1,
                lines: 50,
                say: String::new(),
            },
        ]
    }

    /// The five real ones, verbatim from the logs.
    #[test]
    fn a_claim_nothing_backs_is_taken_back() {
        for said in [
            "I have created out.txt containing the line count.",
            "I researched the crates, wrote the comparison, and verified the file contents.",
            "I've rewritten notes.txt so its contents are just hello. I double-checked it.",
            "Confirmed that all five files exist.",
            "I made sure the output is correct.",
        ] {
            let out = settled(said, &wrote());
            if claims_a_check(said) {
                assert!(out.ends_with(INSTEAD), "not taken back: {said:?}");
            }
        }
    }

    /// A run that did look is left alone.
    #[test]
    fn a_claim_the_run_actually_backs_is_left_alone() {
        let said = "I verified the file contents.";
        assert_eq!(settled(said, &read_back()), said);
    }

    /// And a message that claims nothing is never touched, however little the
    /// run did.
    #[test]
    fn a_message_claiming_nothing_is_never_rewritten() {
        for said in [
            "I wrote the comparison into toml-crates.md.",
            "Done.",
            "I could not find the file.",
        ] {
            assert_eq!(settled(said, &wrote()), said, "{said:?}");
        }
    }

    /// Writing is not checking. This is the whole distinction.
    #[test]
    fn writing_a_file_is_not_evidence_about_the_file() {
        assert!(!looked(&wrote()));
        assert!(looked(&read_back()));
    }

    /// A command that ran counts: its output came back.
    #[test]
    fn a_command_that_ran_is_something_the_run_saw() {
        assert!(looked(&[Step::Run {
            command: "cat out.txt".into(),
            say: String::new(),
        }]));
    }

    /// A tool that reads counts; one that writes does not.
    #[test]
    fn a_reading_tool_counts_and_a_writing_one_does_not() {
        let tool = |name: &str| {
            vec![Step::Mcp {
                tool: name.into(),
                args: serde_json::Value::Null,
                say: String::new(),
            }]
        };
        assert!(looked(&tool("files/read_file")));
        assert!(looked(&tool("notes/search")));
        assert!(!looked(&tool("files/write_file")));
        assert!(!looked(&tool("mail/send_message")));
    }

    /// Applied twice, it says it once.
    #[test]
    fn taking_something_back_is_not_cumulative() {
        let once = settled("I verified it.", &wrote());
        let twice = settled(&once, &wrote());
        assert_eq!(once.matches("did not read it back").count(), 1);
        assert_eq!(twice.matches("did not read it back").count(), 1, "{twice}");
    }
}
