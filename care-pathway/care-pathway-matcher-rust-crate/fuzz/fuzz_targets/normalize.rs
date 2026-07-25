//! SEC-I2 fuzz target: the pure `normalize` helpers over arbitrary UTF-8.
//!
//! These free functions run on caller-supplied pathway names and provider-scoped pathway codes — prime never-panic
//! targets. This feeds them arbitrary UTF-8 and asserts only that they
//! return (no panic, no overflow, no infinite loop within the fuzzer's
//! time budget). `fold_set` is exercised with the input as a single-element
//! slice.

#![no_main]

use care_pathway_matcher::normalize;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };

    let _ = normalize::fold(s);
    let _ = normalize::pathway_code(s);
    let _ = normalize::fold_set(&[s.to_string()]);
});
