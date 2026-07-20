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
search, search-blocked dedup candidates, the durable event bus's real
Fluvio broker sink (Phases 2–3 — transactional outbox + relay/retention
— are done; only the broker-gated `FluvioSink` remains), privacy,
front-end merge action, a PASETO key-set
refresh loop (the boot-time paseto-keys-over-HTTP fetch is done —
`CARE_PATHWAY_PASETO_KEYS_URL`, §9/§13 — but runs once, no re-fetch),
terminology-server code-existence checks, gRPC. Token
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
   `bg_pg` worker: `POST`/`GET /api/care-pathways/import`,
   `POST`/`GET /api/care-pathways/export`,
   `GET /api/care-pathways/bulk-jobs`. The uniform family contract
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
`CARE_PATHWAY_TOKEN_AUDIENCE`. When `CARE_PATHWAY_PASETO_KEYS_URL` is
set, the key set is instead **fetched over HTTP once at boot**
(`Verifier::from_paseto_keys_url`, typically the auth-service
`/.well-known/paseto-keys`; seeded from `App::after_routes` via
`auth::init_from_env`): the fetched set wins over
`CARE_PATHWAY_PASETO_KEYS`; on fetch failure the service warns and falls
back to the env key set, so it always boots. No refresh loop — periodic
re-fetch on key rotation is a future item (§16). See the family contract
`agents/share/jwt-enforcement.md`; the session / token model is fixed by
[`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md),
which supersedes the prior RS256-JWT model.

**Authorization (ABAC).** Inside the same guard — so only when
`CARE_PATHWAY_REQUIRE_AUTH` is on — a verified token is authorized by
**attribute-based access control** per
[`agents/share/authorization-attributes.md`](../../../agents/share/authorization-attributes.md):
the request's action is derived from the HTTP method plus this crate's
destructive named POSTs (`auth::DESTRUCTIVE_POST_SUFFIXES` — `/merge`,
`/deduplicate`, `/import`; the latter two ahead of the dedup-scan and
bulk-import features), and the shared engine in
`authentication-verifier` 0.3 evaluates the policy over the token's
`attrs` claim, first-match-wins. Configure with `CARE_PATHWAY_ABAC_POLICY`
(inline JSON) or `CARE_PATHWAY_ABAC_POLICY_FILE` (path); unset or
unparsable ⇒ warn-log + the built-in default policy (any authenticated
subject reads; `access=write` writes; `access=admin` adds DELETE/merge;
`svc=true` does everything). `401` = missing/bad credential; `403` =
valid credential, policy denied (the body names the deciding rule). This
supersedes the earlier per-crate roles/RBAC sketch.

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
`Ok`, on+protected+{no/valid/expired/tampered} token → `401`/`Ok`; plus
the boot-time key-set fetch — a local ephemeral-port HTTP listener
serving the test key set proves the fetch-built verifier accepts a token
signed by that key, a fast-failing URL proves the env fallback without
panic, and no-URL pins the plain env path),
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
  previous wire shape. **Phase 2** (transactional outbox) is implemented:
  `CARE_PATHWAY_EVENT_TRANSPORT=outbox` writes one `event_outbox` row on
  the entity mutation's transaction (`streaming.rs`; default `memory`).
  **Phase 3** (relay + retention) is implemented — see the dedicated
  Phase-3 item below. Only the real Fluvio broker sink remains
  (broker-gated), designed in
  [`agents/share/event-bus.md`](../../../agents/share/event-bus.md);
  `actor` is wired through `publish_with_actor`.
- [x] **Durable event bus — Phase 3 (relay + retention).** `src/relay.rs`:
  the `EventSink` trait (the bus seam), a working no-broker **`LoggingSink`**
  default, `drain_once` (`unpublished` → `sink.send` → `mark_published`,
  at-least-once, per-pid order preserved on a send failure), and
  `purge_published` (retention: deletes `published_at < now() -
  INTERVAL '<CARE_PATHWAY_EVENT_RETENTION_DAYS> days'`, default 7). A
  background loop (`relay::spawn`, started in `App::after_routes`) ticks
  every `CARE_PATHWAY_EVENT_RELAY_INTERVAL_SECS` (default 5, floored at 1)
  and purges every N ticks — **gated by `CARE_PATHWAY_EVENT_TRANSPORT=outbox`
  AND `CARE_PATHWAY_EVENT_RELAY`** (truthy `1`/`true`/`yes`/`on`), so it is
  a no-op by default. Tests: DB-free `LoggingSink`/capturing-sink send +
  config defaults; the drain/ack seams (`unpublished`/`mark_published`) are
  DB-gated-tested via the outbox suite. **Broker-gated follow-up:** a real
  **`FluvioSink`** (`impl EventSink` behind a `fluvio` cargo feature) — the
  trait is the seam, so the drain loop is unchanged when it lands
  ([`agents/share/event-bus.md`](../../../agents/share/event-bus.md) §5, §8).
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
  auth-service JWKS; the credential has since been switched to PASETO —
  below.)
  - [x] Switch the credential RS256-JWT → **PASETO v4 public** per
    [`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)
    (supersedes the RS256-JWT + JWKS model). **Done:** `Verifier` verifies
    `v4.public.…` tokens against the auth-service's published Ed25519
    key; `from_paseto_keys_value` / `from_paseto_keys_url` replaced
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
  - [x] paseto-keys-over-HTTP fetch from the auth service at boot.
    **Done 2026-07-04:** new `CARE_PATHWAY_PASETO_KEYS_URL` env var (§9);
    when set, `auth::init_from_env` (called from `App::after_routes`
    before serving) fetches the key set once via
    `Verifier::from_paseto_keys_url` (`authentication-verifier` `fetch`
    feature) and seeds the process-wide verifier — fetched set wins
    (`tracing::info!`); on fetch failure it warns and falls back to the
    `CARE_PATHWAY_PASETO_KEYS` env path, so the service always boots.
    Unset/blank URL ⇒ prior env-injection behaviour exactly. Fetch-once
    only; a periodic refresh loop on key rotation stays future work
    (§16). Tests: local ephemeral-port HTTP listener serving the test
    key set (fetched verifier accepts a token signed by that key) +
    fast-failing URL fallback (no panic) + no-URL env path (§11).
