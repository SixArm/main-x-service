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
//!
//! **Not yet adopted** (§8.5 steps 4–5): read/disclosure auditing across
//! the `list` / `search` / `check-duplicates` paths, the GDPR residency
//! and lawful-basis declarations, Art. 17 erasure by redaction, and
//! row-level record integrity. This module ships the integrity half
//! first; claiming the rest before the code exists would be worse than
//! shipping it in two steps.
//!
//! ## Configuration
//!
//! | Variable | Default | Meaning |
//! |---|---|---|
//! | `PERSON_AUDIT_FAIL_CLOSED` | off | Refuse a write when its audit row cannot be chained, rather than proceeding unaccounted for. |

/// Tamper-evident audit history: the SHA-256 hash chain over `audit_log`.
pub mod audit_chain;
