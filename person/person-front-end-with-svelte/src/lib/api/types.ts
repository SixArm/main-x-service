// TypeScript domain types mirroring the Rust Person Service models.
// Reference: person-service-with-loco/AGENTS/models.md.
// Field names use snake_case to match the JSON wire format produced by
// the Axum/SeaORM stack.

// ─── HTTP envelope ───────────────────────────────────────────────────

/**
 * Standard response envelope wrapping every Person Service payload.
 * Exactly one of `data` (on success) / `error` (on failure) is populated,
 * keyed by `success`. {@link ApiClient} unwraps this to the bare `data`.
 */
export interface ApiResponse<T> {
    /** Whether the request succeeded at the application level. */
    success: boolean;
    /** The payload on success; `null` on failure. */
    data: T | null;
    /** The error detail on failure; `null` on success. */
    error: ApiErrorBody | null;
}

/** Machine-readable error detail carried in a failed {@link ApiResponse}. */
export interface ApiErrorBody {
    /** Stable error code for branching (e.g. `"VALIDATION"`). */
    code: string;
    /** Human-readable message suitable for display. */
    message: string;
    /** Optional structured details, e.g. per-field validation errors. */
    details?: unknown;
}

// ─── Shared primitives ───────────────────────────────────────────────

/** Administrative gender (FHIR-aligned). */
export type Gender = "male" | "female" | "other" | "unknown";

/** Intended use of a postal {@link Address}. */
export type AddressUse = "home" | "work" | "temp" | "old" | "billing";

/** A postal address; all parts optional to support partial records. */
export interface Address {
    use_type?: AddressUse | null;
    line1?: string | null;
    line2?: string | null;
    city?: string | null;
    state?: string | null;
    postal_code?: string | null;
    country?: string | null;
}

/** Channel of a {@link ContactPoint}. */
export type ContactPointSystem = "phone" | "fax" | "email" | "pager" | "url" | "sms" | "other";
/** Intended use of a {@link ContactPoint}. */
export type ContactPointUse = "home" | "work" | "temp" | "old" | "mobile";

/** A single means of contact (phone number, email address, …). */
export interface ContactPoint {
    /** The contact channel. */
    system: ContactPointSystem;
    /** The actual contact value (number / address / URL). */
    value: string;
    /** Optional intended use (home/work/mobile/…). */
    use_type?: ContactPointUse | null;
}

// ─── Identifier ──────────────────────────────────────────────────────

/** Intended use of an {@link Identifier}. */
export type IdentifierUse = "usual" | "official" | "temp" | "secondary" | "old";
/** Kind of {@link Identifier} (MRN, SSN, driver's licence, NPI, …). */
export type IdentifierType = "MRN" | "SSN" | "DL" | "NPI" | "PPN" | "TAX" | "Other";

/** A namespaced identifier assigned to a person by some system. */
export interface Identifier {
    /** Optional intended use. */
    use_type?: IdentifierUse | null;
    /** The kind of identifier. */
    identifier_type: IdentifierType;
    /** Issuing system / namespace (URI or OID) the value is scoped to. */
    system: string;
    /** The identifier value within `system`. */
    value: string;
    /** Optional organization that assigned the identifier. */
    assigner?: string | null;
}

// ─── Identity document ───────────────────────────────────────────────

/** Kind of government / official identity document. */
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

/** A government / official identity document (passport, licence, …). */
export interface IdentityDocument {
    /** Kind of document. */
    document_type: DocumentType;
    /** Document number (required). */
    number: string;
    /** ISO country code of the issuer. */
    issuing_country?: string | null;
    /** Issuing authority name. */
    issuing_authority?: string | null;
    /** Issue date (ISO 8601 date). */
    issue_date?: string | null;
    /** Expiry date (ISO 8601 date). */
    expiry_date?: string | null;
    /** Whether the document has been verified. */
    verified?: boolean;
}

// ─── Names ───────────────────────────────────────────────────────────

/** Intended use of a {@link HumanName} (legal, nickname, maiden, …). */
export type NameUse = "usual" | "official" | "temp" | "nickname" | "anonymous" | "old" | "maiden";

