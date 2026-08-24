// TypeScript domain types mirroring the Rust Event Service models
// (schema.org/Event-based). Reference:
// event-service-with-loco/agents/models.md.
// NOTE: Event Service mounts REST under `/api/`.

// ─── HTTP envelope ───────────────────────────────────────────────────

/**
 * Standard JSON envelope returned by every Event Service endpoint.
 *
 * The service always wraps payloads so callers can distinguish transport
 * success from application success: on success `data` is populated and
 * `error` is null; on failure `error` carries the structured cause and
 * `data` is null. {@link ApiClient} unwraps this so callers see `T`.
 *
 * @typeParam T - The shape of the successful payload.
 */
export interface ApiResponse<T> {
  success: boolean;
  data: T | null;
  error: ApiErrorBody | null;
}

/** Structured error body inside a failed {@link ApiResponse}. */
export interface ApiErrorBody {
  /** Stable machine-readable error code (e.g. for branching in the UI). */
  code: string;
  /** Human-readable message safe to surface to operators. */
  message: string;
  /** Optional extra context (validation field errors, conflict candidates, …). */
  details?: unknown;
}

// ─── Status / mode / type enums ──────────────────────────────────────

/**
 * Lifecycle status of an event, mirroring schema.org/EventStatusType
 * (e.g. a postponed or rescheduled occurrence). Wire values are the
 * snake_case forms the service serializes.
 */
export type EventStatus =
  | "scheduled"
  | "cancelled"
  | "moved_online"
  | "postponed"
  | "rescheduled"
  | "completed";

/** All {@link EventStatus} values in canonical order — drives select inputs. */
export const EVENT_STATUSES: EventStatus[] = [
  "scheduled",
  "cancelled",
  "moved_online",
  "postponed",
  "rescheduled",
  "completed",
];

/** How attendees take part: in person, online, or both (schema.org/EventAttendanceModeEnumeration). */
export type EventAttendanceMode = "offline" | "online" | "mixed";

/** All {@link EventAttendanceMode} values in canonical order — drives select inputs. */
export const ATTENDANCE_MODES: EventAttendanceMode[] = [
  "offline",
  "online",
  "mixed",
];

/**
 * Event subtype, mirroring the schema.org/Event subtypes the service
 * recognizes (`generic` is the catch-all base type).
 */
export type EventType =
  | "generic"
  | "appointment"
  | "business"
  | "childrens"
  | "comedy"
  | "conference"
  | "course"
  | "dance"
  | "delivery"
  | "education"
  | "encounter"
  | "exhibition"
  | "festival"
  | "food"
  | "hackathon"
  | "incident"
  | "literary"
  | "music"
  | "performing_arts"
  | "publication"
  | "sale"
  | "screening"
  | "series"
  | "session"
  | "shift"
  | "social"
  | "sports"
  | "theater"
  | "visual_arts";

/** All {@link EventType} values in canonical order — drives select inputs. */
export const EVENT_TYPES: EventType[] = [
  "generic",
  "appointment",
  "business",
  "childrens",
  "comedy",
  "conference",
  "course",
  "dance",
  "delivery",
  "education",
  "encounter",
  "exhibition",
  "festival",
  "food",
  "hackathon",
  "incident",
  "literary",
  "music",
  "performing_arts",
  "publication",
  "sale",
  "screening",
  "series",
  "session",
  "shift",
  "social",
  "sports",
  "theater",
  "visual_arts",
];

// ─── Address (inline for events) ─────────────────────────────────────

/**
 * Postal address embedded inline within an event location (as opposed to
 * a referenced Place record). All parts are optional/nullable because the
 * service treats addresses as best-effort, partially-known data.
 */
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

/**
 * Location variant that references (or names) a physical Place, optionally
 * carrying its own address and geo-coordinates. The `kind` discriminant
 * distinguishes it within the {@link Location} union.
 */
export interface PlaceLocation {
  kind: "place";
  id?: string | null;
  name: string;
  address?: Address | null;
  latitude_as_decimal_degrees?: number | null;
  longitude_as_decimal_degrees?: number | null;
  url?: string | null;
}

/**
 * Location variant that is a bare postal address with no Place identity —
 * used when only the address fields are known.
 */
export interface PostalAddressLocation {
  kind: "postal_address";
  line1?: string | null;
  line2?: string | null;
  city?: string | null;
  state?: string | null;
  postal_code?: string | null;
  country?: string | null;
}

/**
 * Location variant for an online/virtual venue; `url` is required (the
 * join link) while `name` is an optional label.
 */
export interface VirtualLocation {
  kind: "virtual";
  name?: string | null;
  url: string;
}

/** Location variant for unstructured free-text descriptions ("Main hall, 2nd floor"). */
export interface TextLocation {
  kind: "text";
  value: string;
}

/**
 * Discriminated union of where an event takes place (schema.org/Event.location).
 * Switch on the `kind` field to narrow to the concrete variant. An event may
 * have several locations (e.g. a mixed-mode event with a place and a stream).
 */
export type Location =
  | PlaceLocation
  | PostalAddressLocation
  | VirtualLocation
  | TextLocation;

// ─── Party ───────────────────────────────────────────────────────────

/** Whether a {@link Party} is an individual person or an organization. */
export type PartyKind = "person" | "organization";

/**
 * A person or organization related to an event in some role (organizer,
 * performer, attendee, sponsor, …). `id` may reference a record in a
 * sibling index service; `name` is always present for display.
 */
export interface Party {
  kind: PartyKind;
  id?: string | null;
  name: string;
  email?: string | null;
  url?: string | null;
}

// ─── Reference ───────────────────────────────────────────────────────

