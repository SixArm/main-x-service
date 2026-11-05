//! FUZZ-2 target: the validator itself, driven directly.
//!
//! `validate_json` covers the real request path but pays a JSON tax: most
//! random bytes are not a `Organization`, so the deep fields are reached only
//! once libFuzzer has learned the grammar. This target builds the `Organization`
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

use libfuzzer_sys::fuzz_target;
use organization_matcher::{IdentifierScheme, OrgIdentifier, Organization};
use organization_service::validation;

/// Ceiling on the problem count — see `validate_json` for the derivation.
const MAX_PROBLEMS: usize = 4096;

fuzz_target!(|data: &[u8]| {
    let mut chunks = data.split(|b| *b == 0).map(String::from_utf8_lossy);

    let mut record = Organization::new(chunks.next().unwrap_or_default().into_owned());

    // Round-robin the remaining chunks across every scored collection, so
    // one input exercises all of them and their cardinalities move
    // together.
    for (i, chunk) in chunks.enumerate() {
        let s = chunk.into_owned();
        match i % 4 {
            0 => record.alternate_names.push(s),
            1 => record.keywords.push(s),
            2 => record.same_as.push(s),
            _ => record.identifiers.push(OrgIdentifier {
                scheme: IdentifierScheme::Wikidata,
                value: s,
            }),
        }
    }

    let problems = validation::problems(&record);
    assert!(
        problems.len() <= MAX_PROBLEMS,
        "problem report is unbounded: {} problems from {} bytes",
        problems.len(),
        data.len()
    );
    assert_eq!(
        problems,
        validation::problems(&record),
        "validation is not deterministic"
    );
});
