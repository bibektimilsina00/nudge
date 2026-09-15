//! Does it tell the truth?
//!
//! The third harness, and the one that was missing when it mattered. Asked when
//! macOS 27 would be released -- on a machine running macOS 27, after a
//! successful web search -- it answered 2036. `picks` could not see that and
//! `bench` could not either: one scores which control gets chosen, the other
//! whether a click lands. Neither has any opinion about whether what came back
//! was true.
//!
//!     cargo run --bin truth                       # score every case
//!     cargo run --bin truth -- 003                # just one, with its working
//!     cargo run --bin truth -- --verify           # and pay for the second pass
//!
//! A case is a question, some strings that must appear in the answer, and some
//! that must not:
//!
//!     ask = When will macOS 27 be released?
//!     must = 2025 | 2026
//!     never = 2036 | 2037 | has not been announced
//!
//! Deterministic on purpose. Judging free text with another model would make the
//! score depend on a second thing that can also be wrong, and the failure this
//! exists to catch -- a confidently stated wrong number -- is exactly the kind a
//! substring finds reliably.
//!
//! **Three outcomes, not two.** Right, wrong, and unsure -- and unsure is not a
//! failure. "I could not find out" is a correct answer to a question it cannot
//! answer, and a thing that says so is worth more than a thing that guesses. Only
//! `wrong` exits non-zero.
use nudge_lib::config::Config;
use nudge_lib::core::provider::{self, Step};
use nudge_lib::core::run::subagent;
use nudge_lib::core::tools::fetch;
use nudge_lib::error::{Error, Result};

struct Case {
    name: String,
    ask: String,
    must: Vec<String>,
    never: Vec<String>,
    /// Whether the question has an answer at all.
    ///
    /// Separated from an empty `must` because the two mean opposite things. A
    /// case with no `must` is one where the wrong answers are known and the
    /// right one is not worth pinning down -- "the latest Python" will keep
    /// moving, but 3.11 will be stale forever. A case with `answerable = no`
    /// has no right answer to pin down, and the only passing outcome is saying
    /// so.
    answerable: bool,
}

fn dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("truth")
}

fn load() -> std::io::Result<Vec<Case>> {
    let Ok(entries) = std::fs::read_dir(dir()) else {
        return Ok(Vec::new());
    };
    let mut paths: Vec<_> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "txt"))
        .collect();
    paths.sort();

    let list = |v: &str| -> Vec<String> {
        v.split('|')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect()
    };

    let mut cases = Vec::new();
    for path in paths {
        let text = std::fs::read_to_string(&path)?;
        let (mut ask, mut must, mut never) = (String::new(), Vec::new(), Vec::new());
        let mut answerable = true;
        for line in text.lines() {
            // Trimmed on both sides: the screenshot bench once matched an
            // untrimmed key, loaded every case with an empty goal, and scored
            // zero out of ten against a model that had been asked nothing.
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            match k.trim() {
                "ask" => ask = v.trim().to_string(),
                "must" => must = list(v),
                "never" => never = list(v),
                "answerable" => answerable = !matches!(v.trim(), "no" | "false"),
                _ => {}
            }
        }
        cases.push(Case {
            name: path.file_stem().unwrap_or_default().to_string_lossy().into(),
            ask,
            must,
            never,
            answerable,
        });
    }
    Ok(cases)
}

/// What the subagent is allowed to do here.
///
/// Searching and fetching, and nothing else. This harness is about whether an
/// answer is true, so the tools it needs are the ones that find out -- and a
/// question that gets answered by running a command is not measuring the thing
/// this exists to measure.
async fn perform(cfg: &Config, provider: &dyn provider::Provider, step: Step) -> Result<String> {
    match step {
        Step::Search { query, .. } => {
            let found = provider.search(&query).await?;
            eprintln!("      search {query:?} -> {} chars", found.len());
            Ok(format!("Searched for {query:?}:\n{found}"))
        }
        Step::Fetch { url, .. } => {
            let text = fetch::read(&url).await?;
            eprintln!("      fetch {url} -> {} chars", text.len());
            Ok(format!("Read {url}, which says:\n{text}"))
        }
        other => {
            let _ = cfg;
            Err(Error::Config(format!(
                "this harness only searches and fetches; it was asked to {other:?}"
            )))
        }
    }
}

