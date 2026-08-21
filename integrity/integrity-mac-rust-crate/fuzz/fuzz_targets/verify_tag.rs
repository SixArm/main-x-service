//! SEC-I2 fuzz target: stored-MAC verification.
//!
//! This is the crate's one genuinely adversarial input. The MAC is the
//! only stored integrity value an attacker holding just the database
//! cannot forge — but they *can* rewrite the stored string to anything,
//! so `KeySet::verify` parses `[scheme.]id:hex` from bytes chosen by
//! whoever edited the row. (This target's first find was exactly that:
//! `decode_hex` sliced the `&str` by byte index, so a stored value
//! containing a multi-byte character aborted the process.)
//!
//! Invariants pinned here:
//!
//! - **never-panic** on an arbitrary stored value and pre-image;
//! - **tag/verify agree** — the value we would have written always
//!   verifies, or every honest row reads as tampered with;
//! - **domain separation** — that same tag never verifies under a
//!   different domain, which is the whole point of the HKDF `info`;
//! - **no accidental acceptance** — a stored value differing from the
//!   genuine tag in its hex digits does not verify.

#![no_main]

use integrity_mac::{KeyConfig, KeySet, MacVerdict};
use libfuzzer_sys::fuzz_target;

/// A fixed, obviously-fake-but-valid 32-byte key: enough distinct bytes
/// to pass the placeholder rule, so the `KeySet` is actually enabled.
const KEY_HEX: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";

const CONFIG: KeyConfig = KeyConfig::new("fuzz-service", "FUZZ");
const DOMAIN: &str = "audit-chain";
const OTHER_DOMAIN: &str = "record";

fuzz_target!(|data: &[u8]| {
    // Split at the first NUL: the head is the attacker-controlled stored
    // value, the tail the pre-image it claims to cover.
    let (stored_bytes, preimage) = match data.iter().position(|&b| b == 0) {
        Some(i) => (&data[..i], &data[i + 1..]),
        None => (data, &[][..]),
    };
    let Ok(stored) = std::str::from_utf8(stored_bytes) else {
        return;
    };

    let keys = KeySet::load(&CONFIG, Some(KEY_HEX), None);
    assert!(keys.is_enabled(), "the fixed fuzz key must load");

    // The invariant that matters is that this returns at all.
    let verdict = keys.verify(DOMAIN, Some(stored), preimage);

    // The genuine tag for this pre-image, computed the way a write would.
    let genuine = keys.tag(DOMAIN, preimage).expect("enabled keys always tag");
    assert_eq!(
        keys.verify(DOMAIN, Some(&genuine), preimage),
        MacVerdict::Valid,
        "our own tag must verify"
    );
    assert_ne!(
        keys.verify(OTHER_DOMAIN, Some(&genuine), preimage),
        MacVerdict::Valid,
        "a tag must not transfer between domains"
    );

    // An arbitrary stored value may legitimately verify only by *being*
    // the genuine tag: same key id, same scheme, same digits. Hex case is
    // not significant to the decoder, and a scheme-less value is verified
    // against the legacy raw key, so both are excluded from the check
    // rather than asserted about.
    if verdict == MacVerdict::Valid && stored.trim().starts_with("d1.") {
        assert!(
            stored.trim().eq_ignore_ascii_case(genuine.trim()),
            "verify accepted a derived-scheme value it did not compute: {stored:?}"
        );
    }

    // `None` is the no-MAC case, distinct from a mismatch.
    assert_eq!(keys.verify(DOMAIN, None, preimage), MacVerdict::Absent);
});
