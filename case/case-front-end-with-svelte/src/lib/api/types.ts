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
