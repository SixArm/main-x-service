// TypeScript domain types mirroring the Rust Worker Service models.
// Reference: worker-service-with-loco/agents/models.md.
// Field names use snake_case to match the JSON wire format produced by
// the Axum/SeaORM stack.

// ─── HTTP envelope ───────────────────────────────────────────────────

/**
 * Uniform success/error envelope wrapping every Worker Service response
 * body. The service always sets exactly one of `data` / `error` per
 * `success`. {@link ApiClient} unwraps this so callers see only `T`.
 *
 * @typeParam T - The payload type carried in `data` on success.
 */
export interface ApiResponse<T> {
  /** True when the request succeeded; `data` is populated, `error` null. */
  success: boolean;
  /** The success payload, or null on error. */
  data: T | null;
  /** The error detail, or null on success. */
  error: ApiErrorBody | null;
}

/**
 * Machine-readable error detail inside an {@link ApiResponse}. `details` is
 * untyped because its shape is endpoint-specific (e.g. an array of
 * {@link MatchResult} for duplicate-detection `409`s).
 */
export interface ApiErrorBody {
  /** Stable error code (e.g. `NOT_FOUND`, `DUPLICATE`, `VALIDATION`). */
  code: string;
  /** Human-readable error message. */
  message: string;
  /** Optional endpoint-specific extra context (shape varies). */
  details?: unknown;
}

// ─── Shared primitives ───────────────────────────────────────────────

/** Administrative gender, mirroring the service's `Gender` enum. */
export type Gender = "male" | "female" | "other" | "unknown";

/** Intended use of an {@link Address} (FHIR-style address-use vocabulary). */
export type AddressUse = "home" | "work" | "temp" | "old" | "billing";

/** Postal address. All parts optional to tolerate partial records. */
export interface Address {
  /** What the address is used for (home, work, …). */
  use_type?: AddressUse | null;
  /** First street line. */
  line1?: string | null;
  /** Second street line (suite, unit, …). */
  line2?: string | null;
  /** City / locality. */
  city?: string | null;
  /** State / province / region. */
  state?: string | null;
  /** Postal / ZIP code. */
  postal_code?: string | null;
  /** Country (name or ISO code, as stored by the service). */
  country?: string | null;
}

/** Transport of a {@link ContactPoint} (phone, email, …). */
export type ContactPointSystem =
  | "phone"
  | "fax"
  | "email"
  | "pager"
  | "url"
  | "sms"
  | "other";
/** Intended use of a {@link ContactPoint} (home, work, mobile, …). */
export type ContactPointUse = "home" | "work" | "temp" | "old" | "mobile";

/** A single way to reach the worker (one phone number, email, etc.). */
export interface ContactPoint {
  /** Which transport `value` is for. */
  system: ContactPointSystem;
  /** The contact value (phone number, email address, URL, …). */
  value: string;
  /** Context this contact is used in. */
  use_type?: ContactPointUse | null;
}

// ─── Identifier ──────────────────────────────────────────────────────

/** Status/use of an {@link Identifier} (usual, official, …). */
export type IdentifierUse = "usual" | "official" | "temp" | "secondary" | "old";
/**
 * Kind of an {@link Identifier}: MRN, SSN, driver's licence, NPI,
 * passport, tax id, or other.
 */
export type IdentifierType =
  | "MRN"
  | "SSN"
  | "DL"
  | "NPI"
  | "PPN"
  | "TAX"
  | "Other";

/**
 * An external identifier for the worker. A record may carry several; each
 * is the triple (type + issuing `system` + `value`).
 */
export interface Identifier {
  /** Status/use of this identifier. */
  use_type?: IdentifierUse | null;
  /** What kind of identifier this is. */
  identifier_type: IdentifierType;
  /** Namespace/authority URI that issued the identifier. */
  system: string;
  /** The identifier value itself. */
  value: string;
  /** Organization that assigned the identifier, if known. */
  assigner?: string | null;
}

// ─── Identity document ───────────────────────────────────────────────

/** Kind of {@link IdentityDocument} (passport, national id, …). */
export type DocumentType =
  | "PASSPORT"
  | "BIRTH_CERTIFICATE"
  | "NATIONAL_ID"
  | "DRIVERS_LICENSE"
  | "VOTER_ID"
  | "MILITARY_ID"
  | "RESIDENCE_PERMIT"
  | "WORK_PERMIT"
  | "OTHER";

