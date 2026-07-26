//! Regulatory-compliance controls for the case service.
//!
//! Adopted from the family's reference implementation in the
//! [care-pathway service](../../../../care-pathway/care-pathway-service-with-loco/src/compliance/),
//! per [`spec/compliance` §8.5](../../../../spec/compliance/index.md)
//! step 3: the personal-data services take the audit chain and
//! read/disclosure auditing first, because HIPAA and GDPR bite hardest
//! where the records are about people.
//!
//! **Case data is personal data** — a case concerns an identified or
//! identifiable person — so unlike care-pathway, where the templates are
//! reference data and only the trail is personal, here the records
//! themselves are in scope. That makes read-auditing the load-bearing
//! control rather than a refinement.
//!
//! What is adopted here:
//!
//! | Framework | Module |
//! |---|---|
//! | **HIPAA** — audit controls, integrity, accounting of disclosures | [`audit_chain`], [`disclosure`] |
//!
//! What is deliberately **not** adopted yet: the FHIR conformance
//! machinery, the SOUP/SBOM evidence bundle, and Bulk Data. Those follow
//! at §8.5 steps 4–5; adopting them piecemeal would claim more than the
//! code does.
//!
//! ## Configuration
//!
//! | Variable | Default | Meaning |
//! |---|---|---|
//! | `CASE_AUDIT_READS` | off | Write an audit row for reads / searches (HIPAA §164.312(b)). |
//! | `CASE_AUDIT_FAIL_CLOSED` | off | Refuse a read (`503`) when its audit row cannot be written. |

/// Tamper-evident audit history: the SHA-256 hash chain over `audit_logs`.
pub mod audit_chain;
/// Read/disclosure auditing: purpose-of-use capture and access records.
pub mod disclosure;

/// GDPR Art. 17 erasure by redaction (see the module docs).
pub mod erasure;

use std::sync::OnceLock;

/// Whether read-auditing is on, from `CASE_AUDIT_READS` (read once and
/// cached).
///
/// **Default off**, so adopting this module is behaviour-neutral. A
/// deployment holding real case data should turn it on, together with
/// `CASE_REQUIRE_AUTH` — without a verified caller the rows carry no
/// actor and the accounting is close to worthless.
#[must_use]
pub fn audit_reads() -> bool {
    static AUDIT_READS: OnceLock<bool> = OnceLock::new();
    *AUDIT_READS.get_or_init(|| {
        crate::auth::parse_bool(&std::env::var("CASE_AUDIT_READS").unwrap_or_default())
    })
}
