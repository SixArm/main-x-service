//! Keyed integrity MACs with production-grade key handling, shared by
//! every Main X Index service that carries a tamper-evidence tier.
//!
//! # Why this is a crate rather than a copy
//!
//! The services' unkeyed digests (SHA-256, SHA-3) are recomputable by
//! anyone: the pre-image formats are published in each spec's §12.4z, so
//! an adversary who can write SQL edits a row and recomputes them. The
//! **MAC is the only stored value they cannot forge**, and it is the one
//! piece of this system whose correctness rests entirely on getting
//! cryptography right.
//!
//! That code was copied into four service crates before this crate
//! existed. Copying is the family's normal posture for DTOs and
//! scaffolding, and it is the wrong posture here for a specific,
//! observed reason: a latent defect in the sibling `soup.rs` — a test
//! matching the substring `timestamp` rather than the JSON field —
//! survived in three copies and surfaced only when a fourth crate
//! happened to use the word in prose. A key-handling defect that
//! survived that way would not announce itself at all; it would simply
//! make MACs forgeable while every test stayed green.
//!
//! One implementation, one test suite, one place to fix. The same call
//! the family already made for `authentication-verifier`, on the same
//! grounds.
//!
//! # What the MAC defends against, stated plainly
//!
//! An adversary holding **only the database** — a stolen backup, a
//! replica, SQL injection, a DBA without application-server access —
//! cannot forge a MAC, because the key lives in the service's
//! environment and is never written to the database.
//!
//! An adversary holding **both** the database and the service
//! environment has the key and can forge freely. Nothing stored beside
//! the data could prevent that. This is defence against database-only
//! compromise, which is the common case and worth having; it is not
//! defence against full host compromise.
//!
//! It also does not help if the key is put somewhere the database can
//! reach. `pgcrypto` offers `hmac()`, and using it would place the key
//! exactly where the adversary already is. The separation *is* the
//! control.
//!
//! # Domain separation
//!
//! The configured value is a **root** key. It never MACs anything. Each
//! purpose derives its own subkey with HKDF-SHA256 under an `info`
//! string binding the **service name** and the **domain**:
//!
//! ```text
//! mxi/<service>/<domain>/d1
//! ```
//!
//! Without this, separation rests on two pre-images never colliding —
//! and in this family the only thing keeping them apart was a leading
//! version tag that exists for versioning, not separation. Two failure
//! modes followed, both closed here rather than argued away:
//!
//! - **Cross-domain.** Renaming one pre-image's version tag to a value
//!   another already used would silently make one domain's tag verify as
//!   another's. `v1` is an obvious name for more than one.
//! - **Cross-service.** The version tags were *identical across crates*,
//!   and no pre-image names its own crate. A single MAC key shared
//!   across a cluster — the obvious way to deploy one — therefore let a
//!   tag from one service verify against an identically-shaped row in
//!   another.
//!
//! Deriving per (service, domain) removes the reliance entirely: a tag
//! cannot transfer even if two pre-images are byte-identical.
//!
//! # Usage
//!
//! Each service defines its own domains — they differ, and an enum here
//! would have to know every service's set — and holds one [`KeySet`] for
//! the process.
//!
//! ```
//! use integrity_mac::{KeyConfig, KeySet, MacVerdict};
//!
//! // Each service names its own domains; a local enum keeps the call
//! // site exhaustively checked while the cryptography stays shared.
//! const AUDIT_CHAIN: &str = "audit-chain";
//!
//! let config = KeyConfig::new("example-service", "EXAMPLE");
//! let keys = KeySet::load(&config, Some("9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"), None);
//!
//! let tag = keys.tag(AUDIT_CHAIN, b"the row's pre-image").expect("a key is configured");
//! assert!(tag.starts_with("d1.k1:"));
//! assert_eq!(keys.verify(AUDIT_CHAIN, Some(&tag), b"the row's pre-image"), MacVerdict::Valid);
//!
//! // A tag does not transfer between domains.
//! assert_eq!(keys.verify("record", Some(&tag), b"the row's pre-image"), MacVerdict::Invalid);
//! ```

// Always start with high quality coding conventions.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::pedantic)]

use std::collections::HashMap;

use hmac::{Hmac, Mac};
use sha2::Sha256;
use zeroize::Zeroize as _;

/// Minimum root-key length in bytes.
///
/// HMAC accepts any length, but a key shorter than the digest adds no
/// security over a shorter one and usually signals a placeholder that
/// escaped into a deployment. Refusing it is cheap and catches that.
pub const MIN_KEY_LEN: usize = 32;

/// Bytes in a generated key.
///
/// 32 matches HMAC-SHA256's block security. Longer buys nothing: HMAC
/// hashes a key longer than its block size back down.
pub const KEY_BYTES: usize = 32;

/// The current key-derivation scheme, stored in the MAC prefix.
///
/// A stored value with no scheme (`k1:…`) predates derivation and is
/// verified against the **raw** root key, so adopting derivation does not
/// turn existing rows into a wall of false accusations. A value naming a
/// scheme this build does not know is reported unverifiable rather than
/// invalid, for the same reason.
pub const SCHEME: &str = "d1";

