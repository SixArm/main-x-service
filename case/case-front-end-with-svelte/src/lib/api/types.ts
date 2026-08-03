// Types mirroring the Case Service payload, which is the
// `case_matcher::Case` shape itself.
// Source of truth: case-matcher-rust-crate/src/case.rs.

/// Identifier schemes. Rust serializes unit variants as the bare string;
/// `Custom` as `{ "Custom": "label" }`.
export type IdentifierScheme =
  | "Docket"
  | "ExternalCaseId"
  | "Uri"
  | "Uuid"
  | "AgencyCaseNumber"
  | "LocalId"
  | { Custom: string };

/** The unit identifier schemes, for populating the form's scheme `<select>`. Excludes `Custom`. */
export const ALL_SCHEMES: IdentifierScheme[] = [
  "Docket",
  "ExternalCaseId",
  "Uri",
  "Uuid",
  "AgencyCaseNumber",
  "LocalId",
];

/// The type / domain of a case (unit variants only here).
export type CaseType =
  | "Benefit"
  | "Legal"
  | "SocialServices"
  | "Healthcare"
  | "Housing"
  | "Immigration"
  | "Licensing"
  | "Complaint"
  | "Appeal"
  | "Investigation"
  | "Tax"
  | "Employment"
  | { Custom: string };

/** The unit case types, for populating the form's case-type `<select>`. Excludes `Custom`. */
export const ALL_CASE_TYPES: CaseType[] = [
  "Benefit",
  "Legal",
  "SocialServices",
  "Healthcare",
  "Housing",
  "Immigration",
  "Licensing",
  "Complaint",
  "Appeal",
  "Investigation",
  "Tax",
  "Employment",
];

/// The lifecycle status of a case (unit variants only here).
export type CaseStatus =
  | "Open"
  | "InProgress"
  | "Pending"
  | "OnHold"
  | "Closed"
  | "Resolved"
  | "Rejected"
  | "Withdrawn"
  | { Custom: string };

/** The unit lifecycle statuses, for populating the form's status `<select>`. Excludes `Custom`. */
export const ALL_STATUSES: CaseStatus[] = [
  "Open",
  "InProgress",
  "Pending",
  "OnHold",
  "Closed",
  "Resolved",
  "Rejected",
  "Withdrawn",
];

/// Case priority. Data only — never contributes to the score.
export type Priority = "Low" | "Normal" | "High" | "Urgent";

/** All priorities, for populating the form's priority `<select>`. */
export const ALL_PRIORITIES: Priority[] = ["Low", "Normal", "High", "Urgent"];

/** One typed identifier on a case (scheme + opaque value). */
export interface CaseIdentifier {
  /** Which identifier system this value belongs to. */
  scheme: IdentifierScheme;
  /** The identifier value within that scheme. */
  value: string;
}

/**
 * A governmental case — the request/response body of the Case Service,
 * mirroring `case_matcher::Case`. Only `title` is required; the rest are
 * optional and may be `null` (the matcher treats absent and null alike).
 */
export interface Case {
  /** Primary case title (required). */
  title: string;
  /** Other titles the case is known by. */
  alternate_titles?: string[];
  /** Agency-issued case number (e.g. `2026-HB-0042`). */
  case_number?: string | null;
  /** Stable id of the owning agency (scopes case-number matching). */
  agency_id?: string | null;
  /** Human-readable agency name. */
  agency_name?: string | null;
  /** Case domain/type (`Benefit`, `Legal`, … or `{Custom}`). */
  case_type?: CaseType | null;
  /** Lifecycle status (`Open`, `Closed`, … or `{Custom}`). */
  status?: CaseStatus | null;
  /** Operational priority — data only, never scored. */
  priority?: Priority | null;
  /** Date the case was opened (ISO `YYYY-MM-DD`). */
  opened_date?: string | null;
  /** Subjects/parties involved (Jaccard-matched). */
  subjects?: string[];
  /** Free-text keywords/tags (Jaccard-matched). */
  keywords?: string[];
  /** Typed identifiers; some schemes short-circuit matching. */
  identifiers?: CaseIdentifier[];
  /** `schema.org/sameAs` URLs; an exact match short-circuits. */
  same_as?: string[];
  /** ISO 639-1 language codes for the case content. */
  in_language?: string[];
}

/**
 * Lightweight reference `{pid, title}` returned by create / list — enough
 * to render a link without the full record.
 */
export interface CaseRef {
  /** Persistent id of the case. */
  pid: string;
  /** Case title for display. */
  title: string;
}

/**
 * Request body of `POST /api/cases/merge` — which duplicate folds into
 * which survivor, and (optionally) why. Mirrors the service's
 * `MergeRequest` struct in `src/controllers/cases.rs`.
 */
export interface MergeRequest {
  /** The surviving case's persistent id; it keeps its pid. */
  main_pid: string;
  /** The duplicate to fold in and soft-delete. */
  duplicate_pid: string;
  /** Operator-supplied reason, recorded in the merge history. */
  reason?: string | null;
}

/**
 * Response body of `POST /api/cases/merge`. Note the shape: the service
 * returns the two pids plus the survivor's merged payload **inline** —
 * there is no `merge_record` wrapper (unlike some sibling services), so
 * the merge row's own id / timestamp are not available here. Read them
 * from {@link MergeRecordRow} via `merges/recent` instead.
 */
export interface MergeResponse {
  /** Persistent id of the surviving case. */
  main_pid: string;
  /** Persistent id of the soft-deleted duplicate. */
  duplicate_pid: string;
  /** The survivor's payload after the fold. */
  main: Case;
}

/**
 * One row of `GET /api/cases/merges/recent` — the raw `merge_records`
 * table row, mirroring the SeaORM entity in
 * `src/models/_entities/merge_records.rs`.
 */
export interface MergeRecordRow {
  /** When the merge row was written (RFC 3339 with offset). */
  created_at: string;
  /** When the merge row was last updated (RFC 3339 with offset). */
  updated_at: string;
  /** Auto-incrementing primary key (Rust `i64`). */
  id: number;
  /** Persistent id of the survivor. */
  main_pid: string;
  /** Persistent id of the merged-away duplicate. */
  duplicate_pid: string;
  /** Operator-supplied reason, when one was given. */
  reason?: string | null;
  /** Who performed the merge, when the caller was authenticated. */
  actor?: string | null;
  /** Snapshot of the transferred payload (opaque JSON). */
  transferred?: unknown;
}

/**
 * A scored duplicate candidate from `/check-duplicates`: a {@link CaseRef}
 * plus the matcher's verdict.
 */
export interface ScoredRef {
  /** Persistent id of the candidate case. */
  pid: string;
  /** Candidate title for display. */
  title: string;
  /** Match score in `[0,1]`. */
  score: number;
  /** Confidence band label (e.g. `Certain`, `Probable`). */
  confidence: string;
  /** Whether the score clears the match threshold. */
  is_match: boolean;
}
