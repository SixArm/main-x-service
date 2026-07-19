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