/** A government/identity document evidencing the worker's identity. */
export interface IdentityDocument {
  /** Which kind of document this is. */
  document_type: DocumentType;
  /** Document number. */
  number: string;
  /** Issuing country (name or ISO code). */
  issuing_country?: string | null;
  /** Issuing authority/agency. */
  issuing_authority?: string | null;
  /** Issue date (ISO `YYYY-MM-DD`). */
  issue_date?: string | null;
  /** Expiry date (ISO `YYYY-MM-DD`). */
  expiry_date?: string | null;
  /** Whether the document has been verified. */
  verified?: boolean;
}

// ─── Names ───────────────────────────────────────────────────────────

/** Status/use of a {@link HumanName} (usual, official, maiden, …). */
export type NameUse =
  | "usual"
  | "official"
  | "temp"
  | "nickname"
  | "anonymous"
  | "old"
  | "maiden";

/**
 * A structured person name. `given` is an ordered list (first, middle, …);
 * the UI joins it with spaces for display and splits user input back into
 * tokens.
 */
export interface HumanName {
  /** Status/use of this name. */
  use_type?: NameUse | null;
  /** Family (last) name. */
  family: string;
  /** Ordered given names (first, middle, …). */
  given: string[];
  /** Name prefixes (Dr, Mr, …). */
  prefix?: string[];
  /** Name suffixes (Jr, III, …). */
  suffix?: string[];
}

// ─── Emergency contact ───────────────────────────────────────────────

/** A person to contact in an emergency on the worker's behalf. */
export interface EmergencyContact {
  /** Contact's full name. */
  name: string;
  /** Relationship to the worker (e.g. spouse, parent). */
  relationship: string;
  /** Ways to reach the contact. */
  telecom?: ContactPoint[];
  /** Contact's postal address. */
  address?: Address | null;
  /** Whether this is the primary emergency contact. */
  is_primary?: boolean;
}

// ─── Worker link ─────────────────────────────────────────────────────

/**
 * Relationship between two worker records. `replaces` / `replaced-by`
 * are the merge link directions; `refer` / `seealso` are cross-references.
 */
export type LinkType = "replaced-by" | "replaces" | "refer" | "seealso";

/** A typed link from this worker to another worker record. */
export interface WorkerLink {
  /** The id of the linked-to worker. */
  other_worker_id: string;
  /** How the two records relate. */
  link_type: LinkType;
}

// ─── Worker ──────────────────────────────────────────────────────────

/**
 * The core Worker record — the front-end's central domain entity, mirroring
 * the Rust service's `Worker` model. Most fields are optional because the
 * service tolerates partial records and the create form only requires
 * `name` and `gender`. Server-managed fields (`id`, `created_at`,
 * `updated_at`) are present on reads but omitted on create.
 */
export interface Worker {
  /** Server-assigned UUID; absent until the record is created. */
  id?: string;
  /** External identifiers (MRN, SSN, …). */
  identifiers?: Identifier[];
  /** Whether the record is active; soft-deleted records are inactive. */
  active?: boolean;
  /** Primary name (required). */
  name: HumanName;
  /** Aliases / former / alternate names. */
  additional_names?: HumanName[];
  /** Contact points (phone, email, …). */
  telecom?: ContactPoint[];
  /** Administrative gender (required). */
  gender: Gender;
  /** Date of birth (ISO `YYYY-MM-DD`). */
  birth_date?: string | null;
  /** Tax identifier (SSN/CPF/TIN); used as a strong match signal. */
  tax_id?: string | null;
  /** Identity documents on file. */
  documents?: IdentityDocument[];
  /** Emergency contacts. */
  emergency_contacts?: EmergencyContact[];
  /** Whether the worker is recorded as deceased. */
  deceased?: boolean;
  /** Date/time of death (ISO 8601), if known. */
  deceased_datetime?: string | null;
  /** Postal addresses. */
  addresses?: Address[];
  /** Marital status (free-form / coded as stored by the service). */
  marital_status?: string | null;
  /** Whether part of a multiple birth, if recorded. */
  multiple_birth?: boolean | null;
  /** Photo references (URLs or data URIs). */
  photo?: string[];
  /** Organization that manages this record. */
  managing_organization?: string | null;
  /** Links to other worker records (merge / cross-reference). */
  links?: WorkerLink[];
  /** Server-set creation timestamp (ISO 8601). */
  created_at?: string;
  /** Server-set last-update timestamp (ISO 8601). */
  updated_at?: string;
}

// ─── Matching ────────────────────────────────────────────────────────

/**
 * Confidence band a match score falls into, from strongest (`definite`)
 * to weakest (`unlikely`). Drives the colour coding in
 * {@link MatchResultsList}.
 */
