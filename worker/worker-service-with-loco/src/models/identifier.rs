//! External identifiers for workers and organizations.
//!
//! An [`Identifier`] pairs a *type* ([`IdentifierType`] — MRN, SSN, NPI, …)
//! with a *system* (a namespace URI) and a *value*. The type + system + value
//! triple is what the matcher compares: two identifiers only match when their
//! type and system agree and their values are equal (see
//! `crate::matching::algorithms`). Convenience constructors
//! ([`Identifier::mrn`], [`Identifier::ssn`]) fill in the well-known system
//! URIs so callers do not have to memorize them.
//!
//! # Examples
//!
//! ```
//! use worker_service::models::{Identifier, IdentifierType};
//!
//! let ssn = Identifier::ssn("123-45-6789".into());
//! assert_eq!(ssn.identifier_type, IdentifierType::SSN);
//! assert_eq!(ssn.system, "http://hl7.org/fhir/sid/us-ssn");
//! assert_eq!(ssn.value, "123-45-6789");
//! ```

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A worker or organization identifier — a type + system + value triple.
///
/// The matcher keys equality on type, system, and value, so the `system`
/// namespace URI matters: the same value under two different systems is *not*
/// a match.
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

/// How an [`Identifier`] is used, mirroring the FHIR `identifier-use` value
/// set. Serializes in lowercase.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum IdentifierUse {
    /// The identifier recommended for display and use in real-world interactions
    Usual,
    /// The identifier considered to be most trusted for this worker
    Official,
    /// A temporary identifier
    Temp,
    /// An identifier that was assigned in secondary use
    Secondary,
    /// The identifier id no longer considered valid
    Old,
}

/// The kind of an [`Identifier`]. Serializes in UPPERCASE; unknown wire values
/// deserialize to [`Other`](Self::Other) via `#[serde(other)]`.
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
    /// ODS Organisation Code
    ODS,
    /// Other identifier type
    #[serde(other)]
    Other,
}

/// Renders the UPPERCASE wire token for each variant (e.g.
/// `IdentifierType::SSN` → `"SSN"`, `IdentifierType::Other` → `"OTHER"`).
impl std::fmt::Display for IdentifierType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IdentifierType::MRN => write!(f, "MRN"),
            IdentifierType::SSN => write!(f, "SSN"),
            IdentifierType::DL => write!(f, "DL"),
            IdentifierType::NPI => write!(f, "NPI"),
            IdentifierType::PPN => write!(f, "PPN"),
            IdentifierType::TAX => write!(f, "TAX"),
            IdentifierType::ODS => write!(f, "ODS"),
            IdentifierType::Other => write!(f, "OTHER"),
        }
    }
}

impl Identifier {
    /// Creates an identifier with an explicit type, system, and value.
    ///
    /// [`use_type`](Self::use_type) and [`assigner`](Self::assigner) default to
    /// `None`; set them afterward if needed.
    ///
    /// # Examples
    ///
    /// ```
    /// use worker_service::models::{Identifier, IdentifierType};
    ///
    /// let id = Identifier::new(
    ///     IdentifierType::NPI,
    ///     "http://hl7.org/fhir/sid/us-npi".into(),
    ///     "1234567893".into(),
    /// );
    /// assert_eq!(id.identifier_type, IdentifierType::NPI);
    /// assert!(id.use_type.is_none());
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

    /// Creates a Medical Record Number identifier scoped to a facility.
    ///
    /// The `facility` becomes part of the system URI (`urn:oid:facility:<facility>`),
    /// so MRNs from different facilities never collide during matching.
    ///
    /// # Examples
    ///
    /// ```
    /// use worker_service::models::{Identifier, IdentifierType};
    ///
    /// let mrn = Identifier::mrn("HOSP-A", "00112233".into());
    /// assert_eq!(mrn.identifier_type, IdentifierType::MRN);
    /// assert_eq!(mrn.system, "urn:oid:facility:HOSP-A");
    /// ```
    #[must_use]
    pub fn mrn(facility: &str, value: String) -> Self {
        Self::new(
            IdentifierType::MRN,
            // Embed the facility so the same MRN value at two facilities is
            // treated as two distinct identifiers.
            format!("urn:oid:facility:{facility}"),
            value,
        )
    }

    /// Creates a US Social Security Number identifier under the canonical HL7
    /// SSN system URI.
    ///
    /// # Examples
    ///
    /// ```
    /// use worker_service::models::Identifier;
    ///
    /// let ssn = Identifier::ssn("078-05-1120".into());
    /// assert_eq!(ssn.system, "http://hl7.org/fhir/sid/us-ssn");
    /// ```
    #[must_use]
    pub fn ssn(value: String) -> Self {
        Self::new(
            IdentifierType::SSN,
            "http://hl7.org/fhir/sid/us-ssn".to_string(),
            value,
        )
    }
}
