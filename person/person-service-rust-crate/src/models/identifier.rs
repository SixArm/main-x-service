//! The [`Identifier`] model — external identity keys for a person.
//!
//! An identifier is a `(type, system, value)` triple: the
//! [`IdentifierType`] says what kind of key it is (MRN, SSN, passport,
//! …), the `system` URI names the issuing namespace, and `value` is the
//! key itself. The matching engine uses exact identifier matches as a
//! strong (often short-circuiting) signal of identity.
//!
//! # Examples
//!
//! ```
//! use person_service::models::{Identifier, IdentifierType};
//!
//! let ssn = Identifier::ssn("123-45-6789".to_string());
//! assert_eq!(ssn.identifier_type, IdentifierType::SSN);
//! assert_eq!(ssn.system, "http://hl7.org/fhir/sid/us-ssn");
//! ```

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Person or organization identifier (MRN, SSN, NPI, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Identifier {
    /// Identifier use (e.g., "official", "temp", "secondary")
    pub use_type: Option<IdentifierUse>,

    /// Identifier type (e.g., "MRN", "SSN", "DL")
    pub identifier_type: IdentifierType,

    /// Identifier system/namespace URI
    pub system: String,

    /// The actual identifier value
    pub value: String,

    /// Organization that issued the identifier
    pub assigner: Option<String>,
}

/// Intended use of an [`Identifier`], mirroring the FHIR
/// `identifier-use` value set.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum IdentifierUse {
    /// The identifier recommended for display and use in real-world interactions
    Usual,
    /// The identifier considered to be most trusted for this person
    Official,
    /// A temporary identifier
    Temp,
    /// An identifier that was assigned in secondary use
    Secondary,
    /// The identifier id no longer considered valid
    Old,
}

/// The kind of an [`Identifier`].
///
/// Serializes UPPERCASE (`IdentifierType::MRN` ⇄ `"MRN"`). Unknown
/// values deserialize to [`Other`](IdentifierType::Other) rather than
/// failing, so forward-compatible payloads are accepted.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum IdentifierType {
    /// Medical Record Number
    MRN,
    /// Social Security Number
    SSN,
    /// Driver's License
    DL,
    /// National Provider Identifier
    NPI,
    /// Passport Number
    PPN,
    /// Tax ID Number
    TAX,
    /// Other identifier type
    #[serde(other)]
    Other,
}

/// Renders the canonical UPPERCASE code (e.g. `MRN`, `SSN`, `OTHER`).
///
/// This is the form persisted to the database and emitted in FHIR
/// coding `code` fields, so it must stay aligned with the serde
/// representation.
impl std::fmt::Display for IdentifierType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IdentifierType::MRN => write!(f, "MRN"),
            IdentifierType::SSN => write!(f, "SSN"),
            IdentifierType::DL => write!(f, "DL"),
            IdentifierType::NPI => write!(f, "NPI"),
            IdentifierType::PPN => write!(f, "PPN"),
            IdentifierType::TAX => write!(f, "TAX"),
            IdentifierType::Other => write!(f, "OTHER"),
        }
    }
}

impl Identifier {
    /// Create an identifier from an explicit type, system, and value.
    ///
    /// `use_type` and `assigner` default to `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use person_service::models::{Identifier, IdentifierType};
    ///
    /// let id = Identifier::new(
    ///     IdentifierType::NPI,
    ///     "http://hl7.org/fhir/sid/us-npi".to_string(),
    ///     "1234567890".to_string(),
    /// );
    /// assert_eq!(id.value, "1234567890");
    /// ```
    #[must_use]
    pub fn new(identifier_type: IdentifierType, system: String, value: String) -> Self {
        Self {
            use_type: None,
            identifier_type,
            system,
            value,
            assigner: None,
        }
    }

    /// Create a Medical Record Number identifier scoped to a facility.
    ///
    /// The `system` URI is derived from the facility name so MRNs from
    /// different facilities don't collide.
    #[must_use]
    pub fn mrn(facility: &str, value: String) -> Self {
        Self::new(
            IdentifierType::MRN,
            format!("urn:oid:facility:{facility}"),
            value,
        )
    }

    /// Create a US Social Security Number identifier with the standard
    /// HL7 SSN system URI.
    #[must_use]
    pub fn ssn(value: String) -> Self {
        Self::new(
            IdentifierType::SSN,
            "http://hl7.org/fhir/sid/us-ssn".to_string(),
            value,
        )
    }
}