/// The default key id when none is configured.
pub const DEFAULT_KEY_ID: &str = "k1";

/// Minimum distinct byte values before a key stops looking like a
/// placeholder. See [`assess_key_hex`].
const MIN_DISTINCT_BYTES: usize = 8;

/// How a stored MAC could not be checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MacVerdict {
    /// Recomputed and matched.
    Valid,
    /// Recomputed and did **not** match: the content changed and whoever
    /// changed it did not hold the key.
    Invalid,
    /// The row carries no MAC — written before a key was configured.
    Absent,
    /// The row names a key id this service does not hold, so the MAC can
    /// be neither confirmed nor refuted. **Not** a mismatch.
    UnknownKey(String),
    /// The row names a key-derivation scheme this build does not
    /// implement. **Not** a mismatch — it means this binary is older than
    /// the row, and the fix is to upgrade the binary, not to investigate
    /// the data.
    UnknownScheme(String),
    /// The stored value is not `[scheme.]id:hex`.
    Malformed,
}

/// Why a candidate key is unusable, or that it is fine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Assessment {
    /// Usable as an active key.
    Usable {
        /// Decoded length in bytes.
        bytes: usize,
    },
    /// Not valid hex — the commonest paste error, e.g. an `0x` prefix, a
    /// stray quote, or an odd number of characters.
    NotHex,
    /// Shorter than [`MIN_KEY_LEN`].
    TooShort {
        /// Decoded length in bytes.
        bytes: usize,
    },
    /// Full length but too few distinct bytes to be a real secret.
    Placeholder {
        /// How many distinct byte values appeared.
        distinct: usize,
    },
}

impl Assessment {
    /// Whether this key may be used as the active key.
    #[must_use]
    pub fn is_usable(&self) -> bool {
        matches!(self, Self::Usable { .. })
    }

    /// A human-readable verdict. Every failure says why; an operator
    /// reading "NOT USABLE" with no reason has to guess.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Usable { bytes } => format!("usable: {bytes} bytes of valid hex"),
            Self::NotHex => "NOT USABLE: not valid hex (an odd length, an 0x prefix, \
                 whitespace, or a stray quote are the usual causes)"
                .to_string(),
            Self::TooShort { bytes } => {
                format!("NOT USABLE: {bytes} bytes, minimum is {MIN_KEY_LEN}")
            }
            Self::Placeholder { distinct } => format!(
                "NOT USABLE: only {distinct} distinct byte values — this looks like a \
                 placeholder rather than a generated key"
            ),
        }
    }
}

/// Which service and environment-variable family a [`KeySet`] belongs to.
///
/// The service name is bound into every derived subkey, so two services
/// sharing one root key still cannot verify each other's tags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyConfig {
    /// This service's package name, e.g. `care-pathway-service`.
    pub service: &'static str,
    /// Environment-variable prefix, e.g. `CARE_PATHWAY`.
    pub env_prefix: &'static str,
}

impl KeyConfig {
    /// Build a configuration.
    #[must_use]
    pub const fn new(service: &'static str, env_prefix: &'static str) -> Self {
        Self {
            service,
            env_prefix,
        }
    }

    /// The variable holding the hex root key inline.
    #[must_use]
    pub fn key_env(&self) -> String {
        format!("{}_INTEGRITY_MAC_KEY", self.env_prefix)
    }

    /// The variable naming a file whose contents are the hex root key.
    #[must_use]
    pub fn key_file_env(&self) -> String {
        format!("{}_INTEGRITY_MAC_KEY_FILE", self.env_prefix)
    }

    /// The variable naming the active key, for the stored prefix.
    #[must_use]
    pub fn key_id_env(&self) -> String {
        format!("{}_INTEGRITY_MAC_KEY_ID", self.env_prefix)
    }

    /// The variable holding retired keys as comma-separated `id:hex`.
    #[must_use]
    pub fn retired_keys_env(&self) -> String {
        format!("{}_INTEGRITY_MAC_KEYS_RETIRED", self.env_prefix)
    }

    /// The HKDF `info` string for one domain.
    #[must_use]
    pub fn info(&self, domain: &str) -> String {
        format!("mxi/{}/{domain}/{SCHEME}", self.service)
    }
}

/// One key's derived subkeys, plus the raw bytes retained only for
/// verifying legacy (pre-derivation) stored MACs.
struct KeyMaterial {
    /// Derived subkey per domain, keyed by domain name.
    by_domain: HashMap<String, Vec<u8>>,
    /// The underived root, kept solely so a stored `k1:…` written before
    /// derivation existed still verifies instead of being reported as
    /// tampering. New MACs never use it.
    legacy_raw: Vec<u8>,
    /// The config, so a domain not derived at load time can be derived on
    /// demand.
    config: KeyConfig,
}

impl KeyMaterial {
    /// Derive from a root key, then wipe the root.
    fn derive_all(config: KeyConfig, root: &mut Vec<u8>, domains: &[&str]) -> Self {
        let by_domain = domains
            .iter()
            .map(|d| ((*d).to_string(), derive(root, &config.info(d))))
            .collect();
        let material = Self {
            by_domain,
            legacy_raw: root.clone(),
            config,
        };
        root.zeroize();
        material
    }

