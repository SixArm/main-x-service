//! FUZZ-2 target: the real request path into payload validation.
//!
//! `POST /api/cases` deserializes an attacker-supplied body into
//! `case_matcher::Case` and runs `validation::problems` over it, so this
//! target drives exactly that: arbitrary bytes → `serde_json` → the
//! validator. Coverage-guided search learns the JSON shape, which is
//! what makes the deep fields reachable at all.
//!
//! Invariants:
//!
//! - **never-panic** — the validator runs before anything is stored, on
//!   input nobody here controls;
//! - **deterministic** — the same payload validates to the same problems,
//!   so a `422` body cannot depend on allocation order or iteration
//!   chance;
//! - **bounded report** (SEC-M8) — the number of problems has a ceiling
//!   that does not grow with the size of the payload. Without it, ten
//!   thousand blank entries produced ten thousand problem strings, and a
//!   small request bought a large response.

#![no_main]

use case_service::validation;
use libfuzzer_sys::fuzz_target;

/// Ceiling on the problem count, not a tight bound.
///
/// Derivation: six array fields, each inspecting at most `MAX_ARRAY_LEN`
/// (256) entries and able to emit at most two problems per entry (blank
/// plus over-length), then a dozen or so cardinality and scalar checks.
/// The constants are private to `validation`, so this is stated rather
/// than computed — deliberately loose, because what it must catch is the
/// report growing *with the input*, not an off-by-one.
const MAX_PROBLEMS: usize = 4096;

fuzz_target!(|data: &[u8]| {
    let Ok(case) = serde_json::from_slice::<case_matcher::Case>(data) else {
        // A body that is not a `Case` is rejected by the extractor long
        // before validation; reaching here without aborting is the point.
        return;
    };

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
