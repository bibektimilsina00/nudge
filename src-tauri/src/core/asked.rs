//! Recognising that a program has stopped to ask something.
//!
//! A supervised tool prints until it wants an answer, and then it waits. To be
//! the person at that terminal, Nudge has to notice — and it has to notice
//! **mechanically**, because the thing it is reading is untrusted.
//!
//! That is the whole discipline of this module and the reason it exists apart
//! from anything that thinks. What a terminal prints may be a file somebody
//! asked it to summarise, a page it fetched, a commit message. Handing that
//! prose to a model to be asked "is this a question?" is handing an attacker a
//! way to address the thing deciding what Nudge does next.
//!
//! So: shapes, not sentences. A question is output that has stopped, ending in
//! one of a few recognisable forms — a `[y/N]`, a numbered list with a cursor on
//! it, a line ending in a question mark with nothing after it. What comes out is
//! a [`Shape`] and the few facts that can be read off it with certainty. The
//! prose goes no further.
//!
//! **A miss is safe and a false positive is not free.** Failing to notice a
//! prompt leaves the run waiting, which the stall watch catches; deciding that
//! ordinary output is a question would have Nudge typing into a program that
//! never asked. So every rule here requires the output to have *stopped* at the
//! thing it matched.

/// Remove terminal escape sequences.
///
/// A pty carries colour, cursor movement and whatever else the program felt
/// like drawing. None of it is content, and all of it defeats matching on what
/// a line ends with — `"[y/N] "` and `"[y/N] \x1b[0m"` are the same question and
/// only one of them ends in a bracket.
pub fn plain(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\u{1b}' {
            out.push(c);
            continue;
        }
        match chars.peek() {
            // CSI: ends at the first byte in @ to ~.
            Some('[') => {
                chars.next();
                for c in chars.by_ref() {
                    if ('\u{40}'..='\u{7e}').contains(&c) {
                        break;
                    }
                }
            }
            // OSC: ends at BEL or ST.
            Some(']') => {
                chars.next();
                while let Some(c) = chars.next() {
                    if c == '\u{7}' {
                        break;
                    }
                    if c == '\u{1b}' && chars.peek() == Some(&'\\') {
                        chars.next();
                        break;
                    }
                }
            }
            // Anything else is a two-character sequence.
            Some(_) => {
                chars.next();
            }
            None => {}
        }
    }
    out
}

/// What kind of question, as far as can be told from its shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Shape {
    /// `[y/N]`, `(y/n)`, `yes/no`. The default, when one is shown, is recorded:
    /// a capital letter in `[y/N]` is the tool saying what happens on Return.
    YesNo { fallback: Option<bool> },
    /// A numbered list waiting on a choice. The options are carried as written,
    /// because choosing between them means reading them -- but they are carried
    /// as *data to be classified*, never as instructions.
    Pick { options: Vec<String> },
    /// It is waiting, and the shape is not one of the known ones. Always a
    /// person's decision: an unrecognised question is not one to guess at.
    Unknown,
}

/// A program waiting to be answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asked {
    pub shape: Shape,
    /// The last line, cleaned and clipped. For the record and for a person to
    /// read -- never for a judge, which sees facts rather than prose.
    pub line: String,
}

/// How much of the tail is worth looking at.
const TAIL: usize = 2_000;

/// Longest a prompt line is allowed to be.
///
/// A prompt is short. Something enormous ending in a question mark is a
/// document that happens to end in a question mark.
const LONGEST: usize = 300;

/// Is this output waiting for an answer?
///
/// Given everything the program has printed. The caller decides *when* to ask --
/// only once output has gone quiet, or this would fire on a question in the
/// middle of a paragraph that was still being written.
pub fn waiting(output: &str) -> Option<Asked> {
    let text = plain(output);
    let tail: String = match text.char_indices().nth_back(TAIL) {
        Some((i, _)) => text[i..].to_string(),
        None => text.clone(),
    };

    // Trailing whitespace only -- a prompt often ends with "> " or ": " and the
    // space is part of it, but a newline after the question means the program
    // carried on and is not waiting here.
    let trimmed = tail.trim_end_matches([' ', '\t']);
    let last = trimmed.lines().next_back()?.trim();
    if last.is_empty() || last.chars().count() > LONGEST {
        return None;
    }

    let lowered = last.to_lowercase();

    // A yes/no, with whatever it says the default is.
    for (mark, fallback) in [
        ("[y/n]", None),
        ("(y/n)", None),
        ("[yes/no]", None),
        ("y/n", None),
    ] {
        if lowered.contains(mark) {
            // The capital is the tool telling you what Return does.
            let fallback = match (last.contains("/N"), last.contains("[Y/"), fallback) {
                (true, _, _) => Some(false),
                (_, true, _) => Some(true),
                (_, _, f) => f,
            };
            return Some(Asked {
                shape: Shape::YesNo { fallback },
                line: clip(last),
            });
        }
    }

    // A numbered list, with the cursor sitting on one of them.
    let options = menu(trimmed);
    if options.len() > 1 {
        return Some(Asked {
            shape: Shape::Pick { options },
            line: clip(question_above(trimmed).unwrap_or(last)),
        });
    }

    // Stopped on a question, or on something that reads as a prompt.
    if last.ends_with('?') || last.ends_with('>') || last.ends_with(':') {
        return Some(Asked {
            shape: Shape::Unknown,
            line: clip(last),
        });
    }

    None
}

