// TypeScript domain types mirroring the Rust Thing Service models
// (schema.org/Thing-based). Reference:
// thing-service-with-loco/AGENTS/models.md.

// ─── HTTP envelope ───────────────────────────────────────────────────

/**
 * Standard response envelope returned by every Thing Service endpoint.
 *
 * The service always wraps payloads so that callers can distinguish a
 * successful empty result (`data: null`, `success: true`) from a failure
 * (`success: false` with a populated `error`). {@link ApiClient} unwraps
 * this so callers receive only the inner `data`.
 *
 * @typeParam T - The shape of the successful payload in `data`.
 */
export interface ApiResponse<T> {
  success: boolean;
  data: T | null;
  error: ApiErrorBody | null;
}

/**
 * Machine-readable error body carried inside a failed {@link ApiResponse}.
 *
 * `code` is a stable string (e.g. `NOT_FOUND`, `DUPLICATE`, `VALIDATION`)
 * used for branching; `message` is human-facing; `details` is an opaque,
 * endpoint-specific payload (e.g. duplicate `MatchResult[]` on a 409).
 */
export interface ApiErrorBody {
  code: string;
  message: string;
  details?: unknown;
}

// ─── IdentifierType (schema.org/PropertyValue) ───────────────────────

/**
 * Kind of external identifier attached to a Thing, modelled on
 * schema.org/PropertyValue's `propertyID`.
 *
 * Mirrors the Rust enum: the well-known scheme variants are bare string
 * literals (`"Doi"`, `"Isbn"`, …), while an arbitrary caller-supplied
 * scheme is the tagged object `{ Custom: "<label>" }`. This union shape
 * must match the wire format exactly — see `thing-service` models.
 */
export type IdentifierType =
  | "Doi"
  | "Isbn"
  | "Issn"
  | "Gtin"
  | "Sku"
  | "Mpn"
  | "SerialNumber"
  | "Uri"
  | "Uuid"
  | { Custom: string };

/**
 * Identifier schemes that the matcher treats as deterministic — an exact
 * value match on one of these short-circuits probabilistic scoring to a
 * certain match (two Things sharing a DOI/ISBN/GTIN/… are the same Thing).
 *
 * WHY the exclusions: `Sku`, `Uri`, and `Custom` are intentionally absent
 * because they are not globally unique (SKUs are vendor-scoped, URIs and
 * custom schemes are caller-defined), so they must not pin the score.
 */
// Per spec: deterministic-identifier match short-circuits scoring.
// Sku, Uri, and Custom are NOT deterministic.
export const DETERMINISTIC_TYPES: IdentifierType[] = [
  "Doi",
  "Isbn",
  "Issn",
  "Gtin",
  "Mpn",
  "SerialNumber",
  "Uuid",
];

/**
 * The well-known identifier schemes offered in the UI's type dropdown.
 *
 * `Custom` is excluded from this list because the form renders it as a
 * separate "Custom…" option with a free-text label field; including it
 * here would conflate the tagged-object variant with the bare literals.
 */
export const IDENTIFIER_TYPE_OPTIONS: Exclude<
  IdentifierType,
  { Custom: string }
>[] = [
  "Doi",
  "Isbn",
  "Issn",
  "Gtin",
  "Sku",
  "Mpn",
  "SerialNumber",
  "Uri",
  "Uuid",
];

/**
 * A single external identifier on a Thing (schema.org/PropertyValue):
 * its scheme (`property_id`), the `value` in that scheme, an optional
 * display `name`, and an optional canonical `url` for the identifier.
 */
export interface ThingIdentifier {
  property_id: IdentifierType;
  value: string;
  name?: string | null;
  url?: string | null;
}

/**
 * Construct an empty identifier row for the editor.
 *
 * Defaults to the `Sku` scheme (a safe non-deterministic default so a
 * half-filled row can't accidentally force a deterministic match) with an
 * empty value, ready for the user to fill in.
 *
 * @returns A fresh {@link ThingIdentifier} with default scheme and blanks.
 */
export function blankThingIdentifier(): ThingIdentifier {
  return { property_id: "Sku", value: "", name: null, url: null };
}

// ─── Thing ───────────────────────────────────────────────────────────

/**
 * The core domain entity — a schema.org/Thing record as stored by the
 * Thing Service.
 *
 * Field optionality mirrors the wire contract: `id` and the timestamp /
 * soft-delete fields are server-assigned (absent on create), while
 * `name` is the only always-required attribute. Array fields default to
 * empty on the server but are optional here so partial payloads (match
 * requests, patches) type-check. Keep this in lock-step with the Rust
 * `Thing` model — see the project AGENTS drift policy.
 */
export interface Thing {
  id?: string;
  name: string;
  alternate_names?: string[];
  description?: string | null;
  disambiguating_description?: string | null;
  additional_type?: string | null;
  url?: string | null;
  identifiers?: ThingIdentifier[];
  images?: string[];
  main_entity_of_page?: string | null;
  owner?: string | null;
  same_as?: string[];
  subject_of?: string | null;
  potential_action?: string | null;
  is_deleted?: boolean;
  deleted_at?: string | null;
  created_at?: string;
  updated_at?: string;
}

// ─── Matching ────────────────────────────────────────────────────────

/**
 * Confidence bucket for a match score, in descending order of certainty.
 *
 * These are the service's classification of the numeric `score` against
 * configured thresholds; the UI uses them for the coloured quality pills.
 */
