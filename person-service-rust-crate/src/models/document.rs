//! The [`IdentityDocument`] model — passports, licenses, and the like.
//!
//! An identity document is a stronger, often government-issued proof of
//! identity than a bare [`crate::models::Identifier`]. The matcher can
//! short-circuit on an exact `(document_type, number)` match. The
//! [`DocumentType`] enum serializes to SCREAMING_SNAKE_CASE codes
//! (e.g. `"DRIVERS_LICENSE"`) to match the wire/DB contract.
//!
//! # Examples
//!
//! ```
//! use person_service::models::DocumentType;
//!
//! assert_eq!(format!("{}", DocumentType::Passport), "PASSPORT");
//! ```

use jiff::civil::Date;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Identity document types
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

/// Renders the canonical SCREAMING_SNAKE_CASE code (e.g. `PASSPORT`).
///
/// Kept in lock-step with the serde `rename` attributes so the textual
/// form is stable across the wire, the database, and logs.
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

/// An identity document associated with a person
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

    /// Every `DocumentType` has a non-empty `Display` code, and a few
    /// representative ones render to their exact expected strings.
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

    /// An `IdentityDocument` survives a JSON round-trip with its type,
    /// number, country, dates, and verified flag intact.
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
