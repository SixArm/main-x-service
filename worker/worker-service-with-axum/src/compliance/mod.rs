//! Regulatory-compliance controls for the worker service.
//!
//! Adopted from the family's reference implementation in the
//! [care-pathway service](../../../../care-pathway/care-pathway-service-with-axum/src/compliance/),
//! per [`spec/compliance` §8.5](../../../../spec/compliance/index.md)
//! step 3: the personal-data services take the audit chain and
//! read/disclosure auditing first, because HIPAA and GDPR bite hardest
//! where the records are about people.
//!
//! **Worker is the identity spine of the family.** Its records are
//! personal — often special-category — data, and every other service's
//! audit trail points at a person id, so a silently editable trail here is
//! the worst failure mode in the tree.
//!
//! | Framework | Module |
//! |---|---|
//! | **HIPAA** — audit controls, integrity | [`audit_chain`], [`record_integrity`], [`mac`], [`checkpoint`] |
//! | **HIPAA** — read/disclosure auditing (§164.312(b), §164.528) | [`disclosure`] |
//! | **GDPR** — Art. 17 erasure that survives the chain | [`erasure`] |
//! | **GDPR** — Ch. V residency, Art. 6 / Art. 9 declarations | [`DataProtection`] (this module) |
//!
//! ## Configuration
//!
//! | Variable | Default | Meaning |
//! |---|---|---|
//! | `WORKER_AUDIT_READS` | off | Write an audit row for reads / searches / exports (HIPAA §164.312(b)). |
//! | `WORKER_AUDIT_FAIL_CLOSED` | off | Refuse a read (`503`) when its audit row cannot be written, rather than serving data unaccounted for. |
//! | `WORKER_DATA_RESIDENCY` | `undeclared` | The region this deployment's data is confined to (GDPR Ch. V), e.g. `uk`. |
//! | `WORKER_LAWFUL_BASIS` | `undeclared` | GDPR Art. 6 basis, e.g. `contract` or `legal_obligation`. |
//! | `WORKER_ART9_CONDITION` | `undeclared` | GDPR Art. 9(2) condition for special-category data, e.g. `employment`. |
//! | `WORKER_TRANSFER_SAFEGUARD` | `undeclared` | Ch. V safeguard, e.g. `adequacy_regulations`. |
//!
//! The declarations **declare and record**; they do not enforce. A
//! residency declaration makes a cross-border export visible in the audit
//! trail and at `GET /api/compliance`; it does not block one, which is a
//! deployment-network decision.

/// How long an audit row keeps the address of the caller.
pub mod address_retention;
/// Tamper-evident audit history: the SHA-256 hash chain over `audit_log`.
pub mod audit_chain;
/// Read/disclosure auditing: purpose-of-use capture and access records.
/// External witness: signed chain checkpoints kept off-box, so wholesale
/// deletion is detectable (see the module docs).
pub mod checkpoint;

pub mod disclosure;

/// GDPR Art. 17 erasure by redaction (see the module docs).
pub mod erasure;

/// Keyed integrity (HMAC) with a key the database never holds.
pub mod mac;

/// Row-level record integrity hashing (see the module docs).
pub mod record_integrity;

/// SOUP register (IEC 62304 §8.1.2 / ISO 27001 A.8) and `CycloneDX` SBOM.
pub mod soup;

use std::sync::OnceLock;

use serde::Serialize;