/// The numbered options at the end of the output, in order.
///
/// Only a run of them at the very end: a numbered list in the middle of a
/// paragraph is prose, and a program that printed one and carried on is not
/// waiting on it.
fn menu(text: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for line in text.lines().rev() {
        let line = line.trim();
        if line.is_empty() {
            // One blank line inside a menu is ordinary; two means it ended.
            if found.is_empty() {
                continue;
            }
            break;
        }
        // "1. Yes", "❯ 2. No", "> 3. Something else"
        let bare = line
            .trim_start_matches(['❯', '>', '*', '-', ' '])
            .trim_start();
        let Some((number, rest)) = bare.split_once(['.', ')']) else {
            break;
        };
        if number.trim().parse::<u32>().is_err() || rest.trim().is_empty() {
            break;
        }
        found.push(rest.trim().to_string());
        if found.len() > 12 {
            break;
        }
    }
    found.reverse();
    found
}

/// The line above a menu, which is usually the actual question.
fn question_above(text: &str) -> Option<&str> {
    let mut seen_menu = false;
    for line in text.lines().rev() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        let bare = t.trim_start_matches(['❯', '>', '*', '-', ' ']).trim_start();
        let numbered = bare
            .split_once(['.', ')'])
            .map(|(n, _)| n.trim().parse::<u32>().is_ok())
            .unwrap_or(false);
        if numbered {
            seen_menu = true;
            continue;
        }
        if seen_menu {
            return Some(t);
        }
    }
    None
}

fn clip(line: &str) -> String {
    match line.chars().count() > LONGEST {
        true => line.chars().take(LONGEST).collect(),
        false => line.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_codes_are_not_content() {
        assert_eq!(plain("\u{1b}[32mgreen\u{1b}[0m"), "green");
        assert_eq!(plain("\u{1b}]0;a title\u{7}text"), "text");
        assert_eq!(plain("plain"), "plain");
        // The one that matters: colour after the bracket would defeat matching
        // on what a line ends with.
        assert_eq!(
            plain("Proceed? [y/N] \u{1b}[0m").trim_end(),
            "Proceed? [y/N]"
        );
    }

    /// The shapes these tools actually print.
    #[test]
    fn a_yes_no_is_recognised_with_its_default() {
        let a = waiting("Running tests...\nAllow this command? [y/N] ").unwrap();
        assert_eq!(
            a.shape,
            Shape::YesNo {
                fallback: Some(false)
            }
        );
        assert!(a.line.contains("Allow this command?"));

        let a = waiting("Overwrite the file? [Y/n] ").unwrap();
        assert_eq!(
            a.shape,
            Shape::YesNo {
                fallback: Some(true)
            }
        );

        let a = waiting("Continue (y/n)? ").unwrap();
        assert!(matches!(a.shape, Shape::YesNo { .. }));
    }

    /// Claude Code's own shape: a question, then a numbered list with a cursor.
    #[test]
    fn a_numbered_menu_is_recognised_and_the_question_above_it_kept() {
        let out = "Edit file src/main.rs?\n\n\u{1b}[36m❯ 1. Yes\u{1b}[0m\n  2. Yes, and don't ask again\n  3. No, and tell Claude what to do differently";
        let a = waiting(out).unwrap();
        match &a.shape {
            Shape::Pick { options } => {
                assert_eq!(options.len(), 3, "{options:?}");
                assert_eq!(options[0], "Yes");
                assert!(options[2].starts_with("No,"));
            }
            other => panic!("{other:?}"),
        }
        // The question, not the last option.
        assert_eq!(a.line, "Edit file src/main.rs?");
    }

    /// Anything waiting in a shape nobody recognises is a person's decision.
    #[test]
    fn an_unrecognised_prompt_is_not_guessed_at() {
        let a = waiting("Enter your name: ").unwrap();
        assert_eq!(a.shape, Shape::Unknown);
        let a = waiting("What should I do about the failing test?").unwrap();
        assert_eq!(a.shape, Shape::Unknown);
    }

    /// The important negative: ordinary output is not a question.
    ///
    /// A false positive means typing into a program that never asked, which is
    /// worse than missing one -- a missed prompt is caught by the stall watch.
    #[test]
    fn ordinary_output_is_left_alone() {
        for out in [
            "Compiling nudge v0.1.0\nFinished in 3.2s\n",
            "test result: ok. 338 passed\n",
            // Ends with a question mark, but the program carried on afterwards.
            "Is this right? Yes, it is.\nDone.\n",
            "",
            "   \n\n",
        ] {
            assert!(waiting(out).is_none(), "read a question into: {out:?}");
        }
    }

    /// A question in the middle of a paragraph is not one being asked.
    #[test]
    fn a_question_that_was_printed_past_is_not_waiting() {
        assert!(waiting("Do you want to proceed? [y/N]\nProceeding anyway.\n").is_none());
    }

    /// A numbered list somebody printed and moved on from is not a menu.
    #[test]
    fn a_list_in_the_middle_of_output_is_not_a_menu() {
        let out = "Steps:\n 1. build\n 2. test\n 3. ship\n\nAll three finished.\n";
        let found = waiting(out);
        assert!(
            !matches!(found.as_ref().map(|a| &a.shape), Some(Shape::Pick { .. })),
            "{found:?}"
        );
    }

    /// A document that happens to end in a question mark is not a prompt.
    #[test]
    fn something_enormous_is_not_a_prompt() {
        let essay = format!("{}?", "word ".repeat(200));
        assert!(waiting(&essay).is_none());
    }
}
