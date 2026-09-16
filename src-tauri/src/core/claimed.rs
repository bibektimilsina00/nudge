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

/// Does this recap describe a step that read something?
///
/// The recaps are Nudge's own sentences, so this reads a format this repository
/// controls. The trap is `Ran` doing double duty: "Ran `wc -l x`" is a shell
/// command and "Ran files/write_file with {...}" is a tool call, and one of
/// those is a write that happens to name the file it wrote.
fn recap_reads(recap: &str) -> bool {
    let recap = recap.trim().trim_start_matches("task · ").trim_start();
    if let Some(rest) = recap.strip_prefix("Ran ") {
        return match rest.starts_with('`') {
            true => true,
            false => rest.split_whitespace().next().map(reads).unwrap_or(false),
        };
    }
    recap.starts_with("Read ") || recap.starts_with("Showed ") || recap.starts_with("Waited for ")
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

/// Did this run do anything to the world?
///
/// The other half of [`looked`]. A run that wrote a file, clicked something or
/// ran a command has *acted*, and a sentence about what it did is backed by the
/// doing -- whether or not it also read anything.
pub fn acted(steps: &[Step]) -> bool {
    steps
        .iter()
        .any(|s| crate::core::risk::of(s).consequential())
}

/// A run that neither looked at anything nor changed anything is talking from
/// memory, whatever it sounds like.
///
/// Narrow on purpose, and both halves matter. Plenty of honest runs consult
/// nothing -- opening an application and clicking a button learns nothing from
/// outside, and the clicking *is* the evidence for "I opened it". What has no
/// evidence at all is a run that did neither and still states a fact.
///
/// The subagent has done this since it was written: an answer from a run that
/// consulted nothing is relabelled as recollection. This is the same rule for
/// the main agent, with the extra condition that it did not act either.
pub fn from_memory(say: &str, steps: &[Step], elsewhere: &[String]) -> bool {
    // `elsewhere` is what the same task already did before this run existed or
    // one level below it: the foreground turn that handed over, and any
    // subagent's steps. Both arrive as recaps.
    //
    // Without them this marked a true report as recollection. A foreground turn
    // ran a subagent that took thirteen steps, researched, wrote two files and
    // read one back; the agent started with nothing left to do, said what had
    // happened, and was told it was speaking from memory. The work was done and
    // checked -- just not by the object holding the pen.
    let looked_here = looked(steps) || elsewhere.iter().any(|r| recap_reads(r));
    let acted_here = acted(steps) || elsewhere.iter().any(|r| recap_acts(r));
    !looked_here && !acted_here && !hedged(say)
}

/// Does this recap describe a step that changed something?
///
/// The counterpart to [`recap_reads`], and the same format. Clicking and typing
/// count: they teach the run nothing, and they are still the evidence for "I
/// opened it".
fn recap_acts(recap: &str) -> bool {
    let recap = recap.trim().trim_start_matches("task · ").trim_start();
    [
        "Wrote ", "Edited ", "Clicked", "DoubleClick", "Typed ", "Pressed ", "Opened ",
        "Launched ", "Started ", "Handed over", "Noted about ",
    ]
    .iter()
    .any(|verb| recap.starts_with(verb))
        // A tool call that is not a read is something happening.
        || recap
            .strip_prefix("Ran ")
            .filter(|rest| !rest.starts_with('`'))
            .and_then(|rest| rest.split_whitespace().next())
            .map(|tool| !reads(tool))
            .unwrap_or(false)
}

/// Is this already an admission?
///
/// Nothing to add to a sentence that has said it. Borrowed from the subagent,
/// which has needed the same list for the same reason.
fn hedged(say: &str) -> bool {
    let s = say.to_lowercase();
    [
        "i think",
        "i believe",
        "from memory",
        "as of",
        "may have",
        "might be",
        "i am not sure",
        "i'm not sure",
        "cannot confirm",
        "did not check",
        "have not checked",
        "without checking",
    ]
    .iter()
    .any(|h| s.contains(h))
}

/// Files this run made and never looked at again.
///
/// Writing is not checking, and that is the whole of it. `git` says what
/// changed, which is worth having and is not the same question -- "the file is
/// 3,240 bytes and new" does not say whether it contains what was meant. A
/// document nobody read back is a draft the model is describing from memory.
///
/// Matched on the file's own name appearing in a step that reads. Crude and
/// deliberately so: a miss leaves the run exactly as it was, and the alternative
/// -- tracking which path a tool call resolved to -- is guessing at somebody
/// else's server.
pub fn unread(made: &[String], steps: &[Step], elsewhere: &[String]) -> Vec<String> {
    let mut looked_at: Vec<String> = steps
        .iter()
        .filter_map(|s| match s {
            Step::Read { path, .. } | Step::Show { path, .. } => Some(path.clone()),
            Step::Run { command, .. } => Some(command.clone()),
            Step::Mcp { tool, args, .. } if reads(tool) => Some(args.to_string()),
            _ => None,
        })
        .collect();

    // What a subagent did counts. It is the same run doing the same work one
    // level down, and its steps arrive as recaps rather than as `Step`s.
    //
    // Without this the check accused a run of not reading a document its own
    // subagent had read -- which is worse than missing one. A miss leaves a
    // sentence alone; a false correction contradicts something true, and then
    // the model's honest account and Nudge's appended fact disagree in the same
    // message with no way for a reader to tell which is right.
    looked_at.extend(elsewhere.iter().filter(|r| recap_reads(r)).cloned());

    made.iter()
        .filter(|path| {
            let name = std::path::Path::new(path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| (*path).clone());
            !looked_at.iter().any(|seen| seen.contains(&name))
        })
        .cloned()
        .collect()
}

/// What a run set itself and has not done.
///
/// `Step::Plan` exists, the card renders it, and nothing ever looked at it
/// again. A task with four named parts that ends after one is a task that
/// failed, and it reported success -- which is the same defect as an unbacked
/// claim, arriving through the run's own plan instead of its own sentence.
///
/// Mechanical, like everything else here. The plan is a list the model wrote
/// down; this only asks which entries are not marked done.
pub fn unfinished(plan: &[(String, bool)]) -> Vec<String> {
    plan.iter()
        .filter(|(_, done)| !done)
        .map(|(text, _)| text.clone())
        .collect()
}

/// The sentence to add when a run finishes without reading what it wrote.
///
/// Named by their last component: somebody reading this wants to know which
/// document, not where it lives.
pub fn unchecked(unread: &[String]) -> String {
    if unread.is_empty() {
        return String::new();
    }
    let named: Vec<String> = unread
        .iter()
        .map(|p| {
            std::path::Path::new(p)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| p.clone())
        })
        .collect();
    format!(" I did not read back what I wrote to {}.", named.join(", "))
}