    /// The subkey for a domain, derived on demand if it was not
    /// pre-declared.
    ///
    /// Deriving on demand costs one HKDF per call for an undeclared
    /// domain, which is why services declare theirs — but returning
    /// `None`, or falling back to another domain's key, would be worse
    /// than slow.
    fn for_domain(&self, domain: &str) -> Vec<u8> {
        self.by_domain.get(domain).map_or_else(
            || derive(&self.legacy_raw, &self.config.info(domain)),
            Clone::clone,
        )
    }
}

/// The loaded key set: the active key used for writing, and every key
/// available for verification.
pub struct KeySet {
    /// `(id, material)` used to write new MACs; `None` when unconfigured.
    active: Option<(String, KeyMaterial)>,
    /// Every key available for verification, including the active one.
    by_id: HashMap<String, KeyMaterial>,
}

impl std::fmt::Debug for KeySet {
    /// Deliberately opaque: a `Debug` that printed key material would
    /// put it into every error message and log line that formats a
    /// containing struct.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeySet")
            .field("enabled", &self.active.is_some())
            .field("active_key_id", &self.active_key_id())
            .field("verification_keys", &self.by_id.len())
            .finish()
    }
}

impl KeySet {
    /// Load from explicit values, deriving subkeys for `domains`.
    ///
    /// `raw_key` is the hex root key; `retired` is a comma-separated list
    /// of `id:hex` pairs. Both may be `None`.
    ///
    /// A configuration error disables MAC **writing** and is reported by
    /// [`Self::is_enabled`]; it does not panic and does not prevent a
    /// caller booting. The consequence stays visible rather than silent,
    /// because verification then reports `Absent` for every new row.
    #[must_use]
    pub fn load(config: &KeyConfig, raw_key: Option<&str>, retired: Option<&str>) -> Self {
        Self::load_with_domains(config, raw_key, retired, None, &[])
    }

    /// As [`Self::load`], with an explicit key id and pre-derived domains.
    #[must_use]
    pub fn load_with_domains(
        config: &KeyConfig,
        raw_key: Option<&str>,
        retired: Option<&str>,
        key_id: Option<&str>,
        domains: &[&str],
    ) -> Self {
        let mut by_id = retired
            .map(|r| parse_retired(*config, r, domains))
            .unwrap_or_default();

        let active = match raw_key.map(str::trim).filter(|s| !s.is_empty()) {
            None => None,
            Some(raw) => {
                let assessment = assess_key_hex(raw);
                if assessment.is_usable() {
                    let mut key = decode_hex(raw).unwrap_or_default();
                    let id = key_id
                        .map(str::trim)
                        .filter(|s| !s.is_empty() && !s.contains(':') && !s.contains('.'))
                        .unwrap_or(DEFAULT_KEY_ID)
                        .to_string();
                    let mut copy = key.clone();
                    by_id.insert(
                        id.clone(),
                        KeyMaterial::derive_all(*config, &mut copy, domains),
                    );
                    let material = KeyMaterial::derive_all(*config, &mut key, domains);
                    Some((id, material))
                } else {
                    None
                }
            }
        };
        Self { active, by_id }
    }

    /// Whether a key is configured, so MACs are being written.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.active.is_some()
    }

    /// The active key id, when one is configured.
    #[must_use]
    pub fn active_key_id(&self) -> Option<&str> {
        self.active.as_ref().map(|(id, _)| id.as_str())
    }

    /// The prefix new MACs are written with: `"<scheme>.<key id>:"`.
    ///
    /// Exposed so a re-signing tool can tell an already-current row from
    /// one still under a retired key without re-deriving the format,
    /// which would be a second place for it to drift.
    #[must_use]
    pub fn active_prefix(&self) -> Option<String> {
        self.active_key_id().map(|id| format!("{SCHEME}.{id}:"))
    }

    /// Compute the stored MAC for a pre-image: `"<scheme>.<key id>:<hex>"`.
    ///
    /// `None` when no key is configured — the caller stores `NULL`, and
    /// verification reports [`MacVerdict::Absent`].
    #[must_use]
    pub fn tag(&self, domain: &str, preimage: &[u8]) -> Option<String> {
        let (id, material) = self.active.as_ref()?;
        Some(format!(
            "{SCHEME}.{id}:{}",
            raw_tag(&material.for_domain(domain), preimage)
        ))
    }

    /// Check a stored MAC against a pre-image.
    ///
    /// Comparison is **constant-time** ([`Mac::verify_slice`]). A timing
    /// oracle here would let an attacker with write access recover a
    /// valid tag byte by byte without ever holding the key.
    /// # Panics
    ///
    /// Cannot in practice: the only fallible step is `Hmac::new_from_slice`,
    /// and HMAC is defined for a key of any length, so it never returns an
    /// error. The `expect` documents that rather than hiding it behind a
    /// silent fallback — a MAC computed with a wrong or empty key would be
    /// worse than a crash, because it would verify.
    #[must_use]
    pub fn verify(&self, domain: &str, stored: Option<&str>, preimage: &[u8]) -> MacVerdict {
        let Some(stored) = stored else {
            return MacVerdict::Absent;
        };
        let Some((prefix, hex)) = stored.split_once(':') else {
            return MacVerdict::Malformed;
        };
        // `<scheme>.<id>` for a derived MAC; a bare `<id>` predates
        // derivation.
        let (scheme, id) = match prefix.split_once('.') {
            Some((scheme, id)) => (Some(scheme), id),
            None => (None, prefix),
        };
        if let Some(scheme) = scheme
            && scheme != SCHEME
        {
            // Refusing to guess is the same call the token verifier makes
            // for an unknown algorithm: falling through to a default
            // would verify a tag under the wrong key and call it valid.
            return MacVerdict::UnknownScheme(scheme.to_string());
        }
        let Some(material) = self.by_id.get(id) else {
            return MacVerdict::UnknownKey(id.to_string());
        };
        let Some(expected) = decode_hex(hex) else {
            return MacVerdict::Malformed;
        };
        // Legacy values were produced with the raw root key, before
        // domain separation existed. Verifying them with it keeps
        // adoption from producing a wall of false accusations; nothing
        // new is written this way.
        let key = if scheme.is_some() {
            material.for_domain(domain)
        } else {
            material.legacy_raw.clone()
        };
        let mut mac =
            <Hmac<Sha256> as Mac>::new_from_slice(&key).expect("HMAC accepts keys of any length");
        mac.update(preimage);
        if mac.verify_slice(&expected).is_ok() {
            MacVerdict::Valid
        } else {
            MacVerdict::Invalid
        }
    }
}

