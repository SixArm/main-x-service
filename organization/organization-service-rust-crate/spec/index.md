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

### Configuration environment variables

| Variable | Default | Purpose |
|---|---|---|
| `ORGANIZATION_JWKS` | empty key set | JWKS document for offline token verification (`src/auth.rs`). |
| `ORGANIZATION_JWT_ISSUER` | `authentication-service` | Expected `iss`. |
| `ORGANIZATION_JWT_AUDIENCE` | `main-x-service` | Expected `aud`. |
| `ORGANIZATION_REQUIRE_AUTH` | unset ⇒ **off** | Blanket `/api/*` JWT enforcement. Lenient bool: `1`/`true`/`yes`/`on` ⇒ on; else off. See `agents/share/jwt-enforcement.md`. |

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

**Auth.** `GET /api/organizations/whoami` always requires a valid bearer
token (the `AuthUser` extractor; `401` otherwise); other handlers take
`MaybeAuthUser` to stamp the audit/merge `actor` when a token is present.
When `ORGANIZATION_REQUIRE_AUTH` is on (see §7), an `axum` middleware
layer (`App::after_routes` → `auth::enforce`) requires a valid bearer
token on **every** route except the public health/ping + OpenAPI/Swagger
paths, returning `401` otherwise. The flag is read per request and is
**off by default**, so default behaviour is unchanged.

## 10. Persistence

PostgreSQL via SeaORM + `sea-orm-migration`. Migration
`m20220101_000001_organizations`. `auto_migrate` on in development.

## 11. Testing strategy

DB-free tests: `tests/matching.rs` (matcher embedding + JSON
round-trip) and unit tests in `src/` (validation → `422` pin, OpenAPI
shape, streaming, and `auth::tests` — `bearer_claims` plus the pure
`enforce`/`parse_bool` decision: off+no-token ⇒ ok, on+public ⇒ ok,
on+protected without/expired/tampered ⇒ `401`, on+protected+valid ⇒
ok). Request-level tests (`tests/requests/organizations.rs`): boot the
real app via loco's `testing` harness and cover create round-trip,
blank-name `422` (create + update), unknown-pid `404`, search,
check-duplicates, merge, `whoami` `401`, and the blanket-enforcement
gate (with `ORGANIZATION_REQUIRE_AUTH=1` set in-test, un-authed `GET
/api/organizations` ⇒ `401` while `GET /api-docs/openapi.json` ⇒
`200`; `#[serial]` for env-var ordering). These require Postgres
(`config/test.yaml`) and are `#[ignore]`d so the default `cargo test`
stays green — run with `cargo test -- --ignored`.

## 12. Compliance

Organization data is largely public, but contact fields may be
personal data — honour GDPR when the privacy layer lands (§13).

## 13. Tasks (live work queue)

- [x] Event streaming + audit log on CRUD. **Phase 1 (in-memory
  envelope + `EventPublisher` seam) implemented** per
  [`agents/share/event-bus.md`](../../../agents/share/event-bus.md):
  `src/streaming.rs` carries the canonical versioned `Envelope`
  (`event_id`, `schema_version` = 1, `entity`, `kind`, `pid`, `seq`,
  `actor`, `name`; `occurred_at`/`data` deferred to the outbox stage),
  an `EventPublisher` trait, and an `InMemoryPublisher` ring buffer
  (process-wide `OnceLock`). The operator endpoint
  `/api/organizations/events/recent` returns the frozen flat
  `EventView { kind, pid, name, seq }` projection (wire shape unchanged
  — front-end safe). CRUD/merge call sites stamp the bearer `actor`.
  Phases 2–3 (transactional outbox + Fluvio relay) remain infra-gated
  roadmap.
- [x] Name search (Postgres `ILIKE`) + OpenAPI/Swagger.
- [ ] Tantivy full-text search + fuzzy/blocking (replacing the `ILIKE`
      search).
- [ ] Per-field masking + GDPR export endpoint.
- [x] Record merge — `POST /merge` folds a duplicate into a survivor
  (union fields, former-name alias, soft-delete, `merge_records` history
  + snapshot, `Merged` event); pure `src/merge.rs`; `/merges/recent`.
- [ ] Richer validation (identifier formats, URL, country codes).
- [x] Request-level integration tests (Postgres; `#[ignore]`-gated).
- [x] JWT verification consuming the auth-service JWKS — `src/auth.rs`
  embeds `authentication-verifier`; offline RS256 verification via a
  process-wide `Verifier` (env-configured JWKS/issuer/audience);
  `AuthUser`/`MaybeAuthUser` extractors; `/whoami` protected; audit +
  merge `actor` from the token.
  - [x] Blanket `/api/*` enforcement — `auth::enforce` (pure, unit-tested)
    wired as an `axum::middleware::from_fn` layer in `App::after_routes`,
    gated by `ORGANIZATION_REQUIRE_AUTH` (lenient bool, **default off**).
    Public paths (`/_health`, `/_ping`, `/api-docs/openapi.json`,
    `/swagger-ui*`) stay open; everything else needs a valid bearer token
    when the flag is on. Off by default keeps current behaviour and the
    existing DB-gated tests green. Family contract:
    `agents/share/jwt-enforcement.md`.
  - [ ] JWKS-over-HTTP fetch at boot (vs env injection) — follow-up.

## 14. Implementation status

Done: loco boot; organizations table + migration; CRUD (blank name →
`422`, unknown pid → `404`); `/match` and `/check-duplicates` embedding
organization-matcher; audit log; in-memory event streaming (Phase 1:
canonical `Envelope` + `EventPublisher` seam, `EventView` projection
frozen for `/events/recent`); name search (`ILIKE`); record merge
(`/merge` + `merge_records` history); offline
RS256 JWT verification (`AuthUser`/`MaybeAuthUser`, `/whoami`, audit +
merge `actor` from the token); OpenAPI 3 + Swagger UI; DB-free tests;
request-level test suite (Postgres, `#[ignore]`-gated); loco scaffolding
leftovers removed (no workers/tasks/data stubs); green build + clippy.

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
