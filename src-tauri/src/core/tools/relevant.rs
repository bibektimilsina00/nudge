//! Which tools to put in front of the model, out of two hundred.
//!
//! Measured on this machine: 197 tools across eight servers, 24,651 characters,
//! **forty-seven per cent of every prompt** -- sent to answer "what is on this
//! screen". A catalogue that size is not a list, it is a phone book, and it is
//! read out in full on every turn whether or not anything reaches for a tool.
//!
//! ## What is sent instead
//!
//! Every server, always, with its name and how many tools it has. That line is a
//! few hundred characters and it is what keeps this honest: nothing is hidden,
//! the model can always see that `google` exists and has 110 tools in it.
//!
//! Then the tools themselves, for the servers this turn plausibly touches: the
//! ones whose own words overlap the request, and the ones already used in this
//! session, because a task that has started using a server is going to keep
//! using it.
//!
//! ## Matched on the server's words, not on a list of synonyms
//!
//! "Send Sara an email" has to reach `gmail`, and the word "email" appears all
//! over that server's own tool names and descriptions. So the scoring is: how
//! many of the request's words appear in everything this server says about
//! itself. No synonym table to maintain, no model call to decide what to send to
//! a model, and a server that describes itself well is found by the words a
//! person would actually use.
use super::mcp::Tool;

/// Words too common to mean anything. Not a general stopword list -- these are
/// the ones that appear in tool descriptions and would match everything.
const NOISE: [&str; 24] = [
    "the", "this", "that", "and", "for", "with", "from", "into", "what", "when", "where", "which",
    "there", "here", "your", "you", "can", "get", "set", "new", "all", "any", "please", "about",
];

/// Shorter than this and a word is not evidence of anything.
const SHORTEST: usize = 4;

fn words(text: &str) -> Vec<String> {
    text.to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| w.len() >= SHORTEST && !NOISE.contains(w))
        .map(str::to_string)
        .collect()
}

/// How well one tool answers this request: how many of its words appear in what
/// the tool says about itself.
///
/// The server's name counts, so "check my gmail inbox" reaches `gmail` by name
/// as well as by description. A tool on a server already in use gets a point for
/// free, because a task that has started somewhere is usually still there.
fn score(tool: &Tool, asked: &[String], using: &[String]) -> usize {
    let said = format!("{} {} {}", tool.server, tool.name, tool.about).to_ascii_lowercase();
    let hits = asked.iter().filter(|w| said.contains(w.as_str())).count();
    hits + usize::from(using.iter().any(|u| *u == tool.server))
}

/// The most relevant tools, best first, and never more than [`MOST`].
pub fn best<'a>(goal: &str, tools: &'a [Tool], using: &[String]) -> Vec<&'a Tool> {
    let asked = words(goal);
    let mut ranked: Vec<(usize, &Tool)> = tools
        .iter()
        .map(|t| (score(t, &asked, using), t))
        .filter(|(n, _)| *n > 0)
        .collect();
    // Best first, and stable within a score so one server's tools stay together
    // and the order does not wander between turns.
    ranked.sort_by(|a, b| b.0.cmp(&a.0));
    ranked.into_iter().take(MOST).map(|(_, t)| t).collect()
}

/// How many tool lines may go in a prompt.
///
/// Thirty is about four thousand characters, which is a tenth of what the whole
/// catalogue cost and more than any single turn has ever needed. The number is a
/// ceiling rather than a target: a request that matches three tools sends three.
///
/// The failure this bounds is not "too many matched" but "one server is enormous"
/// -- `google` alone offers 110 tools here, and any word at all appears somewhere
/// in 110 descriptions.
const MOST: usize = 30;