/// Read the root key from a file (preferred) or an inline value.
///
/// The file wins when both are given, and the collision is reported in
/// `warning`: it usually means a deployment moved to mounted secrets and
/// left the old variable behind, and silently preferring either one would
/// make which key is live depend on knowledge nobody has.
///
/// An **unreadable** file yields `None` rather than falling back to the
/// inline value: a deployment that mounted a secret and got the path
/// wrong should see MACs stop, not silently continue under a stale key it
/// believed it had replaced.
///
/// Split from any environment access so the rules are testable without
/// mutating the process environment — `std::env::set_var` is `unsafe` in
/// this edition, this crate is `#![forbid(unsafe_code)]`, and it is racy
/// under a parallel harness besides.
#[must_use]
pub fn resolve_root_key(inline: Option<&str>, path: Option<&str>) -> KeySource {
    let inline = inline.map(str::trim).filter(|s| !s.is_empty());
    let Some(path) = path.map(str::trim).filter(|s| !s.is_empty()) else {
        return KeySource {
            key: inline.map(ToString::to_string),
            warning: None,
            error: None,
        };
    };
    let warning = inline.map(|_| {
        "both the inline key and the key file are set; the file takes precedence".to_string()
    });
    match std::fs::read_to_string(path) {
        // Trailing newline stripped: a key file written by `echo` is the
        // normal case, and failing on it would be a poor first experience
        // for the very deployment doing the right thing.
        Ok(contents) => KeySource {
            key: Some(contents.trim().to_string()),
            warning,
            error: None,
        },
        // The path is reported, the contents never are.
        Err(e) => KeySource {
            key: None,
            warning,
            error: Some(format!("cannot read key file {path}: {e}")),
        },
    }
}

/// The outcome of [`resolve_root_key`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeySource {
    /// The hex root key, when one was obtained.
    pub key: Option<String>,
    /// A configuration warning worth logging.
    pub warning: Option<String>,
    /// Why the key could not be read. Never contains key material.
    pub error: Option<String>,
}

/// Assess a hex-encoded candidate key against the rules the loader
/// applies, without loading it.
///
/// Shared with the operator-facing key task so a pre-flight check and the
/// service's own acceptance cannot disagree. A checker that blessed a key
/// the loader then refused would be worse than no checker.
///
/// The placeholder rule counts distinct byte values. A length floor alone
/// accepts 32 zero bytes, `0101…`, or a row of one repeated character —
/// values that appear in examples, get pasted into a deployment, and pass
/// every check. This is **not** an entropy estimator and does not pretend
/// to be: it catches the specific failure of a placeholder reaching
/// production, and says nothing about a poorly-generated key that happens
/// to look varied.
#[must_use]
pub fn assess_key_hex(candidate: &str) -> Assessment {
    let Some(key) = decode_hex(candidate) else {
        return Assessment::NotHex;
    };
    if key.len() < MIN_KEY_LEN {
        return Assessment::TooShort { bytes: key.len() };
    }
    let distinct = distinct_bytes(&key);
    if distinct < MIN_DISTINCT_BYTES {
        return Assessment::Placeholder { distinct };
    }
    Assessment::Usable { bytes: key.len() }
}