/**
 * A loose reference to a related thing or creative work (schema.org
 * `about` / `workFeatured`). `kind` is a free-text type hint; `name` is
 * the display label.
 */
export interface Reference {
  id?: string | null;
  name: string;
  url?: string | null;
  kind?: string | null;
}

// ─── Offer ───────────────────────────────────────────────────────────

/** Ticket/offer availability, mirroring schema.org/ItemAvailability (PascalCase wire values). */
export type OfferAvailability =
  | "InStock"
  | "SoldOut"
  | "PreOrder"
  | "OutOfStock"
  | "Discontinued";

/**
 * A purchasable offer attached to an event, e.g. a ticket tier
 * (schema.org/Offer). `price` is a string to preserve exact decimal
 * formatting; `valid_from`/`valid_through` are RFC 3339 timestamps.
 */
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

/** Category of an event identifier; some categories are "strong" (see {@link STRONG_IDENTIFIER_TYPES}). */
export type IdentifierType =
  | "BookingNumber"
  | "ConfirmationCode"
  | "TicketNumber"
  | "EncounterId"
  | "TransactionId"
  | "ExternalRef"
  | "Tax"
  | "Other";

/**
 * Identifier types treated as "strong" by the matcher: an exact match on
 * any of these short-circuits scoring to 1.0 (a definite match), because
 * they uniquely pin down a single event occurrence.
 */
// Strong identifiers short-circuit matching to score = 1.0.
export const STRONG_IDENTIFIER_TYPES: IdentifierType[] = [
  "BookingNumber",
  "ConfirmationCode",
  "TicketNumber",
  "EncounterId",
  "TransactionId",
];

/** All {@link IdentifierType} values in canonical order — drives select inputs. */
export const IDENTIFIER_TYPES: IdentifierType[] = [
  "BookingNumber",
  "ConfirmationCode",
  "TicketNumber",
  "EncounterId",
  "TransactionId",
  "ExternalRef",
  "Tax",
  "Other",
];

/** FHIR-style "use" qualifier for an identifier (which value is current/official). */
export type IdentifierUse = "usual" | "official" | "temp" | "secondary" | "old";

/**
 * A typed external identifier for an event (booking number, ticket
 * number, …). `system` namespaces the `value` (e.g. issuing system URI);
 * together they form a globally unique key used by deterministic matching.
 */
export interface Identifier {
  use_type?: IdentifierUse | null;
  identifier_type: IdentifierType;
  system: string;
  value: string;
  assigner?: string | null;
}

// ─── EventLink ───────────────────────────────────────────────────────

/**
 * Relationship between two events. `replaces`/`replaced-by` are the merge
 * link directions (surviving ↔ merged record); `refer`/`seealso` are soft
 * cross-references.
 */
export type LinkType = "replaced-by" | "replaces" | "refer" | "seealso";

/** A directed link from this event to another (`other_event_id`) with a typed relationship. */
export interface EventLink {
  other_event_id: string;
  link_type: LinkType;
}

// ─── Event ───────────────────────────────────────────────────────────

/**
 * The core Event resource (schema.org/Event), the front-end's primary
 * domain entity and the JSON shape exchanged with the service.
 *
 * Most fields are optional because the service accepts partial records and
 * fills defaults; only `name` and `start_date` are required to create one.
 * The time-window fields (`start_date`/`end_date`/`door_time`/`duration`/
 * `time_zone`/`all_day`) describe when the event happens and feed window-
 * overlap matching. `id`/`created_at`/`updated_at` are server-assigned and
 * absent on create.
 */
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

/**
 * Confidence band the matcher assigns from the overall score, in
 * descending certainty. `definite` indicates a deterministic
 * short-circuit (e.g. strong identifier hit); the rest are probabilistic
 * bands.
 */
export type MatchQuality =
  | "definite"
  | "certain"
  | "probable"
  | "possible"
  | "unlikely";

/** Per-component score map (component name → 0..1 score, or null if not applicable). */
export type MatchBreakdown = Record<string, number | null>;

/**
 * One candidate returned by a match/duplicate-check call: the candidate
 * `event`, its overall `score` (0..1), the derived `quality` band, and an
 * optional per-component `breakdown` for explainability.
 */
export interface MatchResult {
  event: Event;
  score: number;
  quality: MatchQuality;
  breakdown?: MatchBreakdown;
}

/**
 * Query payload for the match endpoint: the partial event attributes to
 * score candidates against. `threshold` filters out weak matches and
 * `max_candidates` caps the result count.
 */
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

/** Whether a merge is in effect (`Completed`) or has been undone (`Reversed`). */
export type MergeStatus = "Completed" | "Reversed";

/**
 * Request to merge a duplicate event into a surviving "main" event.
 * `merge_reason`/`merged_by` are recorded for audit; the duplicate is
 * soft-deleted and linked back to the survivor.
 */
export interface MergeRequest {
  main_event_id: string;
  duplicate_event_id: string;
  merge_reason?: string | null;
  merged_by?: string | null;
}

/**
 * Persisted history record of a merge operation, including a snapshot of
 * the data transferred from the duplicate (`transferred_data`) so the
 * merge can be audited or reversed.
 */
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

/** Result of a merge call: the {@link MergeRecord} plus the post-merge surviving event. */
export interface MergeResponse {
  merge_record: MergeRecord;
  main_event: Event;
}

// ─── Audit ───────────────────────────────────────────────────────────

/**
 * One immutable audit-trail entry recording a mutation (who/what/when).
 * `old_values`/`new_values` hold JSON snapshots before/after the change;
 * the user_* fields capture request provenance for HIPAA-style auditing.
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
