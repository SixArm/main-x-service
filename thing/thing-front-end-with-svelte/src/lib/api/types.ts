// TypeScript domain types mirroring the Rust Thing Service models
// (schema.org/Thing-based). Reference:
// thing-service-rust-crate/AGENTS/models.md.

// ─── HTTP envelope ───────────────────────────────────────────────────

export interface ApiResponse<T> {
    success: boolean;
    data: T | null;
    error: ApiErrorBody | null;
}

export interface ApiErrorBody {
    code: string;
    message: string;
    details?: unknown;
}

// ─── IdentifierType (schema.org/PropertyValue) ───────────────────────

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

export const IDENTIFIER_TYPE_OPTIONS: Exclude<IdentifierType, { Custom: string }>[] = [
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

export interface ThingIdentifier {
    property_id: IdentifierType;
    value: string;
    name?: string | null;
    url?: string | null;
}

export function blankThingIdentifier(): ThingIdentifier {
    return { property_id: "Sku", value: "", name: null, url: null };
}

// ─── Thing ───────────────────────────────────────────────────────────

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

export type MatchConfidence = "Certain" | "Probable" | "Possible" | "Unlikely";

export interface MatchBreakdown {
    name_score?: number | null;
    identifier_score?: number | null;
    description_score?: number | null;
    url_score?: number | null;
    same_as_score?: number | null;
    phonetic_match?: boolean;
    deterministic_match?: boolean;
}

export interface MatchResult {
    thing: Thing;
    score: number;
    confidence: MatchConfidence;
    breakdown?: MatchBreakdown;
}

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

export type MergeStatus = "Completed" | "Reversed";

export interface MergeRequest {
    main_thing_id: string;
    duplicate_thing_id: string;
    merge_reason?: string | null;
    merged_by?: string | null;
}

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

export interface MergeResponse {
    merge_record: MergeRecord;
    main_thing: Thing;
}

// ─── Batch dedup ─────────────────────────────────────────────────────

export interface BatchDeduplicationRequest {
    threshold?: number;
    max_candidates?: number;
    auto_merge_threshold?: number;
}

export interface BatchDeduplicationResponse {
    things_scanned: number;
    duplicates_found: number;
    auto_merged: number;
    queued_for_review: number;
    review_items: ReviewQueueItem[];
}

export type ReviewStatus = "Pending" | "Confirmed" | "Rejected" | "AutoMerged";

export interface ReviewQueueItem {
    id: string;
    thing_id_a: string;
    thing_id_b: string;
    match_score: number;
    match_quality: string;
    status: ReviewStatus;
    created_at: string;
    reviewed_at?: string | null;
}

// ─── Audit ───────────────────────────────────────────────────────────

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