/// The sentence to add when a run finishes with work still on its own list.
///
/// Named, not counted. "Two items remain" tells nobody what was skipped, and
/// the whole value is in somebody reading the list and noticing that the part
/// they cared about is on it.
pub fn stopped_early(left: &[String]) -> String {
    let named = match left.len() {
        0 => return String::new(),
        1 => format!("{:?}", left[0]),
        _ => left
            .iter()
            .map(|t| format!("{t:?}"))
            .collect::<Vec<_>>()
            .join(", "),
    };
    format!(" I stopped with this still on my own list: {named}.")
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

    /// The sentence names the file, not its path.
    #[test]
    fn what_was_not_read_back_is_named_by_its_own_name() {
        let said = unchecked(&["/Users/x/Work/report.md".to_string()]);
        assert!(said.contains("report.md"), "{said}");
        assert!(
            !said.contains("/Users/x"),
            "a path is not what somebody wants: {said}"
        );
        assert!(unchecked(&[]).is_empty());
    }

    /// What the same task already did counts, even from before this run.
    ///
    /// The bug: a foreground turn ran a subagent that took thirteen steps,
    /// researched, wrote two files and read one back. The agent started with
    /// nothing left to do, reported what had happened, and was told it was
    /// speaking from memory. The work was done -- just not by the object
    /// holding the pen.
    #[test]
    fn work_the_task_already_did_is_not_forgotten_at_a_boundary() {
        let carried = vec![
            "Read frameworks.md from line 1".to_string(),
            "Wrote recommendation.md (546 bytes)".to_string(),
        ];
        assert!(!from_memory(
            "I researched them and wrote the comparison.",
            &[],
            &carried
        ));
    }

    /// But carried talk is not carried work.
    #[test]
    fn being_handed_a_conversation_is_not_being_handed_evidence() {
        let carried = vec![
            "Said: I can look that up for you".to_string(),
            "Plan: 0 of 3 done".to_string(),
        ];
        assert!(from_memory("The population is 1.6 million.", &[], &carried));
    }

    /// A document nobody read back is a draft described from memory.
    #[test]
    fn a_file_that_was_written_and_never_looked_at_is_named() {
        let made = vec!["/w/report.md".to_string(), "/w/notes.txt".to_string()];
        let steps = vec![Step::Read {
            path: "notes.txt".into(),
            from: 1,
            lines: 200,
            say: String::new(),
        }];
        // Read by its bare name, which is how a run refers to its own workspace.
        assert_eq!(unread(&made, &steps, &[]), vec!["/w/report.md".to_string()]);
    }

    /// What a subagent read counts. It is the same run, one level down.
    ///
    /// The bug this is for: a run was told it had not read back a document its
    /// own subagent had read, so its honest sentence and Nudge's appended fact
    /// disagreed in the same message.
    #[test]
    fn what_a_subagent_read_counts_as_the_run_having_read_it() {
        let made = vec!["/w/frameworks.md".to_string()];
        let from_below = vec!["task · Read frameworks.md from line 1".to_string()];
        assert!(unread(&made, &[], &from_below).is_empty());
    }

    /// But a subagent *writing* it is still not reading it -- and the recap for
    /// a tool call starts with the same word as the one for a command.
    #[test]
    fn a_subagent_writing_it_does_not_count_as_reading_it() {
        let made = vec!["/w/frameworks.md".to_string()];
        for recap in [
            "task · Ran files/write_file with {\"path\":\"frameworks.md\"}",
            "task · Wrote frameworks.md (2531 bytes)",
            "task · Edited frameworks.md, replacing \"x\"",
        ] {
            assert_eq!(
                unread(&made, &[], &[recap.to_string()]).len(),
                1,
                "counted a write as a read: {recap}"
            );
        }
        // And one that genuinely reads, through a tool, does count.
        let read = "task · Ran files/read_file with {\"path\":\"frameworks.md\"}";
        assert!(unread(&made, &[], &[read.to_string()]).is_empty());
    }

    /// A command that reads it counts, and so does a tool that does.
    #[test]
    fn looking_at_it_any_way_at_all_counts() {
        let made = vec!["/w/report.md".to_string()];
        for step in [
            Step::Run {
                command: "wc -l report.md".into(),
                say: String::new(),
            },
            Step::Mcp {
                tool: "files/read_file".into(),
                args: serde_json::json!({ "path": "report.md" }),
                say: String::new(),
            },
        ] {
            assert!(
                unread(&made, std::slice::from_ref(&step), &[]).is_empty(),
                "{step:?}"
            );
        }
    }

    /// Writing it again is not reading it.
    #[test]
    fn writing_it_twice_is_still_not_checking_it() {
        let made = vec!["/w/report.md".to_string()];
        let wrote_again = vec![Step::Write {
            path: "report.md".into(),
            content: "x".into(),
            say: String::new(),
        }];
        assert_eq!(unread(&made, &wrote_again, &[]).len(), 1);
    }

    #[test]
    fn a_run_that_made_nothing_owes_nothing() {
        assert!(unread(&[], &wrote(), &[]).is_empty());
    }

    /// A run that neither looked nor acted is talking from memory.
    #[test]
    fn a_run_that_did_nothing_at_all_is_recollection() {
        assert!(from_memory("The capital of France is Paris.", &[], &[]));
    }

    /// But doing something is its own evidence. Clicking a button teaches the
    /// run nothing, and "I opened Safari" is backed by having opened it.
    #[test]
    fn a_run_that_acted_is_not_accused_of_remembering() {
        let clicked = vec![Step::Type {
            text: "hello".into(),
            submit: true,
            say: String::new(),
        }];
        assert!(!from_memory("I typed it in for you.", &clicked, &[]));
        assert!(!from_memory("I wrote the file.", &wrote(), &[]));
    }

    /// And a run that looked is grounded, obviously.
    #[test]
    fn a_run_that_looked_is_grounded() {
        assert!(!from_memory("It says 313 lines.", &read_back(), &[]));
    }

    /// A sentence that already admits it needs nothing added.
    #[test]
    fn an_answer_that_already_hedges_is_left_alone() {
        for said in [
            "I think it is Paris, though I did not check.",
            "From memory, the crate is maintained by epage.",
            "I'm not sure, but it looks like 40.",
        ] {
            assert!(!from_memory(said, &[], &[]), "{said:?}");
        }
    }

    /// A run that set itself four things and did one has not finished.
    #[test]
    fn what_a_run_set_itself_and_skipped_is_named() {
        let plan = vec![
            ("research the crates".to_string(), true),
            ("write the comparison".to_string(), true),
            ("check the file".to_string(), false),
            ("fix anything wrong".to_string(), false),
        ];
        let left = unfinished(&plan);
        assert_eq!(left.len(), 2);

        let said = stopped_early(&left);
        // Named, not counted: "two items remain" tells nobody what was skipped.
        assert!(said.contains("check the file"), "{said}");
        assert!(said.contains("fix anything wrong"), "{said}");
        assert!(!said.contains('2'), "counted instead of named: {said}");
    }

    #[test]
    fn a_finished_plan_says_nothing() {
        let plan = vec![("one".to_string(), true), ("two".to_string(), true)];
        assert!(unfinished(&plan).is_empty());
        assert!(stopped_early(&[]).is_empty());
    }

    /// No plan is not an unfinished plan.
    #[test]
    fn a_run_that_set_itself_nothing_is_not_accused_of_skipping_it() {
        assert!(unfinished(&[]).is_empty());
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
