# Case Service — Specification

> **Single source of truth.** Code conforms to this spec. Behavioural
> change = spec + code + test in one PR. Live work queue is §13.
>
> Sibling matcher: [case-matcher](../../case-matcher-rust-crate/spec/index.md).
> Sibling front-end: [case-front-end-with-svelte](../../case-front-end-with-svelte/spec/index.md).

## 1. Purpose and vision

A registry of governmental case records for the Main X Index family:
create/read/update/delete and detect duplicates with the canonical
case-matcher. Built on loco.rs.

## 2. Scope

MVP: CRUD + `ILIKE` title search + matching, with validation, OpenAPI,
audit, in-memory streaming, record merge, and offline JWT verification.
Deferred (§13): Tantivy full-text search, durable event bus, privacy,
gRPC. Authentication issuance is out of scope here — provided by the
central authentication-service; this service only verifies.

## 3. Stakeholders and users

Agency case-workers and data stewards curating cases; peer services; the
case front-end.

## 4. Glossary

- **case** — an open or historical matter handled by a public agency on
  behalf of one or more subjects (benefit claim, legal action,
  social-services referral, licensing application, complaint, appeal …).
- **pid** — public UUID of a case record.
- **data** — the full `Case` payload stored as JSONB.
- **subject** — an opaque involved-party identifier (e.g. a person pid).

## 5. Domain model

The API DTO is `case_matcher::Case`: `title`, `alternate_titles`,
`case_number`, `agency_id`, `agency_name`, `case_type`, `status`,
`priority`, `opened_date`, `subjects`, `keywords`, `identifiers`,
`same_as`, `in_language`. Enum unit variants serialise as bare
PascalCase strings; `Custom` as `{"Custom":"label"}`.

## 6. Functional requirements

1. `POST /api/cases` — create; `title` required, `opened_date` (if
   present) ISO-8601 `YYYY` / `YYYY-MM-DD`, identifier values non-blank,
   `subjects` / `keywords` entries non-blank; `422` on any problem, all
   reported together — also enforced on update. Rules in
   [`src/validation.rs`](../src/validation.rs).
2. `GET /api/cases` — list active (cap 100), `{pid, title}`.
   `GET /api/cases/search?q=` — case-insensitive title search
   (Postgres `ILIKE`, cap 50; blank `q` → `400`).
3. `GET /api/cases/{pid}` — return the stored `Case`.
4. `PUT /api/cases/{pid}` — replace the payload (`422` on any validation
   problem).
5. `DELETE /api/cases/{pid}` — soft-delete.
6. `POST /api/cases/match` — rank an explicit `{query, candidates}` set
   (no persistence).
7. `POST /api/cases/check-duplicates` — match a query against stored
   cases; return those above threshold, ranked.
8. `POST /api/cases/merge` — fold a duplicate into a survivor (union
   fields, former-title alias, soft-delete the duplicate, `merge_records`
   history, `Merged` event); `422` equal pids, `404` unknown.
   `GET /api/cases/merges/recent` — merge history.
9. `GET /api/cases/audit/recent` + `/{pid}/audit` — audit-log query;
   `GET /api/cases/events/recent` — in-memory event stream. Each
   create/update/delete/merge writes an `audit_logs` row and publishes a
   `created`/`updated`/`deleted`/`merged` event.
10. `GET /api/cases/whoami` — echo verified bearer-token claims (`401`
   without a valid token); proves offline RS256 verification.
11. `GET /api-docs/openapi.json` + `GET /swagger-ui` — OpenAPI 3
   document and a Swagger UI page rendering it.

## 7. Non-functional requirements

loco-idiomatic; Postgres persistence; deterministic matching via the
embedded library; soft-delete with audit-friendly timestamps.

