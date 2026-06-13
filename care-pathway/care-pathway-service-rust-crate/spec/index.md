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

1. `POST /api/care-pathways` — create; `name` required (`422` if
   blank — also enforced on update).
2. `GET /api/care-pathways` — list active (cap 100), `{pid, name}`.
3. `GET /api/care-pathways/{pid}` — return the stored `CarePathway`.
4. `PUT /api/care-pathways/{pid}` — replace the payload (`422` if
   `name` is blank).
5. `DELETE /api/care-pathways/{pid}` — soft-delete.
6. `POST /api/care-pathways/match` — rank an explicit `{query,
   candidates}` set (no persistence).
7. `POST /api/care-pathways/check-duplicates` — match a query against
   stored pathways; return those above threshold, ranked.

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
failure (blank `name` — family convention, via
`Error::CustomError(StatusCode::UNPROCESSABLE_ENTITY, …)`); `400` for a
malformed body.

## 10. Persistence

PostgreSQL via SeaORM + `sea-orm-migration`. Migration
`m20220101_000001_care_pathways`. `auto_migrate` on in development.

## 11. Testing strategy

DB-free tests: `tests/matching.rs` (matcher embedding + JSON
round-trip) and controller validation unit tests (blank-name → `422`
pin). Request-level tests (`tests/requests/care_pathways.rs`, loco
testing harness) cover all seven endpoints but require Postgres, so
they are `#[ignore]`-gated — run with `cargo test -- --ignored` and a
`DATABASE_URL`.

## 12. Compliance

Care pathways are clinical artefacts, not patient data; still, honour
the family healthcare-compliance posture (HIPAA/NHS) for any audit and
access controls added later.

## 13. Tasks (live work queue)

- [ ] Tantivy full-text search.
- [ ] Event streaming + audit log on CRUD.
- [ ] Privacy controls if any restricted fields appear.
- [ ] Record merge with link tracking.
- [ ] OpenAPI/Swagger via utoipa.
- [ ] Richer validation (ICD/SNOMED code formats).
- [x] Request-level integration tests (Postgres) — landed
  `#[ignore]`-gated (entity spec §13 T-4); wiring a DB-backed run
  into CI remains.
- [ ] JWT verification middleware consuming the auth-service JWKS.

## 14. Implementation status

Done: loco boot; care_pathways table + migration; CRUD with `422`
blank-name validation on create/update; `/match` and
`/check-duplicates` embedding care-pathway-matcher; DB-free tests +
gated request-level tests; green build + clippy.

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
