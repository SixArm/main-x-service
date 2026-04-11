//! FHIR R5 resource definitions

use serde::{Deserialize, Serialize};

/// FHIR Worker resource (R5)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirWorker {
    pub resource_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<FhirMeta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<Vec<FhirIdentifier>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<Vec<FhirHumanName>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub telecom: Option<Vec<FhirContactPoint>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gender: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub birth_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deceased: Option<FhirDeceased>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<Vec<FhirAddress>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub marital_status: Option<FhirCodeableConcept>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multiple_birth: Option<FhirMultipleBirth>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub photo: Option<Vec<FhirAttachment>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<Vec<FhirWorkerLink>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub managing_organization: Option<FhirReference>,
}

/// FHIR Meta element
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_updated: Option<String>,
}

/// FHIR Identifier
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirIdentifier {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_: Option<FhirCodeableConcept>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigner: Option<FhirReference>,
}

/// FHIR HumanName
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirHumanName {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub given: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<Vec<String>>,
}

/// FHIR ContactPoint
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirContactPoint {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_: Option<String>,
}

/// FHIR Address
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirAddress {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postal_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
}

/// FHIR CodeableConcept
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirCodeableConcept {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coding: Option<Vec<FhirCoding>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// FHIR Coding
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirCoding {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
}

/// FHIR Reference
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirReference {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
}

/// FHIR Worker Link
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirWorkerLink {
    pub other: FhirReference,
    pub type_: String,
}

/// FHIR Attachment
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirAttachment {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// FHIR Deceased (boolean or dateTime)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FhirDeceased {
    Boolean(bool),
    DateTime(String),
}

/// FHIR MultipleBirth (boolean or integer)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FhirMultipleBirth {
    Boolean(bool),
    Integer(i32),
}

/// FHIR OperationOutcome for errors
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirOperationOutcome {
    pub resource_type: String,
    pub issue: Vec<FhirOperationOutcomeIssue>,
}

/// FHIR OperationOutcome Issue
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FhirOperationOutcomeIssue {
    pub severity: String,
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<FhirCodeableConcept>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<String>,
}

impl FhirOperationOutcome {
    /// Create an error OperationOutcome
    pub fn error(code: &str, diagnostics: &str) -> Self {
        Self {
            resource_type: "OperationOutcome".to_string(),
            issue: vec![FhirOperationOutcomeIssue {
                severity: "error".to_string(),
                code: code.to_string(),
                details: None,
                diagnostics: Some(diagnostics.to_string()),
            }],
        }
    }

    /// Create a not found OperationOutcome
    pub fn not_found(resource_type: &str, id: &str) -> Self {
        Self::error(
            "not-found",
            &format!("{} with id '{}' not found", resource_type, id),
        )
    }

    /// Create an invalid OperationOutcome
    pub fn invalid(message: &str) -> Self {
        Self::error("invalid", message)
    }
}

impl FhirWorker {
    /// Create a new minimal FHIR Worker
    pub fn new() -> Self {
        Self {
            resource_type: "Worker".to_string(),
            id: None,
            meta: None,
            identifier: None,
            active: None,
            name: None,
            telecom: None,
            gender: None,
            birth_date: None,
            deceased: None,
            address: None,
            marital_status: None,
            multiple_birth: None,
            photo: None,
            link: None,
            managing_organization: None,
        }
    }
}

impl Default for FhirWorker {
    fn default() -> Self {
        Self::new()
    }
}
