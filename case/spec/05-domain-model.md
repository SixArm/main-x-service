## 5. Domain Model

The case entity has **one canonical domain model with one shape end to
end**: the matcher crate's `Case` type is the API DTO, the persisted
payload, and the matching input. Unlike the person entity (which
projects a service model through an adapter), there is deliberately
**no separate service model and no adapter to drift**.

### 5.1 Canonical `Case` (matcher crate)

Defined in
[`case-matcher-rust-crate/src/case.rs`](../case-matcher-rust-crate/src/case.rs);
normative reference: matcher
[spec §6](../case-matcher-rust-crate/spec/index.md).

| Field | Type | Notes |
|---|---|---|
| `title` | String | Required (service rejects blank) |
| `alternate_titles` | Vec\<String\> | Aliases, former titles |
| `case_number` | Option\<String\> | Agency-scoped local id, e.g. `BEN-2026-00417` |
| `agency_id` | Option\<String\> | Handling organisation id (scopes `case_number`) |
| `agency_name` | Option\<String\> | Handling organisation display name |
| `case_type` | Option\<CaseType\> | See enum below |
| `status` | Option\<CaseStatus\> | See enum below |
| `priority` | Option\<Priority\> | `Low`/`Normal`/`High`/`Urgent` — data only, not matched |
| `opened_date` | Option\<String\> | ISO 8601 date (`YYYY-MM-DD`) |
| `subjects` | Vec\<String\> | Opaque involved-party ids (e.g. person `pid`s) |
| `keywords` | Vec\<String\> | Descriptive / discovery terms about *what the case is* (subject matter) — see §5.6 |
| `tags` | Vec\<String\> | User-applied operational labels for grouping / filtering / triage / workflow (e.g. `"vip"`, `"review"`, `"archived-2026"`, `"fast-track"`) — see §5.6 |
| `identifiers` | Vec\<CaseIdentifier\> | `{ scheme: IdentifierScheme, value: String }` |
| `same_as` | Vec\<String\> | Canonical URLs (schema.org `sameAs`) |
| `in_language` | Vec\<String\> | ISO 639-1 codes — see [`agents/share/locales.md`](../../agents/share/locales.md) |
| `relationships` | Vec\<CaseRelationship\> | Typed case-to-case links — `{ relation: RelationKind, case_id: String }` (see below) |

Supporting enums (serialized snake_case at field level; **enum unit
variants are bare PascalCase**, e.g. `"Open"`; `Custom` is
`{"Custom":"label"}`):

- `CaseType`: `Benefit`, `Legal`, `SocialServices`, `Healthcare`,
  `Housing`, `Immigration`, `Licensing`, `Complaint`, `Appeal`,
  `Investigation`, `Tax`, `Employment`, `Custom(String)`.
- `CaseStatus`: `Open`, `InProgress`, `Pending`, `OnHold`, `Closed`,
  `Resolved`, `Rejected`, `Withdrawn`, `Custom(String)`.
- `Priority`: `Low`, `Normal`, `High`, `Urgent`.
- `IdentifierScheme`: deterministic — `Docket`, `ExternalCaseId`,
  `Uri`, `Uuid`; agency-scoped — `AgencyCaseNumber`, `LocalId`; plus
  `Custom(String)`.

**Relationships** — `relationships: Vec<CaseRelationship>`, each a
`CaseRelationship { relation: RelationKind, case_id: String }`
**referencing another `Case` in the registry** (by `case_id`). `relation`
is a `RelationKind` enum, initially:

- `RelatedTo` — **symmetric** (A `RelatedTo` B ⇔ B `RelatedTo` A): a
  loosely associated case with no hierarchy or ordering.
- `ParentCase` / `SubCase` — **inverses** (A `ParentCase` B ⇔ B `SubCase`
  A): case consolidation / splitting — a parent matter and its
  constituent sub-matters.
- `Supersedes` / `SupersededBy` — **inverses** (A `Supersedes` B ⇔ B
  `SupersededBy` A): a case that replaces an earlier one (refiled,
  reopened under a new number, …).
- `ConsolidatedWith` — **symmetric** (A `ConsolidatedWith` B ⇔ B
  `ConsolidatedWith` A): cases merged into a single proceeding as peers.

These relationships **generalise** any flat parent/child or
consolidation field: a `ParentCase` / `SubCase` link expresses
consolidation hierarchy, `ConsolidatedWith` expresses peer
consolidation, and `Supersedes` / `SupersededBy` expresses replacement.
The enum is extensible (e.g. `DuplicateOf` later) and is `#[non_exhaustive]`.

### 5.2 Subject sets

`subjects` are the people / organisations the case is about. They are
**opaque references** (e.g. person `pid`s) — never names or other
personal detail in-line — and are compared by Jaccard overlap. Two
cases about the same subject set are strong corroboration; subjects
carry the second-highest matching weight (§6.2).

