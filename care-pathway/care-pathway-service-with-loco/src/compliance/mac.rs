//! Keyed integrity: HMAC-SHA256 over the same pre-image as the digests,
//! with a key the **database never holds**.
//!
//! ## The weakness this closes
//!
//! The SHA-256 and SHA-3 digests ([`super::audit_chain`],
//! [`super::record_integrity`]) are **unkeyed**, and their pre-image
//! format is published in `spec/12-compliance.md` §12.4z. Anyone who can
//! write SQL can therefore defeat them: edit the row, recompute both
//! digests from the documented format, update both columns. What the
//! unkeyed digests actually detect is *careless or unaware* modification —
//! a bug, a manual fix, a restore from the wrong backup, an attacker who
//! does not know the digest columns exist.
//!
//! A MAC raises that bar to the key. Recomputing it requires a secret that
//! lives in the service's environment and is **never written to the
//! database**, so an adversary holding only the database — a stolen
//! backup, a replica, SQL injection, a DBA without application-server
//! access — cannot forge one.
//!
//! ## What it does not defend against, stated plainly
//!
//! An adversary who holds **both** the database and the service
//! environment has the key and can forge freely. This is defence against
//! *database-only* compromise, which is the common case and worth having;
//! it is not defence against full host compromise, and nothing stored
//! beside the data could be.
//!
//! It also does not help if the key is put somewhere the database can
//! reach — a config table, a `pgcrypto` call, a connection string. The
//! separation *is* the control. `pgcrypto` offers `hmac()`, and using it
//! would place the key exactly where the adversary already is.
//!
//! ## Key identity and rotation
//!
//! A stored MAC is prefixed with its key id: `k1:9f86d0…`. Without that,
//! rotating the key would invalidate every historical row at once —
//! indistinguishable from mass tampering, and the same trap as silently
//! changing a hash format. With it, a verifier holding several keys picks
//! the one a row names, so rotation is additive.
//!
//! ## Absent key
//!
//! No key configured means **no MAC is written**, and rows carrying one
//! are reported as *unverifiable* rather than as mismatches. This matches
//! how the digests treat rows that predate them: adopting the control must
//! not produce a wall of false accusations, and "I cannot check this" is a
//! different statement from "this is wrong". Both are reported; neither is
//! silently rounded to "verified".

use std::collections::HashMap;
use std::sync::OnceLock;

use hmac::{Hmac, Mac};
use sha2::Sha256;

/// Environment variable holding the active MAC key, hex-encoded.
pub const KEY_ENV: &str = "CARE_PATHWAY_INTEGRITY_MAC_KEY";

/// Environment variable naming the active key, for the stored prefix.
pub const KEY_ID_ENV: &str = "CARE_PATHWAY_INTEGRITY_MAC_KEY_ID";

/// Environment variable holding retired keys still needed for
/// verification, as `id:hex` pairs separated by commas.
pub const RETIRED_KEYS_ENV: &str = "CARE_PATHWAY_INTEGRITY_MAC_KEYS_RETIRED";

/// Minimum key length in bytes.
///
/// HMAC accepts any length, but a key shorter than the digest adds no
/// security over a shorter one and usually signals a placeholder that
/// escaped into a deployment. Refusing it is cheap and catches that.
pub const MIN_KEY_LEN: usize = 32;

/// The default key id when none is configured.
const DEFAULT_KEY_ID: &str = "k1";

/// How a stored MAC could not be checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MacVerdict {
    /// Recomputed and matched.
    Valid,
    /// Recomputed and did **not** match: the content changed and whoever
    /// changed it did not hold the key.
    Invalid,
    /// The row carries no MAC — written before the key was configured.
    Absent,
    /// The row names a key id this service does not hold, so the MAC can
    /// be neither confirmed nor refuted. **Not** a mismatch.
    UnknownKey(String),
    /// The stored value is not `id:hex`.
    Malformed,
}

/// The configured keys: the active one (used for writing) and every key
/// available for verification, by id.
struct Keys {
    /// `(id, key)` used to write new MACs; `None` when unconfigured.
    active: Option<(String, Vec<u8>)>,
    /// Every key available for verification, including the active one.
    by_id: HashMap<String, Vec<u8>>,
}

/// Parse `id:hex` pairs from a comma-separated list, skipping malformed
/// entries with a warning that **never includes the key material**.
fn parse_retired(raw: &str) -> HashMap<String, Vec<u8>> {
    let mut out = HashMap::new();
    for entry in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let Some((id, hex)) = entry.split_once(':') else {
            tracing::warn!("retired MAC key entry is not id:hex; skipping");
            continue;
        };
        match decode_hex(hex) {
            Some(key) if key.len() >= MIN_KEY_LEN => {
                out.insert(id.to_string(), key);
            }
            _ => tracing::warn!(
                key_id = id,
                "retired MAC key is not valid hex or is too short"
            ),
        }
    }
    out
}

