//! Regulatory-compliance controls for the person service.
//!
//! Adopted from the family's reference implementation in the
//! [care-pathway service](../../../../care-pathway/care-pathway-service-with-loco/src/compliance/),
//! per [`spec/compliance` §8.5](../../../../spec/compliance/index.md)
//! step 3: the personal-data services take the audit chain and
//! read/disclosure auditing first, because HIPAA and GDPR bite hardest
//! where the records are about people.
//!
//! **Person is the identity spine of the family.** Its records are
//! personal — often special-category — data, and every other service's
//! audit trail points at a person id, so a silently editable trail here is
//! the worst failure mode in the tree.
//!
//! | Framework | Module |
//! |---|---|
//! | **HIPAA** — audit controls, integrity | [`audit_chain`] |
//! | **HIPAA** — read/disclosure auditing (§164.312(b), §164.528) | [`disclosure`] |
//!
//! **Not yet adopted** (§8.5 steps 4–5): the GDPR residency and
//! lawful-basis declarations, Art. 17 erasure by redaction, and row-level
//! record integrity. Claiming those before the code exists would be worse
//! than shipping in steps.
//!
//! ## Configuration
//!
//! | Variable | Default | Meaning |
//! |---|---|---|
//! | `PERSON_AUDIT_READS` | off | Write an audit row for reads / searches / exports (HIPAA §164.312(b)). |
//! | `PERSON_AUDIT_FAIL_CLOSED` | off | Refuse a read (`503`) when its audit row cannot be written, rather than serving data unaccounted for. |

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

/// Whether read-auditing is on, from `PERSON_AUDIT_READS` (read once and
/// cached).
///
/// **Default off**, so adopting this module is behaviour-neutral. Person
/// holds personal — often special-category — data, so a deployment
/// serving real records should turn it on together with
/// `PERSON_REQUIRE_AUTH`: without a verified caller the rows carry no
/// actor and the §164.528 accounting is close to worthless.
#[must_use]
pub fn audit_reads() -> bool {
    static AUDIT_READS: OnceLock<bool> = OnceLock::new();
    *AUDIT_READS.get_or_init(|| {
        matches!(
            std::env::var("PERSON_AUDIT_READS")
                .unwrap_or_default()
                .trim()
                .to_ascii_lowercase()
                .as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
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
    use super::Build;

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
