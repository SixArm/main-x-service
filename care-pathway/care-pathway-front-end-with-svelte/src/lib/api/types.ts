// Types mirroring the Care Pathway Service payload, which is the
// `care_pathway_matcher::CarePathway` shape itself.
// Source of truth: care-pathway-matcher-rust-crate/src/care_pathway.rs.

/**
 * Identifier scheme for a {@link PathwayIdentifier}. Mirrors the Rust enum:
 * unit variants serialize as the bare string (e.g. `"Doi"`), while a custom
 * scheme serializes as the tagged object `{ "Custom": "label" }`.
 */
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

/**
 * All unit-variant identifier schemes, in display order — used to populate
 * the scheme `<select>` in the form. Excludes the `Custom` object variant.
 */
export const ALL_SCHEMES: IdentifierScheme[] = [
    "Doi",
    "Wikidata",
    "GuidelineId",
    "Uri",
    "Uuid",
    "PathwayCode",
    "LocalId",
];

/**
 * Clinical coding system for a {@link ConditionCode}. Unit variants
 * (`Icd10`/`Icd11`/`Snomed`) serialize as bare strings; a custom system as
 * `{ "Custom": "label" }`.
 */
/// Clinical coding systems for a condition code.
export type CodeSystem = "Icd10" | "Icd11" | "Snomed" | { Custom: string };

/** All unit-variant code systems, for the condition-code `<select>`. */
export const ALL_CODE_SYSTEMS: CodeSystem[] = ["Icd10", "Icd11", "Snomed"];

/**
 * Care setting in which the pathway is delivered. Unit variants serialize
 * as bare strings; a custom setting as `{ "Custom": "label" }`.
 */
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

/** All unit-variant care settings, for the care-setting `<select>`. */
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

/** A coded target condition: a coding system plus its code value. */
export interface ConditionCode {
    /** The coding system the `code` belongs to. */
    system: CodeSystem;
    /** The code value within `system` (e.g. `"I63"`). */
    code: string;
}

/** An external identifier for a pathway: a scheme plus its value. */
export interface PathwayIdentifier {
    /** The identifier scheme (DOI, GuidelineId, …). */
    scheme: IdentifierScheme;
    /** The identifier value within `scheme`. */
    value: string;
}

/**
 * The full care-pathway payload — the request/response body of the
 * service, which is the `care_pathway_matcher::CarePathway` shape itself.
 * Only `name` is required; the rest are optional and may be `null`.
 */
export interface CarePathway {
    /** Primary name of the pathway (required). */
    name: string;
    /** Additional / former names. */
    alternate_names?: string[];
    /** Provider-scoped pathway code (e.g. `"STROKE-01"`). */
    pathway_code?: string | null;
    /** Identifier of the owning provider. */
    provider_id?: string | null;
    /** Human-readable provider name. */
    provider_name?: string | null;
    /** Care setting in which the pathway is delivered. */
    care_setting?: CareSetting | null;
    /** Target condition codes (ICD/SNOMED/…). */
    condition_codes?: ConditionCode[];
    /** Interventions the pathway prescribes. */
    interventions?: string[];
    /** Free-text keywords for search/matching. */
    keywords?: string[];
    /** External identifiers (DOI, GuidelineId, …). */
    identifiers?: PathwayIdentifier[];
    /** `sameAs` reference URLs to equivalent resources. */
    same_as?: string[];
    /** BCP-47 language tags the pathway is available in. */
    in_language?: string[];
}

/** Lightweight `{pid, name}` reference returned by create / list / search. */
/// `{pid, name}` returned by create / list.
export interface PathwayRef {
    /** Persistent identifier of the pathway. */
    pid: string;
    /** The pathway's name at the time of the response. */
    name: string;
}

/** A scored candidate duplicate returned by `POST /check-duplicates`. */
/// A scored duplicate from /check-duplicates.
export interface ScoredRef {
    /** Persistent identifier of the candidate. */
    pid: string;
    /** The candidate's name. */
    name: string;
    /** Match score in `[0, 1]`. */
    score: number;
    /** Confidence band label (e.g. `"Certain"`, `"Probable"`). */
    confidence: string;
    /** Whether the score clears the service's match threshold. */
    is_match: boolean;
}

/**
 * Result of merging a duplicate into a survivor (`POST /merge`). `main` is
 * the survivor's refreshed `CarePathway` (it may have absorbed data from
 * the duplicate).
 */
/// Result of merging a duplicate into a survivor (`POST /merge`).
/// `main` is the survivor's refreshed `CarePathway`.
export interface MergeResult {
    /** Persistent identifier of the surviving (main) record. */
    main_pid: string;
    /** Persistent identifier of the merged-away duplicate. */
    duplicate_pid: string;
    /** The survivor's refreshed record after the merge. */
    main: CarePathway;
}

/**
 * One audit-log row from `GET /api/care-pathways/{pid}/audit`. Mirrors the
 * service's `audit_logs` SeaORM model serialized field names.
 */
/// One audit-log row from `GET /api/care-pathways/{pid}/audit`.
/// Mirrors the service's `audit_logs` SeaORM model (serialized field
/// names): `action` (created/updated/deleted/merged), nullable `actor`
/// (the caller's user pid when a verified token was presented, else
/// `null`), an optional JSON `snapshot`, and the `created_at` timestamp.
export interface AuditEntry {
    /** The audited operation: `created` / `updated` / `deleted` / `merged`. */
    action: string;
    /** Caller's user pid when a verified token was presented, else `null`. */
    actor: string | null;
    /** Optional JSON snapshot of the record state for the audited action. */
    snapshot?: unknown;
    /** ISO-8601 timestamp of when the action was recorded. */
    created_at?: string;
}

/**
 * One event from the service's in-memory CRUD/merge stream, returned by
 * `GET /api/care-pathways/events/recent`. Mirrors the service's
 * `streaming::PathwayEvent` (its `EventKind` serializes lowercase).
 */
/// One event from the service's in-memory CRUD/merge stream, returned by
/// `GET /api/care-pathways/events/recent`. Mirrors the service's
/// `streaming::PathwayEvent` (its `EventKind` serializes lowercase):
/// `kind` (created/updated/deleted/merged), the pathway's `pid` and
/// `name` at the time of the event, and a per-process monotonic `seq`.
export interface PathwayEvent {
    /** The kind of CRUD/merge operation that produced the event. */
    kind: "created" | "updated" | "deleted" | "merged";
    /** Persistent identifier of the affected pathway. */
    pid: string;
    /** The pathway's name at the time of the event. */
    name: string;
    /** Per-process monotonic sequence number; used to order newest-first. */
    seq: number;
}
