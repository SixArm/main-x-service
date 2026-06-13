// Types mirroring the Care Pathway Service payload, which is the
// `care_pathway_matcher::CarePathway` shape itself.
// Source of truth: care-pathway-matcher-rust-crate/src/care_pathway.rs.

/// Identifier schemes. Rust serializes unit variants as the bare string;
/// `Custom` as `{ "Custom": "label" }`.
export type IdentifierScheme =
    | "Doi"
    | "Wikidata"
    | "GuidelineId"
    | "Uri"
    | "Uuid"
    | "PathwayCode"
    | "LocalId"
    | { Custom: string };

export const ALL_SCHEMES: IdentifierScheme[] = [
    "Doi",
    "Wikidata",
    "GuidelineId",
    "Uri",
    "Uuid",
    "PathwayCode",
    "LocalId",
];

/// Clinical coding systems for a condition code.
export type CodeSystem = "Icd10" | "Icd11" | "Snomed" | { Custom: string };

export const ALL_CODE_SYSTEMS: CodeSystem[] = ["Icd10", "Icd11", "Snomed"];

/// Care settings (unit variants only here).
export type CareSetting =
    | "Inpatient"
    | "Outpatient"
    | "PrimaryCare"
    | "EmergencyDepartment"
    | "Community"
    | "HomeCare"
    | "Rehabilitation"
    | "MentalHealth"
    | "Palliative"
    | { Custom: string };

export const ALL_CARE_SETTINGS: CareSetting[] = [
    "Inpatient",
    "Outpatient",
    "PrimaryCare",
    "EmergencyDepartment",
    "Community",
    "HomeCare",
    "Rehabilitation",
    "MentalHealth",
    "Palliative",
];

export interface ConditionCode {
    system: CodeSystem;
    code: string;
}

export interface PathwayIdentifier {
    scheme: IdentifierScheme;
    value: string;
}

export interface CarePathway {
    name: string;
    alternate_names?: string[];
    pathway_code?: string | null;
    provider_id?: string | null;
    provider_name?: string | null;
    care_setting?: CareSetting | null;
    condition_codes?: ConditionCode[];
    interventions?: string[];
    keywords?: string[];
    identifiers?: PathwayIdentifier[];
    same_as?: string[];
    in_language?: string[];
}

/// `{pid, name}` returned by create / list.
export interface PathwayRef {
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

/// Result of merging a duplicate into a survivor (`POST /merge`).
/// `main` is the survivor's refreshed `CarePathway`.
export interface MergeResult {
    main_pid: string;
    duplicate_pid: string;
    main: CarePathway;
}