- [ ] Bulk import/export — `bulk_jobs` migration (shared doc §3 schema,
  `UNIQUE (entity, kind, idempotency_key)`); the five endpoints
  (§6.13: `POST`/`GET /api/care-pathways/import`,
  `POST`/`GET /api/care-pathways/export`,
  `GET /api/care-pathways/bulk-jobs`); `bg_pg` worker draining
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
- [x] **FHIR R5 API** (`PlanDefinition`) — **Done** (`src/fhir/{mod,resources,search}.rs`
  + mounted `src/controllers/fhir.rs`, `routes()` in `app.rs`; 15 DB-free tests,
  `cargo test --lib` + `cargo clippy --lib` clean). Gaps: the DTO has no `status`
  field (the record's `active` flag is the source of truth — FHIR `status`/`type`
  are emitted but not carried back inbound); instantiated `CarePlan` remains
  roadmap. Original task text follows for reference. — adopt the family contract
  ([`agents/share/fhir.md`](../../../agents/share/fhir.md)). Map the stored
  `care_pathway_matcher::CarePathway` DTO to a FHIR **`PlanDefinition`**
  (§3, `medium` fidelity — a clinical pathway *template*): `name` →
  `title`, provider-scoped `pathway_code` (with `provider_id` /
  `provider_name`) → `identifier`, `condition_codes` (ICD/SNOMED) →
  `useContext` / action `condition`, `care_setting` → `useContext`,
  `interventions` → `action`, `keywords` → `useContext` / topic,
  `identifiers` / `same_as` → `identifier` / `relatedArtifact`, status →
  `status`, `type` = clinical protocol. An instantiated `CarePlan` is a
  roadmap alternative. New `src/fhir/` module (resource structs,
  `to_fhir_plan_definition` / `from_fhir_plan_definition`,
  `FhirOperationOutcome`, searchset `Bundle`, search-param parsing) + a
  mounted `src/controllers/fhir.rs` (`routes()` in `app.rs`) exposing
  read/create/update/delete/search at
  `/fhir/PlanDefinition{,/{id}}` + `GET /fhir/metadata`
  `CapabilityStatement`. Reuses the native model helpers,
  `src/validation.rs`, the event/audit path, and the blanket auth + ABAC
  guard (§8; `/fhir/*` guarded, action from HTTP method). Supported search
  params: `_id`, `_lastUpdated`, `_count`, `identifier`, `name`, `status`.
  Tests: DTO↔`PlanDefinition` round-trip, each interaction, search→Bundle,
  `OperationOutcome` on 404/400/422, `CapabilityStatement` matches routes.
