//! Domain model — a case record. A *case* (governmental case
//! management / case tracking) is an open or historical matter handled
//! by a public agency on behalf of one or more subjects — a benefit
//! claim, legal action, social-services referral, licensing
//! application, complaint, appeal, and so on. The matcher models only
//! the properties that carry identity signal.

use serde::{Deserialize, Serialize};

/// Pairwise input to the matcher.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Case {
    /// Required title (e.g. "Housing benefit appeal — J. Smith").
    pub title: String,
    /// Alternative titles. Also tried when scoring `title`.
    #[serde(default)]
    pub alternate_titles: Vec<String>,
    /// Agency's local case number. Agency-scoped — only meaningful
    /// when both records share `agency_id`.
    #[serde(default)]
    pub case_number: Option<String>,
    /// Handling organization identifier (opaque to the matcher). Gates
    /// the `case_number` short-circuit and component.
    #[serde(default)]
    pub agency_id: Option<String>,
    /// Handling organization name (fallback when `agency_id` is unset).
    #[serde(default)]
    pub agency_name: Option<String>,
    /// The type of case (benefit, legal, housing, …).
    #[serde(default)]
    pub case_type: Option<CaseType>,
    /// The lifecycle status of the case.
    #[serde(default)]
    pub status: Option<CaseStatus>,
    /// Case priority. Carried for downstream consumers — never scored.
    #[serde(default)]
    pub priority: Option<Priority>,
    /// ISO-8601 date the case was opened (`YYYY` or `YYYY-MM-DD`).
    /// Carried for downstream consumers — never scored.
    #[serde(default)]
    pub opened_date: Option<String>,
    /// Opaque involved-party identifiers (e.g. person pids). Scored by
    /// overlap — a strong signal that two records describe the same
    /// matter.
    #[serde(default)]
    pub subjects: Vec<String>,
    /// Descriptive keywords / tags.
    #[serde(default)]
    pub keywords: Vec<String>,
    /// External identifiers — docket, external case id, URI, UUID, etc.
    #[serde(default)]
    pub identifiers: Vec<CaseIdentifier>,
    /// Cross-system identity URLs (court record page, registry entry).
    /// Used by the deterministic short-circuit.
    #[serde(default)]
    pub same_as: Vec<String>,
    /// BCP-47 language codes.
    #[serde(default)]
    pub in_language: Vec<String>,
}

impl Case {
    /// Construct a `Case` with just the required title; every other
    /// field defaults to empty / `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use case_matcher::Case;
    ///
    /// let c = Case::new("Housing benefit appeal — J. Smith");
    /// assert_eq!(c.title, "Housing benefit appeal — J. Smith");
    /// assert!(c.identifiers.is_empty());
    /// ```
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Default::default()
        }
    }
}

/// The type / domain of a case.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaseType {
    /// Benefit claim / entitlement.
    Benefit,
    /// Legal action / court matter.
    Legal,
    /// Social-services referral.
    SocialServices,
    /// Healthcare matter.
    Healthcare,
    /// Housing matter.
    Housing,
    /// Immigration matter.
    Immigration,
    /// Licensing / permitting.
    Licensing,
    /// Complaint.
    Complaint,
    /// Appeal.
    Appeal,
    /// Investigation.
    Investigation,
    /// Tax matter.
    Tax,
    /// Employment matter.
    Employment,
    /// Free-form custom type with a caller-supplied label.
    Custom(String),
}

/// The lifecycle status of a case.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaseStatus {
    /// Newly opened.
    Open,
    /// Actively being worked.
    InProgress,
    /// Awaiting action / information.
    Pending,
    /// Temporarily suspended.
    OnHold,
    /// Closed.
    Closed,
    /// Resolved.
    Resolved,
    /// Rejected / denied.
    Rejected,
    /// Withdrawn by the subject.
    Withdrawn,
    /// Free-form custom status with a caller-supplied label.
    Custom(String),
}

/// Case priority. Data only — never contributes to the score.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Priority {
    /// Low priority.
    Low,
    /// Normal priority.
    Normal,
    /// High priority.
    High,
    /// Urgent priority.
    Urgent,
}

/// An external identifier: a scheme plus its value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseIdentifier {
    /// The scheme under which `value` is published.
    pub scheme: IdentifierScheme,
    /// The identifier value within `scheme`.
    pub value: String,
}

/// The scheme under which an identifier's `value` is published.
///
/// Schemes marked **deterministic** (`Docket` / `ExternalCaseId` /
/// `Uri` / `Uuid`) are globally unique — a match pins the score to
/// `1.0` via the R-0 short-circuit. **Agency-scoped** schemes
/// (`AgencyCaseNumber`, `LocalId`) only make sense within their
/// handling organisation and are intentionally NOT deterministic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IdentifierScheme {
    /// Court / tribunal docket number. **Deterministic.**
    Docket,
    /// Cross-system external case id. **Deterministic.**
    ExternalCaseId,
    /// Generic URI / URN. **Deterministic.**
    Uri,
    /// Bare UUID. **Deterministic.**
    Uuid,
    /// Agency's local case number. Agency-scoped — short-circuits only
    /// via R-1 (`agency_id + case_number`).
    AgencyCaseNumber,
    /// Agency's local record id. Agency-scoped.
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
    /// use case_matcher::IdentifierScheme;
    ///
    /// assert!(IdentifierScheme::Docket.is_deterministic());
    /// assert!(!IdentifierScheme::AgencyCaseNumber.is_deterministic());
    /// ```
    #[must_use]
    pub fn is_deterministic(&self) -> bool {
        matches!(
            self,
            IdentifierScheme::Docket
                | IdentifierScheme::ExternalCaseId
                | IdentifierScheme::Uri
                | IdentifierScheme::Uuid
        )
    }
}
