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
| `keywords` | Vec\<String\> | Descriptive / discovery terms (what the record *is*) |
| `tags` | Vec\<String\> | Operator-applied labels for grouping / workflow — see below |
| `identifiers` | Vec\<PathwayIdentifier\> | `{ scheme: IdentifierScheme, value: String }` |
| `same_as` | Vec\<String\> | Canonical URLs (schema.org `sameAs`) |
| `in_language` | Option\<String\> | ISO 639-1 code — see [`agents/share/locales.md`](../../agents/share/locales.md) |
| `relationships` | Vec\<CarePathwayRelationship\> | Typed pathway-to-pathway links — `{ relation: RelationKind, pathway_id: String }` |

**Relationships** — typed care-pathway-to-care-pathway links:
`relationships: Vec<CarePathwayRelationship>`, each `{ relation,
pathway_id }` **referencing another `CarePathway` in the registry**.
`relation` is a `RelationKind` enum, initially:

- **`PrecededBy`** / **`FollowedBy`** (**inverses** — A `PrecededBy` B
  means B comes before A in the care sequence; B `FollowedBy` A means A
  comes after B), capturing pathway sequencing / care-step ordering.
- **`SimilarTo`** (**symmetric** — A `SimilarTo` B ⇔ B `SimilarTo` A),
  a clinically comparable pathway.
- **`Supersedes`** / **`SupersededBy`** (**inverses** — A `Supersedes` B
  means A is the newer guideline replacing B; B `SupersededBy` A means B
  was replaced by A), capturing guideline versioning.

The enum is extensible to further sequencing / versioning kinds. These
are a **supporting** matching signal (a typed-set Jaccard over the
`(relation, pathway_id)` pairs), never an identifying field on their own.

Supporting enums:

- `CodeSystem`: `Icd10`, `Icd11`, `Snomed`, `Custom(String)`.
- `CareSetting`: `Inpatient`, `Outpatient`, `PrimaryCare`,
  `EmergencyDepartment`, `Community`, `HomeCare`, `Rehabilitation`,
  `MentalHealth`, `Palliative`, `Custom(String)`.
- `IdentifierScheme`: deterministic — `Doi`, `Wikidata`,
  `GuidelineId`, `Uri`, `Uuid`; provider-scoped — `PathwayCode`,
  `LocalId`; plus `Custom(String)`.
- `RelationKind`: `PrecededBy`, `FollowedBy`, `SimilarTo`,
  `Supersedes`, `SupersededBy`; plus `Custom(String)`. `PrecededBy` /
  `FollowedBy` and `Supersedes` / `SupersededBy` are inverse pairs;
  `SimilarTo` is symmetric.

**Tags** — `tags: Vec<String>` is a list of short free-text labels that
operators attach to a record for grouping, filtering, triage, or
workflow (e.g. `vip`, `review`, `archived-2026`, `fast-track`). **Any
`CarePathway` can carry tags.** Each tag is a short, trimmed, non-empty
string; the list is unordered, de-duplicated **case-insensitively**, and
defaults to empty.

Tags are distinct from `keywords`: **keywords** are descriptive /
discovery terms about *what the record is* (clinical vocabulary,
synonyms), whereas **tags** are **user-applied operational labels** for
grouping and workflow. The two fields coexist — neither replaces the
other.

Tags **are** a supporting match signal: they round-trip through the JSONB
payload (§5.3) and reach the matcher unchanged, and the matcher scores
them as a plain **set Jaccard** over the case-insensitively normalised
tag sets (`tags_score = |A ∩ B| / |A ∪ B|`, matcher §13.2), weighted
`tags_weight`. It is a **supporting** signal — like `keywords` and
`relationships`, never an identifying field on its own; `None` (does not
participate) when either side has an empty tag set. As with every
canonical-model field, the matcher DTO is upstream (§5.1): the service
inherits `tags` automatically via re-serialisation and the front-end
TypeScript types (§5.4) MUST gain it in the same change cycle.

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

Because the matcher's `CarePathway` **is** the persisted payload and the
matching input (no adapter, §5), every scored field — including
`relationships[]` — round-trips verbatim through the `data` JSONB column
and reaches the matcher unchanged. There is **no lossy-drop list** to
keep `relationships` out of: the only fields that never enter the matcher
are the registry-plumbing columns (`id`, `pid`, `active`, `deleted_at`),
which live outside the JSONB payload. `relationships[]` is therefore
routed 1:1 into the matcher and scored as a typed-set Jaccard over the
`(relation, pathway_id)` pairs (matcher §13.1), weighted
`relationships_weight`. Likewise `tags` is **not** in any lossy-drop list
(there is none): it routes 1:1 into the matcher `tags` and is scored as a
plain set Jaccard over the case-insensitively normalised tag sets
(matcher §13.2), weighted `tags_weight`.

### 5.4 Front-end TypeScript types

The front-end mirrors the wire shape in
[`src/lib/api/types.ts`](../care-pathway-front-end-with-svelte/src/lib/api/types.ts)
(`CarePathway`, `ConditionCode`, `CareSetting`, `IdentifierScheme`,
`CarePathwayRelationship`, `RelationKind`, `PathwayRef`, `ScoredRef`).
The matcher type is upstream: if a field
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
- A `CarePathwayRelationship` references an **existing** `CarePathway`;
  **no pathway relates to itself**. `PrecededBy` / `FollowedBy` and
  `Supersedes` / `SupersededBy` stay **acyclic** (no pathway precedes or
  supersedes itself, directly or transitively) and inverse-consistent
  (A `PrecededBy` B ⇔ B `FollowedBy` A; A `Supersedes` B ⇔ B
  `SupersededBy` A); `SimilarTo` is **symmetric** (A `SimilarTo` B ⇔ B
  `SimilarTo` A).
- Each `tags` entry is short, trimmed, and non-empty; the list is
  de-duplicated case-insensitively and defaults to empty.
- Match scores are in `[0.00, 1.00]` and always travel with a
  per-component breakdown and `Confidence` band.
- Soft delete (`deleted_at`) is the only delete: the service never
  row-deletes, and the front-end never offers hard delete.
- No patient-level data, ever, anywhere in the payload (§12).
