//! Domain model — a work-item record. A *work item* is a named unit of
//! intended work in a portfolio / project-management registry. It comes
//! in four **kinds**: a `Portfolio` (the umbrella container), or a
//! `Project` / `Product` / `Program` that sits under a portfolio. The
//! kind is the collection / table the record lives in, and the matcher
//! treats it as a **hard gate**: two work items of different kind are
//! distinct record types and never match. The matcher models only the
//! properties that carry identity signal; the high-volume operational
//! sub-resources (tasks, issues) live in the service, not here.

use serde::{Deserialize, Serialize};

/// Pairwise input to the matcher — the thin matchable record.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkItem {
    /// The kind of work item (`Portfolio` / `Project` / `Product` /
    /// `Program`). **Required** — it is the collection the record lives
    /// in and a hard match gate. Deserialisation rejects a record with no
    /// `kind` (no serde default), so a stored payload always carries one.
    pub kind: WorkItemKind,
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
    /// The work-item lead (`person:<id>` / `worker:<id>` `EntityRef`).
    /// Informational-only — a cross-service reference, never scored.
    #[serde(default)]
    pub lead_ref: Option<String>,
    /// Parent portfolio `pid` for a child kind (the umbrella link). An
    /// exact supporting signal for child kinds; absent / ignored for the
    /// `Portfolio` kind.
    #[serde(default)]
    pub portfolio_ref: Option<String>,
    /// Lifecycle status. Informational-only — never scored (the same
    /// initiative routinely sits at different statuses).
    #[serde(default)]
    pub status: Option<WorkItemStatus>,
    /// Work-item objectives. **Part of the payload**; the goal *titles*
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
    /// Descriptive / discovery terms (what the work item *is*). Scored by
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
    pub identifiers: Vec<WorkItemIdentifier>,
    /// Cross-system identity URLs (schema.org `sameAs`). Used by the
    /// deterministic short-circuit.
    #[serde(default)]
    pub same_as: Vec<String>,
    /// BCP-47 language code. Informational-only — never scored.
    #[serde(default)]
    pub in_language: Option<String>,
    /// Typed within-entity links to other work items. Scored by typed-set
    /// Jaccard (a supporting signal).
    #[serde(default)]
    pub relationships: Vec<WorkItemRelationship>,
}

impl WorkItem {
    /// Construct a `WorkItem` of the given kind with just the required
    /// name; every other field defaults to empty / `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use portfolio_matcher::{WorkItem, WorkItemKind};
    ///
    /// let w = WorkItem::new(WorkItemKind::Project, "Apollo platform migration");
    /// assert_eq!(w.kind, WorkItemKind::Project);
    /// assert_eq!(w.name, "Apollo platform migration");
    /// assert!(w.identifiers.is_empty());
    /// ```
    #[must_use]
    pub fn new(kind: WorkItemKind, name: impl Into<String>) -> Self {
        Self {
            kind,
            name: name.into(),
            ..Default::default()
        }
    }

    /// True for the child kinds (`Project` / `Product` / `Program`) that
    /// sit under a portfolio and may carry a `portfolio_ref`; false for
    /// the umbrella `Portfolio` kind.
    #[must_use]
    pub fn is_child_kind(&self) -> bool {
        self.kind.is_child()
    }
}

/// The kind of a work item — the collection / table it lives in. A
/// **closed** set (no `Custom` arm): each variant maps to a fixed table
/// and REST collection, and the matcher gates on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum WorkItemKind {
    /// The umbrella container that groups child work items. The default
    /// when constructing a bare record.
    #[default]
    Portfolio,
    /// A project under a portfolio.
    Project,
    /// A product under a portfolio.
    Product,
    /// A programme under a portfolio.
    Program,
}

impl WorkItemKind {
    /// True for the child kinds (`Project` / `Product` / `Program`);
    /// false for `Portfolio`. Only child kinds carry a `portfolio_ref`
    /// and participate in the parent-portfolio component.
    #[must_use]
    pub fn is_child(self) -> bool {
        !matches!(self, WorkItemKind::Portfolio)
    }
}

/// The lifecycle status of a work item. Data only — never scored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkItemStatus {
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

/// A work-item objective. Only the `title` feeds matching; the rest is
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

/// A typed within-entity link to another work item in the registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkItemRelationship {
    /// The kind of relationship.
    pub relation: RelationKind,
    /// The opaque registry id of the referenced work item.
    pub work_item_id: String,
}

/// The kind of a within-entity work-item-to-work-item relationship. The
/// matcher compares relationship *sets* opaquely — it does not invert or
/// transitively close them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationKind {
    /// This work item is the parent of the target (hierarchy).
    ParentOf,
    /// This work item is a child of the target (hierarchy).
    ChildOf,
    /// This work item depends on the target.
    DependsOn,
    /// This work item is blocked by the target.
    BlockedBy,
    /// This work item supersedes the target (versioning).
    Supersedes,
    /// This work item is superseded by the target (versioning).
    SupersededBy,
    /// A comparable work item (symmetric).
    SimilarTo,
    /// A loosely associated work item (symmetric).
    RelatedTo,
    /// Free-form custom relation with a caller-supplied label.
    Custom(String),
}

/// An external / tool identifier: a scheme plus its value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkItemIdentifier {
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
    /// use portfolio_matcher::IdentifierScheme;
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
    fn new_sets_kind_and_name() {
        let w = WorkItem::new(WorkItemKind::Program, "Delivery programme");
        assert_eq!(w.kind, WorkItemKind::Program);
        assert_eq!(w.name, "Delivery programme");
        assert!(w.goals.is_empty());
    }

    #[test]
    fn default_kind_is_portfolio() {
        assert_eq!(WorkItemKind::default(), WorkItemKind::Portfolio);
        assert_eq!(WorkItem::default().kind, WorkItemKind::Portfolio);
    }

    #[test]
    fn child_kinds_classified() {
        assert!(!WorkItemKind::Portfolio.is_child());
        assert!(WorkItemKind::Project.is_child());
        assert!(WorkItemKind::Product.is_child());
        assert!(WorkItemKind::Program.is_child());
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

    // The `kind` field is required on deserialisation (no serde default).
    #[test]
    fn deserialize_requires_kind() {
        let ok: Result<WorkItem, _> = serde_json::from_str(r#"{"kind":"Project","name":"X"}"#);
        assert!(ok.is_ok());
        let missing: Result<WorkItem, _> = serde_json::from_str(r#"{"name":"X"}"#);
        assert!(missing.is_err(), "missing kind must fail to deserialize");
    }

    // The payload round-trips losslessly through serde_json.
    #[test]
    fn round_trips_through_json() {
        let mut w = WorkItem::new(WorkItemKind::Project, "Apollo");
        w.code = Some("PROJ-1".into());
        w.owner_org_id = Some("organization:9a2f".into());
        w.portfolio_ref = Some("0c4f-portfolio".into());
        w.goals = vec![Goal {
            title: "Cut latency".into(),
            ..Default::default()
        }];
        w.tags = vec!["q3".into()];
        w.relationships = vec![WorkItemRelationship {
            relation: RelationKind::DependsOn,
            work_item_id: "proj-2".into(),
        }];
        let json = serde_json::to_string(&w).unwrap();
        let back: WorkItem = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, w.name);
        assert_eq!(back.kind, w.kind);
        assert_eq!(back.portfolio_ref, w.portfolio_ref);
        assert_eq!(back.goals.len(), 1);
        assert_eq!(back.relationships.len(), 1);
    }
}