export type MatchConfidence = "Certain" | "Probable" | "Possible" | "Unlikely";

/**
 * Per-component score breakdown explaining how an overall match score was
 * reached. Each `*_score` is in `[0, 1]` and may be `null`/absent when that
 * component did not contribute (e.g. no URL on either record). The two
 * boolean flags note a phonetic (Soundex) bonus and a deterministic
 * identifier short-circuit respectively.
 */
export interface MatchBreakdown {
  name_score?: number | null;
  identifier_score?: number | null;
  description_score?: number | null;
  url_score?: number | null;
  same_as_score?: number | null;
  phonetic_match?: boolean;
  deterministic_match?: boolean;
}

/**
 * One candidate returned by a match / duplicate-check call: the matched
 * `thing`, its overall `score` in `[0, 1]`, the `confidence` bucket, and
 * an optional per-component {@link MatchBreakdown}.
 */
export interface MatchResult {
  thing: Thing;
  score: number;
  confidence: MatchConfidence;
  breakdown?: MatchBreakdown;
}

/**
 * Query payload for an ad-hoc match check (`POST /api/things/match`).
 *
 * All entity fields are optional so a caller can probe with as little as a
 * name. `threshold` filters out candidates below a minimum score and
 * `max_candidates` caps how many are returned.
 */
export interface MatchRequest {
  name?: string;
  description?: string;
  url?: string;
  identifiers?: ThingIdentifier[];
  same_as?: string[];
  threshold?: number;
  max_candidates?: number;
}

// ─── Merge ───────────────────────────────────────────────────────────

/** Lifecycle state of a merge: `Completed`, or `Reversed` if undone. */
export type MergeStatus = "Completed" | "Reversed";

/**
 * Payload to merge two Things (`POST /api/things/merge`).
 *
 * `main_thing_id` is the surviving record; `duplicate_thing_id` is folded
 * into it and soft-deleted. `merge_reason` / `merged_by` are recorded on
 * the resulting audit trail.
 */
export interface MergeRequest {
  main_thing_id: string;
  duplicate_thing_id: string;
  merge_reason?: string | null;
  merged_by?: string | null;
}

/**
 * Persisted record of a completed merge, including the `match_score` that
 * justified it and a `transferred_data` snapshot of what moved from the
 * duplicate to the main record — kept for auditability and reversal.
 */
export interface MergeRecord {
  id: string;
  main_thing_id: string;
  duplicate_thing_id: string;
  status: MergeStatus;
  merged_by?: string | null;
  merge_reason?: string | null;
  match_score?: number | null;
  transferred_data?: unknown;
  merged_at: string;
}

/**
 * Result of a successful merge: the new {@link MergeRecord} plus the
 * surviving `main_thing` in its post-merge state (so the UI can navigate
 * to / display it without a follow-up fetch).
 */
export interface MergeResponse {
  merge_record: MergeRecord;
  main_thing: Thing;
}

// ─── Batch dedup ─────────────────────────────────────────────────────

/**
 * Tuning knobs for a full-index deduplication scan
 * (`POST /api/things/deduplicate`).
 *
 * `threshold` is the minimum score to flag a pair; pairs at or above
 * `auto_merge_threshold` are merged automatically while the rest are
 * queued for human review. `max_candidates` bounds work per record.
 */
export interface BatchDeduplicationRequest {
  threshold?: number;
  max_candidates?: number;
  auto_merge_threshold?: number;
}

/**
 * Summary of a deduplication scan: counts of records scanned, duplicate
 * pairs found, pairs auto-merged vs queued, and the {@link ReviewQueueItem}
 * list of pairs awaiting human confirmation.
 */
export interface BatchDeduplicationResponse {
  things_scanned: number;
  duplicates_found: number;
  auto_merged: number;
  queued_for_review: number;
  review_items: ReviewQueueItem[];
}

/**
 * State of a queued duplicate pair: awaiting review (`Pending`), human
 * `Confirmed` / `Rejected`, or `AutoMerged` by the scan above threshold.
 */
export type ReviewStatus =
  | "pending"
  | "confirmed"
  | "rejected"
  | "automerged";

/**
 * A candidate duplicate pair (`thing_id_a` / `thing_id_b`) captured by a
 * dedup scan, with its `match_score`, `match_quality` label, current
 * {@link ReviewStatus}, and review timestamps.
 */
export interface ReviewQueueItem {
  id: string;
  thing_id_a: string;
  thing_id_b: string;
  match_score: number;
  match_quality: string;
  /** How the pair was detected (e.g. `batch_deduplication`). */
  detection_method: string;
  status: ReviewStatus;
  created_at: string;
  reviewed_at?: string | null;
}

// ─── Audit ───────────────────────────────────────────────────────────

/**
 * One immutable audit-log row recording a change to an entity: who
 * (`user_*`), what (`action`, with `old_values` / `new_values` JSON
 * snapshots), to which record (`entity_type` / `entity_id`), and when
 * (`created_at`). Surfaced read-only in the dashboard and per-thing audit
 * views.
 */
export interface AuditEntry {
  id: string;
  entity_type: string;
  entity_id: string;
  action: string;
  user_id?: string | null;
  user_ip_address?: string | null;
  user_agent?: string | null;
  old_values?: unknown;
  new_values?: unknown;
  created_at: string;
}
