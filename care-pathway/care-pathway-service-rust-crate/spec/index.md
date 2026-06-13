# Care Pathway Service — Specification

> **Single source of truth.** Code conforms to this spec. Behavioural
> change = spec + code + test in one PR. Live work queue is §13.
>
> Sibling matcher: [care-pathway-matcher](../../care-pathway-matcher-rust-crate/spec/index.md).
> Sibling front-end: [care-pathway-front-end-with-svelte](../../care-pathway-front-end-with-svelte/spec/index.md).

## 1. Purpose and vision

A registry of clinical care-pathway records for the Main X Index family:
create/read/update/delete and detect duplicates with the canonical
care-pathway-matcher. Built on loco.rs.

## 2. Scope

MVP: CRUD + matching. Deferred (§13): full-text search, streaming,
audit, privacy, OpenAPI, gRPC, rich validation. Authentication is out of
scope here — provided by the central authentication-service.

## 3. Stakeholders and users

Clinical informaticians curating pathways; peer services; the
care-pathway front-end.

## 4. Glossary

- **care pathway** — a standardised, evidence-based care plan.
- **pid** — public UUID of a pathway record.
- **data** — the full `CarePathway` payload stored as JSONB.
- **condition code** — ICD/SNOMED code of the target condition.

## 5. Domain model

The API DTO is `care_pathway_matcher::CarePathway`: `name`,
`alternate_names`, `pathway_code`, `provider_id`, `provider_name`,
`care_setting`, `condition_codes`, `interventions`, `keywords`,
`identifiers`, `same_as`, `in_language`.

## 6. Functional requirements

1. `POST /api/care-pathways` — create; `name` required and
   `condition_codes` format-validated against their `system` (ICD-10 /
   ICD-11 / SNOMED CT SCTID Verhoeff; `Custom` non-blank); `422` on any
   problem, all reported together — also enforced on update. Rules in
   [`src/validation.rs`](../src/validation.rs).
2. `GET /api/care-pathways` — list active (cap 100), `{pid, name}`.
3. `GET /api/care-pathways/{pid}` — return the stored `CarePathway`.
4. `PUT /api/care-pathways/{pid}` — replace the payload (`422` if
   `name` is blank or a `condition_codes` entry is malformed).
5. `DELETE /api/care-pathways/{pid}` — soft-delete.
6. `POST /api/care-pathways/match` — rank an explicit `{query,
   candidates}` set (no persistence).
7. `POST /api/care-pathways/check-duplicates` — match a query against
   stored pathways; return those above threshold, ranked.
8. `GET /api/care-pathways/audit/recent` + `/{pid}/audit` — audit-log
   query; `GET /api/care-pathways/events/recent` — in-memory event
   stream. Each CRUD action writes an `audit_logs` row and publishes a
   `created`/`updated`/`deleted` event.
9. `GET /api-docs/openapi.json` + `GET /swagger-ui` — OpenAPI 3
   document and a Swagger UI page rendering it.

## 7. Non-functional requirements

loco-idiomatic; Postgres persistence; deterministic matching via the
embedded library; soft-delete with audit-friendly timestamps.

## 8. Architecture

loco `App` (`src/app.rs`) registers the care-pathways controller. One
`care_pathways` table stores `pid` + denormalised `name` + the full
`CarePathway` JSONB `data`. Matching calls `care-pathway-matcher`
directly on the deserialised payloads — no adapter.

## 9. API surface

See §6. Raw loco JSON. `404` for unknown `pid`; `422` for a validation
failure (blank `name`, or a `condition_codes` entry malformed for its
coding system — family convention, via
`Error::CustomError(StatusCode::UNPROCESSABLE_ENTITY, …)`, with every
problem reported in one body); `400` for a malformed body.

## 10. Persistence

PostgreSQL via SeaORM + `sea-orm-migration`. Migrations
`m20220101_000001_care_pathways` (the `care_pathways` table) and
`m20220101_000002_audit_logs` (the CRUD `audit_logs` trail).
`auto_migrate` on in development.

## 11. Testing strategy

DB-free tests: `tests/matching.rs` (matcher embedding + JSON
round-trip), the `src/validation.rs` unit tests (ICD-10 / ICD-11 /
SNOMED-Verhoeff format checks), and controller validation unit tests
(blank-name and malformed-code → `422` pins). Request-level tests
(`tests/requests/care_pathways.rs`, loco testing harness) cover all
seven endpoints plus the malformed-code `422` but require Postgres, so
they are `#[ignore]`-gated — run with `cargo test -- --ignored` and a
`DATABASE_URL`.

## 12. Compliance

Care pathways are clinical artefacts, not patient data; still, honour
the family healthcare-compliance posture (HIPAA/NHS) for any audit and
access controls added later.

## 13. Tasks (live work queue)

- [ ] Tantivy full-text search.
- [x] Event streaming + audit log on CRUD — `audit_logs` table +
  best-effort row per create/update/delete (`models/audit_logs.rs`);
  in-memory `PathwayEvent` stream (`streaming.rs`); read at
  `/audit/recent`, `/{pid}/audit`, `/events/recent`. Durable broker +
  `actor` (needs auth) remain roadmap.
- [ ] Privacy controls if any restricted fields appear.
- [ ] Record merge with link tracking.
- [x] OpenAPI/Swagger — hand-written `src/openapi.rs` (matcher DTO is
  dependency-light, so no utoipa, matching the organization service)
  served at `/api-docs/openapi.json` + `/swagger-ui` by
  `controllers/docs.rs`.
- [x] Richer validation (ICD/SNOMED code formats) — `src/validation.rs`
  format-checks `condition_codes` per `system` (ICD-10, ICD-11, SNOMED
  CT SCTID Verhoeff); `422` with all problems. Terminology-server
  existence checks remain out of scope.
- [x] Request-level integration tests (Postgres) — landed
  `#[ignore]`-gated (entity spec §13 T-4); wiring a DB-backed run
  into CI remains.
- [ ] JWT verification middleware consuming the auth-service JWKS.

## 14. Implementation status

Done: loco boot; care_pathways table + migration; CRUD with `422`
validation on create/update (blank `name` + ICD-10 / ICD-11 / SNOMED CT
`condition_codes` format checks, all problems reported together);
`/match` and `/check-duplicates` embedding care-pathway-matcher;
audit log + in-memory event streaming on every CRUD (`/audit/recent`,
`/{pid}/audit`, `/events/recent`); OpenAPI 3 doc + Swagger UI
(`/api-docs/openapi.json`, `/swagger-ui`); DB-free tests + gated
request-level tests; green build + clippy.

## 15. Roadmap

v0.1 (here): CRUD + matching MVP. v0.2: search + audit + streaming.
v0.3: merge + OpenAPI + JWT middleware.

## 16. Open questions

- Normalise condition codes / interventions into their own tables once
  search lands?
- Real-time duplicate check on create (409) vs the explicit endpoint?

## 17. References

- The care-pathway-matcher spec; loco.rs; ICD-10 / SNOMED CT.

## 18. Change control

Update this spec with any behavioural change; bump `CHANGELOG.md`.