/// Whether read-auditing is on, from `WORKER_AUDIT_READS` (read once and
/// cached).
///
/// **Default off**, so adopting this module is behaviour-neutral. Worker
/// holds personal — often special-category — data, so a deployment
/// serving real records should turn it on together with
/// `WORKER_REQUIRE_AUTH`: without a verified caller the rows carry no
/// actor and the §164.528 accounting is close to worthless.
#[must_use]
pub fn audit_reads() -> bool {
    static AUDIT_READS: OnceLock<bool> = OnceLock::new();
    *AUDIT_READS.get_or_init(|| {
        matches!(
            std::env::var("WORKER_AUDIT_READS")
                .unwrap_or_default()
                .trim()
                .to_ascii_lowercase()
                .as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

/// The value every unset data-protection declaration reports. Chosen over
/// an optimistic default so an operator reading `GET /api/compliance`
/// cannot mistake "nobody configured this" for "this is compliant".
pub const UNDECLARED: &str = "undeclared";

/// The declared data-protection posture (GDPR Ch. V / Art. 6 / Art. 9).
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct DataProtection {
    /// The region data is confined to, or [`UNDECLARED`].
    pub residency: String,
    /// GDPR Art. 6 lawful basis, or [`UNDECLARED`].
    pub lawful_basis: String,
    /// GDPR Art. 9(2) condition for special-category data, or [`UNDECLARED`].
    pub art9_condition: String,
    /// GDPR Ch. V transfer safeguard, or [`UNDECLARED`].
    pub transfer_safeguard: String,
}

impl DataProtection {
    /// Read the declaration through `lookup` (an environment reader in
    /// production, a map in tests). Unset or blank ⇒ [`UNDECLARED`].
    #[must_use]
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Self {
        let declared = |name: &str| {
            lookup(name)
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| UNDECLARED.to_string())
        };
        Self {
            residency: declared("WORKER_DATA_RESIDENCY"),
            lawful_basis: declared("WORKER_LAWFUL_BASIS"),
            art9_condition: declared("WORKER_ART9_CONDITION"),
            transfer_safeguard: declared("WORKER_TRANSFER_SAFEGUARD"),
        }
    }

    /// The process-wide declaration, read once and cached.
    #[must_use]
    pub fn get() -> &'static Self {
        static DP: OnceLock<DataProtection> = OnceLock::new();
        DP.get_or_init(|| Self::from_lookup(|name| std::env::var(name).ok()))
    }

    /// Whether sending data to `destination` leaves the declared residency
    /// region — a GDPR Ch. V transfer that must be recorded.
    ///
    /// Conservative by construction: with residency undeclared, or a
    /// destination the caller did not name, the answer is `false`, because
    /// asserting a transfer we cannot substantiate would poison the audit
    /// trail with noise. Comparison is case-insensitive and matches on the
    /// region prefix, so `uk-south` is inside `uk`.
    #[must_use]
    pub fn is_cross_border(&self, destination: Option<&str>) -> bool {
        let (Some(dest), true) = (destination, self.residency != UNDECLARED) else {
            return false;
        };
        let dest = dest.trim().to_ascii_lowercase();
        let home = self.residency.trim().to_ascii_lowercase();
        !(dest == home || dest.starts_with(&format!("{home}-")))
    }
}

/// Build provenance — the "reconstructible release" evidence (IEC 62304
/// §8, ISO/IEC 27001 A.8 configuration management).
///
/// Recorded so a released binary can be tied back to the source it was
/// built from. Without it, "which commit is production running?" is
/// answered by inference from deploy logs rather than by the artefact
/// itself.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct Build {
    /// Crate version.
    pub version: &'static str,
    /// Source commit, from `BUILD_SHA` / `GITHUB_SHA` at compile time.
    pub commit: &'static str,
    /// `SOURCE_DATE_EPOCH` at compile time — present iff the build was
    /// run through the reproducible-build wrapper.
    pub source_date_epoch: Option<&'static str>,
    /// Whether the toolchain was pinned by the repository's
    /// `rust-toolchain.toml` (always true for a repo-local build;
    /// recorded so the field is present in the evidence bundle).
    pub toolchain_pinned: bool,
    /// `true` for a debug build — a release artefact must report `false`.
    pub debug: bool,
}

impl Build {
    /// This binary's provenance.
    #[must_use]
    pub fn current() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION"),
            commit: option_env!("BUILD_SHA")
                .or(option_env!("GITHUB_SHA"))
                .unwrap_or("unknown"),
            source_date_epoch: option_env!("SOURCE_DATE_EPOCH"),
            toolchain_pinned: true,
            debug: cfg!(debug_assertions),
        }
    }

    /// Whether this artefact carries the evidence a reproducible release
    /// needs: a known commit, a pinned `SOURCE_DATE_EPOCH`, and a
    /// non-debug profile.
    #[must_use]
    pub fn is_reproducible_release(&self) -> bool {
        self.commit != "unknown" && self.source_date_epoch.is_some() && !self.debug
    }
}

