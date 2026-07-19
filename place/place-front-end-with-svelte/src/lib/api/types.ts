// TypeScript domain types mirroring the Rust Place Service models
// (schema.org/Place-based). Reference:
// place-service-with-loco/AGENTS/models.md.

// ─── HTTP envelope ───────────────────────────────────────────────────

/**
 * Standard response envelope returned by every Place Service endpoint.
 * Exactly one of `data` (on success) or `error` (on failure) is populated.
 * @typeParam T - The success payload type.
 */
export interface ApiResponse<T> {
  /** `true` on success; `false` signals a structured failure. */
  success: boolean;
  /** Success payload, or `null` on failure / no content. */
  data: T | null;
  /** Structured error, or `null` on success. */
  error: ApiErrorBody | null;
}

/** Structured error body inside a failed {@link ApiResponse}. */
export interface ApiErrorBody {
  /** Machine-readable error code (e.g. `"VALIDATION_ERROR"`). */
  code: string;
  /** Human-readable error message. */
  message: string;
  /** Optional extra context (e.g. per-field validation messages). */
  details?: unknown;
}

// ─── PostalAddress (schema.org/PostalAddress) ────────────────────────

/**
 * Postal address, mirroring schema.org/PostalAddress. All parts are
 * optional/nullable; address validation server-side requires at least a
 * locality, postal code, or country.
 */
export interface PostalAddress {
  /** Street and number (schema.org `streetAddress`). */
  street_address?: string | null;
  /** City / town (schema.org `addressLocality`). */
  address_locality?: string | null;
  /** State / province / region (schema.org `addressRegion`). */
  address_region?: string | null;
  /** Country, ideally ISO 3166 (schema.org `addressCountry`). */
  address_country?: string | null;
  /** Postal / ZIP code (schema.org `postalCode`). */
  postal_code?: string | null;
}

/**
 * Build an empty {@link PostalAddress} with every field explicitly `null`.
 *
 * Used to seed form state so each field is a controlled input from the
 * start (rather than `undefined`, which would make bindings uncontrolled).
 * @returns A fresh address object with all fields `null`.
 */
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

/**
 * Geographic point (schema.org/GeoCoordinates). Latitude/longitude are
 * required when geo is present; server-side validation bounds them to
 * [-90, 90] and [-180, 180] respectively.
 */
export interface GeoCoordinates {
  /** Degrees north (−90 … 90). */
  latitude: number;
  /** Degrees east (−180 … 180). */
  longitude: number;
  /** Optional elevation in metres. */
  elevation?: number | null;
}

// ─── PlaceType ───────────────────────────────────────────────────────

/**
 * schema.org place category. A known string variant, or an open
 * `{ Other: string }` escape hatch for categories outside the enum —
 * mirrors the Rust enum's `Other(String)` variant on the wire.
 */
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

/**
 * The known {@link PlaceType} string variants (excluding `{ Other }`),
 * in display order. Drives the place-type `<select>` options.
 */
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

/**
 * External identifier scheme for a place. Known schemes (GLN, branch
 * code, FIPS, GNIS, OSM) plus an open `{ Custom: string }` variant for
 * any other scheme name.
 */
export type IdentifierType =
  | "GlobalLocationNumber"
  | "BranchCode"
  | "Fips"
  | "Gnis"
  | "OpenStreetMap"
  | { Custom: string };

/** A single external identifier: its scheme plus the value. */
export interface PlaceIdentifier {
  /** The identifier scheme (GLN, GNIS, custom, …). */
  identifier_type: IdentifierType;
  /** The identifier value within that scheme. */
  value: string;
}

// ─── AmenityFeature ──────────────────────────────────────────────────

/** A named amenity (schema.org/LocationFeatureSpecification) for a place. */
export interface AmenityFeature {
  /** Amenity name (e.g. `"WiFi"`, `"Parking"`). */
  name: string;
  /** Optional value/qualifier for the amenity. */
  value?: string | null;
}

// ─── OpeningHoursSpecification ───────────────────────────────────────

/** Day of the week, used by {@link OpeningHoursSpecification}. */
export type DayOfWeek =
  | "Monday"
  | "Tuesday"
  | "Wednesday"
  | "Thursday"
  | "Friday"
  | "Saturday"
  | "Sunday";

