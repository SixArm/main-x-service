//! SEC-I2 fuzz target: the serde path into `EntityRef`.
//!
//! `EntityRef` is `#[serde(try_from = "String")]`, so every link request
//! body and every row read out of a `TEXT` column goes through this,
//! not through `FromStr` directly. Fuzzing the JSON entry point covers
//! the escape decoding and the `TryFrom` bridge as well as the parser —
//! and asserts that anything which deserializes **re-serializes to the
//! same JSON**, the property a stored-then-reloaded ref depends on.

#![no_main]

use entity_ref::EntityRef;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };

    let Ok(parsed) = serde_json::from_str::<EntityRef>(s) else {
        return;
    };

    let json = serde_json::to_string(&parsed).expect("EntityRef always serializes");
    let again: EntityRef = serde_json::from_str(&json).expect("our own JSON must deserialize");
    assert_eq!(parsed, again, "serde round-trip changed the ref: {json}");
});