/** A structured human name (FHIR HumanName-aligned). */
export interface HumanName {
    /** Optional intended use of this name. */
    use_type?: NameUse | null;
    /** Family / surname. */
    family: string;
    /** Given names, in order. */
    given: string[];
    /** Honorific prefixes (Dr, Ms, …). */
    prefix?: string[];
    /** Suffixes (Jr, PhD, …). */
    suffix?: string[];
}

// ─── Emergency contact ───────────────────────────────────────────────

/** A person to contact in an emergency. */
export interface EmergencyContact {
    /** Contact's name. */
    name: string;
    /** Relationship to the person (parent, spouse, …). */
    relationship: string;
    /** Ways to reach the contact. */
    telecom?: ContactPoint[];
    /** Contact's address. */
    address?: Address | null;
    /** Whether this is the primary emergency contact. */
    is_primary?: boolean;
}

// ─── Person link ─────────────────────────────────────────────────────

/** Relationship between two person records (used by merge/dedup). */
export type LinkType = "replaced-by" | "replaces" | "refer" | "seealso";

/** A typed link from this person to another person record. */
export interface PersonLink {
    /** Id of the linked person. */
    other_person_id: string;
    /** Nature of the link. */
    link_type: LinkType;
}

// ─── Person ──────────────────────────────────────────────────────────

/**
 * The core Person aggregate — the system's primary domain entity.
 * Mirrors the Rust service model; `id`/timestamps are server-assigned and
 * absent on create. Most collections are optional to allow partial input.
 */
export interface Person {
    /** Server-assigned UUID; absent on create. */
    id?: string;
    /** Namespaced identifiers (MRN, SSN, …). */
    identifiers?: Identifier[];
    /** Whether the record is active (false once soft-deleted/merged away). */
    active?: boolean;
    /** Primary name (required). */
    name: HumanName;
    /** Aliases / additional names. */
    additional_names?: HumanName[];
    /** Contact points (phone, email, …). */
    telecom?: ContactPoint[];
    /** Administrative gender (required). */
    gender: Gender;
    /** Birth date (ISO 8601 date). */
    birth_date?: string | null;
    /** Tax identifier; a strong matching signal. */
    tax_id?: string | null;
    /** Identity documents. */
    documents?: IdentityDocument[];
    /** Emergency contacts. */
    emergency_contacts?: EmergencyContact[];
    /** Whether the person is deceased. */
    deceased?: boolean;
    /** Date/time of death (ISO 8601). */
    deceased_datetime?: string | null;
    /** Postal addresses. */
    addresses?: Address[];
    /** Marital status (free text / coded). */
    marital_status?: string | null;
    /** Part of a multiple birth, if known. */
    multiple_birth?: boolean | null;
    /** Photo references (URLs or data URIs). */
    photo?: string[];
    /** Id of the managing organization. */
    managing_organization?: string | null;
    /** Links to other person records (merge/dedup relationships). */
    links?: PersonLink[];
    /** Server-set creation timestamp (ISO 8601). */
    created_at?: string;
    /** Server-set last-update timestamp (ISO 8601). */
    updated_at?: string;
}

// ─── Matching ────────────────────────────────────────────────────────

/** Confidence band assigned to a match, derived from its score. */
export type MatchQuality = "definite" | "certain" | "probable" | "possible" | "unlikely";

/**
 * Per-component score breakdown emitted by the matcher. Keys: name,
 * birth_date, gender, address, identifier, tax_id, document. A `null`
 * value means the component was not evaluated (data absent on a side).
 */
export type MatchBreakdown = Record<string, number | null>;

/** A single scored candidate returned by a match/dedup query. */
export interface MatchResult {
    /** The candidate person. */
    person: Person;
    /** Overall confidence score in `[0, 1]`. */
    score: number;
    /** Confidence band for the score. */
    quality: MatchQuality;
    /** Optional per-component contribution to the score. */
    breakdown?: MatchBreakdown;
}

/** Query record + tuning parameters for a probabilistic match request. */
export interface MatchRequest {
    /** Partial name to match against. */
    name?: Partial<HumanName>;
    /** Birth date to match against. */
    birth_date?: string | null;
    /** Gender to match against. */
    gender?: Gender;
    /** Tax id to match against (strong signal). */
    tax_id?: string | null;
    /** Minimum score to include a candidate. */
    threshold?: number;
    /** Cap on the number of candidates returned. */
    max_candidates?: number;
}