/** All seven {@link DayOfWeek} values, Monday-first, in display order. */
export const DAYS_OF_WEEK: DayOfWeek[] = [
  "Monday",
  "Tuesday",
  "Wednesday",
  "Thursday",
  "Friday",
  "Saturday",
  "Sunday",
];

/** A single open/close window for one day (schema.org/OpeningHoursSpecification). */
export interface OpeningHoursSpecification {
  /** Day this window applies to. */
  day_of_week: DayOfWeek;
  /** Opening time, `HH:MM` 24-hour. */
  opens: string; // HH:MM
  /** Closing time, `HH:MM` 24-hour. */
  closes: string; // HH:MM
}

// ─── Place ───────────────────────────────────────────────────────────

/**
 * The core Place entity (schema.org/Place), as exchanged with the
 * service. Server-managed fields (`id`, timestamps, soft-delete) are
 * optional because they are absent when creating a new record.
 */
export interface Place {
  /** Server-assigned UUID; absent on create. */
  id?: string;
  /** Primary display name (required). */
  name: string;
  /** Alternate / former name. */
  alternate_name?: string | null;
  /** Free-text description. */
  description?: string | null;
  /** schema.org category of the place. */
  place_type?: PlaceType | null;
  /** Postal address. */
  address?: PostalAddress | null;
  /** Geographic coordinates. */
  geo?: GeoCoordinates | null;
  /** Primary telephone number. */
  telephone?: string | null;
  /** Fax number. */
  fax_number?: string | null;
  /** Canonical URL for the place. */
  url?: string | null;
  /** GS1 Global Location Number. */
  global_location_number?: string | null;
  /** Organization-internal branch code. */
  branch_code?: string | null;
  /** Id of the place that contains this one (hierarchy). */
  contained_in_place?: string | null;
  /** Free-text search keywords. */
  keywords?: string[];
  /** Additional external identifiers. */
  identifiers?: PlaceIdentifier[];
  /** Amenities offered. */
  amenity_features?: AmenityFeature[];
  /** Weekly opening-hours windows. */
  opening_hours?: OpeningHoursSpecification[];
  /** Whether entry is free of charge. */
  is_accessible_for_free?: boolean | null;
  /** Whether the public may access the place. */
  public_access?: boolean | null;
  /** Whether smoking is permitted. */
  smoking_allowed?: boolean | null;
  /** Maximum occupancy / attendee capacity. */
  maximum_attendee_capacity?: number | null;
  /** Soft-delete flag (server-managed). */
  is_deleted?: boolean;
  /** Soft-delete timestamp (server-managed). */
  deleted_at?: string | null;
  /** Creation timestamp (server-managed). */
  created_at?: string;
  /** Last-update timestamp (server-managed). */
  updated_at?: string;
}

// ─── Matching ────────────────────────────────────────────────────────

/** Confidence band for a match score (highest → lowest). */
export type MatchConfidence = "Certain" | "Probable" | "Possible" | "Unlikely";

/**
 * Per-component score breakdown behind an overall match score, so the UI
 * can explain *why* two places matched. Component scores are 0–1; the
 * boolean flags indicate phonetic and deterministic (short-circuit) hits.
 */
export interface MatchBreakdown {
  /** Name similarity (Jaro-Winkler), 0–1. */
  name_score?: number | null;
  /** Geo proximity (Haversine + decay), 0–1. */
  geo_score?: number | null;
  /** Address similarity, 0–1. */
  address_score?: number | null;
  /** Place-type agreement, 0–1. */
  place_type_score?: number | null;
  /** Identifier overlap, 0–1. */
  identifier_score?: number | null;
  /** True if a phonetic (Soundex) bonus applied. */
  phonetic_match?: boolean;
  /** True if a deterministic rule short-circuited the score to certainty. */
  deterministic_match?: boolean;
}

/** One candidate from a match / duplicate-check call. */
export interface MatchResult {
  /** The candidate place. */
  place: Place;
  /** Overall match score, 0–1. */
  score: number;
  /** Confidence band derived from {@link MatchResult.score}. */
  confidence: MatchConfidence;
  /** Optional per-component breakdown. */
  breakdown?: MatchBreakdown;
}

/**
 * Request body for the `/match` endpoint: a partial place to score
 * against existing records, plus optional tuning of the search.
 */
