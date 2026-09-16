//! Seeing a question, deciding it, and typing the answer -- against a real
//! process on a real terminal.
//!
//! The pieces are unit-tested apart: recognising a shape, deciding what may be
//! answered. This is the one that proves they fit together and that a program
//! on the other end actually acts on what was typed. It is an integration test
//! because there is no way to fake the part that matters -- a pipe does not
//! behave like a terminal, which is the whole reason any of this exists.
use nudge_lib::core::answering::{decide, Answer};
use nudge_lib::core::asked;
use nudge_lib::core::tools::running::Running;

#[test]
fn a_prompt_is_seen_answered_and_acted_on() {
    let r = Running::default();
    // Prints a prompt, waits, then says what it was told.
    let id = r
        .watch(
            &std::env::temp_dir(),
            "node -e \"process.stdout.write('Allow this command? [y/N] '),process.stdin.once('data',function(d){console.log('got:'+d.toString().trim())})\"",
        )
        .unwrap();

    // Wait for it to settle, the way the supervisor does.
    let mut text = String::new();
    for _ in 0..40 {
        std::thread::sleep(std::time::Duration::from_millis(100));
        if let Some((t, _)) = r.peek(id) {
            if asked::waiting(&t).is_some() {
                text = t;
                break;
            }
        }
    }

    let question = asked::waiting(&text).expect("never saw a question");
    println!("saw: {:?}  shape {:?}", question.line, question.shape);

    match decide(&question, Some(true)) {
        Answer::Say { text, why } => {
            println!("decided: say {text:?} -- {why}");
            r.answer(id, &text).unwrap();
        }
        // A plain permission prompt the reviewer agreed with is exactly the
        // case this is supposed to handle without disturbing anybody.
        Answer::Ask => panic!("a plain yes/no should not have needed a person"),
    }

    let out = r
        .settle(id, std::time::Duration::from_secs(10), || false)
        .unwrap();
    println!("after: {:?}", out.fresh.trim());
    assert!(out.fresh.contains("got:y"), "{:?}", out.fresh);
}
