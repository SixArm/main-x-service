//! Domain model — a plan record. A *plan* is a named unit of
//! intended work in a portfolio / project-management registry, forming
//! one recursive tree: any plan may contain any other via
//! `parent_ref`. The former `Portfolio` / `Project` / `Product` /
//! `Program` kinds were unified into this single type; `kind` survives
//! only as **optional descriptive metadata** (a display / grouping
//! label, since extended with `Practice` / `Process` / `Purpose` /
//! `Pathway` / `Proposal`) and no longer gates matching or fixes a
//! collection. The
//! matcher models only the properties that carry identity signal; the
//! high-volume operational sub-resources (tasks, issues) live in the
//! service, not here.

use serde::{Deserialize, Serialize};

/// Pairwise input to the matcher — the thin matchable record.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Plan {
    /// The kind of plan (`Portfolio` / `Project` / `Product` /
    /// `Program` / `Practice` / `Process` / `Purpose` / `Pathway` /
    /// `Proposal`), or `None`. **Optional descriptive metadata** — since
    /// the kinds were
    /// unified into one recursive plan tree, `kind` no longer gates
    /// matching or fixes a collection; it is a label a caller may set
    /// for grouping / display.
    #[serde(default)]
    pub kind: Option<PlanKind>,
    /// Required name (e.g. "Apollo platform migration").
    pub name: String,
    /// Alternative names / former titles / codenames. Also tried when
    /// scoring `name`.
    #[serde(default)]
    pub alternate_names: Vec<String>,
    /// Owner-scoped code (e.g. `PROJ-2026`). Only meaningful within the
    /// same `owner_org_id`.
    #[serde(default)]
    pub code: Option<String>,
    /// Sponsoring / owning organisation identifier (opaque `EntityRef`).
    /// Gates the `code` short-circuit and component; scored exactly.
    #[serde(default)]
    pub owner_org_id: Option<String>,
    /// Owning organisation display name. Informational-only — never
    /// scored, never gates anything.
    #[serde(default)]
    pub owner_org_name: Option<String>,
    /// The plan lead (`person:<id>` / `worker:<id>` `EntityRef`).
    /// Informational-only — a cross-service reference, never scored.
    #[serde(default)]
    pub lead_ref: Option<String>,
    /// Parent plan `pid` (the containment link) — any plan may
    /// contain any other. An exact supporting signal; absent for a root
    /// plan.
    #[serde(default)]
    pub parent_ref: Option<String>,
    /// Lifecycle status. Informational-only — never scored (the same
    /// initiative routinely sits at different statuses).
    #[serde(default)]
    pub status: Option<PlanStatus>,
    /// The sequential project phase this plan is being *managed*
    /// through. Informational-only — **never scored**, for exactly the
    /// reason `status` is not: two records of one initiative may sit in
    /// different phases, and the phase is precisely the field most
    /// likely to differ between two systems describing one plan.
    #[serde(default)]
    pub phase: Option<PlanPhase>,
    /// Plan objectives. **Part of the payload**; the goal *titles*
    /// feed the `goals` component.
    #[serde(default)]
    pub goals: Vec<Goal>,
    /// Planned / actual start date (`YYYY`, `YYYY-MM`, or `YYYY-MM-DD`).
    /// Feeds the timeframe component.
    #[serde(default)]
    pub start_date: Option<String>,
    /// Planned completion / due date. Feeds the timeframe component.
    #[serde(default)]
    pub target_date: Option<String>,
    /// Descriptive / discovery terms (what the plan *is*). Scored by
    /// Jaccard overlap.
    #[serde(default)]
    pub keywords: Vec<String>,
    /// Operator-applied labels for grouping / workflow. Scored by Jaccard
    /// overlap (a supporting signal).
    #[serde(default)]
    pub tags: Vec<String>,
    /// External / tool identifiers — Jira project key, Asana GID, URI,
    /// UUID, etc. Used by the deterministic short-circuit.
    #[serde(default)]
    pub identifiers: Vec<PlanIdentifier>,
    /// Cross-system identity URLs (schema.org `sameAs`). Used by the
    /// deterministic short-circuit.
    #[serde(default)]
    pub same_as: Vec<String>,
    /// BCP-47 language code. Informational-only — never scored.
    #[serde(default)]
    pub in_language: Option<String>,
    /// Typed within-entity links to other plans. Scored by typed-set
    /// Jaccard (a supporting signal).
    #[serde(default)]
    pub relationships: Vec<PlanRelationship>,
}

impl Plan {
    /// Construct a `Plan` with just the required name; every other
    /// field defaults to empty / `None` (including `kind`).
    ///
    /// # Examples
    ///
    /// ```
    /// use project_portfolio_management_matcher::Plan;
    ///
    /// let w = Plan::new("Apollo platform migration");
    /// assert_eq!(w.name, "Apollo platform migration");
    /// assert!(w.kind.is_none());
    /// assert!(w.identifiers.is_empty());
    /// ```
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }
}