### 5.3 Persistence model (JSONB)

The service stores the payload verbatim in one `cases` row:

| Column | Type | Purpose |
|---|---|---|
| `id` | serial PK | Internal row id |
| `pid` | UUID unique | Public id (route param) |
| `title` | string | Denormalised from the payload for cheap listing |
| `data` | JSONB | The full `Case` payload |
| `active` | boolean (default true) | Registry flag |
| `deleted_at` | timestamptz null | Soft delete |

`Model::to_case()` deserialises `data` back into the matcher type;
`Model::create()` / `update_data()` serialise it in. The `title`
column MUST equal `data.title` (the model layer writes both together).

Because the matcher's `Case` is the single end-to-end shape (no service
model, no adapter — §5), every matched field, including `relationships`
and `tags`, travels in the `data` JSONB verbatim and reaches the matcher
**unprojected**: there is no lossy-drop list, and neither
`relationships[]` nor `tags[]` is ever dropped. The matcher scores
`relationships` by typed-set Jaccard over the `(relation, case_id)`
pairs (matcher §13a), weighted `relationships_weight`, and scores `tags`
by case-insensitive set Jaccard over the folded tag sets (matcher §13b),
weighted `tags_weight`.

### 5.4 Front-end TypeScript types

The front-end mirrors the wire shape in
[`src/lib/api/types.ts`](../case-front-end-with-svelte/src/lib/api/types.ts)
(`Case`, `CaseType`, `CaseStatus`, `Priority`, `IdentifierScheme`,
`CaseRelationship`, `RelationKind`, `CaseRef`, `ScoredRef`). The matcher
type is upstream: if a field
changes in the matcher crate, the service inherits it automatically
(re-serialisation) and the front-end types MUST be fixed in the same
change cycle (§18).

### 5.6 Tags vs keywords

Any `Case` (the main concept / record) can carry `tags`. **Tags are
user-applied operational labels** that operators attach to a record for
grouping, filtering, triage, or workflow (e.g. `"vip"`, `"review"`,
`"archived-2026"`, `"fast-track"`). They are distinct from `keywords`:

- `keywords` — descriptive / discovery terms about *what the case is*
  (its subject matter); used for search and corroboration.
- `tags` — operator-curated labels about *how the record is handled*;
  used for grouping and workflow, not for describing the case content.

Each tag is a short, trimmed, non-empty string. The list is unordered,
de-duplicated **case-insensitively**, and defaults to empty.

`tags` is a **supporting match signal**: it is carried end to end and
the matcher scores it by case-insensitive set Jaccard over the folded
tag sets (matcher §13b), weighted `tags_weight` (default `0.05`). As an
operator-curated workflow label rather than an identifying field, shared
tags corroborate but never single-handedly establish a match.

Propagation follows the §5 contract: the canonical matcher `Case` is
upstream, so the service (re-serialisation through the `data` JSONB) and
the front-end TypeScript types (§5.4) inherit `tags` in the same change
cycle.

### 5.5 Shared invariants

All subprojects MUST uphold:

- `title` is non-empty; the stored `title` column equals `data.title`.
- The JSONB payload round-trips losslessly:
  `serde_json::from_value(to_value(c)) == c`.
- Agency-scoped identifiers (`case_number`, `AgencyCaseNumber`,
  `LocalId`) are never treated as globally unique — no cross-agency
  short-circuit, end to end.
- `subjects` carry only opaque references (ids/`pid`s), never personal
  detail; free-text fields (`keywords`, `tags`, `alternate_titles`) must
  not carry substantive case content or personal detail (§12).
- `tags` are short, trimmed, non-empty strings; the list is unordered,
  de-duplicated case-insensitively, and defaults to empty. `tags` is a
  supporting match signal, scored by case-insensitive set Jaccard in the
  matcher (weighted `tags_weight`, matcher §13b).
- A `CaseRelationship` references an **existing** `Case` in the registry;
  **no case relates to itself** (`case_id` is never the case's own id).
  The directional kinds `ParentCase` / `SubCase` and `Supersedes` /
  `SupersededBy` must stay **acyclic** (no case is its own ancestor or
  predecessor, directly or transitively) and, where both directions are
  stored, mutually consistent (A `ParentCase` B ⇔ B `SubCase` A;
  A `Supersedes` B ⇔ B `SupersededBy` A). The symmetric kinds `RelatedTo`
  and `ConsolidatedWith` are symmetric (A `RelatedTo` B ⇔ B `RelatedTo` A;
  A `ConsolidatedWith` B ⇔ B `ConsolidatedWith` A).
- Match scores are in `[0.00, 1.00]` and always travel with a
  per-component breakdown and `Confidence` band.
- Soft delete (`deleted_at`) is the only delete: the service never
  row-deletes, and the front-end never offers hard delete.
- Every create / update / delete / merge writes an audit row and
  publishes a `CaseEvent` (§9, §10).
