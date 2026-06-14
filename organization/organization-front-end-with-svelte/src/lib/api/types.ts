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
