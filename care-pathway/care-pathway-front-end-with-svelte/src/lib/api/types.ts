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

// ---------------------------------------------------------------------------
// Pathway instances (a person enrolled on a pathway template).
// Source of truth: care-pathway-service `models/_entities/pathway_instances`
// + `controllers/instances.rs`.
// ---------------------------------------------------------------------------

/** Enrolment lifecycle status. The service's status machine gates moves. */
export type InstanceStatus =
  | "active"
  | "on_hold"
  | "completed"
  | "discontinued";

/** The four lifecycle columns rendered on the Kanban board, in order. */
export const INSTANCE_STATUSES: InstanceStatus[] = [
  "active",
  "on_hold",
  "completed",
  "discontinued",
];

/** Clinical urgency of an enrolment. */
export type Urgency = "routine" | "urgent" | "emergency";

/**
 * One pathway instance: a `person:<uuid>` subject enrolled on a pathway
 * template, with a status/urgency lifecycle and review cadence.
 */
export interface PathwayInstance {
  /** Persistent identifier of the instance. */
  pid: string;
  /** The template pathway this instance was enrolled on. */
  pathway_pid: string;
  /** The enrolled subject, a `person:<uuid>` URN. */
  subject_ref: string;
  /** Enrolment lifecycle status. */
  status: InstanceStatus;
  /** Clinical urgency. */
  urgency: Urgency;
  /** ISO date the subject was enrolled. */
  enrolled_on: string;
  /** ISO date of the next scheduled review (cleared on terminal status). */
  next_review_on?: string | null;
  /** ISO date the instance was closed, if terminal. */
  closed_on?: string | null;
  /** Free-text closure reason, if closed. */
  closure_reason?: string | null;
  /** Recorded outcome, if any. */
  outcome?: string | null;
}

/** `GET /api/instances/{pid}` — an instance plus its related collections. */
export interface InstanceDetail {
  /** The instance record. */
  instance: PathwayInstance;
  /** Step-completion log rows. */
  steps: unknown[];
  /** Care-team member rows. */
  team: unknown[];
  /** Clinical-event rows. */
  events: unknown[];
  /** Recorded-measure rows. */
  measures: unknown[];
}

// ---------------------------------------------------------------------------
// Registry insights (five read-only lenses). The service returns dynamic
// JSON objects, each carrying a `note` string that the UI shows verbatim.
// Typed loosely — only the fields the UI renders are named.
// ---------------------------------------------------------------------------

/** `GET …/insights/directory`. */
export interface DirectoryInsight {
  as_of: string;
  note: string;
  total: number;
  by_setting: Record<string, Array<{ pid: string; name: string; specialty?: string | null }>>;
  by_specialty: Record<string, number>;
}

/** `GET …/insights/coverage`. */
export interface CoverageInsight {
  as_of: string;
  note: string;
  conditions: Array<{ condition: string; settings: string[] }>;
  gaps: Array<{ rule: string; detail: string; condition: string }>;
}

/** `GET …/insights/variants`. */
export interface VariantsInsight {
  as_of: string;
  note: string;
  variants: Array<{
    condition: string;
    providers: number;
    by_provider: Record<string, Array<{ pid: string; name: string; jurisdiction?: string | null }>>;
  }>;
}

/** `GET …/insights/providers`. */
export interface ProvidersInsight {
  as_of: string;
  note: string;
  providers: Array<{
    provider: string;
    pathways: number;
    by_setting: Record<string, number>;
  }>;
}

/** `GET …/insights/languages`. */
export interface LanguagesInsight {
  as_of: string;
  note: string;
  by_language: Record<string, number>;
  single_language_conditions: Array<{ condition: string; language?: string | null }>;
}
