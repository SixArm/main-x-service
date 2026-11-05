//! FUZZ-2 target: the validator itself, driven directly.
//!
//! `validate_json` covers the real request path but pays a JSON tax: most
//! random bytes are not a `Case`, so the deep fields are reached only
//! once libFuzzer has learned the grammar. This target builds the `Case`
//! from the raw bytes instead, giving the fuzzer direct control over
//! **array cardinality and entry contents** — which is where the
//! input-size caps live.
//!
//! The NUL split matters more than it looks: a run of NUL bytes yields a
//! run of empty chunks, so the fuzzer can reach "ten thousand blank
//! entries" — the exact SEC-M8 shape — for the cost of ten thousand zero
//! bytes, without having to discover any structure.
//!
//! Invariants: never-panic, deterministic, and a **bounded report** whose
//! ceiling does not grow with the input.

#![no_main]

use case_matcher::{Case, CaseIdentifier, IdentifierScheme};
use case_service::validation;
use libfuzzer_sys::fuzz_target;

/// Ceiling on the problem count — see `validate_json` for the derivation.
const MAX_PROBLEMS: usize = 4096;

fuzz_target!(|data: &[u8]| {
    let mut chunks = data.split(|b| *b == 0).map(String::from_utf8_lossy);

    let mut case = Case::new(chunks.next().unwrap_or_default().into_owned());

    // Round-robin the remaining chunks across every scored collection, so
    // one input exercises all of them and their cardinalities move
    // together.
    for (i, chunk) in chunks.enumerate() {
        let s = chunk.into_owned();
        match i % 6 {
            0 => case.subjects.push(s),
            1 => case.keywords.push(s),
            2 => case.alternate_titles.push(s),
            3 => case.same_as.push(s),
            4 => case.in_language.push(s),
            _ => case.identifiers.push(CaseIdentifier {
                scheme: IdentifierScheme::Docket,
                value: s,
            }),
        }
    }

    let problems = validation::problems(&case);
    assert!(
        problems.len() <= MAX_PROBLEMS,
        "problem report is unbounded: {} problems from {} bytes",
        problems.len(),
        data.len()
    );
    assert_eq!(
        problems,
        validation::problems(&case),
        "validation is not deterministic"
    );
});
