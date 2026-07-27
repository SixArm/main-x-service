//! Keyed integrity: this crate's binding to the shared
//! [`integrity_mac`] crate.
//!
//! The cryptography, key sourcing, HKDF domain separation, zeroization,
//! and placeholder refusal all live in `integrity/integrity-mac-rust-crate`
//! — one audited implementation rather than one copy per service. See
//! that crate's module docs for what the MAC defends against and why the
//! separation is per (service, domain).
//!
//! What stays here is the part that is genuinely local:
//!
//! - **[`Domain`]**, this service's own set of MAC purposes. It is an
//!   enum rather than a bare string so every call site stays
//!   exhaustively checked by the compiler; the shared crate takes a
//!   `&str` because the set differs per service and an enum there would
//!   have to know all of them.
//! - **The process key set**, loaded once from this service's own
//!   environment variables.
//!
//! ## Absent key
//!
//! No key configured means **no MAC is written**, and rows carrying one
//! are reported as *unverifiable* rather than as mismatches. This
//! matches how the digests treat rows that predate them: adopting the
//! control must not produce a wall of false accusations, and "I cannot
//! check this" is a different statement from "this is wrong". Both are
//! reported; neither is silently rounded to "verified".

use std::sync::OnceLock;

use integrity_mac::{KeyConfig, KeySet};

pub use integrity_mac::{Assessment, MacVerdict, assess_key_hex, generate_key, write_key_file};

/// This service's identity and environment-variable family.
///
/// The service name is bound into every derived subkey, so this
/// service's tags cannot verify against a sibling's rows even when a
/// deployment shares one root key across a cluster.
const CONFIG: KeyConfig = KeyConfig::new(env!("CARGO_PKG_NAME"), "PERSON");

/// Environment variable holding the active MAC **root** key, hex-encoded.
pub const KEY_ENV: &str = "PERSON_INTEGRITY_MAC_KEY";

/// Environment variable naming a **file** whose contents are the
/// hex-encoded root key. Takes precedence over [`KEY_ENV`]; a mounted
/// secret beats an environment variable that every child process
/// inherits and every crash dump records.
pub const KEY_FILE_ENV: &str = "PERSON_INTEGRITY_MAC_KEY_FILE";

/// Environment variable naming the active key, for the stored prefix.
pub const KEY_ID_ENV: &str = "PERSON_INTEGRITY_MAC_KEY_ID";

/// Environment variable holding retired keys still needed for
/// verification, as `id:hex` pairs separated by commas.
pub const RETIRED_KEYS_ENV: &str = "PERSON_INTEGRITY_MAC_KEYS_RETIRED";

/// Minimum root-key length in bytes.
pub const MIN_KEY_LEN: usize = integrity_mac::MIN_KEY_LEN;

/// The purposes that must not share a key.
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
    /// The domain label passed to the shared crate.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AuditChain => "audit-chain",
            Self::Record => "record",
            Self::Checkpoint => "checkpoint",
        }
    }

    /// Every domain, so the key set derives them all at load time rather
    /// than on first use.
    const ALL: [&'static str; 3] = ["audit-chain", "record", "checkpoint"];
}

/// The process key set, loaded once.
///
/// A configuration error disables MAC writing and is logged; it does not
/// prevent boot, matching the ABAC-policy and PASETO-key loaders. The
/// consequence is visible rather than silent: `mac_absent` climbs in
/// every verification report, which is the signal that the key never
/// loaded.
fn keys() -> &'static KeySet {
    static KEYS: OnceLock<KeySet> = OnceLock::new();
    KEYS.get_or_init(|| {
        let source = integrity_mac::resolve_root_key(
            std::env::var(KEY_ENV).ok().as_deref(),
            std::env::var(KEY_FILE_ENV).ok().as_deref(),
        );
        if let Some(warning) = &source.warning {
            tracing::warn!("{warning}");
        }
        if let Some(error) = &source.error {
            tracing::error!("{error}; integrity MACs disabled");
        }
        if let Some(raw) = &source.key {
            let assessment = assess_key_hex(raw);
            if !assessment.is_usable() {
                // The assessment names the problem, never the key.
                tracing::error!(
                    "MAC root key rejected: {}; integrity MACs disabled",
                    assessment.describe()
                );
            }
        } else if source.error.is_none() {
            tracing::info!(
                "no {KEY_ENV} or {KEY_FILE_ENV} configured; integrity MACs are not being written"
            );
        }
        KeySet::load_with_domains(
            &CONFIG,
            source.key.as_deref(),
            std::env::var(RETIRED_KEYS_ENV).ok().as_deref(),
            std::env::var(KEY_ID_ENV).ok().as_deref(),
            &Domain::ALL,
        )
    })
}

/// Whether a key is configured, so MACs are being written.
#[must_use]
pub fn is_enabled() -> bool {
    keys().is_enabled()
}

/// The active key id, when one is configured.
#[must_use]
pub fn active_key_id() -> Option<&'static str> {
    keys().active_key_id()
}

/// The prefix new MACs are written with: `"<scheme>.<key id>:"`.
#[must_use]
pub fn active_prefix() -> Option<String> {
    keys().active_prefix()
}

/// Compute the stored MAC for a pre-image.
///
/// `None` when no key is configured — the caller stores `NULL`, and
/// verification reports [`MacVerdict::Absent`].
#[must_use]
pub fn tag(domain: Domain, preimage: &[u8]) -> Option<String> {
    keys().tag(domain.as_str(), preimage)
}

/// Check a stored MAC against a pre-image.
#[must_use]
pub fn verify(domain: Domain, stored: Option<&str>, preimage: &[u8]) -> MacVerdict {
    keys().verify(domain.as_str(), stored, preimage)
}

#[cfg(test)]
mod tests {
    use super::Domain;

    /// Every domain has a distinct label, since the label *is* the
    /// separation — two domains sharing one would silently share a key.
    #[test]
    fn every_domain_has_a_distinct_label() {
        let labels: Vec<&str> = [Domain::AuditChain, Domain::Record, Domain::Checkpoint]
            .iter()
            .map(|d| d.as_str())
            .collect();
        let mut unique = labels.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(labels.len(), unique.len(), "duplicate domain label");
    }

    /// `ALL` covers every variant, so the key set pre-derives each one.
    /// A variant missing here still works — the shared crate derives on
    /// demand — but pays an HKDF per call, and the omission would be
    /// invisible without this.
    #[test]
    fn all_lists_every_domain() {
        for domain in [Domain::AuditChain, Domain::Record, Domain::Checkpoint] {
            assert!(
                Domain::ALL.contains(&domain.as_str()),
                "{domain:?} missing from Domain::ALL"
            );
        }
        assert_eq!(Domain::ALL.len(), 3);
    }
}
