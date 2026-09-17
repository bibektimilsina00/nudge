//! Give the agent a real errand, with the real connectors, and watch.
//!
//! Every test so far called tools directly -- which proves a credential works
//! and proves nothing about the product. Nudge is not a tool caller; it is a
//! thing you give a sentence to. This is the smallest harness that exercises
//! the actual path: build a session, wire up whatever is connected, hand it a
//! sentence, and print every step it takes.
//!
//!     cargo run --bin errand -- "how many unread emails do I have?"
//!
//! Read-only errands only, by choice. The point is to find out whether the loop
//! holds together end to end, and a first run that also sends mail is a first
//! run that can be wrong in a way somebody else notices.
use nudge_lib::config::Config;
use nudge_lib::core::provider::Step;
use nudge_lib::core::run::session::Nudge;

fn main() {
    tauri::async_runtime::block_on(go());
}

async fn go() {
    let errand: String = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
    if errand.is_empty() {
        eprintln!("usage: errand \"what you want done\"");
        std::process::exit(2);
    }

    let cfg = Config::load().unwrap_or_default();
    println!(
        "provider: {} / {}",
        cfg.provider,
        cfg.model.clone().unwrap_or_else(|| "default".into())
    );

    let session = match Nudge::new(cfg) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("could not start a session: {e}");
            std::process::exit(1);
        }
    };

    session.connect_tools().await;
    let tools = session.tools();
    println!("tools available: {}", tools.len());
    let mut by_server: std::collections::BTreeMap<&str, usize> = Default::default();
    for t in &tools {
        *by_server.entry(t.server.as_str()).or_default() += 1;
    }
    for (server, n) in &by_server {
        println!("  {server}: {n}");
    }
    for (name, state) in session.tool_servers() {
        if let nudge_lib::core::run::session::ServerState::Failed(why) = state {
            println!("  {name}: DID NOT START -- {why}");
        }
    }
    println!("\nerrand: {errand}\n");

    let mut step_no = 0usize;
    let out = session
        .task(&errand, |step: Step| {
            step_no += 1;
            let session = &session;
            async move {
                match &step {
                    // `tool` is already `server/tool`; `run_tool` splits it.
                    Step::Mcp { tool, args, .. } => {
                        println!("  {step_no}. {tool} {args}");
                        let r = session.run_tool(tool, args).await;
                        match &r {
                            Ok(t) => println!("     -> {}", first(t, 160)),
                            Err(e) => println!("     -> FAILED {}", first(&e.to_string(), 160)),
                        }
                        r
                    }
                    Step::Reply { say } => {
                        println!("  {step_no}. says: {}", first(say, 300));
                        Ok(String::new())
                    }
                    other => {
                        println!("  {step_no}. {other:?}");
                        Ok(String::new())
                    }
                }
            }
        })
        .await;

    println!();
    match out {
        Ok(found) => println!("ANSWER: {}", found.answer),
        Err(e) => println!("FAILED: {e}"),
    }
}

fn first(s: &str, n: usize) -> String {
    let s = s.replace('\n', " ");
    match s.char_indices().nth(n) {
        Some((i, _)) => format!("{}…", &s[..i]),
        None => s,
    }
}