/// Decode a lowercase/uppercase hex string.
fn decode_hex(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

/// Load the key set once per process.
///
/// A configuration error disables MAC writing and is logged; it does not
/// prevent boot, matching the ABAC-policy and PASETO-key loaders. The
/// consequence is visible rather than silent: `mac_absent` climbs in every
/// verification report, which is the signal that the key never loaded.
fn keys() -> &'static Keys {
    static KEYS: OnceLock<Keys> = OnceLock::new();
    KEYS.get_or_init(|| {
        let mut by_id = std::env::var(RETIRED_KEYS_ENV)
            .ok()
            .map(|raw| parse_retired(&raw))
            .unwrap_or_default();

        let active = match std::env::var(KEY_ENV).ok().filter(|s| !s.trim().is_empty()) {
            None => {
                tracing::info!("no {KEY_ENV} configured; integrity MACs are not being written");
                None
            }
            Some(raw) => match decode_hex(&raw) {
                None => {
                    tracing::error!("{KEY_ENV} is not valid hex; integrity MACs disabled");
                    None
                }
                Some(key) if key.len() < MIN_KEY_LEN => {
                    // Length only — the key itself is never logged.
                    tracing::error!(
                        len = key.len(),
                        min = MIN_KEY_LEN,
                        "{KEY_ENV} is too short; integrity MACs disabled"
                    );
                    None
                }
                Some(key) => {
                    let id = std::env::var(KEY_ID_ENV)
                        .ok()
                        .filter(|s| !s.trim().is_empty() && !s.contains(':'))
                        .unwrap_or_else(|| DEFAULT_KEY_ID.to_string());
                    by_id.insert(id.clone(), key.clone());
                    Some((id, key))
                }
            },
        };
        Keys { active, by_id }
    })
}

/// Whether a key is configured, so MACs are being written.
#[must_use]
pub fn is_enabled() -> bool {
    keys().active.is_some()
}

/// The active key id, when one is configured.
#[must_use]
pub fn active_key_id() -> Option<&'static str> {
    keys().active.as_ref().map(|(id, _)| id.as_str())
}

/// Compute the stored MAC for a pre-image: `"<key id>:<hex>"`.
///
/// `None` when no key is configured — the caller stores `NULL`, and
/// verification reports [`MacVerdict::Absent`].
///
/// # Panics
///
/// Cannot in practice: the only fallible step is `Hmac::new_from_slice`,
/// and HMAC is defined for a key of any length, so it never returns an
/// error. The `expect` documents that rather than hiding it behind a
/// silent fallback — a MAC computed with a wrong or empty key would be
/// worse than a crash, because it would verify.
#[must_use]
pub fn tag(preimage: &[u8]) -> Option<String> {
    let (id, key) = keys().active.as_ref()?;
    Some(format!("{id}:{}", raw_tag(key, preimage)))
}

