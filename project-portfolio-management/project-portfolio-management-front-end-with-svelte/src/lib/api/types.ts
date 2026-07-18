// Types mirroring the Portfolio Service payload, which is the
// `project_portfolio_management_matcher::WorkItem` shape itself.
// Source of truth: project-portfolio-management-matcher-rust-crate/src/work_item.rs.

/** The four work-item kinds — each a REST collection. Closed set. */
export type WorkItemKind = "Portfolio" | "Project" | "Product" | "Program";

/** All kinds, for the collection switcher / form. */
export const ALL_KINDS: WorkItemKind[] = [
  "Portfolio",
  "Project",
  "Product",
  "Program",
];

/** The REST collection segment for each kind. */
export const COLLECTIONS = [
  "portfolios",
  "projects",
  "products",
  "programs",
] as const;
export type Collection = (typeof COLLECTIONS)[number];

/** Map a collection segment to its kind. */
export const KIND_OF_COLLECTION: Record<Collection, WorkItemKind> = {
  portfolios: "Portfolio",
  projects: "Project",
  products: "Product",
  programs: "Program",
};

/** Map a kind to its collection segment. */
export const COLLECTION_OF_KIND: Record<WorkItemKind, Collection> = {
  Portfolio: "portfolios",
  Project: "projects",
  Product: "products",
  Program: "programs",
};

/** Narrow an arbitrary string to a known collection segment. */
export function isCollection(value: string): value is Collection {
  return (COLLECTIONS as readonly string[]).includes(value);
}

/** True for the child kinds that carry a `portfolio_ref`. */
export function isChildKind(kind: WorkItemKind): boolean {
  return kind !== "Portfolio";
}

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

/** The lifecycle status of a work item (unit variants only here). */
export type WorkItemStatus =
  | "Proposed"
  | "Active"
  | "OnHold"
  | "Completed"
  | "Cancelled"
  | { Custom: string };

/** The unit statuses, for the form's status `<select>`. Excludes `Custom`. */
export const ALL_STATUSES: WorkItemStatus[] = [
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

/** A work-item objective; only the title feeds matching. */
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

/** One typed identifier on a work item (scheme + opaque value). */
export interface WorkItemIdentifier {
  /** Which identifier system this value belongs to. */
  scheme: IdentifierScheme;
  /** The identifier value within that scheme. */
  value: string;
}

/** A typed within-entity link to another work item. */
export interface WorkItemRelationship {
  /** The kind of relationship. */
  relation: RelationKind;
  /** The opaque registry id of the referenced work item. */
  work_item_id: string;
}

/**
 * A work item — the request/response body of the Portfolio Service,
 * mirroring `project_portfolio_management_matcher::WorkItem`. `kind` and `name` are
 * required; the rest are optional and may be `null`.
 */
export interface WorkItem {
  /** The kind / collection (Portfolio / Project / Product / Program). */
  kind: WorkItemKind;
  /** Primary work-item name (required). */
  name: string;
  /** Other names the work item is known by. */
  alternate_names?: string[];
  /** Owner-scoped code (e.g. `PROJ-2026`). */
  code?: string | null;
  /** Sponsoring/owning organisation id — scopes `code` matching. */
  owner_org_id?: string | null;
  /** Human-readable owning organisation name. */
  owner_org_name?: string | null;
  /** Lead reference (`person:<id>` / `worker:<id>` EntityRef). */
  lead_ref?: string | null;
  /** Parent portfolio pid (child kinds; an exact supporting signal). */
  portfolio_ref?: string | null;
  /** Lifecycle status (`Active`, … or `{Custom}`). */
  status?: WorkItemStatus | null;
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
  identifiers?: WorkItemIdentifier[];
  /** `schema.org/sameAs` URLs; an exact match short-circuits. */
  same_as?: string[];
  /** BCP-47 language tag for the work-item content. */
  in_language?: string | null;
  /** Typed within-entity relationships (Jaccard-matched). */
  relationships?: WorkItemRelationship[];
}

/**
 * Lightweight reference `{pid, name}` returned by create / list — enough
 * to render a link without the full record.
 */
export interface WorkItemRef {
  /** Persistent id of the work item. */
  pid: string;
  /** Work-item name for display. */
  name: string;
}

/**
 * A scored duplicate candidate from `/check-duplicates`: a
 * {@link WorkItemRef} plus the matcher's verdict.
 */
export interface ScoredRef {
  /** Persistent id of the candidate work item. */
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