#[cfg(test)]
mod tests {
    use super::{Build, DataProtection, UNDECLARED};

    fn declared(pairs: &[(&str, &str)]) -> DataProtection {
        DataProtection::from_lookup(|name| {
            pairs
                .iter()
                .find(|(k, _)| *k == name)
                .map(|(_, v)| (*v).to_string())
        })
    }

    /// Nothing set reads as `undeclared`, never as a flattering default.
    #[test]
    fn unset_declarations_are_undeclared() {
        let dp = declared(&[("WORKER_DATA_RESIDENCY", "   ")]);
        assert_eq!(dp.residency, UNDECLARED);
        assert_eq!(dp.lawful_basis, UNDECLARED);
        assert_eq!(dp.art9_condition, UNDECLARED);
        assert_eq!(dp.transfer_safeguard, UNDECLARED);
    }

    #[test]
    fn declarations_are_read_and_trimmed() {
        let dp = declared(&[
            ("WORKER_DATA_RESIDENCY", " uk "),
            ("WORKER_LAWFUL_BASIS", "contract"),
            ("WORKER_ART9_CONDITION", "employment"),
            ("WORKER_TRANSFER_SAFEGUARD", "adequacy_regulations"),
        ]);
        assert_eq!(dp.residency, "uk");
        assert_eq!(dp.lawful_basis, "contract");
        assert_eq!(dp.art9_condition, "employment");
        assert_eq!(dp.transfer_safeguard, "adequacy_regulations");
    }

    /// Cross-border detection is conservative: undeclared residency or an
    /// unnamed destination never manufactures a transfer event.
    #[test]
    fn cross_border_is_conservative() {
        let undeclared = declared(&[]);
        assert!(!undeclared.is_cross_border(Some("us-east-1")));
        let uk = declared(&[("WORKER_DATA_RESIDENCY", "uk")]);
        assert!(!uk.is_cross_border(None));
        assert!(!uk.is_cross_border(Some("uk")));
        assert!(
            !uk.is_cross_border(Some(" UK-South ")),
            "a sub-region is inside"
        );
        assert!(uk.is_cross_border(Some("eu-west-2")));
        assert!(
            uk.is_cross_border(Some("ukraine")),
            "a prefix match must stop at a region boundary"
        );
    }

    /// A debug test build must never claim reproducible-release evidence.
    /// The point of the flag is to distinguish a traceable artefact from
    /// an untraceable one; a version that answered `true` unconditionally
    /// would be worse than not having it.
    #[test]
    fn debug_build_is_not_a_reproducible_release() {
        assert!(!Build::current().is_reproducible_release());
    }

    /// Each of the three conditions is independently necessary.
    #[test]
    fn every_condition_is_load_bearing() {
        let complete = Build {
            version: "0.1.0",
            commit: "abc123",
            source_date_epoch: Some("1700000000"),
            toolchain_pinned: true,
            debug: false,
        };
        assert!(complete.is_reproducible_release());
        assert!(
            !Build {
                commit: "unknown",
                ..complete
            }
            .is_reproducible_release(),
            "an unknown commit is not reconstructible"
        );
        assert!(
            !Build {
                source_date_epoch: None,
                ..complete
            }
            .is_reproducible_release(),
            "without SOURCE_DATE_EPOCH the build is not reproducible"
        );
        assert!(
            !Build {
                debug: true,
                ..complete
            }
            .is_reproducible_release(),
            "a debug build is not a release artefact"
        );
    }
}