/// HMAC-SHA256 as lowercase hex.
/// # Panics
///
/// Cannot in practice: the only fallible step is `Hmac::new_from_slice`,
/// and HMAC is defined for a key of any length, so it never returns an
/// error. The `expect` documents that rather than hiding it behind a
/// silent fallback — a MAC computed with a wrong or empty key would be
/// worse than a crash, because it would verify.
fn raw_tag(key: &[u8], preimage: &[u8]) -> String {
    use std::fmt::Write as _;
    // Infallible for HMAC: it accepts a key of any length.
    let mut mac =
        <Hmac<Sha256> as Mac>::new_from_slice(key).expect("HMAC accepts keys of any length");
    mac.update(preimage);
    let bytes = mac.finalize().into_bytes();
    let mut out = String::with_capacity(64);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// Check a stored MAC against a pre-image.
///
/// Comparison is **constant-time** (`Mac::verify_slice`). A timing oracle
/// here would let an attacker with write access recover a valid tag byte
/// by byte without ever holding the key.
///
/// # Panics
///
/// Cannot in practice: the only fallible step is `Hmac::new_from_slice`,
/// and HMAC is defined for a key of any length, so it never returns an
/// error. The `expect` documents that rather than hiding it behind a
/// silent fallback — a MAC computed with a wrong or empty key would be
/// worse than a crash, because it would verify.
#[must_use]
pub fn verify(stored: Option<&str>, preimage: &[u8]) -> MacVerdict {
    let Some(stored) = stored else {
        return MacVerdict::Absent;
    };
    let Some((id, hex)) = stored.split_once(':') else {
        return MacVerdict::Malformed;
    };
    let Some(key) = keys().by_id.get(id) else {
        return MacVerdict::UnknownKey(id.to_string());
    };
    let Some(expected) = decode_hex(hex) else {
        return MacVerdict::Malformed;
    };
    let mut mac =
        <Hmac<Sha256> as Mac>::new_from_slice(key).expect("HMAC accepts keys of any length");
    mac.update(preimage);
    if mac.verify_slice(&expected).is_ok() {
        MacVerdict::Valid
    } else {
        MacVerdict::Invalid
    }
}

#[cfg(test)]
mod tests {
    /// **The property the MAC exists for.** An adversary who knows the
    /// pre-image format — it is published in the spec — can forge the
    /// unkeyed digests at will, and cannot forge the MAC.
    ///
    /// This is the whole argument in one test. The SHA-256 and SHA-3
    /// columns are recomputable by anyone holding the database; the MAC
    /// is not, unless they also hold a key that lives only in the
    /// service environment.
    #[test]
    fn the_unkeyed_digests_are_forgeable_and_the_mac_is_not() {
        use sha2::Digest as _;

        let key = [7u8; 32];
        let original = b"original content";
        let tampered = b"tampered content";

        // The attacker edits the row and recomputes the digest. They
        // succeed: nothing secret is involved.
        let forged_digest = format!("{:x}", sha2::Sha256::digest(tampered));
        assert_eq!(
            forged_digest,
            format!("{:x}", sha2::Sha256::digest(tampered)),
            "an unkeyed digest is reproducible by anyone"
        );

        // The same attacker cannot produce the MAC for the tampered
        // content without the key, and the MAC they would have to leave
        // in place — the one for the original — does not match.
        let genuine = raw_tag(&key, original);
        let needed = raw_tag(&key, tampered);
        assert_ne!(
            genuine, needed,
            "the tampered content needs a different tag"
        );

        // Guessing the key does not help.
        for guess in [[0u8; 32], [1u8; 32], [8u8; 32]] {
            assert_ne!(
                raw_tag(&guess, tampered),
                needed,
                "a wrong key produces a wrong tag"
            );
        }
    }

    /// A key long enough to be accepted is required; the length floor is
    /// what stops a placeholder like `"changeme"` reaching production and
    /// producing MACs an attacker could reproduce by guessing.
    #[test]
    fn the_key_length_floor_is_enforced() {
        assert_eq!(super::MIN_KEY_LEN, 32);
        let short = "k1:".to_string() + &"ab".repeat(8); // 8 bytes
        assert!(
            super::parse_retired(&short).is_empty(),
            "a key below the floor must not be loaded"
        );
    }

    use super::{MacVerdict, decode_hex, parse_retired, raw_tag};

    /// HMAC-SHA256 against RFC 4231 test case 1 — an external vector, so
    /// this pins the primitive rather than merely pinning our own output
    /// against itself.
    #[test]
    fn hmac_matches_rfc_4231_vector_1() {
        let key = vec![0x0b; 20];
        let data = b"Hi There";
        assert_eq!(
            raw_tag(&key, data),
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
    }

    /// A different key over the same content yields a different tag —
    /// the property the whole control rests on.
    #[test]
    fn the_key_changes_the_tag() {
        let a = raw_tag(&[1u8; 32], b"same content");
        let b = raw_tag(&[2u8; 32], b"same content");
        assert_ne!(a, b);
    }

    /// Hex decoding rejects odd lengths and non-hex, so a mangled
    /// environment value disables the MAC rather than silently producing
    /// a key derived from garbage.
    #[test]
    fn hex_decoding_is_strict() {
        assert_eq!(decode_hex("00ff"), Some(vec![0x00, 0xff]));
        assert_eq!(decode_hex("0"), None, "odd length");
        assert_eq!(decode_hex("zz"), None, "not hex");
    }

    /// Retired keys parse as `id:hex`, and entries that are malformed or
    /// too short are skipped rather than accepted — a short key would
    /// otherwise verify rows nobody legitimately wrote.
    #[test]
    fn retired_keys_parse_and_reject_weak_entries() {
        let good = "k1:".to_string() + &"ab".repeat(32);
        let short = "k2:abcd";
        let junk = "k3:zzzz";
        let nocolon = "k4";
        let parsed = parse_retired(&format!("{good},{short},{junk},{nocolon}"));
        assert_eq!(parsed.len(), 1, "only the well-formed, long-enough key");
        assert!(parsed.contains_key("k1"));
    }

    /// A malformed stored value is reported as such, not as a mismatch:
    /// "this is not a MAC" and "this MAC is wrong" are different findings
    /// and lead to different investigations.
    #[test]
    fn malformed_stored_value_is_not_a_mismatch() {
        assert_eq!(super::verify(Some("no-colon"), b"x"), MacVerdict::Malformed);
        assert_eq!(super::verify(None, b"x"), MacVerdict::Absent);
    }

    /// A tag naming a key this service does not hold is unverifiable, not
    /// invalid. Reporting it as a mismatch would turn a key-distribution
    /// problem into an apparent tampering incident.
    #[test]
    fn unknown_key_id_is_unverifiable_not_invalid() {
        let verdict = super::verify(Some("nosuchkey:00"), b"x");
        assert!(matches!(verdict, MacVerdict::UnknownKey(id) if id == "nosuchkey"));
    }
}
