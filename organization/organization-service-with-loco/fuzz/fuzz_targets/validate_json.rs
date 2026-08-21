//! FUZZ-2 target: the real request path into payload validation.
//!
//! `POST /api/organizations` deserializes an attacker-supplied body into
//! `organization_matcher::Organization` and runs `validation::problems` over it, so this
//! target drives exactly that: arbitrary bytes → `serde_json` → the
//! validator. Coverage-guided search learns the JSON shape, which is what
//! makes the deep fields reachable at all.
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

use libfuzzer_sys::fuzz_target;
use organization_service::validation;

/// Ceiling on the problem count, not a tight bound.
///
/// Derivation: each array field inspects at most `MAX_ARRAY_LEN` (256)
/// entries and can emit at most a couple of problems per entry, then a
/// dozen or so cardinality and scalar checks. Deliberately loose: what it
/// must catch is the report growing *with the input*, not an off-by-one.
const MAX_PROBLEMS: usize = 4096;

fuzz_target!(|data: &[u8]| {
    let Ok(record) = serde_json::from_slice::<organization_matcher::Organization>(data) else {
        // A body that is not a `Organization` is rejected by the extractor long
        // before validation; reaching here without aborting is the point.
        return;
    };

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
