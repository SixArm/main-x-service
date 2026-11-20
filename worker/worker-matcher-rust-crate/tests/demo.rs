//! Runs the demo binary, so a library change that breaks an example, or
//! silently changes what it illustrates, fails here rather than in front
//! of a reader.

use std::process::Command;

fn demo_output() -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_worker-matcher"))
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
        section(&output, "Example 1:").contains("Is Match: true"),
        "Example 1: should show Is Match: true"
    );
    assert!(
        section(&output, "Example 2:").contains("Is Match: true"),
        "Example 2: should show Is Match: true"
    );
    assert!(
        section(&output, "Example 2:").contains("Phonetic Match: Some(1.0)"),
        "Example 2: should show Phonetic Match: Some(1.0)"
    );
    assert!(
        section(&output, "Example 5:").contains("Is Match: false"),
        "Example 5: should show Is Match: false"
    );
    assert!(
        section(&output, "Example 6:").contains("Strict Mode Score: 0.91 (Match: false)"),
        "Example 6: should show Strict Mode Score: 0.91 (Match: false)"
    );
    assert!(
        section(&output, "Example 6:").contains("Lenient Mode Score: 0.91 (Match: true)"),
        "Example 6: should show Lenient Mode Score: 0.91 (Match: true)"
    );
}