/// Generate a root key from the operating-system CSPRNG, hex-encoded.
///
/// `getrandom` is the OS entropy source directly (`getrandom(2)`,
/// `BCryptGenRandom`) rather than a userspace PRNG. For a key that lives
/// for months that is the right call: there is no seeding question, no
/// process-fork hazard, and no PRNG state to reason about.
///
/// # Errors
///
/// When the OS entropy source is unavailable, which is fatal rather than
/// something to paper over — a fallback here would be a predictable key.
pub fn generate_key() -> Result<String, String> {
    let mut bytes = [0u8; KEY_BYTES];
    getrandom::getrandom(&mut bytes)
        .map_err(|e| format!("the operating system entropy source failed: {e}"))?;
    let hex = to_hex(&bytes);
    bytes.zeroize();

    // A generated key that failed our own rules would mean the generator
    // is broken; checking is nearly free and the alternative is shipping
    // it. 32 random bytes have vanishing odds of under 8 distinct values,
    // so this firing at all means something is very wrong.
    if !assess_key_hex(&hex).is_usable() {
        return Err("generated key failed its own validation; refusing to emit it".to_string());
    }
    Ok(hex)
}

/// Write a key to `path` with owner-only permissions.
///
/// Created at mode `0600` on Unix **before** the bytes are written, not
/// after: creating world-readable and then tightening leaves a window in
/// which any local user can read the key, and that window is exactly when
/// the interesting bytes land.
///
/// Refuses to overwrite an existing file. Clobbering the live key makes
/// every stored MAC unverifiable and is not recoverable.
///
/// # Errors
///
/// When the file exists, or cannot be created or written.
pub fn write_key_file(path: &str, key: &str) -> Result<(), String> {
    use std::io::Write as _;

    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|e| format!("cannot create {path}: {e} (it must not already exist)"))?;
    writeln!(file, "{key}").map_err(|e| format!("cannot write {path}: {e}"))
}

/// Derive a domain subkey from a root key with HKDF-SHA256.
///
/// No salt: the root is already full-entropy key material, the case
/// RFC 5869 §3.1 describes as not needing one. The `info` string carries
/// the separation.
fn derive(root: &[u8], info: &str) -> Vec<u8> {
    let hk = hkdf::Hkdf::<Sha256>::new(None, root);
    let mut out = vec![0u8; 32];
    // Fails only when the output length exceeds 255 * HashLen; 32 does not.
    hk.expand(info.as_bytes(), &mut out)
        .expect("32 bytes is a valid HKDF output length");
    out
}

/// HMAC-SHA256 as lowercase hex.
fn raw_tag(key: &[u8], preimage: &[u8]) -> String {
    // Infallible for HMAC: it accepts a key of any length.
    let mut mac =
        <Hmac<Sha256> as Mac>::new_from_slice(key).expect("HMAC accepts keys of any length");
    mac.update(preimage);
    to_hex(&mac.finalize().into_bytes())
}

/// Parse `id:hex` pairs, skipping malformed entries.
///
/// The placeholder rule deliberately does **not** apply here. A retired
/// key exists to verify history that was already written; refusing it
/// would convert "this row is unchanged since it was `MAC`ed" into "I
/// cannot check this", losing information without gaining security.
fn parse_retired(config: KeyConfig, raw: &str, domains: &[&str]) -> HashMap<String, KeyMaterial> {
    let mut out = HashMap::new();
    for entry in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let Some((id, hex)) = entry.split_once(':') else {
            continue;
        };
        if let Some(mut key) = decode_hex(hex)
            && key.len() >= MIN_KEY_LEN
        {
            out.insert(
                id.to_string(),
                KeyMaterial::derive_all(config, &mut key, domains),
            );
        }
    }
    out
}

/// Decode a hex string of even length.
///
/// Works over **bytes**, not `&str` slices. Slicing `&s[i..i + 2]` panics
/// when the boundary lands inside a multi-byte character, and this runs on
/// two inputs nobody here controls: the stored MAC string, which an
/// adversary holding only the database can rewrite to anything, and an
/// operator-supplied key. `decode_hex("€a")` used to abort the process
/// (`agents/share/security.md` invariant 2 — never-panic on untrusted
/// input); the SEC-I2 `verify_tag` fuzz target now pins it.
///
/// Also stricter than the `u8::from_str_radix` it replaces, which accepted
/// a leading sign: `"+f"` decoded as `0x0f`. Nothing writes such a value,
/// and a sign in the middle of a hex digest is a corrupted row, not a
/// number.
fn decode_hex(s: &str) -> Option<Vec<u8>> {
    let bytes = s.trim().as_bytes();
    if !bytes.len().is_multiple_of(2) {
        return None;
    }
    bytes
        .chunks_exact(2)
        .map(|pair| Some((hex_nibble(pair[0])? << 4) | hex_nibble(pair[1])?))
        .collect()
}

/// One hex digit as its 0–15 value, or `None` for any other byte.
const fn hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Lowercase hex.
///
/// Nibble lookup with the output pre-allocated, rather than `write!`ing
/// each byte into a growing `String`. Formatting machinery per byte cost
/// most of a microsecond on every 32-byte digest, and this sits on the
/// per-row path of every audited write — see `benches/mac.rs`, which is
/// how that showed up at all.
fn to_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(DIGITS[(b >> 4) as usize] as char);
        out.push(DIGITS[(b & 0x0f) as usize] as char);
    }
    out
}

