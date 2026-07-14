//! SEC-I2 / SEC-B2 fuzz target: the bulk-import JSONL parsers.
//!
//! `bulk::jsonl` turns **attacker-supplied uploaded file bytes** into
//! `Person` records: `split_lines` / `split_lines_capped` slice the raw
//! upload into lines, and `parse_line` deserializes one JSON line. All three
//! run on hostile input before any validation, so they must **never panic**
//! (no overflow, no unbounded work within the fuzzer's budget) — they return
//! a handled `Err` instead. This complements the crate's `parse_line_never_panics`
//! proptest with coverage-guided input search.

#![no_main]

use libfuzzer_sys::fuzz_target;
use person_service::bulk::jsonl;

fuzz_target!(|data: &[u8]| {
    // The line splitters operate on the raw upload bytes.
    let _ = jsonl::split_lines(data);
    let _ = jsonl::split_lines_capped(data, 1024);

    // `parse_line` operates on a single UTF-8 line. Feed the whole blob, and
    // also each split line, mirroring the import pipeline (bounded so one
    // input can't fan out unboundedly).
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = jsonl::parse_line(s);
        if let Ok(lines) = jsonl::split_lines(data) {
            for line in lines.iter().take(64) {
                let _ = jsonl::parse_line(line);
            }
        }
    }
});
