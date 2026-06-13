## 5. Domain Model

### 5.1 Canonical Worker record (service shape)

The service's `Worker` is the system-of-record shape: FHIR-flavoured,
with vector sub-records. Field-by-field reference:
[service `AGENTS/models.md`](../worker-service-rust-crate/AGENTS/models.md).
Summary:

| Group | Fields |
|---|---|
| Identity | `id` (UUID), `active`, `name` (`HumanName`), `additional_names`, `gender`, `birth_date`, `deceased` (+ datetime) |
| Professional identifiers | `identifiers: Vec<Identifier>` — type (MRN, SSN, DL, NPI, PPN, TAX, ODS, Other) + system URI + value |
| Credentials | `documents: Vec<IdentityDocument>` — type, number, issuing country / authority, issue / expiry dates, verified flag |
| Tax | `tax_id` (plus `effective_tax_id()` falling back to a TAX-type identifier) |
| Contact | `telecom: Vec<ContactPoint>`, `addresses: Vec<Address>`, `emergency_contacts` |
| Relations | `links: Vec<WorkerLink>` (Replaces / ReplacedBy / Refer / Seealso), `managing_organization` |
| Audit | `created_at`, `updated_at` |

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
[`src/matching/adapter.rs`](../worker-service-rust-crate/src/matching/adapter.rs)
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

Invariants:

- The projection is **lossy but well-defined**: service-only fields
  (`id`, `active`, `worker_type`, `deceased_datetime`,
  `managing_organization`, `links`, timestamps) are dropped.
- Identifiers are **scheme-local** on the matcher side; an identifier
  with no matching slot (e.g. `IdentifierType::ODS`, the NHS
  Organisation Data Service code) falls through **unmapped** rather
  than being shoehorned into a wrong scheme.
- Both sides of the contract are pinned by the bridge test suite
  [`tests/duplicate_detection.rs`](../worker-service-rust-crate/tests/duplicate_detection.rs)
  (14 tests): a regression in either the adapter's routing or the
  matcher's scoring fails a test there.

### 5.4 Front-end↔service contract

The front-end consumes the service's JSON envelope
(`{ success, data, error }`) over the REST surface in §9, via
`ApiClient` + `WorkerRepository`
(`src/lib/api/client.ts`, `src/lib/api/workers.ts`). It never talks
to the matcher; matching is reachable only through the service's
`/api/workers/match` and duplicate-detection endpoints.
