## 5. Domain Model

### 5.1 Canonical Worker record (service shape)

The service's `Worker` is the system-of-record shape: FHIR-flavoured,
with vector sub-records. Field-by-field reference:
[service `agents/models.md`](../worker-service-with-loco/agents/models.md).
Summary:

| Group | Fields |
|---|---|
| Identity | `id` (UUID), `active`, `name` (`HumanName`), `additional_names`, `gender`, `birth_date`, `deceased` (+ datetime) |
| Professional identifiers | `identifiers: Vec<Identifier>` — type (MRN, SSN, DL, NPI, PPN, TAX, ODS, Other) + system URI + value |
| Credentials | `documents: Vec<IdentityDocument>` — type, number, issuing country / authority, issue / expiry dates, verified flag |
| Tax | `tax_id` (plus `effective_tax_id()` falling back to a TAX-type identifier) |
| Contact | `telecom: Vec<ContactPoint>`, `addresses: Vec<Address>`, `emergency_contacts` |
| Relations | `links: Vec<WorkerLink>` (Replaces / ReplacedBy / Refer / Seealso), `managing_organization` |
| Relationships | `relationships: Vec<WorkerRelationship>` — typed worker-to-worker links (see below) |
| Tags | `tags: Vec<String>` — operator-applied labels (see below) |
| Audit | `created_at`, `updated_at` |

**Relationships** — typed worker-to-worker links:
`relationships: Vec<WorkerRelationship>`, each `{ relation, worker_id }`
**referencing another `Worker` in the registry**. `relation` is a
`RelationKind` enum, initially **`LineManagerOf`** and **`ReportsTo`**:

- `LineManagerOf` / `ReportsTo` are **inverses** — A `LineManagerOf` B
  (A is B's line manager) ⇔ B `ReportsTo` A (B reports to A).

The enum is extensible (e.g. `MentorOf`, `ColleagueOf` later, the latter
being symmetric). These links are registry-internal worker-to-worker
references, distinct from the merge `links: Vec<WorkerLink>` and from
`managing_organization`.

**Tags** — `tags: Vec<String>` is a list of short free-text labels
that operators attach to a record for grouping, filtering, triage, or
workflow (e.g. `"vip"`, `"review"`, `"archived-2026"`, `"fast-track"`).
**Any `Worker` can carry tags.** Each tag is a short, trimmed,
non-empty string; the list is unordered, de-duplicated
case-insensitively, and defaults to empty.

The Worker model has no `keywords` field, so `tags` are the record's
labelling mechanism: they are **user-applied operational labels** for
grouping and workflow, not descriptive discovery terms about what the
record is. Tags are also a **supporting match signal**: they feed the
matcher (§5.2) via the DTO contract (§5.3) and are scored by set
Jaccard over the case-insensitively normalised tag sets, weighted
`tags_weight` (matcher §13.1). As a supporting signal they raise the
score when two records share tags, but never identify a worker on
their own.

The canonical model above is **upstream**: the matcher DTO (§5.2–§5.3)
and the front-end types (§5.4) follow `tags` in the same change cycle
per the contracts those sections define.

The front-end mirrors this shape in TypeScript
(`src/lib/api/types.ts`); if a field changes in the service, the
front-end types change in the same effort — see
[front-end `AGENTS.md`](../worker-front-end-with-svelte/AGENTS.md).

### 5.2 Matcher Worker record (comparison shape)

The matcher's `Worker` is a flat, builder-shaped comparison input:
explicit `family_name` / `given_name` / `middle_name`,
`phone` / `mobile` / `email`, one current `address` plus
`previous_addresses`, `passport_books: Vec<PassportBook>`, and **one
field per national-identifier scheme** (42 schemes — UK NHS, US SSN,
FR NIR, BR CPF, IN Aadhaar, …). Reference:
[matcher §8 Domain Model](../worker-matcher-rust-crate/spec/08-domain-model.md)
and [§11 Public API](../worker-matcher-rust-crate/spec/11-public-api-specification.md).

### 5.3 The service↔matcher DTO contract

**This is the contract this entity spec is authoritative for.**

The service embeds `worker-matcher` (currently `0.6.1` in
`Cargo.toml`), re-exports it from `src/matching/mod.rs` as
`matcher_lib`, and bridges via
[`src/matching/adapter.rs`](../worker-service-with-loco/src/matching/adapter.rs)
→ `to_matcher_worker(&service::Worker) -> worker_matcher::Worker`.

Routing rules (full table inline in the adapter; highlights):

| Service field | Matcher slot |
|---|---|
| `name.family` / `given[0]` / `given[1]` | `family_name` / `given_name` / `middle_name` |
| `birth_date`, `gender` | `date_of_birth`, `gender` |
| first `addresses[]` | `address` (rest → `previous_addresses`) |
| telecom by `ContactPointSystem` | `Phone` → `phone`, `Sms` → `mobile`, `Email` → `email` |
| `identifiers[]` by `system` URI (type-based fallback) | the matching national-identifier slot |
| `tax_id` | US SSN slot (default routing) |
| passport `documents[]` | `passport_books` |
| `relationships[]` | matcher `relationships` (typed `(relation, worker_id)` refs) |
| `tags[]` | matcher `tags` (case-insensitively normalised label set) |

`relationships[]` route to the matcher `relationships` field as typed
`(relation, worker_id)` refs (matcher §8), scored by typed-set Jaccard
(matcher §12), weighted `relationships_weight`; they are **not** in the
lossy-drop list below.

`tags[]` route to the matcher `tags` field as a case-insensitively
normalised label set (matcher §8), scored by set Jaccard (matcher §12),
weighted `tags_weight`; they are **not** in the lossy-drop list below.

Invariants:

- A `WorkerRelationship` references an **existing** `Worker` in the
  registry; **no worker relates to itself** (not its own line manager /
  report). The directional `LineManagerOf` / `ReportsTo` kinds must stay
  **acyclic** (no worker is their own manager, directly or transitively)
  and, where both directions are stored, mutually consistent
  (A `LineManagerOf` B ⇔ B `ReportsTo` A). Symmetric kinds added later
  (e.g. `ColleagueOf`) must be stored symmetrically.
- The projection is **lossy but well-defined**: service-only fields
  (`id`, `active`, `worker_type`, `deceased_datetime`,
  `managing_organization`, `links`, timestamps) are dropped.
- Identifiers are **scheme-local** on the matcher side; an identifier
  with no matching slot (e.g. `IdentifierType::ODS`, the NHS
  Organisation Data Service code) falls through **unmapped** rather
  than being shoehorned into a wrong scheme.
- Both sides of the contract are pinned by the bridge test suite
  [`tests/duplicate_detection.rs`](../worker-service-with-loco/tests/duplicate_detection.rs)
  (14 tests): a regression in either the adapter's routing or the
  matcher's scoring fails a test there.

### 5.4 Front-end↔service contract

The front-end consumes the service's JSON envelope
(`{ success, data, error }`) over the REST surface in §9, via
`ApiClient` + `WorkerRepository`
(`src/lib/api/client.ts`, `src/lib/api/workers.ts`). It never talks
to the matcher; matching is reachable only through the service's
`/api/workers/match` and duplicate-detection endpoints.
