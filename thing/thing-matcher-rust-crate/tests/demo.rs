//! Runs the demo binary, so a library change that breaks an example, or
//! silently changes what it illustrates, fails here rather than in front
//! of a reader.

use std::process::Command;

fn demo_output() -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_thing-matcher"))
        .output()
        .expect("the demo binary runs");
    assert!(out.status.success(), "demo exited {:?}", out.status);
    String::from_utf8(out.stdout).expect("UTF-8 output")
}

/// The text of the example headed `marker`, up to the next blank line.
fn section<'a>(output: &'a str, marker: &str) -> &'a str {
    let rest = output
        .split_once(marker)
        .unwrap_or_else(|| panic!("no example {marker:?}"))
        .1;
    rest.split("\n\n").next().unwrap_or(rest)
}

#[test]
fn examples_show_what_their_headings_claim() {
    let output = demo_output();
    assert!(
        section(&output, "Example 1:").contains("is_match: true"),
        "Example 1: should show is_match: true"
    );
    assert!(
        section(&output, "Example 3:").contains("Deterministic: true"),
        "Example 3: should show Deterministic: true"
    );
    assert!(
        section(&output, "Example 4:").contains("Deterministic: true"),
        "Example 4: should show Deterministic: true"
    );
    assert!(
        section(&output, "Example 5:").contains("is_match: false"),
        "Example 5: should show is_match: false"
    );
    assert!(
        section(&output, "Example 6:").contains("(match: false; strict mode"),
        "Example 6: should show (match: false; strict mode"
    );
    assert!(
        section(&output, "Example 6:").contains("Lenient: 1.00 (match: true)"),
        "Example 6: should show Lenient: 1.00 (match: true)"
    );
}