// ─── Merge ───────────────────────────────────────────────────────────

/** Lifecycle state of a {@link MergeRecord}. */
export type MergeStatus = "Completed" | "Reversed";

/** Request to merge a confirmed duplicate into a main record. */
export interface MergeRequest {
    /** Surviving record's id. */
    main_person_id: string;
    /** Duplicate record's id (soft-deleted on merge). */
    duplicate_person_id: string;
    /** Optional human-supplied reason for audit. */
    merge_reason?: string | null;
    /** Optional id of the operator performing the merge. */
    merged_by?: string | null;
}

/** Audit record describing a completed (or reversed) merge. */
export interface MergeRecord {
    /** Merge record id. */
    id: string;
    /** Surviving record's id. */
    main_person_id: string;
    /** Duplicate record's id. */
    duplicate_person_id: string;
    /** Whether the merge is completed or has been reversed. */
    status: MergeStatus;
    /** Operator who performed the merge. */
    merged_by?: string | null;
    /** Reason recorded for the merge. */
    merge_reason?: string | null;
    /** Match score at merge time, if computed. */
    match_score?: number | null;
    /** Snapshot of data transferred from duplicate to main. */
    transferred_data?: unknown;
    /** Merge timestamp (ISO 8601). */
    merged_at: string;
}

/** Result of a merge: the audit record plus the updated main person. */
export interface MergeResponse {
    /** The created merge audit record. */
    merge_record: MergeRecord;
    /** The surviving main person after the merge. */
    main_person: Person;
}

// ─── Batch dedup ─────────────────────────────────────────────────────

/** Tuning parameters for a whole-index deduplication scan. */
export interface BatchDeduplicationRequest {
    /** Minimum score to treat a pair as a candidate duplicate. */
    threshold?: number;
    /** Cap on candidates considered per record. */
    max_candidates?: number;
    /** Score at/above which pairs are auto-merged rather than queued. */
    auto_merge_threshold?: number;
}

/** Summary statistics and review items from a deduplication scan. */
export interface BatchDeduplicationResponse {
    /** Number of person records scanned. */
    persons_scanned: number;
    /** Number of duplicate pairs detected. */
    duplicates_found: number;
    /** Number of pairs auto-merged above the auto-merge threshold. */
    auto_merged: number;
    /** Number of pairs queued for human review. */
    queued_for_review: number;
    /** The queued items awaiting review. */
    review_items: ReviewQueueItem[];
}

/** Disposition of a {@link ReviewQueueItem}. */
export type ReviewStatus = "Pending" | "Confirmed" | "Rejected" | "AutoMerged";

/** A candidate duplicate pair awaiting (or having received) human review. */
export interface ReviewQueueItem {
    /** Review item id. */
    id: string;
    /** First person in the pair. */
    person_id_a: string;
    /** Second person in the pair. */
    person_id_b: string;
    /** Overall match score for the pair. */
    match_score: number;
    /** Confidence band for the score. */
    match_quality: string;
    /** How the pair was detected (real-time / batch / …). */
    detection_method: string;
    /** Optional per-component score breakdown. */
    score_breakdown?: unknown;
    /** Current disposition. */
    status: ReviewStatus;
    /** Operator who reviewed the item. */
    reviewed_by?: string | null;
    /** Creation timestamp (ISO 8601). */
    created_at: string;
    /** Review timestamp (ISO 8601), if reviewed. */
    reviewed_at?: string | null;
}

// ─── Audit ───────────────────────────────────────────────────────────

/** One HIPAA-style audit-trail entry recording a change to an entity. */
export interface AuditEntry {
    /** Audit entry id. */
    id: string;
    /** Type of entity changed (e.g. `"person"`). */
    entity_type: string;
    /** Id of the changed entity. */
    entity_id: string;
    /** Action performed (created/updated/deleted/merged/…). */
    action: string;
    /** Id of the acting user. */
    user_id?: string | null;
    /** IP address of the acting user. */
    user_ip_address?: string | null;
    /** User-agent of the acting user. */
    user_agent?: string | null;
    /** JSON snapshot of values before the change. */
    old_values?: unknown;
    /** JSON snapshot of values after the change. */
    new_values?: unknown;
    /** When the change occurred (ISO 8601). */
    created_at: string;
}