export type MatchQuality =
  | "definite"
  | "certain"
  | "probable"
  | "possible"
  | "unlikely";

/**
 * Per-component score breakdown emitted by the matcher, each value in
 * `[0, 1]` (or null when the component didn't contribute). Keys: name,
 * birth_date, gender, address, identifier, tax_id, document.
 */
export type MatchBreakdown = Record<string, number | null>;

/** A single candidate returned by a match / duplicate-check call. */
export interface MatchResult {
  /** The candidate worker record. */
  worker: Worker;
  /** Overall match score in `[0, 1]`. */
  score: number;
  /** Confidence band the score classifies into. */
  quality: MatchQuality;
  /** Optional per-field score breakdown. */
  breakdown?: MatchBreakdown;
}

/**
 * Query criteria for an ad-hoc match check. All fields optional so callers
 * can match on whatever signals they have; `threshold` and
 * `max_candidates` tune the result set.
 */
export interface MatchRequest {
  /** Partial name to match against. */
  name?: Partial<HumanName>;
  /** Birth date to match against (ISO `YYYY-MM-DD`). */
  birth_date?: string | null;
  /** Gender to match against. */
  gender?: Gender;
  /** Tax id to match against (strong signal). */
  tax_id?: string | null;
  /** Minimum score in `[0, 1]` for a candidate to be returned. */
  threshold?: number;
  /** Cap on the number of candidates returned. */
  max_candidates?: number;
}

// ─── Merge ───────────────────────────────────────────────────────────

/** Lifecycle state of a merge: completed, or later reversed/undone. */
export type MergeStatus = "Completed" | "Reversed";

/**
 * Request to merge a duplicate worker into a surviving "main" record. The
 * duplicate is soft-deleted; data is transferred to the main record.
 */
export interface MergeRequest {
  /** Id of the surviving record. */
  main_worker_id: string;
  /** Id of the record to merge in and soft-delete. */
  duplicate_worker_id: string;
  /** Reason recorded in the merge audit trail. */
  merge_reason?: string | null;
  /** User/operator performing the merge. */
  merged_by?: string | null;
}

/** Audit record describing one completed (or reversed) merge. */
export interface MergeRecord {
  /** Merge-record id. */
  id: string;
  /** Surviving record id. */
  main_worker_id: string;
  /** Merged-in (soft-deleted) record id. */
  duplicate_worker_id: string;
  /** Whether the merge is completed or has been reversed. */
  status: MergeStatus;
  /** User/operator who performed the merge. */
  merged_by?: string | null;
  /** Reason supplied for the merge. */
  merge_reason?: string | null;
  /** Match score at merge time, if matching drove the merge. */
  match_score?: number | null;
  /** Snapshot of data transferred from duplicate to main. */
  transferred_data?: unknown;
  /** When the merge happened (ISO 8601). */
  merged_at: string;
}

/** Response from a merge call: the audit record plus the updated main worker. */
export interface MergeResponse {
  /** The merge audit record. */
  merge_record: MergeRecord;
  /** The surviving worker after the merge. */
  main_worker: Worker;
}

// ─── Batch dedup ─────────────────────────────────────────────────────

/** Tuning knobs for a whole-index batch deduplication scan. */
export interface BatchDeduplicationRequest {
  /** Minimum score for a pair to count as a candidate duplicate. */
  threshold?: number;
  /** Cap on candidates considered per record. */
  max_candidates?: number;
  /** At/above this score, pairs are auto-merged instead of queued. */
  auto_merge_threshold?: number;
}

/** Summary of a batch deduplication run plus the items queued for review. */
export interface BatchDeduplicationResponse {
  /** Total records scanned. */
  workers_scanned: number;
  /** Duplicate pairs found. */
  duplicates_found: number;
  /** Pairs auto-merged (score ≥ `auto_merge_threshold`). */
  auto_merged: number;
  /** Pairs queued for manual review. */
  queued_for_review: number;
  /** The review-queue items produced. */
  review_items: ReviewQueueItem[];
}

/** Status of a {@link ReviewQueueItem} as an operator works through it. */
export type ReviewStatus = "pending" | "confirmed" | "rejected" | "automerged";

