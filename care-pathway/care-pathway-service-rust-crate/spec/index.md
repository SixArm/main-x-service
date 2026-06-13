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

1. `POST /api/care-pathways` — create; `name` required,
   `condition_codes` format-validated against their `system` (ICD-10 /
   ICD-11 / SNOMED CT SCTID Verhoeff; `Custom` non-blank), `identifiers`
   structurally checked (canonical UUID for `Uuid`; `10.…/…` shape for
   `Doi`; other schemes non-blank), and `in_language` checked for BCP-47
   syntax; `422` on any problem, all reported together — also enforced on
   update. Rules in [`src/validation.rs`](../src/validation.rs).
2. `GET /api/care-pathways` — list active (cap 100), `{pid, name}`.
   `GET /api/care-pathways/search?q=` — case-insensitive name search
   (Postgres `ILIKE`, cap 50; blank `q` → `400`).
3. `GET /api/care-pathways/{pid}` — return the stored `CarePathway`.
4. `PUT /api/care-pathways/{pid}` — replace the payload (`422` if
   `name` is blank, or any `condition_codes` / `identifiers` /
   `in_language` entry is malformed).
5. `DELETE /api/care-pathways/{pid}` — soft-delete.
6. `POST /api/care-pathways/match` — rank an explicit `{query,
   candidates}` set (no persistence).
7. `POST /api/care-pathways/check-duplicates` — match a query against
   stored pathways; return those above threshold, ranked.
8. `POST /api/care-pathways/merge` — fold a duplicate into a survivor
   (union fields, former-title alias, soft-delete the duplicate,
   `merge_records` history, `Merged` event); `422` equal pids, `404`
   unknown. `GET /api/care-pathways/merges/recent` — merge history.
9. `GET /api/care-pathways/audit/recent` + `/{pid}/audit` — audit-log
   query; `GET /api/care-pathways/events/recent` — in-memory event
   stream. Each create/update/delete/merge writes an `audit_logs` row
   and publishes a `created`/`updated`/`deleted`/`merged` event.
10. `GET /api/care-pathways/whoami` — echo verified bearer-token claims
   (`401` without a valid token); proves offline RS256 verification.
11. `GET /api-docs/openapi.json` + `GET /swagger-ui` — OpenAPI 3
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
failure (blank `name`, a `condition_codes` entry malformed for its
coding system, an `identifiers` entry malformed for its scheme, or an
`in_language` tag that is not valid BCP-47 — family convention, via
`Error::CustomError(StatusCode::UNPROCESSABLE_ENTITY, …)`, with every
problem reported in one body); `400` for a malformed body.

**Auth.** Every route may carry `Authorization: Bearer <jwt>` (offline
RS256 verification against the auth-service JWKS); handlers take
`MaybeAuthUser` to stamp the audit `actor`. Blanket `/api/*` enforcement
is wired (an `after_routes` middleware calling `auth::enforce`) but
**off by default** — gated by `CARE_PATHWAY_REQUIRE_AUTH`
(`1`/`true`/`yes`/`on` ⇒ on). When on, any `/api/*` route without a valid
token is `401`; the public paths `/_health`, `/_ping`,
`/api-docs/openapi.json`, `/swagger-ui*` stay open. JWKS/issuer/audience
come from `CARE_PATHWAY_JWKS` / `CARE_PATHWAY_JWT_ISSUER` /
`CARE_PATHWAY_JWT_AUDIENCE`. See the family contract
`agents/share/jwt-enforcement.md`.

## 10. Persistence

PostgreSQL via SeaORM + `sea-orm-migration`. Migrations
`m20220101_000001_care_pathways` (the `care_pathways` table),
`m20220101_000002_audit_logs` (the CRUD `audit_logs` trail), and
`m20220101_000003_merge_records` (record-merge history).
`auto_migrate` on in development.

## 11. Testing strategy

DB-free tests: `tests/matching.rs` (matcher embedding + JSON
round-trip), the `src/validation.rs` unit tests (ICD-10 / ICD-11 /
SNOMED-Verhoeff code formats, UUID / DOI identifier shapes, and BCP-47
`in_language` syntax), the `src/auth.rs` unit tests (mint a
real RS256 token + matching JWKS in-process, then assert valid → claims
and missing / non-bearer / expired / tampered / empty-verifier → `401`;
plus `parse_bool` cases and `enforce` — off+no-token → `Ok`, on+public →
`Ok`, on+protected+{no/valid/expired/tampered} token → `401`/`Ok`),
the `src/merge.rs` unit tests (former-title alias, scalar fallback, list
union, transferred snapshot), the `escape_like` unit test (search
wildcard neutralisation), and controller validation unit tests
(blank-name and malformed-code → `422` pins). Request-level tests (`tests/requests/care_pathways.rs`,
loco testing harness) cover the CRUD + match endpoints, the audit/event
trail, `whoami` (no token → `401`), blanket enforcement (with
`CARE_PATHWAY_REQUIRE_AUTH=1` set in-test: un-authed `GET
/api/care-pathways` → `401`, public `GET /api-docs/openapi.json` →
`200`; `#[serial]`), and OpenAPI/Swagger but require
Postgres, so they are `#[ignore]`-gated — run with
`cargo test -- --ignored` and a `DATABASE_URL`.

