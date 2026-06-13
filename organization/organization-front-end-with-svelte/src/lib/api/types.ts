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

export const ALL_SCHEMES: IdentifierScheme[] = [
    ...DETERMINISTIC_SCHEMES,
    "TaxId",
    "Naics",
    "IsicV4",
    "Sic",
];

export interface OrgIdentifier {
    scheme: IdentifierScheme;
    value: string;
}

export interface PostalAddress {
    street_address?: string | null;
    locality?: string | null;
    region?: string | null;
    postal_code?: string | null;
    country?: string | null;
}

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
