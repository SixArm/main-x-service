// Types mirroring the Portfolio Service payload, which is the
// `project_portfolio_management_matcher::Plan` shape itself.
// Source of truth: project-portfolio-management-matcher-rust-crate/src/plan.rs.
//
// The service now exposes ONE recursive `/api/plans` collection: a plan may
// contain any other plan (via `parent_ref`), and `kind` is only an optional
// descriptive label — it no longer selects a collection or gates matching.

/** The plan kinds — an OPTIONAL descriptive label. Closed set. */
export type PlanKind =
  | "Portfolio"
  | "Project"
  | "Product"
  | "Program"
  | "Practice"
  | "Process"
  | "Purpose"
  | "Pathway"
  | "Proposal";

/** All kinds, for the optional-label `<select>` in the form. */
export const ALL_KINDS: PlanKind[] = [
  "Portfolio",
  "Project",
  "Product",
  "Program",
  "Practice",
  "Process",
  "Purpose",
  "Pathway",
  "Proposal",
];

/**
 * Legacy "collection" segments, kept only for the PPM catalogue wire
 * shapes (proposals / ideas / reports) whose backend endpoints were not
 * part of the `/api/plans` unification. Not used for plan routing anymore.
 */
export const COLLECTIONS = [
  "portfolios",
  "projects",
  "products",
  "programs",
  "practices",
  "processes",
  "purposes",
  "pathways",
] as const;
export type Collection = (typeof COLLECTIONS)[number];

/**
 * Identifier schemes. Rust serializes unit variants as the bare string;
 * `Custom` as `{ "Custom": "label" }`.
 */
export type IdentifierScheme =
  | "Uri"
  | "Uuid"
  | "JiraProjectKey"
  | "AsanaGid"
  | "TrelloBoardId"
  | "MsProjectId"
  | "GitHubProjectId"
  | "LinearId"
  | "Code"
  | "LocalId"
  | { Custom: string };

/** The unit identifier schemes, for the form's scheme `<select>`. Excludes `Custom`. */
export const ALL_SCHEMES: IdentifierScheme[] = [
  "Uri",
  "Uuid",
  "JiraProjectKey",
  "AsanaGid",
  "TrelloBoardId",
  "MsProjectId",
  "GitHubProjectId",
  "LinearId",
  "Code",
  "LocalId",
];

/** The lifecycle status of a plan (unit variants only here). */
export type PlanStatus =
  | "Proposed"
  | "Active"
  | "OnHold"
  | "Completed"
  | "Cancelled"
  | { Custom: string };

/** The unit statuses, for the form's status `<select>`. Excludes `Custom`. */
export const ALL_STATUSES: PlanStatus[] = [
  "Proposed",
  "Active",
  "OnHold",
  "Completed",
  "Cancelled",
];

/** The status of a single goal. */
export type GoalStatus =
  | "NotStarted"
  | "InProgress"
  | "Achieved"
  | "Missed"
  | { Custom: string };

/** A plan objective; only the title feeds matching. */
export interface Goal {
  /** The goal title (the matchable surface). */
  title: string;
  /** Optional longer description. */
  description?: string | null;
  /** Optional target date (ISO `YYYY-MM-DD`). */
  target_date?: string | null;
  /** Optional goal status. */
  status?: GoalStatus | null;
}

/** Typed within-entity relationship kinds. */
export type RelationKind =
  | "ParentOf"
  | "ChildOf"
  | "DependsOn"
  | "BlockedBy"
  | "Supersedes"
  | "SupersededBy"
  | "SimilarTo"
  | "RelatedTo"
  | { Custom: string };

/** The unit relation kinds, for the relationships editor's `<select>`. */
export const ALL_RELATION_KINDS: RelationKind[] = [
  "ParentOf",
  "ChildOf",
  "DependsOn",
  "BlockedBy",
  "Supersedes",
  "SupersededBy",
  "SimilarTo",
  "RelatedTo",
];

/** One typed identifier on a plan (scheme + opaque value). */
export interface PlanIdentifier {
  /** Which identifier system this value belongs to. */
  scheme: IdentifierScheme;
  /** The identifier value within that scheme. */
  value: string;
}

/** A typed within-entity link to another plan. */
export interface PlanRelationship {
  /** The kind of relationship. */
  relation: RelationKind;
  /** The opaque registry id of the referenced plan. */
  plan_id: string;
}

/**
 * A plan — the request/response body of the Portfolio Service,
 * mirroring `project_portfolio_management_matcher::Plan`. `name` is
 * required; the rest are optional and may be `null`. `kind` is an
 * optional descriptive label; `parent_ref` names any containing plan.
 */
export interface Plan {
  /**
   * Optional descriptive label (Portfolio / Project / Product / Program /
   * Practice / Process / Purpose / Pathway / Proposal).
   */
  kind?: PlanKind | null;
  /** Primary plan name (required). */
  name: string;
  /** Other names the plan is known by. */
  alternate_names?: string[];
  /** Owner-scoped code (e.g. `PROJ-2026`). */
  code?: string | null;
  /** Sponsoring/owning organisation id — scopes `code` matching. */
  owner_org_id?: string | null;
  /** Human-readable owning organisation name. */
  owner_org_name?: string | null;
  /** Lead reference (`person:<id>` / `worker:<id>` EntityRef). */
  lead_ref?: string | null;
  /** Parent plan pid — any plan may contain any other plan (recursive tree). */
  parent_ref?: string | null;
  /** Lifecycle status (`Active`, … or `{Custom}`). */
  status?: PlanStatus | null;
  /** Objectives; goal titles feed matching. */
  goals?: Goal[];
  /** Planned/actual start date (ISO). */
  start_date?: string | null;
  /** Planned completion / due date (ISO). */
  target_date?: string | null;
  /** Descriptive keywords (Jaccard-matched). */
  keywords?: string[];
  /** Operator labels (Jaccard-matched). */
  tags?: string[];
  /** Typed identifiers; some schemes short-circuit matching. */
  identifiers?: PlanIdentifier[];
  /** `schema.org/sameAs` URLs; an exact match short-circuits. */
  same_as?: string[];
  /** BCP-47 language tag for the plan content. */
  in_language?: string | null;
  /** Typed within-entity relationships (Jaccard-matched). */
  relationships?: PlanRelationship[];
}

/**
 * Lightweight reference `{pid, name}` returned by create / list — enough
 * to render a link without the full record.
 */
export interface PlanRef {
  /** Persistent id of the plan. */
  pid: string;
  /** Plan name for display. */
  name: string;
}

/**
 * A scored duplicate candidate from `/check-duplicates`: a
 * {@link PlanRef} plus the matcher's verdict.
 */
export interface ScoredRef {
  /** Persistent id of the candidate plan. */
  pid: string;
  /** Candidate name for display. */
  name: string;
  /** Match score in `[0,1]`. */
  score: number;
  /** Confidence band label (e.g. `High`, `Medium`). */
  confidence: string;
  /** Whether the score clears the match threshold. */
  is_match: boolean;
}
