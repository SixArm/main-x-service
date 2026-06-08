//! Identity documents (passport, national ID, driver's license, …).
//!
//! An [`IdentityDocument`] records the document's [`DocumentType`], its number,
//! and optional issuing/validity metadata. The matcher compares documents by
//! *type plus number*, which gives a strong deterministic signal: a matching
//! passport number, for example, is near-conclusive evidence that two records
//! describe the same worker (see `crate::matching::algorithms`).
//!
//! # Examples
//!
//! ```
//! use worker_service::models::{IdentityDocument, DocumentType};
//!
//! let doc = IdentityDocument {
//!     document_type: DocumentType::Passport,
//!     number: "X1234567".into(),
//!     issuing_country: Some("GB".into()),
//!     issuing_authority: None,
//!     issue_date: None,
//!     expiry_date: None,
//!     verified: false,
//! };
//! assert_eq!(doc.document_type.to_string(), "PASSPORT");
//! ```

use jiff::civil::Date;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// The kind of an [`IdentityDocument`].
///
/// Each variant serializes to a fixed SCREAMING_SNAKE_CASE token (e.g.
/// `"PASSPORT"`, `"DRIVERS_LICENSE"`); unrecognized wire values deserialize to
/// [`Other`](Self::Other) via `#[serde(other)]`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub enum DocumentType {
    /// Passport
    #[serde(rename = "PASSPORT")]
    Passport,
    /// Birth certificate
    #[serde(rename = "BIRTH_CERTIFICATE")]
    BirthCertificate,
    /// National ID card
    #[serde(rename = "NATIONAL_ID")]
    NationalId,
    /// Driver's license
    #[serde(rename = "DRIVERS_LICENSE")]
    DriversLicense,
    /// Voter registration card
    #[serde(rename = "VOTER_ID")]
    VoterId,
    /// Military ID
    #[serde(rename = "MILITARY_ID")]
    MilitaryId,
    /// Residence permit
    #[serde(rename = "RESIDENCE_PERMIT")]
    ResidencePermit,
    /// Work permit
    #[serde(rename = "WORK_PERMIT")]
    WorkPermit,
    /// Other document type
    #[serde(other, rename = "OTHER")]
    Other,
}

/// Renders the SCREAMING_SNAKE_CASE wire token for each variant (e.g.
/// `DocumentType::DriversLicense` → `"DRIVERS_LICENSE"`), matching its serde form.
impl std::fmt::Display for DocumentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DocumentType::Passport => write!(f, "PASSPORT"),
            DocumentType::BirthCertificate => write!(f, "BIRTH_CERTIFICATE"),
            DocumentType::NationalId => write!(f, "NATIONAL_ID"),
            DocumentType::DriversLicense => write!(f, "DRIVERS_LICENSE"),
            DocumentType::VoterId => write!(f, "VOTER_ID"),
            DocumentType::MilitaryId => write!(f, "MILITARY_ID"),
            DocumentType::ResidencePermit => write!(f, "RESIDENCE_PERMIT"),
            DocumentType::WorkPermit => write!(f, "WORK_PERMIT"),
            DocumentType::Other => write!(f, "OTHER"),
        }
    }
}

/// An identity document associated with a worker.
///
/// The [`document_type`](Self::document_type) + [`number`](Self::number) pair
/// is the matching key; the remaining fields are descriptive metadata that the
/// validation layer can check (e.g. issue date must precede expiry date).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IdentityDocument {
    /// Type of document
    pub document_type: DocumentType,

    /// Document number / identifier
    pub number: String,

    /// Issuing country (ISO 3166 alpha-2 code)
    pub issuing_country: Option<String>,

    /// Issuing authority or organization
    pub issuing_authority: Option<String>,

    /// Date the document was issued
    pub issue_date: Option<Date>,

    /// Date the document expires
    pub expiry_date: Option<Date>,

    /// Whether the document has been verified
    pub verified: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every [`DocumentType`] has a non-empty, correct `Display` rendering.
    #[test]
    fn test_document_type_variants() {
        let types = vec![
            DocumentType::Passport,
            DocumentType::BirthCertificate,
            DocumentType::NationalId,
            DocumentType::DriversLicense,
            DocumentType::VoterId,
            DocumentType::MilitaryId,
            DocumentType::ResidencePermit,
            DocumentType::WorkPermit,
            DocumentType::Other,
        ];
        for dt in &types {
            let display = format!("{}", dt);
            assert!(!display.is_empty(), "DocumentType Display should not be empty");
        }
        // Check specific display values
        assert_eq!(format!("{}", DocumentType::Passport), "PASSPORT");
        assert_eq!(format!("{}", DocumentType::DriversLicense), "DRIVERS_LICENSE");
        assert_eq!(format!("{}", DocumentType::Other), "OTHER");
    }

    /// An [`IdentityDocument`] survives a JSON round-trip with all fields intact.
    #[test]
    fn test_document_serialization() {
        let doc = IdentityDocument {
            document_type: DocumentType::Passport,
            number: "AB1234567".into(),
            issuing_country: Some("US".into()),
            issuing_authority: Some("State Dept".into()),
            issue_date: Some(jiff::civil::date(2020, 1, 15)),
            expiry_date: Some(jiff::civil::date(2030, 1, 15)),
            verified: true,
        };

        let json = serde_json::to_string(&doc).expect("Serialization should succeed");
        let deser: IdentityDocument = serde_json::from_str(&json).expect("Deserialization should succeed");

        assert_eq!(deser.document_type, DocumentType::Passport);
        assert_eq!(deser.number, "AB1234567");
        assert_eq!(deser.issuing_country.as_deref(), Some("US"));
        assert!(deser.verified);
        assert_eq!(deser.issue_date, doc.issue_date);
        assert_eq!(deser.expiry_date, doc.expiry_date);
    }
}