**Configuration (environment).** JWKS / verification:
`CASE_JWKS` (the auth-service JWKS JSON; absent ⇒ empty key set, all
tokens rejected), `CASE_JWT_ISSUER` (default `authentication-service`),
`CASE_JWT_AUDIENCE` (default `main-x-service`). Access control:
`CASE_REQUIRE_AUTH` — blanket-enforcement flag, parsed leniently
(`1`/`true`/`yes`/`on`, case-insensitive ⇒ on; unset/blank/other ⇒ off),
**off by default** (see §9). Plus loco's own `DATABASE_URL` etc.

## 8. Architecture

loco `App` (`src/app.rs`) registers the cases controller. One `cases`
table stores `pid` + denormalised `title` + the full `Case` JSONB
`data`. Matching calls `case-matcher` directly on the deserialised
payloads — no adapter.

## 9. API surface

See §6. Raw loco JSON. `404` for unknown `pid`; `422` for a validation
failure (blank `title`, malformed `opened_date`, blank identifier value,
or blank `subjects` / `keywords` entry — family convention, via
`Error::CustomError(StatusCode::UNPROCESSABLE_ENTITY, …)`, with every
problem reported in one body); `400` for a malformed body.

**Authentication / blanket enforcement.** Offline RS256 JWT verification
(`src/auth.rs`, embedding `authentication-verifier`) underpins the
`AuthUser` / `MaybeAuthUser` extractors. When `CASE_REQUIRE_AUTH` is on,
an Axum `from_fn` middleware wired in `App::after_routes` (delegating to
the pure `auth::enforce(require_auth, path, headers, verifier)`) rejects
every non-public request lacking a valid bearer token with `401`;
`/_health`, `/_ping`, `/api-docs/openapi.json` and `/swagger-ui*` stay
public. The flag is read once per process and the layer is always wired,
so it is a near-noop when off. Enforcement is **off by default**;
because case data is personal data, this blanket gate is the
access-control boundary in front of the case API once activated (an
operations decision taken with the family SSO rollout). The contract is
the family-wide [`agents/share/jwt-enforcement.md`](../../../agents/share/jwt-enforcement.md).

## 10. Persistence

PostgreSQL via SeaORM + `sea-orm-migration`. Migrations
`m20220101_000001_cases` (the `cases` table),
`m20220101_000002_audit_logs` (the CRUD `audit_logs` trail), and
`m20220101_000003_merge_records` (record-merge history).
`auto_migrate` on in development.

## 11. Testing strategy

DB-free tests: `tests/matching.rs` (matcher embedding + JSON
round-trip), the `src/validation.rs` unit tests (title, `opened_date`
formats, blank identifier / subject / keyword), the `src/auth.rs` unit
tests (mint a real RS256 token + matching JWKS in-process, then assert
valid → claims and missing / non-bearer / expired / tampered /
empty-verifier → `401`; plus the blanket-enforcement decision —
`parse_bool` truthy/falsey cases and `enforce` off-no-token → `Ok`,
on-public → `Ok`, on-protected-no-token / expired / tampered → `401`,
on-protected-valid → `Ok`), the `src/merge.rs` unit tests (former-title
alias, scalar fallback, list union, transferred snapshot), the
`escape_like` unit test (search wildcard neutralisation), the
`src/openapi.rs` unit tests (well-formed doc; core + merge + whoami +
search endpoints), the `src/streaming.rs` unit test (publish/read-back),
and controller validation unit tests (blank-title and malformed-date →
`422` pins). Request-level tests (`tests/requests/cases.rs`, loco testing
harness) cover the CRUD + match endpoints, the audit/event trail,
`whoami` (no token → `401`), blanket enforcement (with
`CASE_REQUIRE_AUTH=1` set in-test: un-authed `GET /api/cases` → `401`,
public `GET /api-docs/openapi.json` → `200`; `#[serial]`), and
OpenAPI/Swagger but require Postgres, so they are `#[ignore]`-gated —
run with `cargo test -- --ignored` and a `DATABASE_URL`.

## 12. Compliance

Cases can hold government and personal data; honour the family
compliance posture (HIPAA/NHS/GDPR) for any audit and access controls
added later. Subjects are stored as opaque identifiers, not embedded PII.

