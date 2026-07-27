//! Regulatory-compliance controls for the person service.
//!
//! Adopted from the family's reference implementation in the
//! [care-pathway service](../../../../care-pathway/care-pathway-service-with-loco/src/compliance/),
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
//! | `WORKER_AUDIT_READS` | off | Write an audit row for reads / searches / exports (HIPAA §164.312(b)). |
//! | `WORKER_AUDIT_FAIL_CLOSED` | off | Refuse a read (`503`) when its audit row cannot be written, rather than serving data unaccounted for. |

/// Tamper-evident audit history: the SHA-256 hash chain over `audit_log`.
pub mod audit_chain;
/// Read/disclosure auditing: purpose-of-use capture and access records.
pub mod disclosure;

/// GDPR Art. 17 erasure by redaction (see the module docs).
pub mod erasure;

/// Keyed integrity (HMAC) with a key the database never holds.
pub mod mac;

/// Row-level record integrity hashing (see the module docs).
pub mod record_integrity;

use std::sync::OnceLock;

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
