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

MVP: CRUD + `ILIKE` name search + matching + record merge + audit log +
in-memory event streaming (durable-bus Phase 1) + OpenAPI/Swagger +
Prometheus metrics + offline PASETO v4 public token verification + blanket
`/api/*` enforcement (off by default) + rich payload validation
(ICD/SNOMED/UUID/DOI/BCP-47). Deferred (§13): Tantivy full-text/fuzzy
search, search-blocked dedup candidates, durable event bus Phases 2–3
(outbox → Fluvio), privacy, front-end merge action, paseto-keys-over-HTTP
fetch at boot, terminology-server code-existence checks, gRPC. Token
issuance is out of scope — provided by the central authentication-service.
The session / cross-service token model is fixed by
[`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md),
which supersedes the prior RS256-JWT model.

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
   (`401` without a valid token); proves offline PASETO v4 public verification.
11. `GET /api-docs/openapi.json` + `GET /swagger-ui` — OpenAPI 3
   document and a Swagger UI page rendering it.
12. `GET /metrics.prom` — Prometheus metrics in text-exposition format
   (`Content-Type: text/plain; version=0.0.4`), mounted at the root (not
   under `/api`) and public under blanket enforcement so a scraper
   needs no token. Exposes care-pathway CRUD/merge counters
   (`care_pathway_created_total` / `_updated_total` / `_deleted_total` /
   `_merged_total`) plus `http_requests_total`. Registry in
   [`src/metrics.rs`](../src/metrics.rs); handler in
   [`src/controllers/metrics.rs`](../src/controllers/metrics.rs).
13. Bulk import/export (deferred, §13) — async, job-based, on the loco
   `bg_pg` worker: `POST`/`GET /api/v1/care-pathways/import`,
   `POST`/`GET /api/v1/care-pathways/export`,
   `GET /api/v1/care-pathways/bulk-jobs`. The uniform family contract
   (execution model, five endpoints, JSONL/CSV/Parquet codecs,
   upsert-by-stable-key + dedupe-to-review, per-row error report, export
   masking + audit) is fixed in
   [`agents/share/bulk-import-export.md`](../../../agents/share/bulk-import-export.md).
   Care-pathway-specific bits — stable upsert keys (a deterministic
   scheme-scoped identifier the matcher short-circuits on /
   `(provider_id, pathway_code)`, same-provider only / `pid`); CSV
   flattening with every repeated/nested field a JSON-in-cell; clinical
   reference data (no patient-level data), masked-by-default export, still
   audited — are declared in the entity spec
   [§9.4](../../spec/09-api-surface.md) and
   [§10.4](../../spec/10-persistence.md).

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

**Auth.** Every route may carry `Authorization: Bearer v4.public.…`
(offline PASETO v4 public verification against the auth-service's
published Ed25519 key); handlers take `MaybeAuthUser` to stamp the audit
`actor`. Blanket `/api/*` enforcement is wired (an `after_routes`
middleware calling `auth::enforce`) but **off by default** — gated by
`CARE_PATHWAY_REQUIRE_AUTH` (`1`/`true`/`yes`/`on` ⇒ on). When on, any
`/api/*` route without a valid token is `401`; the public paths
`/_health`, `/_ping`, `/api-docs/openapi.json`, `/swagger-ui*`, and
`/metrics.prom` stay open (matching §6.12 and
`src/auth.rs::is_public_path`). The paseto-keys / issuer / audience come
from `CARE_PATHWAY_PASETO_KEYS` / `CARE_PATHWAY_TOKEN_ISSUER` /
`CARE_PATHWAY_TOKEN_AUDIENCE`. See the family contract
`agents/share/jwt-enforcement.md`; the session / token model is fixed by
[`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md),
which supersedes the prior RS256-JWT model.

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
real PASETO v4 public token + matching Ed25519 key in-process, then assert
valid → claims
and missing / non-bearer / expired / tampered / empty-verifier → `401`;
plus `parse_bool` cases and `enforce` — off+no-token → `Ok`, on+public →
`Ok`, on+protected+{no/valid/expired/tampered} token → `401`/`Ok`),
the `src/merge.rs` unit tests (former-title alias, scalar fallback, list
union, transferred snapshot), the `escape_like` unit test (search
wildcard neutralisation), the `src/metrics.rs` unit tests (the rendered
Prometheus text carries every metric name plus the `# HELP`/`# TYPE`
preamble, and the content type is `text/plain; version=0.0.4`), and
controller validation unit tests
(blank-name and malformed-code → `422` pins, plus an `is_self_merge`
equal-pid pin for the §6.8 self-merge `422` guard).
Request-level tests (`tests/requests/care_pathways.rs`,
loco testing harness) cover the CRUD + match endpoints, unknown-pid
`404` on GET / PUT / DELETE (and the merge `404`), the audit/event
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
  in-memory event stream (`streaming.rs`); read at
  `/audit/recent`, `/{pid}/audit`, `/events/recent`. **Phase 1** of the
  durable event bus is implemented: the canonical versioned `Envelope`
  (`event_id`, `schema_version` 1, `entity`, `kind`, `pid`, `seq`,
  `actor`, `name`) plus the `EventPublisher` trait seam with an
  `InMemoryPublisher` ring buffer; `/events/recent` returns the frozen
  `EventView` projection (`{kind, pid, name, seq}`), byte-identical to the
  previous wire shape. `occurred_at` / `data` are deferred to the outbox
  stage (Phase 2) per the design. Phases 2–3 (transactional outbox →
  Fluvio) remain infra-gated roadmap, designed in
  [`agents/share/event-bus.md`](../../../agents/share/event-bus.md);
  `actor` is wired through `publish_with_actor`.
- [ ] Privacy controls if any restricted fields appear.
- [x] Record merge — `POST /merge` folds a duplicate into a survivor
  (union fields, former-title alias, soft-delete, `merge_records`
  history + snapshot, `Merged` event); pure `src/merge.rs`;
  `/merges/recent`. Front-end merge action is a follow-up.
- [x] OpenAPI/Swagger — hand-written `src/openapi.rs` (matcher DTO is
  dependency-light, so no utoipa, matching the organization service)
  served at `/api-docs/openapi.json` + `/swagger-ui` by
  `controllers/docs.rs`.
- [x] Prometheus metrics — `GET /metrics.prom` (root path,
  `text/plain; version=0.0.4`) for parity with the older Axum services.
  Process-wide `OnceLock` registry in `src/metrics.rs`
  (`care_pathway_created_total` / `_updated_total` / `_deleted_total` /
  `_merged_total` counters + `http_requests_total` `IntCounterVec`);
  handler in `controllers/metrics.rs`, mounted at the root like
  `controllers/docs.rs` and added to `auth::is_public_path` so it stays
  open under blanket enforcement. The CRUD/merge controllers
  increment one counter per success path.
- [x] Richer validation (ICD/SNOMED code formats, identifier shapes,
  language tags) — `src/validation.rs` format-checks `condition_codes`
  per `system` (ICD-10, ICD-11, SNOMED CT SCTID Verhoeff), `identifiers`
  per `scheme` (canonical UUID for `Uuid`, `10.…/…` shape for `Doi`,
  non-blank for the rest), and `in_language` for BCP-47 syntax; `422`
  with all problems. Terminology-server / IANA-registry existence checks
  remain out of scope.
- [x] Request-level integration tests (Postgres) — landed
  `#[ignore]`-gated (entity spec §13 T-4), and the CI `test` job now runs
  them via a dedicated `cargo test ... -- --ignored` step against the
  provisioned Postgres service (`.github/workflows/ci.yaml`). Coverage
  includes unknown-pid `404` on GET / PUT / DELETE and the merge `404`.
- [x] Offline token verification — `src/auth.rs` embeds
  `authentication-verifier`; offline verification via a process-wide
  `Verifier` (env-configured keys/issuer/audience);
  `AuthUser`/`MaybeAuthUser` extractors; `/whoami` protected; audit
  `actor` stamped from the token. (Originally RS256-JWT against the
  auth-service JWKS; the credential is being switched to PASETO — below.)
  - [ ] Switch the credential RS256-JWT → **PASETO v4 public** per
    [`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)
    (supersedes the RS256-JWT + JWKS model): `Verifier` verifies
    `v4.public.…` tokens against the auth-service's published Ed25519
    key; `from_paseto_keys_value` / `from_paseto_keys_url` replace
    `from_jwks_*`; same `Claims` shape (`kid`/`iss`/`aud`/`exp`, `kid`
    in the footer); env vars `CARE_PATHWAY_PASETO_KEYS` /
    `CARE_PATHWAY_TOKEN_ISSUER` / `CARE_PATHWAY_TOKEN_AUDIENCE`.
  - [x] Blanket `/api/*` enforcement — pure `auth::enforce(require_auth,
    path, headers, verifier)` + an `axum::middleware::from_fn` layer in
    `app.rs after_routes`, wired unconditionally and gated per-request by
    `CARE_PATHWAY_REQUIRE_AUTH` (`auth::require_auth`, off by default;
    `1`/`true`/`yes`/`on` ⇒ on). Public paths (`/_health`, `/_ping`,
    `/api-docs/openapi.json`, `/swagger-ui*`) stay open. Family contract:
    `agents/share/jwt-enforcement.md` (credential now PASETO, semantics
    unchanged). Activation is an operations decision once the SSO token
    flow is live.
  - [ ] paseto-keys-over-HTTP fetch from the auth service at boot (still
    env-injected today).
- [ ] Bulk import/export — `bulk_jobs` migration (shared doc §3 schema,
  `UNIQUE (entity, kind, idempotency_key)`); the five endpoints
  (§6.13: `POST`/`GET /api/v1/care-pathways/import`,
  `POST`/`GET /api/v1/care-pathways/export`,
  `GET /api/v1/care-pathways/bulk-jobs`); `bg_pg` worker draining
  `queued → running → completed | completed_with_errors | failed`;
  JSONL/CSV/Parquet codecs (CSV flattening per entity spec §9.4 —
  every repeated/nested field a JSON-in-cell; Parquet export-only,
  feature-gated); per-row pipeline reusing `src/validation.rs` +
  the matcher + the review queue (upsert by a deterministic scheme-scoped
  identifier / `(provider_id, pathway_code)` / `pid`; keyless rows →
  duplicate detection → review queue, `provenance = import`; events +
  audit not bypassed); downloadable per-row error report
  (`row_number, source_line, field, code, message`); export masking
  (`masking_profile`, masked default) + `include_soft_deleted` gating +
  per-export audit (even zero-row). Uniform contract:
  [`agents/share/bulk-import-export.md`](../../../agents/share/bulk-import-export.md);
  entity-level detail: entity spec §9.4 / §10.4 / §13 T-10. Tests:
  idempotent re-import, per-row error report, keyless dedupe-to-review,
  masked vs full export, zero-row export still audited.

## 14. Implementation status

Done: loco boot; care_pathways table + migration; CRUD with `422`
validation on create/update (blank `name`; ICD-10 / ICD-11 / SNOMED CT
`condition_codes` format checks; UUID / DOI `identifiers` shapes; BCP-47
`in_language` syntax — all problems reported together);
`ILIKE` name search; `/match`, `/check-duplicates`, and `/merge`
(record merge + history)
embedding care-pathway-matcher; audit log + in-memory event streaming on
every CRUD/merge (`/audit/recent`, `/{pid}/audit`, `/events/recent`,
`/merges/recent`) — Phase 1 of the durable event bus (canonical
`Envelope` + `EventPublisher` seam + `InMemoryPublisher`; frozen
`EventView` projection on `/events/recent`); offline bearer-token verification (`AuthUser`/
`MaybeAuthUser`, `/whoami`, audit `actor` from the token — switching
RS256-JWT → PASETO v4 public per §13); OpenAPI 3 doc
+ Swagger UI (`/api-docs/openapi.json`, `/swagger-ui`); a root-level
Prometheus `/metrics.prom` endpoint (CRUD/merge counters +
`http_requests_total`, public under enforcement); blanket `/api/*`
enforcement middleware (`auth::enforce` + `after_routes` layer,
off by default via `CARE_PATHWAY_REQUIRE_AUTH`); DB-free tests +
gated request-level tests; green build + clippy.

## 15. Roadmap

All of the scope below shipped together in the still-unreleased `0.1.0`
line (Cargo.toml is `0.1.0`; CHANGELOG keeps it under `[Unreleased]`):
the CRUD + matching MVP, then `ILIKE` search + audit + in-memory
streaming, then record merge + OpenAPI/Swagger + Prometheus + offline
bearer-token verification + blanket `/api/*` enforcement middleware. The
original v0.2 / v0.3 milestone split was never cut as a tagged release.
Next (deferred, §13): switch the credential RS256-JWT → PASETO v4 public
per [`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)
(supersedes the RS256-JWT model), Tantivy full-text/fuzzy search, durable
event bus Phases 2–3 (outbox → Fluvio), paseto-keys-over-HTTP fetch at
boot, privacy, front-end merge action.

## 16. Open questions

- Normalise condition codes / interventions into their own tables once
  search lands?
- Real-time duplicate check on create (409) vs the explicit endpoint?

## 17. References

- The care-pathway-matcher spec; loco.rs; ICD-10 / SNOMED CT.

## 18. Change control

Update this spec with any behavioural change; bump `CHANGELOG.md`.
