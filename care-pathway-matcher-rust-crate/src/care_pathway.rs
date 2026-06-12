//! Domain model — a care-pathway record. A *care pathway* (also
//! "clinical pathway", "critical pathway", "integrated care pathway") is
//! a structured, evidence-based, multidisciplinary plan of care for a
//! specific clinical condition or patient group over a defined episode.
//! The matcher models only the properties that carry identity signal.

use serde::{Deserialize, Serialize};

/// Pairwise input to the matcher.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CarePathway {
    /// Required title (e.g. "Acute Stroke Care Pathway").
    pub name: String,
    /// Alternative titles / abbreviations. Also tried when scoring `name`.
    #[serde(default)]
    pub alternate_names: Vec<String>,
    /// Provider's local pathway code. Provider-scoped — only meaningful
    /// when both records share `provider_id`.
    #[serde(default)]
    pub pathway_code: Option<String>,
    /// Issuing organization identifier (opaque to the matcher). Gates
    /// the `pathway_code` short-circuit and component.
    #[serde(default)]
    pub provider_id: Option<String>,
    /// Issuing organization name (fallback when `provider_id` is unset).
    #[serde(default)]
    pub provider_name: Option<String>,
    /// Care setting the pathway applies to.
    #[serde(default)]
    pub care_setting: Option<CareSetting>,
    /// Target clinical conditions (ICD / SNOMED codes). The defining
    /// attribute of a pathway — scored by overlap.
    #[serde(default)]
    pub condition_codes: Vec<ConditionCode>,
    /// Key interventions / pathway steps.
    #[serde(default)]
    pub interventions: Vec<String>,
    /// Descriptive keywords / tags.
    #[serde(default)]
    pub keywords: Vec<String>,
    /// External identifiers — DOI, guideline id, URI, UUID, etc.
    #[serde(default)]
    pub identifiers: Vec<PathwayIdentifier>,
    /// Cross-system identity URLs (guideline page, registry entry).
    /// Used by the deterministic short-circuit.
    #[serde(default)]
    pub same_as: Vec<String>,
    /// BCP-47 language codes.
    #[serde(default)]
    pub in_language: Vec<String>,
}

impl CarePathway {
    /// Construct a `CarePathway` with just the required name; every other
    /// field defaults to empty / `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use care_pathway_matcher::CarePathway;
    ///
    /// let p = CarePathway::new("Acute Stroke Care Pathway");
    /// assert_eq!(p.name, "Acute Stroke Care Pathway");
    /// assert!(p.identifiers.is_empty());
    /// ```
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }
}

/// A target clinical condition: a coding system plus its code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionCode {
    /// The coding system the `code` belongs to.
    pub system: CodeSystem,
    /// The code value within `system`.
    pub code: String,
}

/// Clinical coding system for a [`ConditionCode`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CodeSystem {
    /// ICD-10.
    Icd10,
    /// ICD-11.
    Icd11,
    /// SNOMED CT.
    Snomed,
    /// Free-form custom system with a caller-supplied label.
    Custom(String),
}

/// The setting a care pathway applies to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CareSetting {
    /// Inpatient / hospital ward.
    Inpatient,
    /// Outpatient / ambulatory clinic.
    Outpatient,
    /// Primary care / general practice.
    PrimaryCare,
    /// Emergency department.
    EmergencyDepartment,
    /// Community care.
    Community,
    /// Home care.
    HomeCare,
    /// Rehabilitation.
    Rehabilitation,
    /// Mental health.
    MentalHealth,
    /// Palliative / end-of-life.
    Palliative,
    /// Free-form custom setting with a caller-supplied label.
    Custom(String),
}

/// An external identifier: a scheme plus its value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathwayIdentifier {
    /// The scheme under which `value` is published.
    pub scheme: IdentifierScheme,
    /// The identifier value within `scheme`.
    pub value: String,
}

/// The scheme under which an identifier's `value` is published.
///
/// Schemes marked **deterministic** (DOI / Wikidata / `GuidelineId` /
/// URI / UUID) are globally unique — a match pins the score to `1.0` via the
/// R-0 short-circuit. **Provider-scoped** schemes (`PathwayCode`,
/// `LocalId`) only make sense within their issuing organisation and are
/// intentionally NOT deterministic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IdentifierScheme {
    /// Digital Object Identifier. **Deterministic.**
    Doi,
    /// Wikidata entity id. **Deterministic.**
    Wikidata,
    /// Clinical-guideline registry id (e.g. NICE `NG128`). **Deterministic.**
    GuidelineId,
    /// Generic URI / URN. **Deterministic.**
    Uri,
    /// Bare UUID. **Deterministic.**
    Uuid,
    /// Provider's local pathway code. Provider-scoped — short-circuits
    /// only via R-1 (`provider_id + pathway_code`).
    PathwayCode,
    /// Provider's local record id. Provider-scoped.
    LocalId,
    /// Free-form custom scheme with a caller-supplied label.
    /// Non-deterministic.
    Custom(String),
}

impl IdentifierScheme {
    /// Schemes whose values are globally unique. A match pins the score
    /// to `1.0`.
    ///
    /// # Examples
    ///
    /// ```
    /// use care_pathway_matcher::IdentifierScheme;
    ///
    /// assert!(IdentifierScheme::Doi.is_deterministic());
    /// assert!(!IdentifierScheme::PathwayCode.is_deterministic());
    /// ```
    #[must_use]
    pub fn is_deterministic(&self) -> bool {
        matches!(
            self,
            IdentifierScheme::Doi
                | IdentifierScheme::Wikidata
                | IdentifierScheme::GuidelineId
                | IdentifierScheme::Uri
                | IdentifierScheme::Uuid
        )
    }
}
