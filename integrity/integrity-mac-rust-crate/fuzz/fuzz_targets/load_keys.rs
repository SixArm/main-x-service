//! SEC-I2 fuzz target: key-set loading from configuration strings.
//!
//! `KeySet::load` parses the inline root key and the retired-key list
//! (`id:hex,id:hex,…`) out of environment configuration. A malformed
//! value there must disable MAC writing and stay visible — never panic,
//! and never silently produce a key set that writes tags nothing can
//! verify.

#![no_main]

use integrity_mac::{KeyConfig, KeySet, MacVerdict};
use libfuzzer_sys::fuzz_target;

const CONFIG: KeyConfig = KeyConfig::new("fuzz-service", "FUZZ");
const DOMAIN: &str = "audit-chain";

fuzz_target!(|data: &[u8]| {
    // First NUL splits the inline key from the retired list.
    let (key_bytes, retired_bytes) = match data.iter().position(|&b| b == 0) {
        Some(i) => (&data[..i], &data[i + 1..]),
        None => (data, &[][..]),
    };
    let (Ok(key), Ok(retired)) = (
        std::str::from_utf8(key_bytes),
        std::str::from_utf8(retired_bytes),
    ) else {
        return;
    };

    let keys = KeySet::load(&CONFIG, Some(key), Some(retired));

    if keys.is_enabled() {
        // An enabled key set must be self-consistent: what it writes, it
        // reads back. A key set that tagged with one key and verified
        // with another would fail open on every row it produced.
        let tag = keys.tag(DOMAIN, b"pre-image").expect("enabled");
        assert_eq!(
            keys.verify(DOMAIN, Some(&tag), b"pre-image"),
            MacVerdict::Valid
        );
        let prefix = keys.active_prefix().expect("enabled");
        assert!(
            tag.starts_with(&prefix),
            "tag {tag:?} does not carry the advertised prefix {prefix:?}"
        );
        // The key id is what a reader looks up, so it must not contain
        // either separator or the lookup cannot round-trip.
        let id = keys.active_key_id().expect("enabled");
        assert!(!id.is_empty() && !id.contains(':') && !id.contains('.'));
    } else {
        assert!(keys.tag(DOMAIN, b"pre-image").is_none());
        assert!(keys.active_prefix().is_none());
    }
});