/// How many distinct byte values appear.
fn distinct_bytes(key: &[u8]) -> usize {
    let mut seen = [false; 256];
    let mut distinct = 0usize;
    for &b in key {
        if !seen[b as usize] {
            seen[b as usize] = true;
            distinct += 1;
        }
    }
    distinct
}

#[cfg(test)]
mod tests {
    use super::*;

    const AUDIT: &str = "audit-chain";
    const RECORD: &str = "record";
    const CHECKPOINT: &str = "checkpoint";

    fn config() -> KeyConfig {
        KeyConfig::new("care-pathway-service", "CARE_PATHWAY")
    }

    /// A realistic root key: varied bytes, so it passes the placeholder
    /// rule the loader applies. (An earlier fixture of 32 identical bytes
    /// was rejected by that rule — correctly.)
    fn root_hex() -> String {
        "0123456789abcdef".repeat(4)
    }

    /// HMAC-SHA256 against RFC 4231 test case 1 — an external vector, so
    /// this pins the primitive rather than pinning our own output against
    /// itself.
    #[test]
    fn hmac_matches_rfc_4231_vector_1() {
        let key = [0x0bu8; 20];
        assert_eq!(
            raw_tag(&key, b"Hi There"),
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
    }

    /// **Golden vectors for the derivation**, cross-checked against an
    /// independent HKDF-SHA256 implementation (Python `hmac`/`hashlib`,
    /// RFC 5869) rather than against our own output.
    ///
    /// These are the same vectors the four service crates carried before
    /// this crate existed, so extraction is proven byte-compatible: every
    /// MAC already stored in a database still verifies.
    #[test]
    fn derivation_matches_the_service_crates_byte_for_byte() {
        let root = [9u8; 32];
        let cases = [
            (
                AUDIT,
                "7db2a364f332d4ed2a108764bbc268966d1208538135c8ea54aa559cd84ceb25",
            ),
            (
                RECORD,
                "414c9b7adef5a0ba216c8c20b7b8a6b25812da1261073375af3f0c2b0f51163a",
            ),
            (
                CHECKPOINT,
                "e2a716fc0f479895dd7915b83bd6f2e3af9935e4d70271918924c194f1c2a264",
            ),
        ];
        for (domain, want) in cases {
            assert_eq!(
                to_hex(&derive(&root, &config().info(domain))),
                want,
                "{domain} subkey drifted from the service crates"
            );
        }
    }

    /// The info string binds the service name, which is what stops one
    /// cluster-wide key making two services interchangeable.
    #[test]
    fn the_info_string_binds_the_service_and_scheme() {
        let info = config().info(AUDIT);
        assert_eq!(info, "mxi/care-pathway-service/audit-chain/d1");
        assert!(info.starts_with("mxi/"));
        assert!(info.ends_with("/d1"));
    }

    /// **The cross-domain property.** The same key and the same bytes
    /// produce a different tag per domain, so a tag written for one
    /// purpose can never verify as another.
    #[test]
    fn a_tag_does_not_transfer_between_domains() {
        let keys = KeySet::load(&config(), Some(&root_hex()), None);
        let preimage = b"identical bytes in both domains";
        let tag = keys.tag(AUDIT, preimage).expect("enabled");

        assert_eq!(keys.verify(AUDIT, Some(&tag), preimage), MacVerdict::Valid);
        assert_eq!(
            keys.verify(RECORD, Some(&tag), preimage),
            MacVerdict::Invalid,
            "an audit tag must not verify as a record tag"
        );
        assert_eq!(
            keys.verify(CHECKPOINT, Some(&tag), preimage),
            MacVerdict::Invalid
        );
    }

    /// **The cross-service property.** Two services sharing one root key
    /// cannot verify each other's tags.
    #[test]
    fn a_tag_does_not_transfer_between_services() {
        let shared = root_hex();
        let ours = KeySet::load(&config(), Some(&shared), None);
        let theirs = KeySet::load(&KeyConfig::new("case-service", "CASE"), Some(&shared), None);
        let preimage = b"a row shape both services could produce";

        let tag = ours.tag(AUDIT, preimage).expect("enabled");
        assert_eq!(
            theirs.verify(AUDIT, Some(&tag), preimage),
            MacVerdict::Invalid,
            "one cluster-wide key must not make services interchangeable"
        );
    }

    /// A tampered pre-image is caught; the unkeyed digests it sits beside
    /// would not be, which is the whole reason this exists.
    #[test]
    fn a_changed_preimage_invalidates_the_tag() {
        let keys = KeySet::load(&config(), Some(&root_hex()), None);
        let tag = keys.tag(RECORD, b"original content").expect("enabled");
        assert_eq!(
            keys.verify(RECORD, Some(&tag), b"tampered content"),
            MacVerdict::Invalid
        );
    }

    /// Every non-verdict outcome is distinguished from a mismatch, because
    /// "I cannot check this" and "this is wrong" lead to different
    /// investigations.
    #[test]
    fn unverifiable_is_never_reported_as_invalid() {
        let keys = KeySet::load(&config(), Some(&root_hex()), None);
        assert_eq!(keys.verify(RECORD, None, b"x"), MacVerdict::Absent);
        assert_eq!(
            keys.verify(RECORD, Some("no-colon"), b"x"),
            MacVerdict::Malformed
        );
        assert_eq!(
            keys.verify(RECORD, Some("d1.k9:00"), b"x"),
            MacVerdict::UnknownKey("k9".to_string())
        );
        assert_eq!(
            keys.verify(RECORD, Some("d99.k1:00"), b"x"),
            MacVerdict::UnknownScheme("d99".to_string())
        );
        assert_eq!(
            keys.verify(RECORD, Some("d1.k1:zz"), b"x"),
            MacVerdict::Malformed
        );
    }

    /// A legacy MAC with no scheme prefix still verifies, against the raw
    /// root key. Adopting derivation must not turn existing rows into a
    /// wall of false accusations.
    #[test]
    fn legacy_unprefixed_macs_still_verify() {
        let keys = KeySet::load(&config(), Some(&root_hex()), None);
        let root = decode_hex(&root_hex()).expect("hex");
        let legacy = format!("k1:{}", raw_tag(&root, b"an old row"));

        assert_eq!(
            keys.verify(RECORD, Some(&legacy), b"an old row"),
            MacVerdict::Valid
        );
        assert_eq!(
            keys.verify(RECORD, Some(&legacy), b"an edited row"),
            MacVerdict::Invalid,
            "legacy verification must still detect tampering"
        );
    }

    /// Rotation is additive: a retired key keeps verifying its own rows
    /// while the active key writes new ones.
    #[test]
    fn retired_keys_still_verify_and_new_writes_use_the_active_key() {
        let old = "fedcba9876543210".repeat(4);
        let new = "13579bdf02468ace".repeat(4);
        let old_keys = KeySet::load(&config(), Some(&old), None);
        let old_tag = old_keys.tag(AUDIT, b"historic row").expect("enabled");

        let rotated = KeySet::load_with_domains(
            &config(),
            Some(&new),
            Some(&format!("k0:{old}")),
            Some("k1"),
            &[AUDIT],
        );
        // The historic row names k0; re-tag it under k0's id to model the
        // stored value.
        let stored = old_tag.replace("d1.k1:", "d1.k0:");
        assert_eq!(
            rotated.verify(AUDIT, Some(&stored), b"historic row"),
            MacVerdict::Valid,
            "a retired key must keep verifying its own rows"
        );
        assert_eq!(rotated.active_key_id(), Some("k1"));
        assert!(
            rotated
                .tag(AUDIT, b"new row")
                .expect("enabled")
                .starts_with("d1.k1:")
        );
    }

    /// An unusable key disables writing rather than being used.
    #[test]
    fn unusable_keys_disable_macs_rather_than_weakening_them() {
        for bad in ["", "zz", &"00".repeat(32), "abcd", &"0101".repeat(16)] {
            let keys = KeySet::load(&config(), Some(bad), None);
            assert!(
                !keys.is_enabled(),
                "{bad:?} must not be accepted as an active key"
            );
            assert_eq!(keys.tag(AUDIT, b"x"), None);
        }
        let keys = KeySet::load(&config(), Some(&root_hex()), None);
        assert!(keys.is_enabled());
    }

    /// Placeholder detection catches the values that pass a length floor.
    #[test]
    fn placeholders_are_refused_and_real_keys_are_not() {
        assert_eq!(
            assess_key_hex(&"00".repeat(32)),
            Assessment::Placeholder { distinct: 1 }
        );
        assert_eq!(
            assess_key_hex(&"0102".repeat(16)),
            Assessment::Placeholder { distinct: 2 }
        );
        assert_eq!(assess_key_hex("abcd"), Assessment::TooShort { bytes: 2 });
        assert_eq!(assess_key_hex("0x00"), Assessment::NotHex);
        assert_eq!(assess_key_hex(&"zz".repeat(32)), Assessment::NotHex);
        assert!(assess_key_hex(&generate_key().expect("generates")).is_usable());
    }

    /// Every verdict explains itself.
    #[test]
    fn every_assessment_is_explained() {
        for a in [
            Assessment::Usable { bytes: 32 },
            Assessment::NotHex,
            Assessment::TooShort { bytes: 4 },
            Assessment::Placeholder { distinct: 2 },
        ] {
            let text = a.describe();
            assert!(text.len() > 20, "{a:?}: {text}");
            assert_eq!(text.contains("NOT USABLE"), !a.is_usable());
        }
    }

    /// Generated keys are full length, usable, and distinct per call. A
    /// generator returning a constant would pass every other check.
    #[test]
    fn generated_keys_are_usable_and_distinct() {
        let a = generate_key().expect("generates");
        let b = generate_key().expect("generates");
        assert_eq!(a.len(), KEY_BYTES * 2);
        assert_ne!(a, b);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }

    /// The file source wins over the inline value, and an unreadable file
    /// does not fall back.
    #[test]
    fn key_sourcing_prefers_the_file_and_never_falls_back() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("mac.key");
        std::fs::write(&path, format!("{}\n", "aa".repeat(32))).expect("write");
        let path = path.to_str().expect("utf-8");

        // Trailing newline stripped.
        let from_file = resolve_root_key(None, Some(path));
        assert_eq!(from_file.key.as_deref(), Some("aa".repeat(32).as_str()));
        assert!(from_file.warning.is_none());

        // Both set: file wins, collision reported.
        let both = resolve_root_key(Some(&"bb".repeat(32)), Some(path));
        assert_eq!(both.key.as_deref(), Some("aa".repeat(32).as_str()));
        assert!(both.warning.is_some(), "the collision must be surfaced");

        // Unreadable file: no fallback to the inline key.
        let missing = resolve_root_key(Some(&"cc".repeat(32)), Some("/nonexistent/mac.key"));
        assert_eq!(missing.key, None);
        assert!(missing.error.is_some());
        assert!(
            !missing.error.expect("error").contains("cc"),
            "an error must never contain key material"
        );

        // Inline only, and neither.
        assert_eq!(
            resolve_root_key(Some(&"dd".repeat(32)), None)
                .key
                .as_deref(),
            Some("dd".repeat(32).as_str())
        );
        assert_eq!(resolve_root_key(None, None).key, None);
    }

