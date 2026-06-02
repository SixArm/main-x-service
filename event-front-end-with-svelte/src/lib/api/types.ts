// TypeScript domain types mirroring the Rust Event Service models
// (schema.org/Event-based). Reference:
// event-service-rust-crate/AGENTS/models.md.
// NOTE: Event Service mounts REST under `/api/v1/`.

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

// ─── Status / mode / type enums ──────────────────────────────────────

export type EventStatus =
    | "scheduled"
    | "cancelled"
    | "moved_online"
    | "postponed"
    | "rescheduled"
    | "completed";

export const EVENT_STATUSES: EventStatus[] = [
    "scheduled", "cancelled", "moved_online", "postponed", "rescheduled", "completed",
];

export type EventAttendanceMode = "offline" | "online" | "mixed";

export const ATTENDANCE_MODES: EventAttendanceMode[] = ["offline", "online", "mixed"];

export type EventType =
    | "generic" | "appointment" | "business" | "childrens" | "comedy"
    | "conference" | "course" | "dance" | "delivery" | "education"
    | "encounter" | "exhibition" | "festival" | "food" | "hackathon"
    | "incident" | "literary" | "music" | "performing_arts" | "publication"
    | "sale" | "screening" | "series" | "session" | "shift"
    | "social" | "sports" | "theater" | "visual_arts";

export const EVENT_TYPES: EventType[] = [
    "generic", "appointment", "business", "childrens", "comedy",
    "conference", "course", "dance", "delivery", "education",
    "encounter", "exhibition", "festival", "food", "hackathon",
    "incident", "literary", "music", "performing_arts", "publication",
    "sale", "screening", "series", "session", "shift",
    "social", "sports", "theater", "visual_arts",
];

// ─── Address (inline for events) ─────────────────────────────────────

export interface Address {
    use_type?: string | null;
    line1?: string | null;
    line2?: string | null;
    city?: string | null;
    state?: string | null;
    postal_code?: string | null;
    country?: string | null;
}

// ─── Location union (schema.org/Event.location) ──────────────────────

export interface PlaceLocation {
    kind: "place";
    id?: string | null;
    name: string;
    address?: Address | null;
    latitude?: number | null;
    longitude?: number | null;
    url?: string | null;
}

export interface PostalAddressLocation {
    kind: "postal_address";
    line1?: string | null;
    line2?: string | null;
    city?: string | null;
    state?: string | null;
    postal_code?: string | null;
    country?: string | null;
}

export interface VirtualLocation {
    kind: "virtual";
    name?: string | null;
    url: string;
}

export interface TextLocation {
    kind: "text";
    value: string;
}

export type Location = PlaceLocation | PostalAddressLocation | VirtualLocation | TextLocation;

// ─── Party ───────────────────────────────────────────────────────────

export type PartyKind = "person" | "organization";

export interface Party {
    kind: PartyKind;
    id?: string | null;
    name: string;
    email?: string | null;
    url?: string | null;
}

// ─── Reference ───────────────────────────────────────────────────────

export interface Reference {
    id?: string | null;
    name: string;
    url?: string | null;
    kind?: string | null;
}

// ─── Offer ───────────────────────────────────────────────────────────

export type OfferAvailability = "InStock" | "SoldOut" | "PreOrder" | "OutOfStock" | "Discontinued";

export interface Offer {
    name?: string | null;
    price?: string | null;
    price_currency?: string | null;
    url?: string | null;
    availability?: OfferAvailability | null;
    valid_from?: string | null;
    valid_through?: string | null;
}

// ─── Identifier ──────────────────────────────────────────────────────

export type IdentifierType =
    | "BookingNumber"
    | "ConfirmationCode"
    | "TicketNumber"
    | "EncounterId"
    | "TransactionId"
    | "ExternalRef"
    | "Tax"
    | "Other";

// Strong identifiers short-circuit matching to score = 1.0.
export const STRONG_IDENTIFIER_TYPES: IdentifierType[] = [
    "BookingNumber", "ConfirmationCode", "TicketNumber", "EncounterId", "TransactionId",
];

export const IDENTIFIER_TYPES: IdentifierType[] = [
    "BookingNumber", "ConfirmationCode", "TicketNumber", "EncounterId",
    "TransactionId", "ExternalRef", "Tax", "Other",
];

export type IdentifierUse = "usual" | "official" | "temp" | "secondary" | "old";

export interface Identifier {
    use_type?: IdentifierUse | null;
    identifier_type: IdentifierType;
    system: string;
    value: string;
    assigner?: string | null;
}

// ─── EventLink ───────────────────────────────────────────────────────

export type LinkType = "replaced-by" | "replaces" | "refer" | "seealso";

export interface EventLink {
    other_event_id: string;
    link_type: LinkType;
}

// ─── Event ───────────────────────────────────────────────────────────

export interface Event {
    id?: string;
    identifiers?: Identifier[];
    active?: boolean;
    name: string;
    alternate_names?: string[];
    description?: string | null;
    disambiguating_description?: string | null;
    url?: string | null;
    image?: string[];
    same_as?: string[];
    keywords?: string[];
    start_date: string; // RFC 3339 — required
    end_date?: string | null;
    door_time?: string | null;
    duration?: string | null; // ISO 8601 e.g. "PT1H30M"
    previous_start_date?: string | null;
    time_zone?: string | null;
    all_day?: boolean;
    event_status?: EventStatus | null;
    event_attendance_mode?: EventAttendanceMode | null;
    event_type?: EventType | null;
    typical_age_range?: string | null;
    in_language?: string[];
    is_accessible_for_free?: boolean | null;
    maximum_attendee_capacity?: number | null;
    maximum_physical_attendee_capacity?: number | null;
    maximum_virtual_attendee_capacity?: number | null;
    remaining_attendee_capacity?: number | null;
    location?: Location[];
    organizers?: Party[];
    performers?: Party[];
    attendees?: Party[];
    sponsors?: Party[];
    funders?: Party[];
    contributors?: Party[];
    about?: Reference[];
    works?: Reference[];
    super_event?: string | null;
    sub_events?: string[];
    offers?: Offer[];
    links?: EventLink[];
    created_at?: string;
    updated_at?: string;
}

// ─── Matching ────────────────────────────────────────────────────────

export type MatchQuality = "definite" | "certain" | "probable" | "possible" | "unlikely";

export type MatchBreakdown = Record<string, number | null>;

export interface MatchResult {
    event: Event;
    score: number;
    quality: MatchQuality;
    breakdown?: MatchBreakdown;
}

export interface MatchRequest {
    name?: string;
    start_date?: string;
    end_date?: string | null;
    location?: Location[];
    organizers?: Party[];
    identifiers?: Identifier[];
    threshold?: number;
    max_candidates?: number;
}

// ─── Merge ───────────────────────────────────────────────────────────

export type MergeStatus = "Completed" | "Reversed";

export interface MergeRequest {
    main_event_id: string;
    duplicate_event_id: string;
    merge_reason?: string | null;
    merged_by?: string | null;
}

export interface MergeRecord {
    id: string;
    main_event_id: string;
    duplicate_event_id: string;
    status: MergeStatus;
    merged_by?: string | null;
    merge_reason?: string | null;
    match_score?: number | null;
    transferred_data?: unknown;
    merged_at: string;
}

export interface MergeResponse {
    merge_record: MergeRecord;
    main_event: Event;
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