## 13. Tasks (live work queue)

- [x] Title search — `GET /search?q=` Postgres `ILIKE` on the
  denormalised `title` (cap 50, wildcards escaped). Tantivy full-text /
  fuzzy search over the JSONB payload remains deferred.
- [x] Event streaming + audit log on CRUD — `audit_logs` table +
  best-effort row per create/update/delete (`models/audit_logs.rs`);
  in-memory `CaseEvent` stream (`streaming.rs`); read at `/audit/recent`,
  `/{pid}/audit`, `/events/recent`. Durable broker remains roadmap.
- [ ] Privacy controls if any restricted fields appear.
- [x] Record merge — `POST /merge` folds a duplicate into a survivor
  (union fields, former-title alias, soft-delete, `merge_records`
  history + snapshot, `Merged` event); pure `src/merge.rs`;
  `/merges/recent`. Front-end merge action is a follow-up.
- [x] OpenAPI/Swagger — hand-written `src/openapi.rs` (matcher DTO is
  dependency-light, so no utoipa, matching the organization service)
  served at `/api-docs/openapi.json` + `/swagger-ui` by
  `controllers/docs.rs`.
- [x] Payload validation — `src/validation.rs` checks `title`,
  `opened_date` (ISO-8601 `YYYY` / `YYYY-MM-DD` with calendar-range
  checks), non-blank identifier values, and non-blank `subjects` /
  `keywords`; `422` with all problems reported together.
- [x] Request-level integration tests (Postgres) — landed
  `#[ignore]`-gated; wiring a DB-backed run into CI remains.
- [x] JWT verification consuming the auth-service JWKS — `src/auth.rs`
  embeds `authentication-verifier`; offline RS256 verification via a
  process-wide `Verifier` (env-configured `CASE_JWKS` / `CASE_JWT_ISSUER`
  / `CASE_JWT_AUDIENCE`); `AuthUser`/`MaybeAuthUser` extractors;
  `/whoami` protected; audit `actor` stamped from the token.
  - [x] Blanket `/api/*` enforcement — `CASE_REQUIRE_AUTH` flag +
    `auth::enforce` middleware wired in `App::after_routes` (off by
    default; public paths exempt; un-gated `enforce`/`parse_bool` unit
    tests + DB-gated request test). Family contract
    [`agents/share/jwt-enforcement.md`](../../../agents/share/jwt-enforcement.md).
    Case data is personal data, so this is the access-control gate.
  - [ ] JWKS-over-HTTP fetch (instead of env injection) remains a
    follow-up, as does activating the flag (operations decision).

## 14. Implementation status

Done: loco boot; cases table + migration; CRUD with `422` validation on
create/update (blank `title`, `opened_date` format, non-blank
identifier / subject / keyword, all problems reported together);
`ILIKE` title search; `/match`, `/check-duplicates`, and `/merge`
(record merge + history) embedding case-matcher; audit log + in-memory
event streaming on every CRUD/merge (`/audit/recent`, `/{pid}/audit`,
`/events/recent`, `/merges/recent`); offline RS256 JWT verification
(`AuthUser`/`MaybeAuthUser`, `/whoami`, audit `actor` from the token);
OpenAPI 3 doc + Swagger UI (`/api-docs/openapi.json`, `/swagger-ui`);
DB-free tests + gated request-level tests; green build + clippy.

## 15. Roadmap

v0.1 (here): CRUD + title search + matching + merge + audit + streaming
+ OpenAPI + JWT verification. v0.2: Tantivy full-text/fuzzy search,
durable event bus. v0.3: privacy controls, blanket `/api/*` enforcement.

## 16. Open questions

- Normalise subjects / identifiers into their own tables once search
  lands?
- Real-time duplicate check on create (409) vs the explicit endpoint?

## 17. References

- The case-matcher spec; loco.rs; schema.org case-management vocabulary.

## 18. Change control

Update this spec with any behavioural change; bump `CHANGELOG.md`.
