# Organization Service — Specification

> **Single source of truth.** Code conforms to this spec. Behavioural
> change = spec + code + test in one PR. Live work queue is §13.
>
> Sibling matcher: [organization-matcher](../../organization-matcher-rust-crate/spec/index.md).
> Sibling front-end: [organization-front-end-with-svelte](../../organization-front-end-with-svelte/spec/index.md).

## 1. Purpose and vision

A registry of organization identities (schema.org/Organization) for the
Main X Index family: create/read/update/delete records and detect
duplicates with the canonical organization-matcher. Built on loco.rs.

## 2. Scope

MVP: CRUD + matching. Out of scope for the MVP (deferred, §13): full-text
search, streaming, audit, privacy/GDPR export, OpenAPI, gRPC, rich
validation. Authentication is out of scope here — provided by the
central authentication-service.

## 3. Stakeholders and users

Operators curating an organization registry; peer services resolving
organization identity; the organization front-end.

## 4. Glossary

- **pid** — public UUID of an organization record.
- **data** — the full `Organization` payload stored as JSONB.
- **deterministic identifier** — LEI/DUNS/etc. that pins a match to 1.0
  (see the matcher spec).

## 5. Domain model

The API DTO is `organization_matcher::Organization`: `name`,
`legal_name`, `alternate_names`, `identifiers`, `url`, `same_as`,
`address`, `jurisdiction`, `founding_date`, `telephone`, `email`,
`keywords`. The service does not fork this type.

## 6. Functional requirements

1. `POST /api/organizations` — create; `name` required (422 if blank).
2. `GET /api/organizations` — list active (cap 100), `{pid, name}`.
3. `GET /api/organizations/{pid}` — return the stored `Organization`.
4. `PUT /api/organizations/{pid}` — replace the payload; `name`
   required (422 if blank).
5. `DELETE /api/organizations/{pid}` — soft-delete (`active=false`,
   `deleted_at` stamped).
6. `POST /api/organizations/match` — rank an explicit `{query,
   candidates}` set (no persistence).
7. `POST /api/organizations/check-duplicates` — match a query against
   stored organizations; return the ones above threshold, ranked.

## 7. Non-functional requirements

loco-idiomatic; Postgres persistence; deterministic matching via the
embedded library; soft-delete with audit-friendly timestamps.

## 8. Architecture

loco `App` (`src/app.rs`) registers the organizations controller. One
`organizations` table stores `pid` + denormalised `name` + the full
`Organization` JSONB `data`. Matching calls `organization-matcher`
directly on the deserialised payloads — no adapter.

## 9. API surface

See §6. Responses are raw loco JSON. `404` for unknown `pid`; `422
Unprocessable Entity` for validation failures (blank `name` on create
or replace — family convention); `400` for malformed requests (blank
search `q`, invalid audit pid).

## 10. Persistence

PostgreSQL via SeaORM + `sea-orm-migration`. Migration
`m20220101_000001_organizations`. `auto_migrate` on in development.

## 11. Testing strategy

DB-free tests: `tests/matching.rs` (matcher embedding + JSON
round-trip) and unit tests in `src/` (validation → `422` pin, OpenAPI
shape, streaming). Request-level tests
(`tests/requests/organizations.rs`): boot the real app via loco's
`testing` harness and cover create round-trip, blank-name `422`
(create + update), unknown-pid `404`, search, and check-duplicates;
they require Postgres (`config/test.yaml`) and are `#[ignore]`d so
the default `cargo test` stays green — run with `cargo test --
--ignored`.

## 12. Compliance

Organization data is largely public, but contact fields may be
personal data — honour GDPR when the privacy layer lands (§13).

## 13. Tasks (live work queue)

- [x] Event streaming + audit log on CRUD.
- [x] Name search (Postgres `ILIKE`) + OpenAPI/Swagger.
- [ ] Tantivy full-text search + fuzzy/blocking (replacing the `ILIKE`
      search).
- [ ] Per-field masking + GDPR export endpoint.
- [ ] Record merge with link tracking.
- [ ] Richer validation (identifier formats, URL, country codes).
- [x] Request-level integration tests (Postgres; `#[ignore]`-gated).
- [ ] JWT verification middleware consuming the auth-service JWKS.

## 14. Implementation status

Done: loco boot; organizations table + migration; CRUD (blank name →
`422`, unknown pid → `404`); `/match` and `/check-duplicates` embedding
organization-matcher; audit log; in-memory event streaming; name search
(`ILIKE`); OpenAPI 3 + Swagger UI; DB-free tests; request-level test
suite (Postgres, `#[ignore]`-gated); loco scaffolding leftovers removed
(no workers/tasks/data stubs); green build + clippy.

## 15. Roadmap

v0.1 (here): CRUD + matching MVP. v0.2: search + audit + streaming.
v0.3: privacy + merge + OpenAPI + JWT middleware.

## 16. Open questions

- Should identifiers/address be normalised into their own tables (vs the
  single JSONB payload) once search lands?
- Real-time duplicate check on create (409) vs the explicit endpoint?

## 17. References

- schema.org/Organization; loco.rs; the organization-matcher spec.

## 18. Change control

Update this spec with any behavioural change; bump `CHANGELOG.md`.