/// The catalogue, as the prompt should carry it.
///
/// `None` when there are no servers at all, which is the default and says
/// nothing rather than explaining an absence.
pub fn catalogue(goal: &str, tools: &[Tool], using: &[String]) -> Option<String> {
    if tools.is_empty() {
        return None;
    }
    // Every server and its size, so nothing is hidden: the model can always see
    // that a server exists, how big it is, and therefore that there is more to
    // ask for.
    let mut servers: Vec<(&str, usize)> = Vec::new();
    for tool in tools {
        match servers.iter_mut().find(|(s, _)| *s == tool.server) {
            Some((_, n)) => *n += 1,
            None => servers.push((tool.server.as_str(), 1)),
        }
    }
    let index = servers
        .iter()
        .map(|(name, n)| format!("{name} ({n})"))
        .collect::<Vec<_>>()
        .join(" · ");

    let picked = best(goal, tools, using);
    let mut out = format!(
        "Tool servers connected: {index}. Answer `tools` with a server's name for \
         its full list.\n\n"
    );
    if picked.is_empty() {
        out.push_str("None of them look related to this, so none are listed.\n\n");
        return Some(out);
    }
    out.push_str(&format!(
        "The ones that look related to this. Use `mcp` with the full name and an \
         `args` object; starred arguments are required. If a call is refused for \
         the shape of its arguments, read what it said and try again -- the server \
         is describing itself more precisely than a line can.\n{}\n\n",
        picked
            .iter()
            .map(|t| format!("  {}", t.line()))
            .collect::<Vec<_>>()
            .join("\n")
    ));
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tool(server: &str, name: &str, about: &str) -> Tool {
        Tool {
            server: server.into(),
            name: name.into(),
            about: about.into(),
            schema: json!({}),
        }
    }

    fn kit() -> Vec<Tool> {
        vec![
            tool(
                "gmail",
                "send_message",
                "Send an email message to a recipient",
            ),
            tool("gmail", "list_messages", "List email messages in a mailbox"),
            tool("github", "create_issue", "Open an issue on a repository"),
            tool("calendar", "add_event", "Put a meeting in the calendar"),
        ]
    }

    /// The word a person uses reaches the server that uses it too, with no table
    /// of synonyms in between.
    fn names(found: &[&Tool]) -> Vec<String> {
        found.iter().map(|t| t.id()).collect()
    }

    /// The word a person uses reaches the tool that uses it too, with no table
    /// of synonyms in between.
    #[test]
    fn the_request_finds_the_tool_that_talks_like_it() {
        let kit = kit();
        assert_eq!(
            names(&best("send sara an email about friday", &kit, &[])),
            vec!["gmail/send_message", "gmail/list_messages"]
        );
        assert_eq!(
            names(&best("open an issue about the crash", &kit, &[])),
            vec!["github/create_issue"]
        );
    }

    #[test]
    fn a_server_already_in_use_stays_listed_whatever_the_words_say() {
        let kit = kit();
        let found = best("and the one after that", &kit, &["github".into()]);
        assert_eq!(names(&found), vec!["github/create_issue"]);
    }

    /// The failure this exists to prevent: one enormous server, and a word that
    /// appears somewhere in a hundred descriptions.
    #[test]
    fn an_enormous_server_cannot_fill_the_prompt() {
        let mut huge: Vec<Tool> = (0..200)
            .map(|n| {
                tool(
                    "google",
                    &format!("thing_{n}"),
                    "Does a thing with a document",
                )
            })
            .collect();
        huge.push(tool("gmail", "send_message", "Send an email"));
        let found = best("write a document", &huge, &[]);
        assert_eq!(found.len(), MOST, "capped at {MOST}, got {}", found.len());
    }

    /// The point of the exercise: the tools of servers this turn is not about
    /// do not appear, and the fact that they exist still does.
    #[test]
    fn the_unrelated_servers_cost_their_name_and_nothing_else() {
        let kit = kit();
        let said = catalogue("send sara an email", &kit, &[]).expect("a catalogue");
        assert!(said.contains("gmail/send_message"), "{said}");
        assert!(!said.contains("github/create_issue"), "{said}");
        // Still visible, still reachable, just not spelled out.
        assert!(said.contains("github (1)"), "{said}");
        assert!(said.contains("`tools`"), "{said}");
    }

    #[test]
    fn nothing_matching_says_so_rather_than_listing_everything() {
        let kit = kit();
        let said = catalogue("click the red button", &kit, &[]).expect("a catalogue");
        assert!(said.contains("none are listed"), "{said}");
        assert!(said.contains("gmail (2)"), "{said}");
        assert!(!said.contains("send_message"), "{said}");
    }

    #[test]
    fn no_servers_says_nothing_at_all() {
        assert!(catalogue("anything", &[], &[]).is_none());
    }
}
