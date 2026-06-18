# Domain Model Reference — Care Pathway Entity

One model, one shape, end to end: the matcher crate's `CarePathway`
is the API DTO, the persisted JSONB payload, and the matching input.
Normative definitions: entity spec
[§5](../spec/05-domain-model.md) and matcher
[spec §6](../care-pathway-matcher-rust-crate/spec/index.md).

## `CarePathway`

**File:** [`care-pathway-matcher-rust-crate/src/care_pathway.rs`](../care-pathway-matcher-rust-crate/src/care_pathway.rs)

| Field | Type | Description |
|---|---|---|
| name | String | Pathway title (required; service rejects blank) |
| alternate_names | Vec\<String\> | Aliases, former titles |
| pathway_code | Option\<String\> | Provider-scoped code (e.g. `STROKE-01`) |
| provider_id | Option\<String\> | Issuing organisation id — scopes `pathway_code` |
| provider_name | Option\<String\> | Issuing organisation display name |
| care_setting | Option\<CareSetting\> | Where the pathway applies |
| condition_codes | Vec\<ConditionCode\> | Target clinical condition codes |
| interventions | Vec\<String\> | Key treatments / actions |
| keywords | Vec\<String\> | Free-text tags |
| identifiers | Vec\<PathwayIdentifier\> | Typed document identifiers |
| same_as | Vec\<String\> | Canonical URLs (schema.org `sameAs`) |
| in_language | Option\<String\> | ISO 639-1 language code |

## Supporting types

| Type | Variants / shape |
|---|---|
| `ConditionCode` | `{ system: CodeSystem, code: String }` |
| `CodeSystem` | `Icd10`, `Icd11`, `Snomed`, `Custom(String)` |
| `CareSetting` | `Inpatient`, `Outpatient`, `PrimaryCare`, `EmergencyDepartment`, `Community`, `HomeCare`, `Rehabilitation`, `MentalHealth`, `Palliative`, `Custom(String)` |
| `PathwayIdentifier` | `{ scheme: IdentifierScheme, value: String }` |
| `IdentifierScheme` | Deterministic: `Doi`, `Wikidata`, `GuidelineId`, `Uri`, `Uuid` · Provider-scoped: `PathwayCode`, `LocalId` · `Custom(String)` |

Deterministic schemes pin a match to 1.0 on a shared value;
provider-scoped schemes never do (see [matching.md](matching.md)).

## Service persistence model

**Files:**
[`src/models/care_pathways.rs`](../care-pathway-service-with-loco/src/models/care_pathways.rs),
[`migration/src/m20220101_000001_care_pathways.rs`](../care-pathway-service-with-loco/migration/src/m20220101_000001_care_pathways.rs)

One `care_pathways` table: `id` (PK), `pid` (public UUID), `name`
(denormalised from `data.name`), `data` (JSONB `CarePathway`),
`active`, `deleted_at` (soft delete). Model helpers: `create`,
`find_by_pid`, `list(limit)`, `to_pathway()` (deserialise),
`update_data`, `soft_delete`.

## Wire DTOs (service controller)

**File:** [`src/controllers/care_pathways.rs`](../care-pathway-service-with-loco/src/controllers/care_pathways.rs)

| Type | Shape | Used by |
|---|---|---|
| `PathwayRef` | `{ pid, name }` | create / update / list responses |
| `MatchRequest` | `{ query: CarePathway, candidates: [CarePathway] }` | `POST …/match` |
| `ScoredRef` | `{ pid, name, score, confidence, is_match }` | `POST …/check-duplicates` |

## Front-end TypeScript mirror

**File:** [`src/lib/api/types.ts`](../care-pathway-front-end-with-svelte/src/lib/api/types.ts)
— `CarePathway`, `ConditionCode`, `CareSetting`, `IdentifierScheme`,
`PathwayRef`, `ScoredRef`. Hand-mirrored; MUST be updated in the same
change cycle as any matcher-type change (entity spec §18).