## 12. Compliance

Care pathways are clinical artefacts, not patient data; still, honour
the family healthcare-compliance posture (HIPAA/NHS) for any audit and
access controls added later.

## 13. Tasks (live work queue)

- [x] Name search — `GET /search?q=` Postgres `ILIKE` on the
  denormalised `name` (cap 50, wildcards escaped). Tantivy full-text /
  fuzzy search over the JSONB payload remains deferred.
- [x] Event streaming + audit log on CRUD — `audit_logs` table +
  best-effort row per create/update/delete (`models/audit_logs.rs`);
  in-memory `PathwayEvent` stream (`streaming.rs`); read at
  `/audit/recent`, `/{pid}/audit`, `/events/recent`. The durable broker
  is designed in
  [`agents/share/event-bus.md`](../../../agents/share/event-bus.md)
  (transactional outbox → Fluvio) and remains roadmap; `actor` is wired.
- [ ] Privacy controls if any restricted fields appear.
- [x] Record merge — `POST /merge` folds a duplicate into a survivor
  (union fields, former-title alias, soft-delete, `merge_records`
  history + snapshot, `Merged` event); pure `src/merge.rs`;
  `/merges/recent`. Front-end merge action is a follow-up.
- [x] OpenAPI/Swagger — hand-written `src/openapi.rs` (matcher DTO is
  dependency-light, so no utoipa, matching the organization service)
  served at `/api-docs/openapi.json` + `/swagger-ui` by
  `controllers/docs.rs`.
- [x] Richer validation (ICD/SNOMED code formats, identifier shapes,
  language tags) — `src/validation.rs` format-checks `condition_codes`
  per `system` (ICD-10, ICD-11, SNOMED CT SCTID Verhoeff), `identifiers`
  per `scheme` (canonical UUID for `Uuid`, `10.…/…` shape for `Doi`,
  non-blank for the rest), and `in_language` for BCP-47 syntax; `422`
  with all problems. Terminology-server / IANA-registry existence checks
  remain out of scope.
- [x] Request-level integration tests (Postgres) — landed
  `#[ignore]`-gated (entity spec §13 T-4); wiring a DB-backed run
  into CI remains.
- [x] JWT verification consuming the auth-service JWKS — `src/auth.rs`
  embeds `authentication-verifier`; offline RS256 verification via a
  process-wide `Verifier` (env-configured JWKS/issuer/audience);
  `AuthUser`/`MaybeAuthUser` extractors; `/whoami` protected; audit
  `actor` stamped from the token.
  - [x] Blanket `/api/*` enforcement — pure `auth::enforce(require_auth,
    path, headers, verifier)` + an `axum::middleware::from_fn` layer in
    `app.rs after_routes`, wired unconditionally and gated per-request by
    `CARE_PATHWAY_REQUIRE_AUTH` (`auth::require_auth`, off by default;
    `1`/`true`/`yes`/`on` ⇒ on). Public paths (`/_health`, `/_ping`,
    `/api-docs/openapi.json`, `/swagger-ui*`) stay open. Family contract:
    `agents/share/jwt-enforcement.md`. Activation is an operations
    decision once the SSO token flow is live.
  - [ ] JWKS-over-HTTP fetch from the auth service at boot (still
    env-injected today).

## 14. Implementation status

Done: loco boot; care_pathways table + migration; CRUD with `422`
validation on create/update (blank `name`; ICD-10 / ICD-11 / SNOMED CT
`condition_codes` format checks; UUID / DOI `identifiers` shapes; BCP-47
`in_language` syntax — all problems reported together);
`ILIKE` name search; `/match`, `/check-duplicates`, and `/merge`
(record merge + history)
embedding care-pathway-matcher; audit log + in-memory event streaming on
every CRUD/merge (`/audit/recent`, `/{pid}/audit`, `/events/recent`,
`/merges/recent`); offline RS256 JWT verification (`AuthUser`/
`MaybeAuthUser`, `/whoami`, audit `actor` from the token); OpenAPI 3 doc
+ Swagger UI (`/api-docs/openapi.json`, `/swagger-ui`); blanket `/api/*`
JWT enforcement middleware (`auth::enforce` + `after_routes` layer,
off by default via `CARE_PATHWAY_REQUIRE_AUTH`); DB-free tests +
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