/** One candidate-duplicate pair awaiting (or having had) operator review. */
export interface ReviewQueueItem {
  /** Queue-item id. */
  id: string;
  /** First record of the candidate pair. */
  worker_id_a: string;
  /** Second record of the candidate pair. */
  worker_id_b: string;
  /** Overall match score for the pair. */
  match_score: number;
  /** Confidence band label for the pair. */
  match_quality: string;
  /** How the duplicate was detected (e.g. batch, real-time). */
  detection_method: string;
  /** Per-component score breakdown (shape varies). */
  score_breakdown?: unknown;
  /** Current review status. */
  status: ReviewStatus;
  /** Operator who reviewed the item, if any. */
  reviewed_by?: string | null;
  /** When the item was created (ISO 8601). */
  created_at: string;
  /** When the item was reviewed (ISO 8601), if reviewed. */
  reviewed_at?: string | null;
}

/** One operator verdict for a pending review item. */
export type ReviewDecision = "confirmed" | "rejected";

/** Response envelope for the stored review-queue list. */
export interface ReviewQueueListResponse {
  /** The stored review-queue items (newest first). */
  items: ReviewQueueItem[];
  /** Number of items returned. */
  total: number;
}

// ─── Audit ───────────────────────────────────────────────────────────

/**
 * One immutable audit-trail entry recording who changed what and when.
 * `old_values`/`new_values` are untyped JSON snapshots of the entity
 * around the change.
 */
export interface AuditEntry {
  /** Audit-entry id. */
  id: string;
  /** Type of entity affected (e.g. `worker`). */
  entity_type: string;
  /** Id of the affected entity. */
  entity_id: string;
  /** Action performed (e.g. created, updated, deleted, merged). */
  action: string;
  /** Acting user id, if known. */
  user_id?: string | null;
  /** Acting user's IP address, if captured. */
  user_ip_address?: string | null;
  /** Acting user's user-agent string, if captured. */
  user_agent?: string | null;
  /** JSON snapshot of the entity before the change. */
  old_values?: unknown;
  /** JSON snapshot of the entity after the change. */
  new_values?: unknown;
  /** When the change happened (ISO 8601). */
  created_at: string;
}

// ─── Cross-service links ─────────────────────────────────────────────

/**
 * An `EntityRef` URN naming a record in another service, e.g.
 * `person:0c4f1e2a-…` or `organization:9a2f-…`. See
 * `agents/share/cross-service-linking.md` §3.
 */
export type EntityRefUrn = string;

/**
 * The cross-service edge kinds a **worker** may originate (the service's
 * `PERMITTED_KINDS`, cross-service-linking §9):
 *
 * - `same_identity` — this worker is the same human as a `person` record
 *   (the federation backbone; either side may assert it).
 * - `employed_by` — this worker is employed by an `organization`; the
 *   edge's `role` carries the job title.
 *
 * Distinct from {@link LinkType} / {@link WorkerLink}, which are
 * *within-service* worker↔worker references.
 */
export type WorkerEdgeKind = "same_identity" | "employed_by";

/**
 * A stored cross-service edge as returned by the service (`LinkView`).
 * Named `EntityLink` to avoid colliding with the within-service
 * {@link WorkerLink}.
 */
export interface EntityLink {
  /** The edge id (also the `linked` event's `edge_id`). */
  id: string;
  /** This worker as an `EntityRef` URN (`worker:<id>`). */
  from_ref: EntityRefUrn;
  /** The edge kind token. */
  kind: string;
  /** The far record's `EntityRef` URN. */
  to_ref: EntityRefUrn;
  /** Role label — the job title on an `employed_by` edge. */
  role?: string | null;
  /** Confidence in `[0.0, 1.0]`; 1.0 for an operator assertion. */
  confidence?: number | null;
  /** How the edge was asserted (`operator`, `import`, …). */
  provenance: string;
  /** Validity start (`YYYY-MM-DD`). */
  valid_from?: string | null;
  /** Validity end (`YYYY-MM-DD`) — a "former" affiliation. */
  valid_to?: string | null;
}

/**
 * Body of `POST /api/workers/{id}/links`. Everything but `kind` and
 * `to_ref` is optional; `provenance` defaults to `operator` server-side.
 */
export interface CreateLinkRequest {
  /** The edge kind to assert. */
  kind: WorkerEdgeKind;
  /** The far record's `EntityRef` URN (target type is kind-specific). */
  to_ref: EntityRefUrn;
  /** Job title, for an `employed_by` edge. */
  role?: string | null;
  /** Confidence in `[0.0, 1.0]`. */
  confidence?: number | null;
  /** Provenance override; blank/omitted ⇒ `operator`. */
  provenance?: string | null;
  /** Validity start (`YYYY-MM-DD`). */
  valid_from?: string | null;
  /** Validity end (`YYYY-MM-DD`). */
  valid_to?: string | null;
}
