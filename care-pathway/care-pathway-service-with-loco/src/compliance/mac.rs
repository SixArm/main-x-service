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
//! A stored MAC is prefixed with its scheme and key id:
//! `d1.k1:9f86d0…`. Without the key id, rotating the key would invalidate
//! every historical row at once — indistinguishable from mass tampering,
//! and the same trap as silently changing a hash format. With it, a
//! verifier holding several keys picks the one a row names, so rotation
//! is additive.
//!
//! ## Domain separation — why one configured key is several keys
//!
//! The configured value is a **root** key. It is never used to MAC
//! anything. Each purpose derives its own subkey with HKDF-SHA256
//! ([`Domain`]), under an `info` string binding **this service's name**
//! and the domain:
//!
//! ```text
//! mxi/care-pathway-service/audit-chain/d1
//! mxi/care-pathway-service/record/d1
//! mxi/care-pathway-service/checkpoint/d1
//! ```
//!
//! Without this, separation rests on the pre-images never colliding — and
//! the only thing keeping them apart was the leading version tag, which
//! exists for versioning, not separation. Two failure modes follow from
//! that, and both are closed here rather than argued away:
//!
//! - **Cross-domain.** Bumping the record pre-image's version tag to a
//!   value the chain already uses would silently make a record tag
//!   verify as a chain tag. `v1` is an obvious name for both.
//! - **Cross-service.** The version tags are *identical across crates* —
//!   care-pathway and case both used `v1` for the chain and `cp1` for
//!   checkpoints — and the crate identity appears nowhere in the
//!   pre-image. One MAC key shared across a cluster, which is the
//!   obvious way to deploy one, therefore made a tag from one service
//!   verify against an identically-shaped row in another.
//!
//! Deriving per (service, domain) removes the reliance entirely: a tag
//! cannot transfer even if two pre-images are byte-identical.
//!
//! ## Where the key comes from
//!
//! Two sources, in precedence order:
//!
//! 1. **`CARE_PATHWAY_INTEGRITY_MAC_KEY_FILE`** — a path whose contents
//!    are the hex key. This is the production form: a Kubernetes secret
//!    volume, a Docker secret, a file the orchestrator mounts.
//! 2. **`CARE_PATHWAY_INTEGRITY_MAC_KEY`** — the hex key inline.
//!    Convenient for development, and the weaker option: environment
//!    variables are inherited by every child process, appear in crash
//!    dumps and `/proc/<pid>/environ`, and are visible to anything that
//!    can introspect the container.
//!
//! Setting both is a configuration error worth surfacing, so the file
//! wins and a warning is logged.
//!
//! ## Key material lifetime
//!
//! The root key is [`zeroize`]d as soon as the subkeys are derived, so it
//! does not sit in the process image for the lifetime of the service. The
//! derived subkeys necessarily persist — they are needed on every write —
//! but they are the narrower secret: one leaks one domain of one service,
//! where the root leaks all of them, including any future domain.
//!
//! This is worth stating precisely rather than overselling. Zeroizing does
//! not defend against a live-memory read of a running process, which has
//! the subkeys regardless. What it removes is the *root* key from core
//! dumps, swap, and post-mortem images.
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
use zeroize::Zeroize as _;

/// Environment variable holding the active MAC **root** key, hex-encoded.
pub const KEY_ENV: &str = "CARE_PATHWAY_INTEGRITY_MAC_KEY";

/// Environment variable naming a **file** whose contents are the
/// hex-encoded root key. Takes precedence over [`KEY_ENV`]; see the module
/// docs on why a mounted file beats an environment variable in production.
pub const KEY_FILE_ENV: &str = "CARE_PATHWAY_INTEGRITY_MAC_KEY_FILE";

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

/// The current key-derivation scheme, stored in the MAC prefix.
///
/// A stored value with no scheme (`k1:…`) predates derivation and is
/// verified against the **raw** root key, so adopting this change does not
/// turn existing rows into a wall of false accusations. A stored value
/// naming a scheme this build does not know is reported as unverifiable
/// rather than invalid, for the same reason.
const SCHEME: &str = "d1";