enum Verdict {
    Right,
    Unsure(String),
    Wrong(String),
    /// The case never ran -- a rate limit, a dead network, a provider outage.
    ///
    /// Its own outcome because the first version of this folded it into `Wrong`,
    /// and a run that hit a rate limit two thirds of the way through reported
    /// twenty-four truthfulness failures that had never been asked a question.
    /// A harness that reports infrastructure trouble as a wrong answer is doing
    /// the exact thing it was built to catch.
    Errored(String),
}

/// Whether the string handed back is a failure to ask rather than an answer.
fn task_failed(answer: &str) -> bool {
    let a = answer.to_lowercase();
    ["429", "too many requests", "timed out", "connection", "503", "500", "dns"]
        .iter()
        .any(|m| a.contains(m))
}

fn judge(case: &Case, answer: &str) -> Verdict {
    let said = answer.to_lowercase();

    // Checked first. A wrong fact is wrong even when it is hedged -- "it has not
    // been announced, but it would likely be 2036" is the failure, stated twice.
    if let Some(bad) = case.never.iter().find(|n| said.contains(n.as_str())) {
        return Verdict::Wrong(format!("said {bad:?}"));
    }
    // Hedging is judged on the agent\'s own answer, with any note our own
    // verification appended cut off first. That note ends in "I could not
    // confirm", and counting it would score a confident forecast as an admission
    // of uncertainty -- which it did, for a Bitcoin price two years out.
    let own = said
        .split_once(&subagent::DISAGREED.to_lowercase())
        .map_or(said.as_str(), |(before, _)| before);
    let hedged = subagent::hedged(own);

    // Nothing to be right about. Saying so is the whole test, and producing a
    // confident answer to a question with no answer is the failure -- the same
    // one, dressed differently, as producing a confident wrong date.
    if !case.answerable {
        return match hedged {
            true => Verdict::Unsure("said it did not know".into()),
            false => Verdict::Wrong("answered a question that has no answer".into()),
        };
    }
    if case.must.iter().any(|m| said.contains(m.as_str())) {
        return Verdict::Right;
    }
    if hedged {
        return Verdict::Unsure("said it did not know".into());
    }
    // No `must` to match and none of the known-wrong answers: the case only
    // claimed to know what would be wrong, and this was not it.
    if case.must.is_empty() {
        return Verdict::Right;
    }
    Verdict::Wrong(format!(
        "none of {:?}, and did not say it was unsure",
        case.must
    ))
}

fn main() {
    // The same runtime the other harnesses use: `tokio::main` needs a macros
    // feature this crate only carries for tests.
    tauri::async_runtime::block_on(score());
}

