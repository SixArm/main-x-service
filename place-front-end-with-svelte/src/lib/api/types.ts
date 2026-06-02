// TypeScript domain types mirroring the Rust Place Service models
// (schema.org/Place-based). Reference:
// place-service-rust-crate/AGENTS/models.md.

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

// ─── PostalAddress (schema.org/PostalAddress) ────────────────────────

export interface PostalAddress {
    street_address?: string | null;
    address_locality?: string | null;
    address_region?: string | null;
    address_country?: string | null;
    postal_code?: string | null;
}

export function blankPostalAddress(): PostalAddress {
    return {
        street_address: null,
        address_locality: null,
        address_region: null,
        address_country: null,
        postal_code: null,
    };
}

// ─── GeoCoordinates ──────────────────────────────────────────────────

export interface GeoCoordinates {
    latitude: number;
    longitude: number;
    elevation?: number | null;
}

// ─── PlaceType ───────────────────────────────────────────────────────

export type PlaceType =
    | "LocalBusiness"
    | "CivicStructure"
    | "AdministrativeArea"
    | "Landform"
    | "Park"
    | "Airport"
    | "Hospital"
    | "School"
    | "Library"
    | "Museum"
    | "Restaurant"
    | "Hotel"
    | { Other: string };

export const PLACE_TYPES: Exclude<PlaceType, { Other: string }>[] = [
    "LocalBusiness",
    "CivicStructure",
    "AdministrativeArea",
    "Landform",
    "Park",
    "Airport",
    "Hospital",
    "School",
    "Library",
    "Museum",
    "Restaurant",
    "Hotel",
];

// ─── PlaceIdentifier ─────────────────────────────────────────────────

export type IdentifierType =
    | "GlobalLocationNumber"
    | "BranchCode"
    | "Fips"
    | "Gnis"
    | "OpenStreetMap"
    | { Custom: string };

export interface PlaceIdentifier {
    identifier_type: IdentifierType;
    value: string;
}

// ─── AmenityFeature ──────────────────────────────────────────────────

export interface AmenityFeature {
    name: string;
    value?: string | null;
}

// ─── OpeningHoursSpecification ───────────────────────────────────────

export type DayOfWeek =
    | "Monday"
    | "Tuesday"
    | "Wednesday"
    | "Thursday"
    | "Friday"
    | "Saturday"
    | "Sunday";

export const DAYS_OF_WEEK: DayOfWeek[] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

export interface OpeningHoursSpecification {
    day_of_week: DayOfWeek;
    opens: string; // HH:MM
    closes: string; // HH:MM
}

// ─── Place ───────────────────────────────────────────────────────────

export interface Place {
    id?: string;
    name: string;
    alternate_name?: string | null;
    description?: string | null;
    place_type?: PlaceType | null;
    address?: PostalAddress | null;
    geo?: GeoCoordinates | null;
    telephone?: string | null;
    fax_number?: string | null;
    url?: string | null;
    global_location_number?: string | null;
    branch_code?: string | null;
    contained_in_place?: string | null;
    keywords?: string[];
    identifiers?: PlaceIdentifier[];
    amenity_features?: AmenityFeature[];
    opening_hours?: OpeningHoursSpecification[];
    is_accessible_for_free?: boolean | null;
    public_access?: boolean | null;
    smoking_allowed?: boolean | null;
    maximum_attendee_capacity?: number | null;
    is_deleted?: boolean;
    deleted_at?: string | null;
    created_at?: string;
    updated_at?: string;
}

// ─── Matching ────────────────────────────────────────────────────────

export type MatchConfidence = "Certain" | "Probable" | "Possible" | "Unlikely";

export interface MatchBreakdown {
    name_score?: number | null;
    geo_score?: number | null;
    address_score?: number | null;
    place_type_score?: number | null;
    identifier_score?: number | null;
    phonetic_match?: boolean;
    deterministic_match?: boolean;
}

export interface MatchResult {
    place: Place;
    score: number;
    confidence: MatchConfidence;
    breakdown?: MatchBreakdown;
}

export interface MatchRequest {
    name?: string;
    address?: PostalAddress;
    geo?: GeoCoordinates;
    place_type?: PlaceType;
    identifiers?: PlaceIdentifier[];
    threshold?: number;
    max_candidates?: number;
}

// ─── Merge ───────────────────────────────────────────────────────────

export type MergeStatus = "Completed" | "Reversed";

export interface MergeRequest {
    main_place_id: string;
    duplicate_place_id: string;
    merge_reason?: string | null;
    merged_by?: string | null;
}

export interface MergeRecord {
    id: string;
    main_place_id: string;
    duplicate_place_id: string;
    status: MergeStatus;
    merged_by?: string | null;
    merge_reason?: string | null;
    match_score?: number | null;
    transferred_data?: unknown;
    merged_at: string;
}

export interface MergeResponse {
    merge_record: MergeRecord;
    main_place: Place;
}

// ─── Batch dedup ─────────────────────────────────────────────────────

export interface BatchDeduplicationRequest {
    threshold?: number;
    max_candidates?: number;
    auto_merge_threshold?: number;
}

export interface BatchDeduplicationResponse {
    places_scanned: number;
    duplicates_found: number;
    auto_merged: number;
    queued_for_review: number;
    review_items: ReviewQueueItem[];
}

export type ReviewStatus = "Pending" | "Confirmed" | "Rejected" | "AutoMerged";

export interface ReviewQueueItem {
    id: string;
    place_id_a: string;
    place_id_b: string;
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