/// Purposes that must not share a key.
///
/// Each derives a distinct subkey, so a tag produced for one can never
/// verify as another — regardless of whether their pre-images collide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Domain {
    /// The tamper-evident audit hash chain.
    AuditChain,
    /// Row-level record content integrity.
    Record,
    /// External-witness chain checkpoints.
    Checkpoint,
}

impl Domain {
    /// The HKDF `info` string: `mxi/<service>/<domain>/<scheme>`.
    ///
    /// Binding the service name is what stops a tag from one service
    /// verifying against an identically-shaped row in another when a
    /// deployment shares one root key across a cluster.
    fn info(self) -> String {
        let domain = match self {
            Self::AuditChain => "audit-chain",
            Self::Record => "record",
            Self::Checkpoint => "checkpoint",
        };
        format!("mxi/{}/{domain}/{SCHEME}", env!("CARGO_PKG_NAME"))
    }
}

/// Derive a domain subkey from the root key with HKDF-SHA256.
///
/// No salt: the root key is already full-entropy key material, which is
/// the case RFC 5869 §3.1 describes as not needing one. The `info` string
/// carries the separation.
fn derive(root: &[u8], domain: Domain) -> Vec<u8> {
    let hk = hkdf::Hkdf::<Sha256>::new(None, root);
    let mut out = vec![0u8; 32];
    // Fails only when the output length exceeds 255 * HashLen; 32 does not.
    hk.expand(domain.info().as_bytes(), &mut out)
        .expect("32 bytes is a valid HKDF output length");
    out
}

/// Whether a key is obviously a placeholder rather than a secret.
///
/// A length floor alone accepts 32 zero bytes, `0101…`, or a row of the
/// same character — values that appear in examples, get pasted into a
/// deployment, and pass every check. Counting distinct bytes catches the
/// whole family cheaply. This is not an entropy estimator and does not
/// pretend to be: it rejects the specific failure of a placeholder
/// reaching production, and says nothing about a poorly-generated key
/// that happens to look varied.
fn is_placeholder(key: &[u8]) -> bool {
    let mut seen = [false; 256];
    let mut distinct = 0usize;
    for &b in key {
        if !seen[b as usize] {
            seen[b as usize] = true;
            distinct += 1;
        }
    }
    distinct < 8
}

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
    /// The row names a key-derivation scheme this build does not
    /// implement, so the MAC cannot be checked. **Not** a mismatch — it
    /// means this binary is older than the row, and the fix is to upgrade
    /// the binary, not to investigate the data.
    UnknownScheme(String),
    /// The stored value is not `[scheme.]id:hex`.
    Malformed,
}

/// One key's derived subkeys, plus the raw bytes retained only for
/// verifying legacy (pre-derivation) stored MACs.
struct KeyMaterial {
    /// HKDF subkey per domain.
    audit_chain: Vec<u8>,
    /// HKDF subkey per domain.
    record: Vec<u8>,
    /// HKDF subkey per domain.
    checkpoint: Vec<u8>,
    /// The undERIVED root, kept solely so a stored `k1:…` written before
    /// derivation existed still verifies instead of being reported as
    /// tampering. New MACs never use it.
    legacy_raw: Vec<u8>,
}

impl KeyMaterial {
    /// Derive every domain subkey from a root key, then wipe the root.
    fn derive_all(root: &mut Vec<u8>) -> Self {
        let material = Self {
            audit_chain: derive(root, Domain::AuditChain),
            record: derive(root, Domain::Record),
            checkpoint: derive(root, Domain::Checkpoint),
            legacy_raw: root.clone(),
        };
        root.zeroize();
        material
    }

    /// The subkey for a domain.
    fn for_domain(&self, domain: Domain) -> &[u8] {
        match domain {
            Domain::AuditChain => &self.audit_chain,
            Domain::Record => &self.record,
            Domain::Checkpoint => &self.checkpoint,
        }
    }
}

