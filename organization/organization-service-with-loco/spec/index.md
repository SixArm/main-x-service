# Organization Service — Specification

> **Single source of truth (crate internals).** Code conforms to this
> spec. Behavioural change = spec + code + test in one PR. Live work
> queue is §13.
>
> **Entity umbrella spec:** the fuller cross-subproject contract lives
> at [`organization/spec/index.md`](../../spec/index.md) (numbered files
> `01-…`–`18-…`). It carries the entity-wide **requirement / task IDs**
> (`R-DUP`, `T-2`, `T-7`, `T-9`, `T-12`, `OQ-1`) that the source-code
> comments cite (e.g. `src/controllers/organizations.rs` → "spec §6
> R-DUP, task T-7"; `src/auth.rs` → "spec §13 T-9"; `src/app.rs` →
> "entity spec §13 T-12"). Those citations resolve against the umbrella
> spec's §6 / §13. When the umbrella spec and this crate spec disagree
> about crate internals, this crate spec wins; about the integration
> contract, the umbrella spec wins (see its header).
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

**Partition rule.** The within-entity `relationships[]` (org → org,
inside this registry) are a matcher signal. **Cross-service links**
(e.g. `person works_at organization`) are separate: they live only in
the aggregator + the originating service, never in `relationships`, and
are never fed to the matcher. See §8 and
[`agents/share/cross-service-linking.md`](../../../agents/share/cross-service-linking.md) §7.

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
8. `GET /metrics.prom` — Prometheus metrics in text-exposition format
   (`text/plain; version=0.0.4`). Mounted at the application **root**
   (not under `/api`), public even under blanket auth enforcement.

## 7. Non-functional requirements

loco-idiomatic; Postgres persistence; deterministic matching via the
embedded library; soft-delete with audit-friendly timestamps.

### Configuration environment variables

