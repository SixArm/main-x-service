// Types mirroring the Organization Service payload, which is the
// `organization_matcher::Organization` shape itself.
// Source of truth: organization-matcher-rust-crate/src/organization.rs.

/// Deterministic + scoped identifier schemes. Rust serializes unit
/// variants as the bare string; `Custom` as `{ "Custom": "label" }`.
export type IdentifierScheme =
  | "Lei"
  | "Duns"
  | "Iso6523"
  | "Gln"
  | "Wikidata"
  | "Ror"
  | "Isni"
  | "Vat"
  | "TaxId"
  | "Naics"
  | "IsicV4"
  | "Sic"
  | { Custom: string };

/**
 * Identifier schemes that the matcher treats as deterministic, i.e. an
 * exact value match on any of these short-circuits matching to a
 * certain duplicate (LEI / DUNS / ISO 6523 / GLN / Wikidata / ROR /
 * ISNI / VAT). Surfaced in the form's scheme dropdown.
 */
export const DETERMINISTIC_SCHEMES: IdentifierScheme[] = [
  "Lei",
  "Duns",
  "Iso6523",
  "Gln",
  "Wikidata",
  "Ror",
  "Isni",
  "Vat",
];

/**
 * Every selectable scheme in the form's dropdown: the deterministic
 * ones first, then the non-deterministic / scoped ones (TaxId, NAICS,
 * ISIC v4, SIC). The `Custom` variant is intentionally excluded because
 * the form only edits unit-variant (bare-string) schemes.
 */
export const ALL_SCHEMES: IdentifierScheme[] = [
  ...DETERMINISTIC_SCHEMES,
  "TaxId",
  "Naics",
  "IsicV4",
  "Sic",
];

/** A single typed identifier on an organization: a scheme + its value. */
export interface OrgIdentifier {
  scheme: IdentifierScheme;
  value: string;
}

/**
 * schema.org/PostalAddress subset. Every field is optional/nullable; an
 * organization with no usable address parts omits the address entirely
 * (see `OrganizationForm.build`).
 */
export interface PostalAddress {
  street_address?: string | null;
  locality?: string | null;
  region?: string | null;
  postal_code?: string | null;
  country?: string | null;
}

/**
 * The full schema.org/Organization payload, mirroring
 * `organization_matcher::Organization`. This same shape is both the
 * stored record and the request body for create / update / match, so
 * the form edits it directly and `/check-duplicates` scores a draft of
 * it. Only `name` is required; nullable scalars distinguish "cleared"
 * (`null`) from "left as default".
 */
export interface Organization {
  name: string;
  legal_name?: string | null;
  alternate_names?: string[];
  identifiers?: OrgIdentifier[];
  url?: string | null;
  same_as?: string[];
  address?: PostalAddress | null;
  jurisdiction?: string | null;
  founding_date?: string | null;
  telephone?: string | null;
  email?: string | null;
  keywords?: string[];
}

/// `{pid, name}` returned by create / list.
export interface OrgRef {
  pid: string;
  name: string;
}

/// A scored duplicate from /check-duplicates.
export interface ScoredRef {
  pid: string;
  name: string;
  score: number;
  confidence: string;
  is_match: boolean;
}

/**
 * Request body for `POST /api/organizations/merge`.
 *
 * Mirrors the service's `MergeRequest` (controllers/organizations.rs):
 * the survivor and the duplicate are named by **pid**, not by a nested
 * record, and `reason` is an optional free-text note stored in the merge
 * history. The service rejects equal pids with `422`.
 */
export interface MergeRequest {
  /** The surviving organization's public id. */
  main_pid: string;
  /** The duplicate to fold in and soft-delete. */
  duplicate_pid: string;
  /** Optional operator-supplied reason, recorded in the merge history. */
  reason?: string | null;
}

/**
 * Response body for a successful merge.
 *
 * Note the shape differs from some sibling services: there is **no**
 * `merge_record` wrapper — the service returns the two pids plus the
 * post-merge survivor payload directly.
 */
export interface MergeResponse {
  /** The surviving organization's public id. */
  main_pid: string;
  /** The merged-away duplicate's public id (now soft-deleted). */
  duplicate_pid: string;
  /** The survivor's payload after the fold. */
  main: Organization;
}

/**
 * One row of merge history from `GET /api/organizations/merges/recent`.
 *
 * Serialized straight from the `merge_records` SeaORM entity, so the
 * field names and types are the table's: `id` is the auto-increment
 * `i64` surrogate key (a JSON number), the timestamps are RFC 3339
 * strings, and `transferred` is the JSON snapshot of the duplicate's
 * payload at merge time.
 */