/// A descriptive kind label for a plan. Since the kinds were unified
/// into one recursive plan tree this is **optional metadata** (see
/// [`Plan::kind`]) — it no longer fixes a collection or gates
/// matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanKind {
    /// An umbrella container that groups child plans.
    Portfolio,
    /// A project.
    Project,
    /// A product.
    Product,
    /// A programme.
    Program,
    /// A practice — an ongoing discipline or capability.
    Practice,
    /// A process — a repeatable way of working.
    Process,
    /// A purpose — an intent or mission the work serves.
    Purpose,
    /// A pathway — a route through a sequence of plans.
    Pathway,
    /// A proposal — work put forward for approval.
    ///
    /// Note: this is only a descriptive label on a `Plan`; it is
    /// unrelated to the service's separate `proposals` intake pipeline.
    Proposal,
}

/// The sequential project phase a plan is managed through — the
/// classic five process groups. Data only — **never scored**.
///
/// This is one of **three ordered vocabularies** in this entity and
/// they are deliberately uncoupled (entity spec §1.5.1): the lifecycle
/// funnel (`idea` … `closed`) says where an item of demand sits
/// portfolio-wide, the gate stage (`g0` … `g5`) says what the last
/// approved governance decision was, and this says where management of
/// the plan has got to. No constraint is enforced between them: a rule
/// such as "not Executing before g3" would make a true state
/// unrecordable whenever governance lags delivery, which is the case
/// most worth being able to see.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PlanPhase {
    /// Should this exist? Sponsor, purpose, lead.
    Initiating,
    /// What is the work, in what order, at what cost, with what risks?
    Planning,
    /// Doing it — where the board, sprints and effort belong.
    Executing,
    /// Is it going as planned, and what is being changed in response?
    Controlling,
    /// Formal completion — acceptance, handover, lessons, release.
    Closing,
}

impl PlanPhase {
    /// Every phase, in process order.
    pub const ALL: [Self; 5] = [
        Self::Initiating,
        Self::Planning,
        Self::Executing,
        Self::Controlling,
        Self::Closing,
    ];

    /// Position in the sequence, `0`-based.
    #[must_use]
    pub const fn ordinal(self) -> usize {
        match self {
            Self::Initiating => 0,
            Self::Planning => 1,
            Self::Executing => 2,
            Self::Controlling => 3,
            Self::Closing => 4,
        }
    }

    /// The wire token (`initiating`, `planning`, …).
    #[must_use]
    pub const fn token(self) -> &'static str {
        match self {
            Self::Initiating => "initiating",
            Self::Planning => "planning",
            Self::Executing => "executing",
            Self::Controlling => "controlling",
            Self::Closing => "closing",
        }
    }

    /// Parse a wire token. Unknown input is `None` — refused by the
    /// caller rather than coerced to a default, so a typo can never
    /// silently place a plan in Initiating.
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|phase| phase.token() == raw.trim().to_ascii_lowercase())
    }
}

/// The lifecycle status of a plan. Data only — never scored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanStatus {
    /// Proposed / not yet started.
    Proposed,
    /// Actively being worked.
    Active,
    /// Temporarily suspended.
    OnHold,
    /// Completed.
    Completed,
    /// Cancelled.
    Cancelled,
    /// Free-form custom status with a caller-supplied label.
    Custom(String),
}

/// A plan objective. Only the `title` feeds matching; the rest is
/// informational and carried for downstream consumers.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Goal {
    /// The goal title — the matchable surface (folded into the goals set).
    pub title: String,
    /// Optional longer description. Never scored.
    #[serde(default)]
    pub description: Option<String>,
    /// Optional target date (`YYYY-MM-DD`). Never scored by the matcher.
    #[serde(default)]
    pub target_date: Option<String>,
    /// Optional goal status. Never scored.
    #[serde(default)]
    pub status: Option<GoalStatus>,
}

/// The status of a single goal. Data only — never scored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GoalStatus {
    /// Not started.
    NotStarted,
    /// In progress.
    InProgress,
    /// Achieved.
    Achieved,
    /// Missed.
    Missed,
    /// Free-form custom status with a caller-supplied label.
    Custom(String),
}

/// A typed within-entity link to another plan in the registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanRelationship {
    /// The kind of relationship.
    pub relation: RelationKind,
    /// The opaque registry id of the referenced plan.
    pub plan_id: String,
}

/// The kind of a within-entity plan-to-plan relationship. The
/// matcher compares relationship *sets* opaquely — it does not invert or
/// transitively close them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationKind {
    /// This plan is the parent of the target (hierarchy).
    ParentOf,
    /// This plan is a child of the target (hierarchy).
    ChildOf,
    /// This plan depends on the target.
    DependsOn,
    /// This plan is blocked by the target.
    BlockedBy,
    /// This plan supersedes the target (versioning).
    Supersedes,
    /// This plan is superseded by the target (versioning).
    SupersededBy,
    /// A comparable plan (symmetric).
    SimilarTo,
    /// A loosely associated plan (symmetric).
    RelatedTo,
    /// Free-form custom relation with a caller-supplied label.
    Custom(String),
}

