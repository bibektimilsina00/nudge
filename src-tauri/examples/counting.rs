//! Does a count from the real client reach the real endpoint?
//!
//! The two halves were written and tested separately, which proves each is
//! self-consistent and nothing about whether they agree on the wire. Run the
//! server locally and point this at it:
//!
//! ```text
//! cd server && uv run uvicorn app.main:app --port 8099
//! NUDGE_API=http://127.0.0.1:8099 cargo run --example counting
//! cd server && uv run counts.py
//! ```
fn main() {
    let rt = tokio::runtime::Runtime::new().expect("a runtime");
    rt.block_on(async {
        println!("sending to {}", nudge_lib::core::account::api());
        nudge_lib::core::counted::send(
            nudge_lib::core::counted::Count::of("turn")
                .taking(3.25)
                .shaped("gemini"),
        );
        nudge_lib::core::counted::send(
            nudge_lib::core::counted::Count::of("agent")
                .taking(41.0)
                .shaped("done in 7"),
        );
        nudge_lib::core::counted::send(nudge_lib::core::counted::Count::of("install"));
        // Fire and forget means there is nothing to await; give the spawned
        // sends a moment to land before the runtime is dropped out from under.
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    });
    println!("sent -- now run: cd server && uv run counts.py");
}
