//! FUZZ-2 target: the pure record-merge fold.
//!
//! `POST /api/organizations/merge` folds a duplicate into a survivor. The inputs are
//! two stored payloads, and "stored" is not the same as "trusted" — rows
//! predate today's validators, and bulk import writes them too. The fold
//! is pure and total, so it fuzzes without a database.
//!
//! Invariants, each one a property the merge contract states:
//!
//! - **never-panic** on any pair;
//! - **the survivor keeps its name** — merge must never rename a
//!   record;
//! - **deterministic** — the same pair merges to the same survivor;
//! - **absorbing** — re-merging the same duplicate into the result adds
//!   nothing. The lists are unions, so a second merge that kept growing
//!   them would mean the union is not deduplicating, and a merge retried
//!   after a failed transaction would inflate the record every time.

#![no_main]

use libfuzzer_sys::fuzz_target;
use organization_matcher::Organization;
use organization_service::merge;

/// Build a `Organization` from a byte slice, round-robining NUL-separated chunks
/// across the collections the fold unions.
fn build(data: &[u8]) -> Organization {
    let mut chunks = data.split(|b| *b == 0).map(String::from_utf8_lossy);
    let mut record = Organization::new(chunks.next().unwrap_or_default().into_owned());
    for (i, chunk) in chunks.enumerate() {
        let s = chunk.into_owned();
        match i % 3 {
            0 => record.alternate_names.push(s),
            1 => record.keywords.push(s),
            _ => record.same_as.push(s),
        }
    }
    record
}

fuzz_target!(|data: &[u8]| {
    // Split the input so both sides vary independently.
    let mid = data.len() / 2;
    let main = build(&data[..mid]);
    let duplicate = build(&data[mid..]);

    let first = merge::merge_orgs(&main, &duplicate);
    assert_eq!(first.merged.name, main.name, "merge renamed the survivor");
    assert_eq!(
        merge::merge_orgs(&main, &duplicate).merged.name,
        first.merged.name,
        "merge is not deterministic"
    );

    let second = merge::merge_orgs(&first.merged, &duplicate);
    for (field, a, b) in [
        (
            "alternate_names",
            first.merged.alternate_names.len(),
            second.merged.alternate_names.len(),
        ),
        (
            "keywords",
            first.merged.keywords.len(),
            second.merged.keywords.len(),
        ),
        (
            "same_as",
            first.merged.same_as.len(),
            second.merged.same_as.len(),
        ),
    ] {
        assert_eq!(
            a, b,
            "re-merging grew {field} from {a} to {b}: the union is not absorbing"
        );
    }
});
