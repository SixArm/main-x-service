//! Runs the demo binary, so a library change that breaks or silently
//! changes what an example illustrates fails here rather than in front of
//! a reader.

use std::process::Command;

fn demo_output() -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_course-matcher"))
        .output()
        .expect("the demo binary runs");
    assert!(out.status.success(), "demo exited {:?}", out.status);
    String::from_utf8(out.stdout).expect("UTF-8 output")
}

/// The line after `heading` that starts with `label`.
fn after<'a>(output: &'a str, heading: &str, label: &str) -> &'a str {
    output
        .split_once(heading)
        .unwrap_or_else(|| panic!("no section {heading:?}"))
        .1
        .lines()
        .skip(1) // the rest of the heading line itself
        .find(|l| l.trim_start().starts_with(label))
        .unwrap_or_else(|| panic!("no {label:?} under {heading:?}"))
}

#[test]
fn every_example_runs() {
    let output = demo_output();
    for n in 1..=8 {
        assert!(
            output.contains(&format!("Example {n}:")),
            "example {n} missing"
        );
    }
}

#[test]
fn examples_show_what_their_headings_claim() {
    let output = demo_output();
    assert_eq!(
        after(&output, "Example 1:", "Deterministic"),
        "Deterministic: true"
    );
    assert_eq!(
        after(&output, "Example 2:", "Deterministic"),
        "Deterministic: true"
    );
    assert_eq!(after(&output, "Example 3:", "Is Match"), "Is Match: true");
    assert!(after(&output, "Example 4:", "Name Score").contains("Soundex bonus applied"));
    assert_eq!(after(&output, "Example 6:", "Is Match"), "Is Match: false");
    // Strict and lenient must disagree on the same pair, or the example
    // illustrates nothing.
    assert!(after(&output, "Example 7:", "Strict").ends_with("(Match: false)"));
    assert!(after(&output, "Example 7:", "Lenient").ends_with("(Match: true)"));
    // The exact-code candidate ranks first.
    assert!(after(&output, "Example 8:", "candidate").contains("CS101"));
}