export interface MergeRecordRow {
  /** Row insert time (UTC) — when the merge happened. */
  created_at: string;
  /** Row last-modification time (UTC); merge rows are append-only. */
  updated_at: string;
  /** Auto-increment surrogate primary key; also the recency order key. */
  id: number;
  /** The surviving organization's pid. */
  main_pid: string;
  /** The merged-away duplicate's pid. */
  duplicate_pid: string;
  /** Operator-supplied free-text reason, when one was given. */
  reason?: string | null;
  /** The actor that performed the merge (bearer `sub`), when known. */
  actor?: string | null;
  /** JSON snapshot of the duplicate's payload at merge time. */
  transferred?: unknown;
}

/**
 * Per-component scores from `organization_matcher::MatchBreakdown`.
 * `undefined`/`null` means the component was skipped because one or both
 * records lacked the data, so it did not contribute to the weighted sum
 * (never rendered as zero — see `$lib/review`'s `breakdownRows`).
 */
export interface MatchBreakdown {
  name_score?: number | null;
  address_score?: number | null;
  url_score?: number | null;
  jurisdiction_score?: number | null;
  founding_date_score?: number | null;
  keywords_score?: number | null;
  /** Whether a deterministic short-circuit (LEI/DUNS/…) fired. */
  deterministic_match: boolean;
}

/**
 * The outcome of matching two organizations, from `POST
 * /api/organizations/match`. `confidence` is the matcher's `Confidence`
 * enum, serialized verbatim (`"High" | "Medium" | "Low"`).
 */
export interface MatchResult {
  score: number;
  is_match: boolean;
  confidence: "High" | "Medium" | "Low";
  breakdown: MatchBreakdown;
}

/**
 * One `[candidateIndex, result]` pair from `POST /api/organizations/match`
 * — the service ranks `candidates` against `query` and returns a Rust
 * `(usize, MatchResult)` tuple per candidate, which serializes as a
 * 2-element JSON array.
 */
export type MatchRankResult = [number, MatchResult];

/// Review disposition wire tokens (family-wide lowercase form).
export type ReviewStatus = "pending" | "confirmed" | "rejected" | "automerged";

/// One operator verdict for a pending review item.
export type ReviewDecision = "confirmed" | "rejected";

/// One stored review-queue item from the batch-deduplication scan.
export interface ReviewQueueItem {
  /** Stable review-item id (survives re-scans). */
  id: string;
  /** First organization in the candidate pair (public id). */
  organization_id_a: string;
  /** Second organization in the candidate pair (public id). */
  organization_id_b: string;
  /** Overall match score for the pair, 0-1. */
  match_score: number;
  /** Confidence band label (matcher confidence, lowercased). */
  match_quality: string;
  /** How the pair was detected (`batch_deduplication`). */
  detection_method: string;
  /** Current review disposition. */
  status: ReviewStatus;
  /**
   * How this pair was first surfaced (`operator` / `import` /
   * `matcher_suggested` — the cross-service-linking provenance
   * vocabulary; BLK-5). Free text server-side, so an unrecognised value
   * is real data, not a bug.
   */
  provenance: string;
  /** Operator who decided the item, if decided. */
  reviewed_by?: string | null;
  /** When the pair was first queued. */
  created_at: string;
  /** When the decision was recorded, if decided. */
  reviewed_at?: string | null;
}

/// Response envelope for the stored review-queue list.
export interface ReviewQueueListResponse {
  /** The stored review-queue items (newest first). */
  items: ReviewQueueItem[];
  /** Number of items returned. */
  total: number;
}

/// Report returned by the batch-deduplication scan (stored rows).
export interface BatchDeduplicationResponse {
  /** Number of organizations scanned. */
  organizations_scanned: number;
  /** Number of duplicate pairs found. */
  duplicates_found: number;
  /** Auto-merged count (always 0 — no auto-merge path). */
  auto_merged: number;
  /** Number of stored pairs currently pending review. */
  queued_for_review: number;
  /** The stored candidate pairs (stable ids across re-scans). */
  review_items: ReviewQueueItem[];
}

/// One audit-log row from `GET /api/organizations/{pid}/audit`.
/// Mirrors the service's `audit_logs` SeaORM model (serialized field
/// names): `action` (created/updated/deleted/merged/exported), nullable
/// `actor` (the caller's user pid when a verified token was presented,
/// else `null`), an optional JSON `snapshot`, and the `created_at`
/// timestamp.
export interface AuditEntry {
  /** The audited operation: `created` / `updated` / `deleted` / `merged` / `exported`. */
  action: string;
  /** Caller's user pid when a verified token was presented, else `null`. */
  actor: string | null;
  /** Optional JSON snapshot of the record state for the audited action. */
  snapshot?: unknown;
  /** ISO-8601 timestamp of when the action was recorded. */
  created_at?: string;
}
