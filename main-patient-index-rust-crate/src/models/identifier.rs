//! Identifier model definition

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Patient or organization identifier (MRN, SSN, NPI, etc.)
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

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum IdentifierUse {
    /// The identifier recommended for display and use in real-world interactions
    Usual,
    /// The identifier considered to be most trusted for this patient
    Official,
    /// A temporary identifier
    Temp,
    /// An identifier that was assigned in secondary use
    Secondary,
    /// The identifier id no longer considered valid
    Old,
}

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
    /// Create a new identifier
    pub fn new(
        identifier_type: IdentifierType,
        system: String,
        value: String,
    ) -> Self {
        Self {
            use_type: None,
            identifier_type,
            system,
            value,
            assigner: None,
        }
    }

    /// Create a Medical Record Number identifier
    pub fn mrn(facility: String, value: String) -> Self {
        Self::new(
            IdentifierType::MRN,
            format!("urn:oid:facility:{}", facility),
            value,
        )
    }

    /// Create a Social Security Number identifier
    pub fn ssn(value: String) -> Self {
        Self::new(
            IdentifierType::SSN,
            "http://hl7.org/fhir/sid/us-ssn".to_string(),
            value,
        )
    }
}