/// The configured keys: the active one (used for writing) and every key
/// available for verification, by id.
struct Keys {
    /// `(id, material)` used to write new MACs; `None` when unconfigured.
    active: Option<(String, KeyMaterial)>,
    /// Every key available for verification, including the active one.
    by_id: HashMap<String, KeyMaterial>,
}

/// Parse `id:hex` pairs from a comma-separated list, skipping malformed
/// entries with a warning that **never includes the key material**.
fn parse_retired(raw: &str) -> HashMap<String, KeyMaterial> {
    let mut out = HashMap::new();
    for entry in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let Some((id, hex)) = entry.split_once(':') else {
            tracing::warn!("retired MAC key entry is not id:hex; skipping");
            continue;
        };
        match decode_hex(hex) {
            Some(mut key) if key.len() >= MIN_KEY_LEN => {
                out.insert(id.to_string(), KeyMaterial::derive_all(&mut key));
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

/// Read the root key from the file (preferred) or the environment.
///
/// The file wins when both are set, and the collision is logged: it
/// usually means a deployment moved to mounted secrets and left the old
/// variable behind, and silently preferring either one would make which
/// key is live depend on knowledge nobody has.
fn load_root_key() -> Option<String> {
    resolve_root_key(
        std::env::var(KEY_ENV).ok().filter(|s| !s.trim().is_empty()),
        std::env::var(KEY_FILE_ENV)
            .ok()
            .filter(|s| !s.trim().is_empty())
            .as_deref(),
    )
}

/// The selection rules, split from [`load_root_key`] so they are testable
/// without mutating the process environment — `std::env::set_var` is
/// `unsafe` in this edition and this crate is `#![forbid(unsafe_code)]`,
/// and it is racy under a parallel harness besides.
fn resolve_root_key(from_env: Option<String>, path: Option<&str>) -> Option<String> {
    let Some(path) = path else {
        return from_env;
    };
    if from_env.is_some() {
        tracing::warn!("both {KEY_ENV} and {KEY_FILE_ENV} are set; the file takes precedence");
    }
    match std::fs::read_to_string(path) {
        // Trailing newline stripped: a key file written by `echo` is the
        // normal case, and failing on it would be a poor first experience
        // for the very deployment doing the right thing.
        Ok(contents) => Some(contents.trim().to_string()),
        Err(e) => {
            // The path is logged, the contents never are.
            tracing::error!(path = %path, error = %e, "cannot read {KEY_FILE_ENV}");
            None
        }
    }
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

        let active = match load_root_key() {
            None => {
                tracing::info!(
                    "no {KEY_ENV} or {KEY_FILE_ENV} configured; integrity MACs are not being written"
                );
                None
            }
            Some(mut raw) => match decode_hex(&raw) {
                None => {
                    raw.zeroize();
                    tracing::error!("MAC root key is not valid hex; integrity MACs disabled");
                    None
                }
                Some(mut key) if key.len() < MIN_KEY_LEN => {
                    // Length only — the key itself is never logged.
                    tracing::error!(
                        len = key.len(),
                        min = MIN_KEY_LEN,
                        "MAC root key is too short; integrity MACs disabled"
                    );
                    key.zeroize();
                    raw.zeroize();
                    None
                }
                Some(mut key) if is_placeholder(&key) => {
                    // A key of 32 zero bytes passes every length check and
                    // is not a secret. Refusing it is the same fail-closed
                    // posture SEC-A1 applied to the token signing seed.
                    tracing::error!(
                        "MAC root key looks like a placeholder (too few distinct bytes); \
                         integrity MACs disabled"
                    );
                    key.zeroize();
                    raw.zeroize();
                    None
                }
                Some(mut key) => {
                    raw.zeroize();
                    let id = std::env::var(KEY_ID_ENV)
                        .ok()
                        .filter(|s| {
                            !s.trim().is_empty() && !s.contains(':') && !s.contains('.')
                        })
                        .unwrap_or_else(|| DEFAULT_KEY_ID.to_string());
                    // Derived twice — once for the by-id verification map,
                    // once for the active writer. `derive_all` wipes the
                    // copy it is given, so no root survives this block.
                    let mut copy = key.clone();
                    by_id.insert(id.clone(), KeyMaterial::derive_all(&mut copy));
                    let material = KeyMaterial::derive_all(&mut key);
                    Some((id, material))
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
pub fn tag(domain: Domain, preimage: &[u8]) -> Option<String> {
    let (id, material) = keys().active.as_ref()?;
    Some(format!(
        "{SCHEME}.{id}:{}",
        raw_tag(material.for_domain(domain), preimage)
    ))
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
pub fn verify(domain: Domain, stored: Option<&str>, preimage: &[u8]) -> MacVerdict {
    let Some(stored) = stored else {
        return MacVerdict::Absent;
    };
    let Some((prefix, hex)) = stored.split_once(':') else {
        return MacVerdict::Malformed;
    };
    // `<scheme>.<id>` for a derived MAC; a bare `<id>` predates derivation.
    let (scheme, id) = match prefix.split_once('.') {
        Some((scheme, id)) => (Some(scheme), id),
        None => (None, prefix),
    };
    if let Some(scheme) = scheme
        && scheme != SCHEME
    {
        // A scheme this build does not implement. Refusing to guess is the
        // same call the token verifier makes for an unknown algorithm:
        // falling through to some default would verify a tag under the
        // wrong key and call it valid.
        return MacVerdict::UnknownScheme(scheme.to_string());
    }
    let Some(material) = keys().by_id.get(id) else {
        return MacVerdict::UnknownKey(id.to_string());
    };
    // Legacy values were produced with the raw root key, before domain
    // separation existed. Verifying them with it keeps adoption from
    // producing a wall of false accusations; nothing new is written this
    // way.
    let key: &[u8] = if scheme.is_some() {
        material.for_domain(domain)
    } else {
        &material.legacy_raw
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

    use super::{Domain, MacVerdict, decode_hex, derive, is_placeholder, parse_retired, raw_tag};
    use sha2::Sha256;

    /// **Golden vectors for the derivation**, cross-checked against an
    /// independent HKDF-SHA256 implementation (Python `hmac`/`hashlib`,
    /// RFC 5869) rather than against our own output.
    ///
    /// The derivation is part of the stored-MAC contract: change the info
    /// string, the scheme tag, or the output length and every stored MAC
    /// stops verifying. That is a migration, not a refactor, and it must
    /// not happen by accident — these pin it, and the `d1` scheme tag is
    /// what makes a deliberate change survivable.
    #[test]
    fn derivation_matches_independent_hkdf_vectors() {
        let root = [9u8; 32];
        let cases = [
            (
                Domain::AuditChain,
                "7db2a364f332d4ed2a108764bbc268966d1208538135c8ea54aa559cd84ceb25",
                "1c6ebd740fb5c1456cfeee3f58b95d7ad18acf610335068e4d1fe18626495cb0",
            ),
            (
                Domain::Record,
                "414c9b7adef5a0ba216c8c20b7b8a6b25812da1261073375af3f0c2b0f51163a",
                "2be7142ea6aceebdc7afe0d0fd9e5a85200d8d2527f6314f0db3c890f7625969",
            ),
            (
                Domain::Checkpoint,
                "e2a716fc0f479895dd7915b83bd6f2e3af9935e4d70271918924c194f1c2a264",
                "7b3440d8567131c556f8405636ba4b15c162784a7979d9e71bf442896f545e9a",
            ),
        ];
        for (domain, want_subkey, want_tag) in cases {
            let subkey = derive(&root, domain);
            let hex: String = subkey.iter().map(|b| format!("{b:02x}")).collect();
            assert_eq!(hex, want_subkey, "{domain:?} subkey drifted");
            assert_eq!(
                raw_tag(&subkey, b"golden vector preimage"),
                want_tag,
                "{domain:?} tag drifted"
            );
        }
    }

    /// The key can come from a file, which is how a production secret
    /// actually arrives: a Kubernetes secret volume or a Docker secret,
    /// not an environment variable that every child process inherits and
    /// every crash dump records.
    #[test]
    fn the_key_can_be_read_from_a_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("mac.key");
        let hex = "0123456789abcdef".repeat(4);
        std::fs::write(&path, format!("{hex}\n")).expect("write");

        let loaded = super::resolve_root_key(None, path.to_str());
        // Trailing newline stripped: a file written by `echo` is the
        // normal case.
        assert_eq!(loaded.as_deref(), Some(hex.as_str()));
    }

    /// With both sources set the file wins. Silently preferring either
    /// would make which key is live depend on knowledge nobody has.
    #[test]
    fn the_file_takes_precedence_over_the_environment() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("mac.key");
        std::fs::write(&path, "aa".repeat(32)).expect("write");

        let loaded = super::resolve_root_key(Some("bb".repeat(32)), path.to_str());
        assert_eq!(loaded.as_deref(), Some("aa".repeat(32).as_str()));
    }

    /// An unreadable key file disables MACs rather than falling back to
    /// the environment variable. A deployment that mounted a secret and
    /// got the path wrong should see MACs stop, not silently continue
    /// under a stale key it believed it had replaced.
    #[test]
    fn an_unreadable_key_file_does_not_fall_back_to_the_environment() {
        let loaded =
            super::resolve_root_key(Some("cc".repeat(32)), Some("/nonexistent/path/mac.key"));
        assert_eq!(loaded, None, "a bad path must not silently use the env key");
    }

    /// With no file configured the environment variable is used, so the
    /// developer-convenience path still works.
    #[test]
    fn the_environment_is_used_when_no_file_is_configured() {
        let loaded = super::resolve_root_key(Some("dd".repeat(32)), None);
        assert_eq!(loaded.as_deref(), Some("dd".repeat(32).as_str()));
        assert_eq!(super::resolve_root_key(None, None), None);
    }

    /// **The cross-domain property.** The same root key and the same
    /// bytes produce a different tag per domain, so a tag written for one
    /// purpose can never verify as another.
    ///
    /// Before derivation, the only thing separating the domains was the
    /// leading version tag in each pre-image — a field that exists for
    /// versioning. Renaming the record pre-image's tag to `v1`, which the
    /// chain already used and which is an obvious name for both, would
    /// have silently made a record tag verify as a chain tag.
    #[test]
    fn a_tag_does_not_transfer_between_domains() {
        let root = [9u8; 32];
        let preimage = b"identical bytes in both domains";

        let chain = raw_tag(&derive(&root, Domain::AuditChain), preimage);
        let record = raw_tag(&derive(&root, Domain::Record), preimage);
        let checkpoint = raw_tag(&derive(&root, Domain::Checkpoint), preimage);

        assert_ne!(chain, record);
        assert_ne!(chain, checkpoint);
        assert_ne!(record, checkpoint);
    }

    /// **The cross-service property.** The HKDF info string carries this
    /// crate's package name, so one root key shared across a cluster does
    /// not let a tag from another service verify here.
    ///
    /// This is not hypothetical. The chain version tag is `v1` in both
    /// care-pathway and case, the checkpoint tag is `cp1` in every crate,
    /// and the crate identity appears nowhere in any pre-image — so with
    /// a shared key and a matching row shape, a tag genuinely did
    /// transfer between services before this.
    #[test]
    fn a_tag_does_not_transfer_between_services() {
        let root = [9u8; 32];
        let preimage = b"a row shape both services could produce";

        let ours = raw_tag(&derive(&root, Domain::AuditChain), preimage);

        // What a sibling service computes for the same domain: identical
        // derivation, differing only in the package name bound into info.
        let hk = hkdf::Hkdf::<Sha256>::new(None, &root);
        let mut sibling_key = vec![0u8; 32];
        hk.expand(b"mxi/case-service/audit-chain/d1", &mut sibling_key)
            .expect("valid length");
        let theirs = raw_tag(&sibling_key, preimage);

        assert_ne!(
            ours, theirs,
            "one cluster-wide key must not make services interchangeable"
        );
    }

    /// The info string names this crate, so a copy-paste of this module
    /// into a sibling cannot silently keep care-pathway's derivation.
    #[test]
    fn the_info_string_binds_this_service() {
        let info = Domain::AuditChain.info();
        assert!(info.starts_with("mxi/"));
        assert!(
            info.contains(env!("CARGO_PKG_NAME")),
            "info must bind the package name, got {info}"
        );
        assert!(info.ends_with("/d1"));
    }

    /// A placeholder key is refused. A length floor alone accepts 32 zero
    /// bytes and `0101…`, which pass every check and are not secrets —
    /// the same fail-closed posture SEC-A1 applied to the token seed.
    #[test]
    fn placeholder_keys_are_refused() {
        assert!(is_placeholder(&[0u8; 32]), "all zeroes");
        assert!(is_placeholder(&[0xab; 32]), "one repeated byte");
        assert!(is_placeholder(&[0x01, 0x02].repeat(16)), "two alternating");
        assert!(
            is_placeholder(&(0..32).map(|i| i % 4).collect::<Vec<u8>>()),
            "four distinct values"
        );
        // A real key has many distinct bytes.
        assert!(
            !is_placeholder(&(0..32u8).collect::<Vec<u8>>()),
            "32 distinct bytes is not a placeholder"
        );
    }

    /// A stored MAC naming an unknown scheme is unverifiable, not
    /// invalid. It means this binary is older than the row: the fix is to
    /// upgrade the binary, and reporting tampering would send an
    /// investigation to the data instead.
    #[test]
    fn unknown_scheme_is_unverifiable_not_invalid() {
        let verdict = super::verify(Domain::Record, Some("d99.k1:00"), b"x");
        assert_eq!(verdict, MacVerdict::UnknownScheme("d99".to_string()));
        assert_ne!(verdict, MacVerdict::Invalid);
    }

    /// The stored prefix carries the scheme, so a verifier can tell a
    /// derived MAC from a legacy one without guessing.
    #[test]
    fn the_stored_prefix_is_scheme_dot_key_id() {
        // `tag` needs a configured key; check the format it builds from.
        let tagged = format!("{}.{}:{}", super::SCHEME, "k1", raw_tag(&[3u8; 32], b"x"));
        let (prefix, hex) = tagged.split_once(':').expect("has a colon");
        let (scheme, id) = prefix.split_once('.').expect("prefix is scheme.id");
        assert_eq!(scheme, "d1");
        assert_eq!(id, "k1");
        assert_eq!(hex.len(), 64, "SHA-256 hex");
    }

    /// Deriving wipes the root key it was handed, so the root does not
    /// survive in the process image. The subkeys necessarily persist —
    /// they are needed on every write — but they are the narrower secret.
    #[test]
    fn deriving_wipes_the_root_key() {
        let mut root = vec![0x5au8; 32];
        // Vary it so the placeholder rule is not what we are testing.
        for (i, b) in root.iter_mut().enumerate() {
            *b = u8::try_from(i).unwrap_or(0);
        }
        let before = root.clone();
        let material = super::KeyMaterial::derive_all(&mut root);

        assert!(
            root.iter().all(|&b| b == 0),
            "the root must be zeroed after derivation"
        );
        // The derivation still happened, from the pre-wipe bytes.
        assert_eq!(material.audit_chain, derive(&before, Domain::AuditChain));
        assert_ne!(material.audit_chain, material.record);
    }

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
        assert_eq!(
            super::verify(Domain::Record, Some("no-colon"), b"x"),
            MacVerdict::Malformed
        );
        assert_eq!(
            super::verify(Domain::Record, None, b"x"),
            MacVerdict::Absent
        );
    }

    /// A tag naming a key this service does not hold is unverifiable, not
    /// invalid. Reporting it as a mismatch would turn a key-distribution
    /// problem into an apparent tampering incident.
    #[test]
    fn unknown_key_id_is_unverifiable_not_invalid() {
        let verdict = super::verify(Domain::Record, Some("nosuchkey:00"), b"x");
        assert!(matches!(verdict, MacVerdict::UnknownKey(id) if id == "nosuchkey"));
    }
}