    /// Writing refuses to clobber, and is owner-only from creation.
    #[test]
    fn key_files_are_owner_only_and_never_clobbered() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("mac.key");
        let path_str = path.to_str().expect("utf-8");

        write_key_file(path_str, "first").expect("first write");
        assert!(
            write_key_file(path_str, "second").is_err(),
            "an existing key must never be overwritten"
        );
        assert!(
            std::fs::read_to_string(&path)
                .expect("read")
                .contains("first"),
            "the original key must survive"
        );

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mode = std::fs::metadata(&path)
                .expect("metadata")
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
        }
    }

    /// `Debug` never prints key material. A `Debug` that did would put the
    /// key into every error message that formats a containing struct.
    #[test]
    fn debug_never_prints_key_material() {
        let keys = KeySet::load(&config(), Some(&root_hex()), None);
        let text = format!("{keys:?}");
        assert!(text.contains("enabled"));
        assert!(
            !text.contains(&root_hex()),
            "the root key leaked into Debug"
        );
        assert!(!text.contains("0909"), "key bytes leaked into Debug");
    }

    /// The environment-variable names are derived from one prefix, so a
    /// service cannot half-rename its configuration.
    #[test]
    fn env_var_names_derive_from_one_prefix() {
        let c = config();
        assert_eq!(c.key_env(), "CARE_PATHWAY_INTEGRITY_MAC_KEY");
        assert_eq!(c.key_file_env(), "CARE_PATHWAY_INTEGRITY_MAC_KEY_FILE");
        assert_eq!(c.key_id_env(), "CARE_PATHWAY_INTEGRITY_MAC_KEY_ID");
        assert_eq!(
            c.retired_keys_env(),
            "CARE_PATHWAY_INTEGRITY_MAC_KEYS_RETIRED"
        );
    }

    /// A domain not pre-declared still derives correctly rather than
    /// falling back to another domain's key or returning nothing.
    #[test]
    fn an_undeclared_domain_still_derives_its_own_key() {
        let declared =
            KeySet::load_with_domains(&config(), Some(&root_hex()), None, None, &[AUDIT]);
        let tag = declared.tag("assessment", b"x").expect("enabled");
        assert_eq!(
            declared.verify("assessment", Some(&tag), b"x"),
            MacVerdict::Valid
        );
        assert_eq!(
            declared.verify(AUDIT, Some(&tag), b"x"),
            MacVerdict::Invalid,
            "an on-demand domain must still be separated"
        );
    }
    /// A stored MAC is whatever whoever edited the row put there. Before
    /// this, `decode_hex` sliced the `&str` by byte index, so a
    /// multi-byte character straddling an even offset aborted the
    /// process — a database-write adversary could crash every reader.
    #[test]
    fn a_stored_mac_containing_a_multibyte_character_is_rejected_not_a_panic() {
        let keys = KeySet::load(&config(), Some(&root_hex()), None);
        for stored in [
            "k1:\u{20ac}a",
            "d1.k1:\u{20ac}a",
            "\u{20ac}\u{20ac}",
            "d1.k1:\u{e9}\u{e9}",
        ] {
            assert!(
                matches!(
                    keys.verify(AUDIT, Some(stored), b"payload"),
                    MacVerdict::Malformed
                        | MacVerdict::UnknownKey(_)
                        | MacVerdict::UnknownScheme(_)
                ),
                "{stored:?} must be reported, not panic"
            );
        }
        assert_eq!(assess_key_hex("\u{20ac}a"), Assessment::NotHex);
    }

    /// `u8::from_str_radix` accepted a leading sign, so `"+f"` decoded as
    /// `0x0f`. A sign inside a digest is a corrupted row, not a number.
    #[test]
    fn a_signed_hex_pair_is_not_a_digit() {
        assert_eq!(decode_hex("+f"), None);
        assert_eq!(decode_hex("-f"), None);
        assert_eq!(decode_hex("0f"), Some(vec![0x0f]));
        assert_eq!(
            decode_hex("0F"),
            Some(vec![0x0f]),
            "uppercase still decodes"
        );
        assert_eq!(decode_hex("f"), None, "odd length");
        assert_eq!(decode_hex(""), Some(vec![]));
    }
}