| Variable | Default | Purpose |
|---|---|---|
| `ORGANIZATION_PASETO_KEYS` | empty key set | Published Ed25519 public-key set (`paseto-keys` JSON) for offline PASETO v4.public token verification (`src/auth.rs`). |
| `ORGANIZATION_PASETO_KEYS_URL` | unset ⇒ no fetch | When set, fetch the key set over HTTP **once at boot** (`Verifier::from_paseto_keys_url`, typically the auth-service `/.well-known/paseto-keys`; seeded from `App::after_routes` via `auth::init_from_env`). Success ⇒ the fetched set wins over `ORGANIZATION_PASETO_KEYS`; failure ⇒ warn + fall back to the env key set — the service always boots. No refresh loop (periodic re-fetch on key rotation is a future item, §16). |
| `ORGANIZATION_TOKEN_ISSUER` | `authentication-service` | Expected `iss` (see [`authentication-sessions.md`](../../../agents/share/authentication-sessions.md) §5 claims). |
| `ORGANIZATION_TOKEN_AUDIENCE` | `main-x-service` | Expected `aud` (see [`authentication-sessions.md`](../../../agents/share/authentication-sessions.md) §5 claims). |
| `ORGANIZATION_REQUIRE_AUTH` | unset ⇒ **off** | Blanket `/api/*` enforcement (credential is now a PASETO v4.public token or BFF cookie session). Lenient bool: `1`/`true`/`yes`/`on` ⇒ on; else off. See [`agents/share/jwt-enforcement.md`](../../../agents/share/jwt-enforcement.md) (credential superseded by [`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)). |
| `ORGANIZATION_ABAC_POLICY` | unset ⇒ built-in default policy | ABAC authorization policy as inline JSON (evaluated only when enforcement is on). Unparsable ⇒ warn-log + built-in default. See [`agents/share/authorization-attributes.md`](../../../agents/share/authorization-attributes.md). |
| `ORGANIZATION_ABAC_POLICY_FILE` | unset | Path to the ABAC policy JSON file (used when `ORGANIZATION_ABAC_POLICY` is unset). Unreadable/unparsable ⇒ warn-log + built-in default. |
| `ORGANIZATION_EVENT_TRANSPORT` | `memory` | Durable event-bus transport ([`agents/share/event-bus.md`](../../../agents/share/event-bus.md) §7). `memory` ⇒ the process-wide ring buffer (Phase 1; no DB, no tx — today's behaviour). `outbox` ⇒ the transactional outbox (Phase 2): every CRUD/merge handler writes one `event_outbox` row **on the same transaction** as the entity mutation, so the change and its event commit or roll back together. Unrecognised value ⇒ `memory` (fail-safe). Read once at boot and cached. |
| `ORGANIZATION_EVENT_RELAY` | off | Phase-3 relay switch. Truthy (`1`/`true`/`yes`/`on`) **and** `EVENT_TRANSPORT=outbox` ⇒ `App::after_routes` spawns the background relay loop (`src/relay.rs`: drain `event_outbox` → `EventSink` → `mark_published`, + periodic retention purge). Off by default ⇒ no loop. |
| `ORGANIZATION_EVENT_RELAY_INTERVAL_SECS` | `5` | Relay drain-loop tick interval (floored at 1). |
| `ORGANIZATION_EVENT_RETENTION_DAYS` | `7` | Outbox row TTL. **Enforced** by the Phase-3 relay's periodic `purge_published` (deletes `published_at < now() - INTERVAL '<n> days'`) when the relay runs ([`agents/share/event-bus.md`](../../../agents/share/event-bus.md) §3). |

## 8. Architecture

loco `App` (`src/app.rs`) registers the organizations controller. One
`organizations` table stores `pid` + denormalised `name` + the full
`Organization` JSONB `data`. Matching calls `organization-matcher`
directly on the deserialised payloads — no adapter.

**Cross-service linking — target only (v1).** Per
[`agents/share/cross-service-linking.md`](../../../agents/share/cross-service-linking.md),
organization is a v1 link **target**: inbound edges point at it
(`person → organization` `works_at`/`member_of`; `worker → organization`
`employed_by`). It therefore has **no `entity_links` write-side table and
no `/links` surface**. It participates by (a) emitting its existing
`created`/`deleted`/`merged` events, which feed the aggregator's
`entity_presence` verification oracle and merge-repointing, and (b) being
addressable as an `EntityRef` URN `organization:<pid>` so inbound edges
resolve. The inverse edges (`has_member`, `employs`) are materialised in
the aggregator read-model, **not** stored here. Origination from the org
side is a roadmap item (umbrella spec §15).

**Bulk import / export (roadmap, §13).** The family-wide contract — async
`bg_pg` jobs, the `bulk_jobs` table, the five `/api/organizations/{import,export,bulk-jobs}`
endpoints, JSONL/CSV/Parquet, idempotent upsert-by-key, the per-row error
report, and the export privacy/audit posture — is fixed in
[`agents/share/bulk-import-export.md`](../../../agents/share/bulk-import-export.md).
Organization declares only the differences (umbrella spec §8.7): **stable
key** = a deterministic globally-unique scheme-scoped identifier the matcher
short-circuits on (`Lei`/`Duns`/`Iso6523`/`Gln`/`Wikidata`/`Ror`/`Isni`/`Vat`,
matched as `(scheme, value)`) or the record `pid`; **CSV** flattens `address.*`
to dotted columns and JSON-encodes the `identifiers` / `alternate_names` /
`same_as` / `keywords` / `tags` / `relationships` arrays, with JSONL the
lossless reference; **export sensitivity** is low–medium (light default
masking protecting `telephone` / `email` + sole-trader records, parity with
the §13 masking task), every export audited.

## 9. API surface

See §6. Responses are raw loco JSON. `404` for unknown `pid`; `422
Unprocessable Entity` for validation failures (blank `name` on create
or replace — family convention); `400` for malformed requests (blank
search `q`, invalid audit pid).

**Auth.** The credential is a short-lived **PASETO v4.public** token
(Ed25519, riding in `Authorization: Bearer v4.public.…`), verified
offline against the auth-service's published Ed25519 key — see
[`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)
(source of truth; supersedes the prior RS256-JWT + JWKS model).
`GET /api/organizations/whoami` always requires a valid bearer
token (the `AuthUser` extractor; `401` otherwise); other handlers take
`MaybeAuthUser` to stamp the audit/merge `actor` when a token is present.
When `ORGANIZATION_REQUIRE_AUTH` is on (see §7), an `axum` middleware
layer (`App::after_routes` → `auth::enforce`) requires a valid bearer
token on **every** route except the public health/ping, OpenAPI/Swagger,
and `/metrics.prom` paths, returning `401` otherwise. The flag is read
per request and is **off by default**, so default behaviour is unchanged.

**Authorization (ABAC).** Inside the same guard — so only when
`ORGANIZATION_REQUIRE_AUTH` is on — a verified token is authorized by
**attribute-based access control** per
[`agents/share/authorization-attributes.md`](../../../agents/share/authorization-attributes.md):
the request's action is derived from the HTTP method plus this crate's
destructive named POSTs (`auth::DESTRUCTIVE_POST_SUFFIXES` — `/merge`,
`/deduplicate`, `/import`; the latter two ahead of the dedup-scan and
bulk-import features), and the shared engine in
`authentication-verifier` 0.3 evaluates the policy over the token's
`attrs` claim, first-match-wins. Configure with `ORGANIZATION_ABAC_POLICY`
(inline JSON) or `ORGANIZATION_ABAC_POLICY_FILE` (path); unset or
unparsable ⇒ warn-log + the built-in default policy (any authenticated
subject reads; `access=write` writes; `access=admin` adds DELETE/merge;
`svc=true` does everything). `401` = missing/bad credential; `403` =
valid credential, policy denied (the body names the deciding rule). This
supersedes the earlier per-crate roles/RBAC sketch.

**Observability.** `GET /metrics.prom` (root path, public) serves the
process-wide Prometheus registry (`src/metrics.rs`) in text-exposition
format. The metric set: `organization_created_total`,
`organization_updated_total`, `organization_deleted_total`,
`organization_merged_total` (counters incremented one per success path
in the CRUD/merge handlers), plus a labelled `http_requests_total`
(`path`/`status`) reserved for a future request middleware. Configure
the scraper with `metrics_path: /metrics.prom`.

## 10. Persistence

PostgreSQL via SeaORM + `sea-orm-migration`. Migration
`m20220101_000001_organizations`. `auto_migrate` on in development.

## 11. Testing strategy

DB-free tests: `tests/matching.rs` (matcher embedding + JSON
round-trip) and unit tests in `src/` (validation → `422` pin, OpenAPI
shape incl. the `/metrics.prom` path, the Prometheus registry render +
counter increment in `metrics::tests`, streaming, and `auth::tests` —
`bearer_claims` plus the pure
`enforce`/`parse_bool` decision: off+no-token ⇒ ok, on+public ⇒ ok,
on+protected without/expired/tampered ⇒ `401`, on+protected+valid ⇒
ok). Request-level tests (`tests/requests/organizations.rs`): boot the
real app via loco's `testing` harness and cover create round-trip,
blank-name `422` (create + update), unknown-pid `404`, search,
check-duplicates, merge, `whoami` `401`, the blanket-enforcement
gate (with `ORGANIZATION_REQUIRE_AUTH=1` set in-test, un-authed `GET
/api/organizations` ⇒ `401` while `GET /api-docs/openapi.json` ⇒
`200`; `#[serial]` for env-var ordering), the audit endpoints
(`/audit/recent` + `/{pid}/audit` record CRUD actions; invalid pid ⇒
`400`), and the plain-CRUD `created`/`updated`/`deleted` events on
`/events/recent`. These require Postgres
(`config/test.yaml`) and are `#[ignore]`d so the default `cargo test`
stays green — run with `cargo test -- --ignored`.

## 12. Compliance

Organization data is largely public, but contact fields may be
personal data — honour GDPR when the privacy layer lands (§13).

## 13. Tasks (live work queue)

- [x] **SEC-M5 (security): check-digit / format validation of deterministic
  identifiers.** `validation::problems` now validates LEI (ISO 17442 + ISO
  7064 MOD 97-10), GLN (13 digits + GS1 mod-10), DUNS (9 digits), and VAT
  (country-prefix format) before store, since they drive the matcher's
  deterministic short-circuit — a malformed one could produce a false
  deterministic match. A bad value is a field-scoped `422`; non-deterministic
  schemes are unconstrained. Pure check-digit helpers unit-tested. (Repo
  tasks.md Phase 5 SEC-M5.)

- [x] **SEC-M1 (security): input-size caps on the `Organization` payload.**
  New `src/validation.rs` (`problems`) bounds every scalar text field
  (`MAX_TEXT_LEN = 1024`, incl. nested `address.*`), array cardinality
  (`MAX_ARRAY_LEN = 256`), and per-entry length (`MAX_ITEM_LEN = 512`),
  keeping the blank-`name` / non-blank-`identifiers[i].value` rules — all
  collected into one `422` before the record is stored or matched, closing
  the O(n·m) matcher `DoS`. Controller `validate` delegates to it. Unit
  tested. (Repo tasks.md Phase 5 SEC-M1.)

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
- [x] **Durable event bus — Phase 2 (transactional outbox).** This is the
  family **reference** implementation
  ([`agents/share/event-bus.md`](../../../agents/share/event-bus.md) §3–§8).
  New `event_outbox` table (`migration/…_000004_event_outbox`: `BIGSERIAL
  id`, `event_id UUID UNIQUE`, `entity`, `entity_pid`, `kind`,
  `occurred_at`, `actor`, `schema_version`, `payload JSONB`,
  `published_at`, partial index on unpublished rows); SeaORM entity
  `models/_entities/event_outbox.rs`; `models/event_outbox.rs` with the
  **pure** DB-free `OutboxInsert::from_envelope` mapping (unit-tested),
  `insert_on(&impl ConnectionTrait)`, `recent(db, limit) → Vec<EventView>`,
  and the relay poll/ack (`unpublished`/`mark_published`, unused until
  Phase 3). New `EventTransport`/`transport()` selector +
  `OutboxPublisher` in `src/streaming.rs`, plus transport-aware
  `create_and_emit`/`update_and_emit`/`delete_and_emit`/`merge_and_emit`
  used by **both** the native and FHIR controllers. The model write
  helpers (`create`/`update_data`/`soft_delete`) are now generic over
  `sea_orm::ConnectionTrait`, so the `outbox` path runs the entity write
  **and** the `event_outbox` insert on one `db.begin()` transaction (crash
  can't persist one without the other); `memory` keeps the ring buffer,
  no tx. Gated by `ORGANIZATION_EVENT_TRANSPORT` (default `memory` ⇒
  behaviour and existing tests unchanged). Tests: DB-free envelope→row
  mapping (create/update/delete/merge fields, non-UUID pid rejected),
  transport-string parse, `EventView` projection frozen; DB-gated
  (`tests/requests/event_outbox.rs`, `#[ignore]`) atomicity — one tx
  writes org + exactly one outbox row, a rollback drops both.
- [x] **Durable event bus — Phase 3 (relay + retention).** `src/relay.rs`:
  the `EventSink` trait (the bus seam), a working no-broker **`LoggingSink`**
  default, `drain_once` (`unpublished` → `sink.send` → `mark_published`,
  at-least-once, per-pid order preserved on a send failure), and
  `purge_published` (retention). A background loop (`relay::spawn`, started
  in `App::after_routes`) ticks every `ORGANIZATION_EVENT_RELAY_INTERVAL_SECS`
  and purges every N ticks — **gated by `ORGANIZATION_EVENT_TRANSPORT=outbox`
  AND `ORGANIZATION_EVENT_RELAY`**, so it is a no-op by default. Tests:
  DB-free `LoggingSink`/capturing-sink send + config defaults; the drain/ack
  seams (`unpublished`/`mark_published`) are DB-gated-tested via the outbox
  suite. **Broker-gated follow-up:** a real **`FluvioSink`** (`impl EventSink`
  behind a `fluvio` cargo feature + `ORGANIZATION_FLUVIO_ENDPOINT`/
  `ORGANIZATION_EVENT_TOPIC`) — the trait is the seam, so the drain loop is
  unchanged when it lands ([`agents/share/event-bus.md`](../../../agents/share/event-bus.md) §5, §8).
- [x] Name search (Postgres `ILIKE`) + OpenAPI/Swagger.
- [x] Prometheus metrics — `GET /metrics.prom` (root, public) serves a
  process-wide `prometheus::Registry` (`src/metrics.rs`, `OnceLock`)
  in text-exposition format; CRUD/merge handlers increment
  `organization_{created,updated,deleted,merged}_total`; a labelled
  `http_requests_total` is declared for a future request middleware.
  Brings parity with the older Axum services. DB-free render test +
  OpenAPI path test; `/metrics.prom` added to `auth::is_public_path`.
- [ ] Tantivy full-text search + fuzzy/blocking (replacing the `ILIKE`
      search).
- [ ] Per-field masking + GDPR export endpoint.
- [x] Record merge — `POST /merge` folds a duplicate into a survivor
  (union fields, former-name alias, soft-delete, `merge_records` history
  + snapshot, `Merged` event); pure `src/merge.rs`; `/merges/recent`.
- [ ] Richer validation (identifier formats, URL, country codes).
- [ ] Cross-service link **target** readiness — organization is a v1
  link target only (§8;
  [`agents/share/cross-service-linking.md`](../../../agents/share/cross-service-linking.md)),
  so no `entity_links` table. Confirm the `created`/`deleted`/`merged`
  events carry the fields the aggregator's presence oracle + merge-repoint
  need (`pid`; `merged_from` on merge), and confirm the matcher adapter
  never sees cross-service links (only within-entity `relationships[]`
  reach `MatchingEngine`). Mirrors umbrella spec §13 T-13.
- [x] Request-level integration tests (Postgres; `#[ignore]`-gated).
- [x] Offline token verification — `src/auth.rs` embeds
  `authentication-verifier` behind a process-wide `Verifier`
  (env-configured keys/issuer/audience); `AuthUser`/`MaybeAuthUser`
  extractors; `/whoami` protected; audit + merge `actor` from the token.
  (Originally shipped against the prior RS256-JWT + JWKS model.)
  - [x] **Switch to PASETO v4.public** per
    [`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md).
    **Done:** `src/auth.rs` verifies PASETO v4.public tokens
    against the auth-service's published Ed25519 key (`authentication-verifier`
    0.2 `from_paseto_keys_*`); same `Claims` shape (`kid`/`iss`/`aud`/`exp`);
    env vars `ORGANIZATION_PASETO_KEYS` / `ORGANIZATION_TOKEN_ISSUER` /
    `ORGANIZATION_TOKEN_AUDIENCE`. Supersedes the RS256-JWT model.
  - [x] Blanket `/api/*` enforcement — `auth::enforce` (pure, unit-tested)
    wired as an `axum::middleware::from_fn` layer in `App::after_routes`,
    gated by `ORGANIZATION_REQUIRE_AUTH` (lenient bool, **default off**).
    Public paths (`/_health`, `/_ping`, `/api-docs/openapi.json`,
    `/swagger-ui*`) stay open; everything else needs a valid bearer token
    when the flag is on. Off by default keeps current behaviour and the
    existing DB-gated tests green. Family contract:
    [`agents/share/jwt-enforcement.md`](../../../agents/share/jwt-enforcement.md)
    (credential now PASETO per `authentication-sessions.md`; `enforce()`
    shape unchanged).
  - [x] paseto-keys-over-HTTP fetch at boot (vs env injection) — fetch +
    cache the auth-service `/.well-known/paseto-keys` at startup.
    **Done 2026-07-04:** new `ORGANIZATION_PASETO_KEYS_URL` env var (§7);
    when set, `auth::init_from_env` (called from `App::after_routes`
    before serving) fetches the key set once via
    `Verifier::from_paseto_keys_url` (`authentication-verifier` `fetch`
    feature) and seeds the process-wide verifier — fetched set wins
    (`tracing::info!`); on fetch failure it warns and falls back to the
    `ORGANIZATION_PASETO_KEYS` env path, so the service always boots.
    Unset/blank URL ⇒ prior behaviour exactly. Fetch-once only; a
    periodic refresh loop on key rotation stays future work (§16).
    Tests: local ephemeral-port HTTP listener serving the test key set
    (fetched verifier accepts a token signed by that key) + fast-failing
    URL fallback (no panic) + no-URL env path.
- [ ] Bulk import / export — adopt the family contract
  ([`agents/share/bulk-import-export.md`](../../../agents/share/bulk-import-export.md)):
  `bulk_jobs` migration, the five `/api/organizations/{import,export,bulk-jobs}`
  endpoints, a `bg_pg` worker, JSONL/CSV/Parquet codecs, a per-row pipeline
  reusing the single-create validators + organization-matcher + review queue
  (`provenance = import`; upsert by deterministic scheme-scoped identifier or
  `pid`), the per-row error report, and export masking + audit (light default
  masking, gated `include_soft_deleted`). Organization-specific declarations
  (stable key, CSV column set, sensitivity) are umbrella spec §8.7; mirrors
  umbrella spec §13 T-14. Tests: idempotent re-import, error report,
  dedupe-to-review, masked vs full export, export audit.
- [x] **FHIR R5 API** (`Organization`) — **reference implementation** for
  the family contract (**Done**: `src/fhir/{mod,resources,search}.rs` +
  mounted `src/controllers/fhir.rs`, wired in `app.rs`; read/create/update/
  delete/search at `/fhir/Organization{,/{id}}` + `GET /fhir/metadata`
  `CapabilityStatement`; `OperationOutcome` errors, searchset `Bundle`,
  `application/fhir+json`; 9 DB-free unit tests; clippy-clean)
  ([`agents/share/fhir.md`](../../../agents/share/fhir.md)). Map the stored
  `organization_matcher::Organization` DTO to a FHIR **`Organization`**
  resource (`high` fidelity, §3): `name`/`alias` → `name`/`alias`,
  identifiers (LEI/DUNS/…) → `identifier` (token `system|value`),
  addresses → `address`, telecom → `telecom`, `part_of` → `partOf`
  reference; `active`. New `src/fhir/` module (resource structs,
  `to_fhir_organization`/`from_fhir_organization`, `FhirOperationOutcome`,
  searchset `Bundle`, search-param parsing) + a mounted
  `src/controllers/fhir.rs` (`routes()` added in `app.rs`): read/create/
  update/delete/search at `/fhir/Organization{,/{id}}` + `GET
  /fhir/metadata` `CapabilityStatement`. Reuses the native model helpers,
  validators, event/audit path, and the blanket auth+ABAC guard (§8 —
  `/fhir/*` guarded, action from HTTP method). Supported search params:
  `_id`, `_lastUpdated`, `_count`, `identifier`, `name`, `address`,
  `address-city`, `address-postalcode`. Tests: DTO↔resource round-trip,
  each interaction, search→Bundle, `OperationOutcome` on 404/400/422,
  `CapabilityStatement` matches mounted routes. First; copied by the other
  in-scope services.

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

## 14. Implementation status

Done: loco boot; organizations table + migration; CRUD (blank name →
`422`, unknown pid → `404`); `/match` and `/check-duplicates` embedding
organization-matcher; audit log; in-memory event streaming (Phase 1:
canonical `Envelope` + `EventPublisher` seam, `EventView` projection
frozen for `/events/recent`); name search (`ILIKE`); record merge
(`/merge` + `merge_records` history); offline
**PASETO v4.public** verification (`AuthUser`/`MaybeAuthUser`, `/whoami`,
audit + merge `actor` from the token) per
[`authentication-sessions.md`](../../../agents/share/authentication-sessions.md)
— originally shipped against RS256-JWT/JWKS, since switched (§13) —
including the boot-time paseto-keys-over-HTTP fetch
(`ORGANIZATION_PASETO_KEYS_URL`, fetch-once, env fallback; §7, §13);
OpenAPI 3 + Swagger UI; Prometheus
metrics (`/metrics.prom`, root + public, CRUD/merge counters); DB-free
tests;
request-level test suite (Postgres, `#[ignore]`-gated); loco scaffolding
leftovers removed (no workers/tasks/data stubs); green build + clippy.

## 15. Roadmap

v0.1 (here): CRUD + matching MVP. v0.2: search + audit + streaming.
v0.3: privacy + merge + OpenAPI + auth middleware (PASETO v4.public per
[`authentication-sessions.md`](../../../agents/share/authentication-sessions.md),
superseding the RS256-JWT model).

## 16. Open questions

- Should identifiers/address be normalised into their own tables (vs the
  single JSONB payload) once search lands?
- Real-time duplicate check on create (409) vs the explicit endpoint?
- Periodic re-fetch of the PASETO key set (key rotation) — the boot
  fetch (§7 `ORGANIZATION_PASETO_KEYS_URL`) runs once; is a refresh
  loop (or refetch-on-`UnknownKid`) needed before rotation goes live?

## 17. References

- schema.org/Organization; loco.rs; the organization-matcher spec.

## 18. Change control

Update this spec with any behavioural change; bump `CHANGELOG.md`.