/// An external / tool identifier: a scheme plus its value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanIdentifier {
    /// The scheme under which `value` is published.
    pub scheme: IdentifierScheme,
    /// The identifier value within `scheme`.
    pub value: String,
}

/// The scheme under which an identifier's `value` is published.
///
/// Schemes marked **deterministic** (the tool/registry ids plus `Uri` /
/// `Uuid`) are globally unique — a match pins the score to `1.0` via the
/// R-0 short-circuit. **Owner-scoped** schemes (`Code`, `LocalId`) only
/// make sense within their owning organisation and are intentionally NOT
/// deterministic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IdentifierScheme {
    /// Generic URI / URN. **Deterministic.**
    Uri,
    /// Bare UUID. **Deterministic.**
    Uuid,
    /// Jira project key. **Deterministic.**
    JiraProjectKey,
    /// Asana GID. **Deterministic.**
    AsanaGid,
    /// Trello board id. **Deterministic.**
    TrelloBoardId,
    /// Microsoft Project id. **Deterministic.**
    MsProjectId,
    /// GitHub project id. **Deterministic.**
    GitHubProjectId,
    /// Linear id. **Deterministic.**
    LinearId,
    /// Owner-scoped code. Short-circuits only via R-1
    /// (`owner_org_id + code`).
    Code,
    /// Owner-scoped local record id.
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
    /// use project_portfolio_management_matcher::IdentifierScheme;
    ///
    /// assert!(IdentifierScheme::JiraProjectKey.is_deterministic());
    /// assert!(!IdentifierScheme::Code.is_deterministic());
    /// ```
    #[must_use]
    pub fn is_deterministic(&self) -> bool {
        // Only globally-unique schemes are listed. Owner-scoped schemes
        // (Code / LocalId) and Custom are intentionally excluded — their
        // values collide across organisations, so a bare value match
        // cannot prove identity.
        matches!(
            self,
            IdentifierScheme::Uri
                | IdentifierScheme::Uuid
                | IdentifierScheme::JiraProjectKey
                | IdentifierScheme::AsanaGid
                | IdentifierScheme::TrelloBoardId
                | IdentifierScheme::MsProjectId
                | IdentifierScheme::GitHubProjectId
                | IdentifierScheme::LinearId
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_sets_name_and_leaves_kind_none() {
        let w = Plan::new("Delivery programme");
        assert!(w.kind.is_none());
        assert_eq!(w.name, "Delivery programme");
        assert!(w.goals.is_empty());
    }

    #[test]
    fn default_kind_is_none() {
        assert!(Plan::default().kind.is_none());
    }

    #[test]
    fn kind_is_optional_descriptive_metadata() {
        let mut w = Plan::new("Apollo");
        w.kind = Some(PlanKind::Project);
        assert_eq!(w.kind, Some(PlanKind::Project));
    }

    #[test]
    fn deterministic_schemes_are_exactly_the_tool_and_uri_uuid_set() {
        for s in [
            IdentifierScheme::Uri,
            IdentifierScheme::Uuid,
            IdentifierScheme::JiraProjectKey,
            IdentifierScheme::AsanaGid,
            IdentifierScheme::TrelloBoardId,
            IdentifierScheme::MsProjectId,
            IdentifierScheme::GitHubProjectId,
            IdentifierScheme::LinearId,
        ] {
            assert!(s.is_deterministic(), "{s:?} should be deterministic");
        }
        for s in [
            IdentifierScheme::Code,
            IdentifierScheme::LocalId,
            IdentifierScheme::Custom("legacy".into()),
        ] {
            assert!(!s.is_deterministic(), "{s:?} must not be deterministic");
        }
    }

    // The `kind` field is optional on deserialisation (serde default None).
    #[test]
    fn deserialize_kind_is_optional() {
        let with: Result<Plan, _> = serde_json::from_str(r#"{"kind":"Project","name":"X"}"#);
        assert_eq!(with.unwrap().kind, Some(PlanKind::Project));
        let without: Plan = serde_json::from_str(r#"{"name":"X"}"#).unwrap();
        assert!(without.kind.is_none(), "missing kind deserializes to None");
    }

    // The payload round-trips losslessly through serde_json.
    #[test]
    fn round_trips_through_json() {
        let mut w = Plan::new("Apollo");
        w.code = Some("PROJ-1".into());
        w.owner_org_id = Some("organization:9a2f".into());
        w.parent_ref = Some("0c4f-portfolio".into());
        w.goals = vec![Goal {
            title: "Cut latency".into(),
            ..Default::default()
        }];
        w.tags = vec!["q3".into()];
        w.relationships = vec![PlanRelationship {
            relation: RelationKind::DependsOn,
            plan_id: "proj-2".into(),
        }];
        let json = serde_json::to_string(&w).unwrap();
        let back: Plan = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, w.name);
        assert_eq!(back.kind, w.kind);
        assert_eq!(back.parent_ref, w.parent_ref);
        assert_eq!(back.goals.len(), 1);
        assert_eq!(back.relationships.len(), 1);
    }
}
