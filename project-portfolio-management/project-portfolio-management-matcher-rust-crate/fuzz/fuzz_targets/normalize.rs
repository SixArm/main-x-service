//! SEC-I2 fuzz target: the pure `normalize` helpers over arbitrary UTF-8.
//!
//! These free functions run on caller-supplied plan names, owner-scoped codes, URLs, and ISO dates — prime never-panic
//! targets. This feeds them arbitrary UTF-8 and asserts only that they
//! return (no panic, no overflow, no infinite loop within the fuzzer's
//! time budget). `fold_set` is exercised with the input as a single-element
//! slice.

#![no_main]

use libfuzzer_sys::fuzz_target;
use project_portfolio_management_matcher::normalize;

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };

    let _ = normalize::fold(s);
    let _ = normalize::code(s);
    let _ = normalize::url(s);
    let _ = normalize::fold_set(&[s.to_string()]);
    let _ = normalize::iso_date_to_days(s);
});
