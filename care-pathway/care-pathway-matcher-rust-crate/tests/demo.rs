//! Runs the demo binary, so a library change that breaks an example, or
//! silently changes what it illustrates, fails here rather than in front
//! of a reader.

use std::process::Command;

fn demo_output() -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_care-pathway-matcher"))
        .output()
        .expect("the demo binary runs");
    assert!(out.status.success(), "demo exited {:?}", out.status);
    String::from_utf8(out.stdout).expect("UTF-8 output")
}

/// The demo line starting with `label`.
fn line<'a>(output: &'a str, label: &str) -> &'a str {
    output
        .lines()
        .find(|l| l.starts_with(label))
        .unwrap_or_else(|| panic!("no line {label:?}"))
}

#[test]
fn examples_show_what_their_headings_claim() {
    let output = demo_output();
    assert!(
        line(&output, "name+condition").contains("is_match=true"),
        "name+condition should show is_match=true"
    );
    assert!(
        line(&output, "guideline id").contains("deterministic=true"),
        "guideline id should show deterministic=true"
    );
    assert!(
        line(&output, "pathway code").contains("deterministic=true"),
        "pathway code should show deterministic=true"
    );
    assert!(
        line(&output, "unrelated").contains("is_match=false"),
        "unrelated should show is_match=false"
    );
}