- [x] **Input-size caps (SEC-M1).** `src/validation.rs` rejects oversized
  payloads before storage/matching (the O(n·m) matcher over unbounded
  text/arrays is a DoS, amplified by `check-duplicates`): `MAX_TEXT_LEN`
  1024 per free-text field, `MAX_ARRAY_LEN` 256 per array, `MAX_ITEM_LEN`
  512 per string array entry — all collected as `422` problems. DB-free
  tests for oversized field/array/entry + a within-caps large record.

- [x] **Fix: fresh-Postgres `db migrate` failed in the `event_outbox`
  migration (2026-07-18).** The loco `create_table` helper pluralizes
  table names (`cruet::to_plural`: `event_outbox` → `event_outboxes`),
  so the migration's own index DDL (`ON event_outbox`) failed and
  rolled the whole fresh migrate back — no tables were ever created.
  The migration is now explicit SQL creating exactly `event_outbox`
  (matching the `SeaORM` entity), `IF NOT EXISTS`-guarded; same
  migration name (the old form could never have applied anywhere).
  Found and fixed family-wide from the patient-flow implementation
  round; verified by a live fresh-database migrate. Every other table
  this crate creates via the helper is already plural (no-op).

- [x] **2026-07-20 — Registry insight views.** Five read-only
  derived views (`controllers/insights.rs`, prefix
  `/api/care-pathways/insights`) over the stored `CarePathway`
  templates, for the provider / setting / coverage lenses:
  `GET /directory` (faceted by `care_setting` + the `specialty:<x>`
  keyword convention), `/coverage` (per condition code, which settings
  have a pathway + disclosed gap rules: no primary-care / no emergency
  pathway), `/variants` (a condition offered by ≥2 providers, with the
  `jurisdiction:<x>` facet — a comparison directory, never a match
  signal), `/providers` (pathways per issuing provider by setting),
  `/languages` (per-language counts + the single-language-condition
  equity lens). No migration, no matcher change: facets come from
  existing DTO fields plus two disclosed keyword conventions.
  **Acceptance:** the seeded five-view request round-trip green first
  run — full `--ignored` suite 23/23 vs Postgres 18; clippy pedantic
  clean.

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
`EventView` projection on `/events/recent`); offline **PASETO v4 public**
verification (`AuthUser`/`MaybeAuthUser`, `/whoami`, audit `actor` from
the token — credential switched from RS256-JWT per §13), including the
boot-time paseto-keys-over-HTTP fetch (`CARE_PATHWAY_PASETO_KEYS_URL`,
fetch-once, env fallback; §9/§13); OpenAPI 3 doc
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
The credential switch RS256-JWT → PASETO v4 public per
[`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)
has since landed (§13), as has the boot-time paseto-keys-over-HTTP fetch
(`CARE_PATHWAY_PASETO_KEYS_URL`, fetch-once, env fallback; §9/§13). Next
(deferred, §13): Tantivy full-text/fuzzy search, the durable event bus's
real Fluvio broker sink (Phases 2–3 — outbox + relay/retention — are done;
only the broker-gated `FluvioSink` remains), a PASETO key-set refresh loop,
privacy, front-end merge action.

## 16. Open questions

- Normalise condition codes / interventions into their own tables once
  search lands?
- Real-time duplicate check on create (409) vs the explicit endpoint?
- Periodic re-fetch of the PASETO key set (key rotation) — the boot
  fetch (§9 `CARE_PATHWAY_PASETO_KEYS_URL`) runs once; is a refresh
  loop (or refetch-on-`UnknownKid`) needed before rotation goes live?

## 17. References

- The care-pathway-matcher spec; loco.rs; ICD-10 / SNOMED CT.

## 18. Change control

Update this spec with any behavioural change; bump `CHANGELOG.md`.
