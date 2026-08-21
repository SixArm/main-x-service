//! FUZZ-2 target: the bulk **row decoders** — delimited text and JSONL.
//!
//! A bulk import is the one path that takes a whole *file* from a caller.
//! These decoders run on those bytes before any row is validated or
//! stored, so they are the outermost parser in the service and the
//! natural fuzz target.
//!
//! Both delimiters are driven from one input, because CSV and TSV share
//! a codec and differ only in the byte: fuzzing one and not the other
//! would leave the shared framing, quoting, and header-resolution logic
//! covered exactly once while claiming two formats.
//!
//! Invariants:
//!
//! - **never-panic** on arbitrary bytes, for either delimiter and for
//!   JSONL;
//! - **the per-row error contract holds** — a decode either fails as a
//!   whole (unreadable framing) or yields one result per row, each of
//!   which may independently be an error. A malformed row must never
//!   abort the load, because §7 promises the good rows still commit;
//! - **deterministic** — the same bytes decode to the same row count.

#![no_main]

use case_service::bulk::{csv, jsonl};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    for delimiter in [b',', b'\t'] {
        match csv::decode(data, delimiter) {
            // A readable file: one slot per row, each independently an
            // `Ok` row or a per-row parse error.
            Ok(rows) => {
                let again = csv::decode(data, delimiter).expect("decode is deterministic");
                assert_eq!(
                    rows.len(),
                    again.len(),
                    "row count changed between identical decodes"
                );
            }
            // Structurally unreadable (bad framing): a whole-file error is
            // the documented outcome, not a panic.
            Err(_) => {}
        }
    }

    // JSONL: split then parse per line, the same order the import worker
    // uses. A line that is not a record is a per-row error, never fatal.
    if let Ok(lines) = jsonl::split_lines(data) {
        for line in &lines {
            let _ = jsonl::parse_line(line);
        }
    }
});
