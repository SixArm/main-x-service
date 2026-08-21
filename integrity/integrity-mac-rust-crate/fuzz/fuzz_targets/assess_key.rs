//! SEC-I2 fuzz target: operator-supplied key assessment.
//!
//! `assess_key_hex` is the pre-flight an operator runs before a key
//! reaches production, and the loader applies the same rule — so the two
//! agreeing is the property that keeps a "checked" key from being
//! refused at boot. It parses a hex string pasted by hand, which is
//! where the multi-byte-character panic lived.
//!
//! Pins never-panic, and that a **usable** verdict really does describe
//! a decodable key of at least the minimum length.

#![no_main]

use integrity_mac::{Assessment, KeyConfig, KeySet, MIN_KEY_LEN, assess_key_hex};
use libfuzzer_sys::fuzz_target;

const CONFIG: KeyConfig = KeyConfig::new("fuzz-service", "FUZZ");

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };

    let assessment = assess_key_hex(s);
    // Every verdict must render; an operator reading a bare enum name
    // has nothing to act on.
    assert!(!assessment.describe().is_empty());

    match assessment {
        Assessment::Usable { bytes } => {
            assert!(
                bytes >= MIN_KEY_LEN,
                "usable key reported {bytes} bytes, below the {MIN_KEY_LEN} floor"
            );
            // The checker and the loader must not disagree: a key blessed
            // here has to actually enable the key set.
            assert!(
                KeySet::load(&CONFIG, Some(s), None).is_enabled(),
                "assess_key_hex blessed a key the loader then refused: {s:?}"
            );
        }
        Assessment::TooShort { bytes } => assert!(bytes < MIN_KEY_LEN),
        Assessment::NotHex | Assessment::Placeholder { .. } => {
            assert!(
                !KeySet::load(&CONFIG, Some(s), None).is_enabled(),
                "the loader accepted a key assess_key_hex rejected: {s:?}"
            );
        }
    }
});
