## 5. Domain Model

The care-pathway entity has **one canonical domain model with one
shape end to end**: the matcher crate's `CarePathway` type is the
API DTO, the persisted payload, and the matching input. Unlike the
person entity (which projects a service model through an adapter),
there is deliberately **no separate service model and no adapter to
drift**.

### 5.1 Canonical `CarePathway` (matcher crate)

Defined in
[`care-pathway-matcher-rust-crate/src/care_pathway.rs`](../care-pathway-matcher-rust-crate/src/care_pathway.rs);
normative reference: matcher
[spec §6](../care-pathway-matcher-rust-crate/spec/index.md).

| Field | Type | Notes |
|---|---|---|
| `name` | String | Required (service rejects blank) |
| `alternate_names` | Vec\<String\> | Aliases, former titles |
| `pathway_code` | Option\<String\> | Provider-scoped code, e.g. `STROKE-01` |
| `provider_id` | Option\<String\> | Issuing organisation id (scopes `pathway_code`) |
| `provider_name` | Option\<String\> | Issuing organisation display name |
| `care_setting` | Option\<CareSetting\> | See enum below |
| `condition_codes` | Vec\<ConditionCode\> | `{ system: CodeSystem, code: String }` |
| `interventions` | Vec\<String\> | Key treatments / actions |
| `keywords` | Vec\<String\> | Free-text tags |
| `identifiers` | Vec\<PathwayIdentifier\> | `{ scheme: IdentifierScheme, value: String }` |
| `same_as` | Vec\<String\> | Canonical URLs (schema.org `sameAs`) |
| `in_language` | Option\<String\> | ISO 639-1 code — see [`agents/share/locales.md`](../../agents/share/locales.md) |

Supporting enums:

- `CodeSystem`: `Icd10`, `Icd11`, `Snomed`, `Custom(String)`.
- `CareSetting`: `Inpatient`, `Outpatient`, `PrimaryCare`,
  `EmergencyDepartment`, `Community`, `HomeCare`, `Rehabilitation`,
  `MentalHealth`, `Palliative`, `Custom(String)`.
- `IdentifierScheme`: deterministic — `Doi`, `Wikidata`,
  `GuidelineId`, `Uri`, `Uuid`; provider-scoped — `PathwayCode`,
  `LocalId`; plus `Custom(String)`.

### 5.2 Condition-code sets

Condition codes are the defining attribute of a pathway. They render
to lower-cased `"system:code"` tokens (e.g. `icd10:i63`) and are
compared by Jaccard overlap. Codes are **system-qualified**: an
ICD-10 `I63` never matches a SNOMED code with the same digits.

### 5.3 Persistence model (JSONB)

The service stores the payload verbatim in one `care_pathways` row:

| Column | Type | Purpose |
|---|---|---|
| `id` | serial PK | Internal row id |
| `pid` | UUID unique | Public id (route param) |
| `name` | string | Denormalised from the payload for cheap listing |
| `data` | JSONB | The full `CarePathway` payload |
| `active` | boolean (default true) | Registry flag |
| `deleted_at` | timestamptz null | Soft delete |

`Model::to_pathway()` deserialises `data` back into the matcher type;
`Model::create()` / `update_data()` serialise it in. The `name`
column MUST equal `data.name` (the model layer writes both together).

### 5.4 Front-end TypeScript types

The front-end mirrors the wire shape in
[`src/lib/api/types.ts`](../care-pathway-front-end-with-svelte/src/lib/api/types.ts)
(`CarePathway`, `ConditionCode`, `CareSetting`, `IdentifierScheme`,
`PathwayRef`, `ScoredRef`). The matcher type is upstream: if a field
changes in the matcher crate, the service inherits it automatically
(re-serialisation) and the front-end types MUST be fixed in the same
change cycle.

### 5.5 Shared invariants

All subprojects MUST uphold:

- `name` is non-empty; the stored `name` column equals `data.name`.
- The JSONB payload round-trips losslessly:
  `serde_json::from_value(to_value(p)) == p`.
- Provider-scoped codes (`pathway_code`, `PathwayCode`, `LocalId`)
  are never treated as globally unique — no cross-provider
  short-circuit, end to end.
- Match scores are in `[0.00, 1.00]` and always travel with a
  per-component breakdown and `Confidence` band.
- Soft delete (`deleted_at`) is the only delete: the service never
  row-deletes, and the front-end never offers hard delete.
- No patient-level data, ever, anywhere in the payload (§12).
