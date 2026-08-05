//! SEC-I2 / SEC-B2 fuzz target: the bulk-import JSONL parsers.
//!
//! `bulk::jsonl` turns **attacker-supplied uploaded file bytes** into
//! `Person` records: `LineReader` frames the raw upload into rows (the
//! streaming path the import worker actually uses), `split_lines` does the
//! same over a whole buffer, and `parse_line` deserializes one JSON line.
//! All of them run on hostile input before any validation, so they must
//! **never panic** (no overflow, no unbounded work within the fuzzer's
//! budget) — they return a handled `Err` instead. This complements the
//! crate's `parse_line_never_panics` / `line_reader_never_panics_on_random_bytes`
//! proptests with coverage-guided input search.

#![no_main]

use libfuzzer_sys::fuzz_target;
use person_service::bulk::jsonl;

fuzz_target!(|data: &[u8]| {
    // The whole-buffer splitter, still used for encode-side round-trips.
    let _ = jsonl::split_lines(data);

    // The streaming reader — the import path's real framing. Driven on a
    // current-thread runtime built per input; the reader is `async` because
    // its source is, but it does no I/O beyond the slice handed to it.
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("build a current-thread runtime");
    rt.block_on(async {
        let mut reader = jsonl::LineReader::new(data, 1024);
        // Bounded so one input cannot fan out unboundedly.
        for _ in 0..64 {
            match reader.next_line().await {
                Some(Ok(line)) => {
                    let _ = jsonl::parse_line(&line);
                }
                Some(Err(_)) | None => break,
            }
        }
    });

    // `parse_line` also fed the whole blob, as one pathological line.
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = jsonl::parse_line(s);
    }
});
