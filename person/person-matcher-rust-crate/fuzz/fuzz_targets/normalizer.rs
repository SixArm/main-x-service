//! SEC-I2 fuzz target: the pure string normalizers over arbitrary UTF-8.
//!
//! The `Normalizer` helpers run on caller-supplied names, postcodes,
//! phones, addresses, and emails — prime never-panic targets. This feeds
//! them arbitrary UTF-8 and asserts only that they return (no panic, no
//! overflow, no infinite loop within the fuzzer's time budget).

#![no_main]

use libfuzzer_sys::fuzz_target;
use person_matcher::Normalizer;

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };

    // Each of these is a pure `&str -> String`/`Option<String>` transform;
    // the assertion is implicit — the process must not abort.
    let _ = Normalizer::normalize_name(s);
    let _ = Normalizer::normalize_postcode(s);
    let _ = Normalizer::normalize_phone(s);
    let _ = Normalizer::normalize_phone_e164(s, Some("GB"));
    let _ = Normalizer::expand_street_abbreviations(s);
    let _ = Normalizer::normalize_address_line(s);
    let _ = Normalizer::parse_address_line(s);
    let _ = Normalizer::phonetic_code(s);
    let _ = Normalizer::normalize_email(s, true);
});
