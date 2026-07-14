//! SEC-I2 fuzz target: the pure string-similarity scorers.
//!
//! Splits the fuzz bytes into two UTF-8 strings and runs the
//! Jaro-Winkler / Levenshtein / combined similarity scorers, pinning the
//! never-panic invariant and that every similarity is **finite and within
//! `[0.0, 1.0]`** on arbitrary input (including the empty-string and
//! multi-byte edge cases the O(n·m) algorithms must handle).

#![no_main]

use libfuzzer_sys::fuzz_target;
use event_matcher::Scorer;

fuzz_target!(|data: &[u8]| {
    // Split at the first NUL so a single blob yields two independent
    // strings; fall back to (whole, "") when there is no separator.
    let (a_bytes, b_bytes) = match data.iter().position(|&b| b == 0) {
        Some(i) => (&data[..i], &data[i + 1..]),
        None => (data, &[][..]),
    };
    let (Ok(a), Ok(b)) = (std::str::from_utf8(a_bytes), std::str::from_utf8(b_bytes)) else {
        return;
    };

    for score in [
        Scorer::jaro_winkler_similarity(a, b),
        Scorer::levenshtein_similarity(a, b),
        Scorer::exact_match(a, b),
        Scorer::combined_similarity(a, b),
    ] {
        assert!(score.is_finite(), "similarity must be finite, got {score}");
        assert!(
            (0.0..=1.0).contains(&score),
            "similarity must be in [0,1], got {score}"
        );
    }
});
