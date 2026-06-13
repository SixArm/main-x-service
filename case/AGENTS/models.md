# Domain Model Reference — Case Entity

One model, one shape, end to end: the matcher crate's `Case` is the API
DTO, the persisted JSONB payload, and the matching input. Normative
definitions: entity spec [§5](../spec/05-domain-model.md) and matcher
[spec §6](../case-matcher-rust-crate/spec/index.md).

## `Case`

**File:** [`case-matcher-rust-crate/src/case.rs`](../case-matcher-rust-crate/src/case.rs)

| Field | Type | Description |
|---|---|---|
| title | String | Case title (required; service rejects blank) |
| alternate_titles | Vec\<String\> | Aliases, former titles |
| case_number | Option\<String\> | Agency-scoped local id (e.g. `BEN-2026-00417`) |
| agency_id | Option\<String\> | Handling organisation id — scopes `case_number` |
| agency_name | Option\<String\> | Handling organisation display name |
| case_type | Option\<CaseType\> | Kind of case |
| status | Option\<CaseStatus\> | Lifecycle status |
| priority | Option\<Priority\> | `Low`/`Normal`/`High`/`Urgent` — data only, not matched |
| opened_date | Option\<String\> | ISO 8601 date (`YYYY-MM-DD`) |
| subjects | Vec\<String\> | Opaque involved-party ids (e.g. person `pid`s) |
| keywords | Vec\<String\> | Free-text tags |
| identifiers | Vec\<CaseIdentifier\> | Typed identifiers |
| same_as | Vec\<String\> | Canonical URLs (schema.org `sameAs`) |
| in_language | Vec\<String\> | ISO 639-1 language codes |

## Supporting types

| Type | Variants / shape |
|---|---|
| `CaseType` | `Benefit`, `Legal`, `SocialServices`, `Healthcare`, `Housing`, `Immigration`, `Licensing`, `Complaint`, `Appeal`, `Investigation`, `Tax`, `Employment`, `Custom(String)` |
| `CaseStatus` | `Open`, `InProgress`, `Pending`, `OnHold`, `Closed`, `Resolved`, `Rejected`, `Withdrawn`, `Custom(String)` |
| `Priority` | `Low`, `Normal`, `High`, `Urgent` |
| `CaseIdentifier` | `{ scheme: IdentifierScheme, value: String }` |
| `IdentifierScheme` | Deterministic: `Docket`, `ExternalCaseId`, `Uri`, `Uuid` · Agency-scoped: `AgencyCaseNumber`, `LocalId` · `Custom(String)` |

Serialization: fields are snake_case; enum unit variants serialize as
bare PascalCase (e.g. `"Open"`); `Custom` serializes as
`{"Custom":"label"}`.

Deterministic schemes pin a match to 1.0 on a shared value;
agency-scoped schemes never do (see [matching.md](matching.md)).

**Privacy note:** case records are personal data (entity spec §12).
`subjects` carry only opaque references; never put personal detail or
substantive case content in free-text fields.

## Service persistence model

**Files:**
[`src/models/cases.rs`](../case-service-rust-crate/src/models/cases.rs),
[`migration/src/m20220101_000001_cases.rs`](../case-service-rust-crate/migration/src/m20220101_000001_cases.rs)

One `cases` table: `id` (PK), `pid` (public UUID), `title`
(denormalised from `data.title`), `data` (JSONB `Case`), `active`,
`deleted_at` (soft delete). Model helpers: `create`, `find_by_pid`,
`list(limit)`, `search(q, limit)`, `to_case()` (deserialise),
`update_data`, `soft_delete`. Companion tables: `audit_logs`,
`merge_records` (entity spec §10).

## Wire DTOs (service controller)

**File:** [`src/controllers/cases.rs`](../case-service-rust-crate/src/controllers/cases.rs)

| Type | Shape | Used by |
|---|---|---|
| `CaseRef` | `{ pid, title }` | create / update / list / search responses |
| `MatchRequest` | `{ query: Case, candidates: [Case] }` | `POST …/match` |
| `ScoredRef` | `{ pid, title, score, confidence, is_match }` | `POST …/check-duplicates` |
| `MergeRequest` | `{ main_pid, duplicate_pid, reason? }` | `POST …/merge` |

## Front-end TypeScript mirror

**File:** [`src/lib/api/types.ts`](../case-front-end-with-svelte/src/lib/api/types.ts)
— `Case`, `CaseType`, `CaseStatus`, `Priority`, `IdentifierScheme`,
`CaseRef`, `ScoredRef`. Hand-mirrored; MUST be updated in the same
change cycle as any matcher-type change (entity spec §18).
