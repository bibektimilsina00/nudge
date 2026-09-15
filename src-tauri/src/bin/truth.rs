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
                _ => {}
            }
        }
        cases.push(Case {
            name: path.file_stem().unwrap_or_default().to_string_lossy().into(),
            ask,
            must,
            never,
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
}

fn judge(case: &Case, answer: &str) -> Verdict {
    let said = answer.to_lowercase();

    // Checked first. A wrong fact is wrong even when it is hedged -- "it has not
    // been announced, but it would likely be 2036" is the failure, stated twice.
    if let Some(bad) = case.never.iter().find(|n| said.contains(n.as_str())) {
        return Verdict::Wrong(format!("said {bad:?}"));
    }
    if case.must.iter().any(|m| said.contains(m.as_str())) {
        return Verdict::Right;
    }
    if nudge_lib::core::run::subagent::hedged(&said) {
        return Verdict::Unsure("said it did not know".into());
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
    let only = std::env::args().nth(1);
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

    let cfg = Config::load().unwrap_or_default();
    let provider = match provider::build(&cfg) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    let workspace = std::env::temp_dir();

    let (mut right, mut unsure, mut wrong) = (0, 0, 0);
    for case in &cases {
        if only.as_ref().is_some_and(|o| !case.name.contains(o.as_str())) {
            continue;
        }
        let found = subagent::run(provider.as_ref(), &workspace, &case.ask, |step| {
            perform(&cfg, provider.as_ref(), step)
        })
        .await;

        let answer = match found {
            Ok(f) => f.answer,
            Err(e) => format!("(failed: {e})"),
        };
        let verdict = judge(case, &answer);
        let (mark, why) = match &verdict {
            Verdict::Right => {
                right += 1;
                ("ok", String::new())
            }
            Verdict::Unsure(w) => {
                unsure += 1;
                ("unsure", w.clone())
            }
            Verdict::Wrong(w) => {
                wrong += 1;
                ("WRONG", w.clone())
            }
        };
        println!("  {:<34} {mark:<7} {why}", case.name);
        println!("      {}", answer.replace('\n', " ").chars().take(150).collect::<String>());
    }

    let total = right + unsure + wrong;
    println!(
        "\n  {right}/{total} right, {unsure} said they did not know, {wrong} WRONG\n\n  \
         Saying it does not know is not a failure -- it is the correct answer to a\n  \
         question it cannot answer, and worth more than a guess that reads the same\n  \
         as knowledge. Only WRONG exits non-zero."
    );
    if wrong > 0 {
        std::process::exit(1);
    }
}