async fn score() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // Off here too, and not only in config. The point of switching the checking
    // off was to stop paying for it, and a harness that turns it back on every
    // run is still paying for it -- just somewhere the bill is easier to miss.
    let verify = args.iter().any(|a| a == "--verify");
    let only = args.into_iter().find(|a| !a.starts_with("--"));
    let cases = match load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    if cases.is_empty() {
        println!(
            "No cases yet. One file per question in truth/:\n\n    \
             ask = When will macOS 27 be released?\n    \
             must = 2025 | 2026\n    never = 2036 | 2037"
        );
        return;
    }

    // Leaked so the per-case tasks can be `'static`. This is a harness that
    // scores and exits; the alternative is threading an Arc through a closure
    // to defer a free that process exit does anyway.
    let cfg: &'static Config = Box::leak(Box::new(Config::load().unwrap_or_default()));
    let provider: &'static dyn provider::Provider = match provider::build(cfg) {
        Ok(p) => Box::leak(p),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    /// How many cases are in flight at once.
    ///
    /// Serially, forty-eight cases take twenty minutes, and a harness that takes
    /// twenty minutes is one nobody runs before committing -- which is the same
    /// as not having it. Six was too many: the run collected rate limits instead
    /// of answers from case twenty-five onwards. Three, plus the backoff below.
    const AT_ONCE: usize = 3;

    let todo: Vec<&Case> = cases
        .iter()
        .filter(|c| only.as_ref().is_none_or(|o| c.name.contains(o.as_str())))
        .collect();

    let (mut right, mut unsure, mut wrong, mut errored) = (0, 0, 0, 0);
    let mut lines: Vec<(String, &'static str, String, String)> = Vec::new();

    for batch in todo.chunks(AT_ONCE) {
        let mut running = Vec::new();
        for case in batch {
            let ask = case.ask.clone();
            let workspace = std::env::temp_dir();
            running.push(tauri::async_runtime::spawn(async move {
                // Rate limits are the common failure and they are temporary, so
                // they are worth waiting out rather than reporting. Backs off
                // rather than retrying immediately: a retry that arrives while
                // the limit still applies has spent a request to learn nothing.
                let mut last = String::new();
                for attempt in 0..3u32 {
                    if attempt > 0 {
                        let secs = 10 * u64::from(attempt);
                        eprintln!("      retrying in {secs}s: {last}");
                        tokio::time::sleep(std::time::Duration::from_secs(secs)).await;
                    }
                    match subagent::run(provider, &workspace, &ask, &[], "", false, verify, |step| {
                        perform(cfg, provider, step)
                    })
                    .await
                    {
                        Ok(f) => return Ok(f.answer),
                        Err(e) => last = e.to_string(),
                    }
                }
                Err(last)
            }));
        }
        for (case, task) in batch.iter().zip(running) {
            let answer = match task.await {
                Ok(Ok(a)) => a,
                Ok(Err(e)) => e,
                Err(e) => e.to_string(),
            };
            let verdict = match task_failed(&answer) {
                true => Verdict::Errored(answer.clone()),
                false => judge(case, &answer),
            };
            let (mark, why) = match verdict {
                Verdict::Right => {
                    right += 1;
                    ("ok", String::new())
                }
                Verdict::Unsure(w) => {
                    unsure += 1;
                    ("unsure", w)
                }
                Verdict::Wrong(w) => {
                    wrong += 1;
                    ("WRONG", w)
                }
                Verdict::Errored(w) => {
                    errored += 1;
                    ("error", w.chars().take(80).collect())
                }
            };
            lines.push((case.name.clone(), mark, why, answer));
        }
    }

    // Printed after the running, not during it: six cases interleaving their
    // searches produce a transcript in no particular order, and a scoreboard
    // that cannot be read down the page is not a scoreboard.
    println!();
    for (name, mark, why, answer) in &lines {
        // Worth seeing even on a pass. An answer that happened to be right
        // without anything being consulted is right the way a guess is right,
        // and a column of these means the agent is reciting, not looking.
        let how = match answer.starts_with(provider::RECALLED) {
            true => "  (from memory)",
            false => "",
        };
        println!("  {name:<34} {mark:<7} {why}{how}");
        if *mark != "ok" {
            let one = answer.replace('\n', " ");
            println!("      {}", one.chars().take(160).collect::<String>());
        }
    }

    let total = right + unsure + wrong;
    println!(
        "\n  {right}/{total} right, {unsure} said they did not know, {wrong} WRONG"
    );
    if errored > 0 {
        println!("  {errored} never ran, so they are not scored either way");
    }
    println!(
        "\n  Saying it does not know is not a failure -- it is the correct answer to a\n  \
         question it cannot answer, and worth more than a guess that reads the same\n  \
         as knowledge."
    );
    // A case that never ran is not a pass. It is also not a wrong answer, and
    // the exit code says which.
    if wrong > 0 || errored > 0 {
        std::process::exit(1);
    }
}
