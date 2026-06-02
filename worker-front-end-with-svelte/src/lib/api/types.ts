// TypeScript domain types mirroring the Rust Worker Service models.
// Reference: worker-service-rust-crate/AGENTS/models.md.
// Field names use snake_case to match the JSON wire format produced by
// the Axum/SeaORM stack.

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

// ─── Shared primitives ───────────────────────────────────────────────

export type Gender = "male" | "female" | "other" | "unknown";

export type AddressUse = "home" | "work" | "temp" | "old" | "billing";

export interface Address {
    use_type?: AddressUse | null;
    line1?: string | null;
    line2?: string | null;
    city?: string | null;
    state?: string | null;
    postal_code?: string | null;
    country?: string | null;
}

export type ContactPointSystem = "phone" | "fax" | "email" | "pager" | "url" | "sms" | "other";
export type ContactPointUse = "home" | "work" | "temp" | "old" | "mobile";

export interface ContactPoint {
    system: ContactPointSystem;
    value: string;
    use_type?: ContactPointUse | null;
}

// ─── Identifier ──────────────────────────────────────────────────────

export type IdentifierUse = "usual" | "official" | "temp" | "secondary" | "old";
export type IdentifierType = "MRN" | "SSN" | "DL" | "NPI" | "PPN" | "TAX" | "Other";

export interface Identifier {
    use_type?: IdentifierUse | null;
    identifier_type: IdentifierType;
    system: string;
    value: string;
    assigner?: string | null;
}

// ─── Identity document ───────────────────────────────────────────────

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

export interface IdentityDocument {
    document_type: DocumentType;
    number: string;
    issuing_country?: string | null;
    issuing_authority?: string | null;
    issue_date?: string | null;
    expiry_date?: string | null;
    verified?: boolean;
}

// ─── Names ───────────────────────────────────────────────────────────

export type NameUse = "usual" | "official" | "temp" | "nickname" | "anonymous" | "old" | "maiden";

export interface HumanName {
    use_type?: NameUse | null;
    family: string;
    given: string[];
    prefix?: string[];
    suffix?: string[];
}

// ─── Emergency contact ───────────────────────────────────────────────

export interface EmergencyContact {
    name: string;
    relationship: string;
    telecom?: ContactPoint[];
    address?: Address | null;
    is_primary?: boolean;
}

// ─── Worker link ─────────────────────────────────────────────────────

export type LinkType = "replaced-by" | "replaces" | "refer" | "seealso";

export interface WorkerLink {
    other_worker_id: string;
    link_type: LinkType;
}

// ─── Worker ──────────────────────────────────────────────────────────

export interface Worker {
    id?: string;
    identifiers?: Identifier[];
    active?: boolean;
    name: HumanName;
    additional_names?: HumanName[];
    telecom?: ContactPoint[];
    gender: Gender;
    birth_date?: string | null;
    tax_id?: string | null;
    documents?: IdentityDocument[];
    emergency_contacts?: EmergencyContact[];
    deceased?: boolean;
    deceased_datetime?: string | null;
    addresses?: Address[];
    marital_status?: string | null;
    multiple_birth?: boolean | null;
    photo?: string[];
    managing_organization?: string | null;
    links?: WorkerLink[];
    created_at?: string;
    updated_at?: string;
}

// ─── Matching ────────────────────────────────────────────────────────

export type MatchQuality = "definite" | "certain" | "probable" | "possible" | "unlikely";

// Per-component score breakdown emitted by the matcher. Keys: name,
// birth_date, gender, address, identifier, tax_id, document.
export type MatchBreakdown = Record<string, number | null>;

export interface MatchResult {
    worker: Worker;
    score: number;
    quality: MatchQuality;
    breakdown?: MatchBreakdown;
}

export interface MatchRequest {
    name?: Partial<HumanName>;
    birth_date?: string | null;
    gender?: Gender;
    tax_id?: string | null;
    threshold?: number;
    max_candidates?: number;
}

// ─── Merge ───────────────────────────────────────────────────────────

export type MergeStatus = "Completed" | "Reversed";

export interface MergeRequest {
    main_worker_id: string;
    duplicate_worker_id: string;
    merge_reason?: string | null;
    merged_by?: string | null;
}

export interface MergeRecord {
    id: string;
    main_worker_id: string;
    duplicate_worker_id: string;
    status: MergeStatus;
    merged_by?: string | null;
    merge_reason?: string | null;
    match_score?: number | null;
    transferred_data?: unknown;
    merged_at: string;
}

export interface MergeResponse {
    merge_record: MergeRecord;
    main_worker: Worker;
}

// ─── Batch dedup ─────────────────────────────────────────────────────

export interface BatchDeduplicationRequest {
    threshold?: number;
    max_candidates?: number;
    auto_merge_threshold?: number;
}

export interface BatchDeduplicationResponse {
    workers_scanned: number;
    duplicates_found: number;
    auto_merged: number;
    queued_for_review: number;
    review_items: ReviewQueueItem[];
}

export type ReviewStatus = "Pending" | "Confirmed" | "Rejected" | "AutoMerged";

export interface ReviewQueueItem {
    id: string;
    worker_id_a: string;
    worker_id_b: string;
    match_score: number;
    match_quality: string;
    detection_method: string;
    score_breakdown?: unknown;
    status: ReviewStatus;
    reviewed_by?: string | null;
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