export interface MatchRequest {
  /** Name to match on. */
  name?: string;
  /** Address to match on. */
  address?: PostalAddress;
  /** Geo point to match on. */
  geo?: GeoCoordinates;
  /** Place type to match on. */
  place_type?: PlaceType;
  /** Identifiers to match on (can trigger deterministic hits). */
  identifiers?: PlaceIdentifier[];
  /** Minimum score to return, 0–1. */
  threshold?: number;
  /** Cap on the number of candidates returned. */
  max_candidates?: number;
}

// ─── Merge ───────────────────────────────────────────────────────────

/** Lifecycle state of a merge record. */
export type MergeStatus = "Completed" | "Reversed";

/** Request body for merging a duplicate place into a surviving main place. */
export interface MergeRequest {
  /** Id of the surviving (main) place. */
  main_place_id: string;
  /** Id of the place to be merged away (soft-deleted). */
  duplicate_place_id: string;
  /** Optional human-supplied reason for the merge. */
  merge_reason?: string | null;
  /** Optional id of the operator performing the merge. */
  merged_by?: string | null;
}

/** Audit record of a completed (or reversed) merge. */
export interface MergeRecord {
  /** Merge-record id. */
  id: string;
  /** Surviving place id. */
  main_place_id: string;
  /** Merged-away place id. */
  duplicate_place_id: string;
  /** Current merge state. */
  status: MergeStatus;
  /** Operator who performed the merge. */
  merged_by?: string | null;
  /** Reason supplied for the merge. */
  merge_reason?: string | null;
  /** Match score at merge time, if known. */
  match_score?: number | null;
  /** Snapshot of data transferred from duplicate to main. */
  transferred_data?: unknown;
  /** Merge timestamp. */
  merged_at: string;
}

/** Response from a successful merge: the record plus the updated main place. */
export interface MergeResponse {
  /** The created merge record. */
  merge_record: MergeRecord;
  /** The surviving place after the merge. */
  main_place: Place;
}

// ─── Batch dedup ─────────────────────────────────────────────────────

/** Tuning parameters for a batch deduplication scan. */
export interface BatchDeduplicationRequest {
  /** Minimum score to treat a pair as a duplicate. */
  threshold?: number;
  /** Cap on candidates considered per record. */
  max_candidates?: number;
  /** Score at/above which pairs are auto-merged rather than queued. */
  auto_merge_threshold?: number;
}

/** Summary returned by a batch deduplication scan. */
export interface BatchDeduplicationResponse {
  /** Number of places examined. */
  places_scanned: number;
  /** Number of duplicate pairs detected. */
  duplicates_found: number;
  /** Number auto-merged (score ≥ auto-merge threshold). */
  auto_merged: number;
  /** Number queued for manual review. */
  queued_for_review: number;
  /** The review-queue items created by the scan. */
  review_items: ReviewQueueItem[];
}

/** Disposition of a review-queue item. */
export type ReviewStatus =
  | "pending"
  | "confirmed"
  | "rejected"
  | "automerged";

/** A pair of possibly-duplicate places awaiting a merge decision. */
export interface ReviewQueueItem {
  /** Review-item id. */
  id: string;
  /** First place in the candidate pair. */
  place_id_a: string;
  /** Second place in the candidate pair. */
  place_id_b: string;
  /** Match score for the pair, 0–1. */
  match_score: number;
  /** Confidence band label for the pair. */
  match_quality: string;
  /** How the pair was detected (e.g. `batch_deduplication`). */
  detection_method: string;
  /** Current review disposition. */
  status: ReviewStatus;
  /** Operator who decided the item, if decided. */
  reviewed_by?: string | null;
  /** When the item was queued. */
  created_at: string;
  /** When the item was reviewed, if it has been. */
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

/** A single HIPAA-style audit-log entry for a CRUD/merge action. */
export interface AuditEntry {
  /** Audit-entry id. */
  id: string;
  /** Entity kind acted upon (e.g. `"place"`). */
  entity_type: string;
  /** Id of the affected entity. */
  entity_id: string;
  /** Action performed (e.g. `"create"`, `"update"`, `"merge"`). */
  action: string;
  /** Acting user id, if known. */
  user_id?: string | null;
  /** Acting user IP address, if captured. */
  user_ip_address?: string | null;
  /** Acting user agent string, if captured. */
  user_agent?: string | null;
  /** Entity state before the action. */
  old_values?: unknown;
  /** Entity state after the action. */
  new_values?: unknown;
  /** When the action occurred. */
  created_at: string;
}
