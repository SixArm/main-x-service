# Changelog

All notable changes to this crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md), [README.md](./README.md), [AGENTS.md](./AGENTS.md).

## [Unreleased]
### Added — BLK-5 async bulk import/export (2026-08-03)

- **`POST`/`GET /api/organizations/import[/{id}]` and
  `export[/{id}]`, plus `GET /api/organizations/bulk-jobs`** — async,
  loco-worker-driven bulk import and export
  (`agents/share/bulk-import-export.md`), scoped to what BLK-1/BLK-2
  need: **JSONL + CSV only** (no Parquet) and a **local-filesystem-only**
  artifact store (no S3 backend; the trait is async so a future S3
  backend needs no signature change).
- New `src/bulk/` module: the wire "bulk row" shape (an organization's
  own fields plus an optional top-level `pid`, since
  `organization_matcher::Organization` carries no id of its own), the
  JSONL/CSV codecs, the stable-key resolver (LEI → DUNS → explicit
  `pid`; a keyless row runs the same search-blocking + matcher
  duplicate detection `POST /check-duplicates` uses and is queued in
  the review queue with `provenance = "import"`), the per-row error
  report, the pipeline (reuses `streaming::create_and_emit`/
  `update_and_emit` for every written row — a bulk-imported
  organization gets the same event/audit/search-index side effects as
  one created interactively), the local artifact store, the
  `BulkJobWorker`, and the REST handlers.
- New `bulk_jobs` table (`m20260803_000002_bulk_jobs`) and a
  `review_queue.provenance` column
  (`m20260803_000001_review_queue_provenance`, mirroring person's
  `m20260802_000001`).
- Export defaults to the masked view (`crate::privacy::mask_organization`);
  the privileged `full` profile requires elevated authorisation.
  `include_soft_deleted=true` is `400` (not yet supported). Every
  export is audited, and the audit write gates delivery (SEC-B8): a
  failed audit write fails the job before the artifact is stored.
- **Known limitation:** the per-row upsert is not wrapped in a SEC-B3
  stable-key advisory lock, unlike the family reference pattern — see
  spec §10.7 "Concurrency" for why (a lock held on a separate guard
  transaction deadlocked every import under this crate's own
  `config/test.yaml` `max_connections: 1`, since
  `streaming::create_and_emit`/`update_and_emit` are hard-coded to
  `&DatabaseConnection` rather than generic over `ConnectionTrait`).
  Two importers racing the identical stable key in the same instant can
  both create a row; closing this is a tracked follow-up.
- 8 new request-level tests (`tests/requests/bulk.rs`, Postgres-gated)
  plus DB-free unit tests throughout `src/bulk/`.

### Changed — loco-rs 1.0.1 (2026-08-02)

- **loco-rs 0.16 → 1.0.1**, the framework's first stable release: sea-orm
  1.1 → 2.0, sea-orm-migration → 2.0, sea-query → 1.0. Mechanical
  fallout: raw `Statement` queries in `models/review_queue.rs` move from
  `.execute`/`.query_one`/`.query_all` to the `_raw` variants (sea-orm 2.0
  splits typed `StatementBuilder` calls from raw-SQL ones); a
  `useless_conversion` in `models/event_outbox.rs` from a now-unneeded
  `.into()`.
- **loco's `ColType::PkAuto` now generates a 64-bit primary key**
  (`BIGINT`, was `SERIAL`). The `organizations`, `audit_logs`, and
  `merge_records` generated entities (and the compliance-report /
  test-fixture code that carries their row ids) move from `i32` to
  `i64` to match. `event_outbox` is unaffected — its migration writes
  raw SQL (`id SERIAL PRIMARY KEY`) rather than the loco schema DSL,
  specifically to control the exact table name, and that raw SQL was
  left as `SERIAL`.
- No behavioural change; verified with the full DB-gated suite (26
  tests) against a freshly migrated Postgres 18.

### Added — pagination on list and search (2026-08-01)

- **`GET /api/organizations` and `GET /api/organizations/search` take
  `?limit=` and `?offset=`**, and report `X-Total-Count` / `X-Limit` /
  `X-Offset` (the family convention, now written down in
  `agents/share/restful.md`). The body shape is unchanged — these
  endpoints return a bare array and every existing caller parses one, so
  the count goes in a header rather than in an envelope that would break
  them all for a number most do not use.
- **Defaults preserve the old behaviour**: no parameters ⇒ the first 100
  (list) or 50 (search), which is exactly what the hard caps returned.
  `limit` **clamps** to 500 rather than erroring — a caller asking for
  100 000 wants "as many as you'll give me" — while an `offset` past
  10 000 is a `400`, because that one is a cheap denial of service rather
  than an unusual request (SEC-G7).
- Search's total comes from Tantivy's `Count` collector, not the page
  length: a page cannot tell a caller how much there is, which is the
  whole point of the header. The count is the index's match count rather
  than the number of rows that resolved, so it does not wobble when a hit
  refers to a since-deleted row.
- Tests: DB-free pins on the clamp/default/bound rules, plus a DB-gated
  request test walking a window, checking the total exceeds the page,
  the clamp, and the `400`.

**Found while writing it:** `#[serde(flatten)]` on a query-parameter
struct silently breaks typed fields — a flattened struct deserializes
from a string-keyed map, so `limit=2` arrives as the string `"2"` and
fails as a `u64`, turning a valid request into a `400`. The page fields
are therefore declared inline on the search params rather than flattened.

### Added — key rotation and policy hot-reload without a restart (2026-08-01)

AU-2, the loco-style half of the rollout (case was the reference; the
five axum-style services landed the same day as AU-1).

- **The verifier and the ABAC policy are now reloadable holders**
  (`ReloadableVerifier` / `ReloadablePolicy`) that the blanket guard
  **and** the bearer extractors read per request. They were boot-only
  `OnceLock` snapshots, so a rotated key set or an edited policy could
  not have reached a running process at all.
- **`spawn_key_refresh`** re-fetches `ORGANIZATION_PASETO_KEYS_URL` every
  `ORGANIZATION_PASETO_KEYS_REFRESH_SECS` (default 3600; `0` disables; a no-op
  when the URL is unset). A failed fetch **keeps the current key set** —
  a transient auth-service outage must not lock every caller out.
- **`spawn_policy_watcher`** polls `ORGANIZATION_ABAC_POLICY_FILE`'s mtime every
  15 s and calls `reload_policy()`; a malformed edit falls back to the
  built-in default rather than leaving the service unprotected.
- **`tests/enforcement.rs`** — the activation proof, in its own binary
  because the auth `OnceLock`s are process-wide: public paths stay open,
  a protected read and write without a token are `401`, a malformed
  bearer is `401` (not a 500), a valid token with no attributes reads
  `200` and writes `403`, and `access=write` creates. The record-level
  `authorize_record` added with the privacy layer reads the same holder,
  so masking decisions follow a reloaded policy too.
- New environment variable: `ORGANIZATION_PASETO_KEYS_REFRESH_SECS`.

### Added — field masking + GDPR export (2026-08-01)

- **`src/privacy.rs`** — `mask_organization` redacts what is genuinely
  sensitive about an organization and nothing else: `telephone` and
  `email` (routinely a named individual's line or inbox) are masked to
  their tail, the address's `street_address` is dropped (for a sole
  trader the registered address is a home address, and there is no
  `is_sole_trader` flag to key on), and `TaxId` / `Vat` identifier
  values are masked. Public registry identifiers — LEI, DUNS, ROR,
  ISNI, Wikidata — the names, `url`, and `jurisdiction` are untouched:
  masking those would break the lookups a registry exists for.
- **`GET /api/organizations/{pid}/masked`** — the redacted view on
  demand.
- **`GET /api/organizations/{pid}/export`** — the GDPR right-of-access
  envelope (`entity`, `pid`, `exported_at`, `masked`, `record`, `note`).
  **Audited on every call**, masked or not: a disclosure of personal
  data is itself a recordable event.
- **The ABAC `mask` obligation is wired.** `src/auth.rs` gains
  `authorize_record` + `organization_resource_attrs`
  (`resource.jurisdiction`, `resource.has_fiscal_id`), and
  `GET /{pid}` honours a `mask`-obligation allow by returning the
  redacted record **from the same URL** — so a policy can grant a
  partial read without a second endpoint, and the caller cannot ask for
  the unredacted form. The export follows the same decision and reports
  `masked: true`, because an access request answered with redactions
  must not look complete. All of it is a no-op while
  `ORGANIZATION_REQUIRE_AUTH` is off.
- **No consent model, deliberately.** The shared contract's consent is a
  *data subject* granting a purpose. An organization is not one; the
  natural persons behind it are, and the person service owns their
  consent. A second, unauthoritative home for it would be worse than
  none.
- Tests: 10 DB-free pins (each redaction, the fields that must survive,
  char-safe masking, the export envelope) plus a dedicated
  `tests/masking.rs` binary — its own process because the auth
  `OnceLock`s are process-wide — proving end to end that the obligation
  redacts the ordinary `GET`, carries into the export, and audits both.
  Mutation-checked: dropping the obligation branch fails the suite.


### Added — Tantivy full-text search, fuzzy + phonetic, dedup blocking (2026-07-31)

- **`src/search/`** — a Tantivy index (`index.rs`: schema + lifecycle;
  `mod.rs`: the `SearchEngine` facade and a process-wide `OnceLock`
  engine). Indexed: `name`, `legal_name`, `alternate_names`, Soundex
  codes of every name token, identifier values, `keywords`, the
  flattened postal address, `url` (full-text) plus `jurisdiction` and
  `active` (exact). Only `pid` is stored — hits are resolved against
  Postgres, which stays the source of truth.
- **`GET /api/organizations/search`** is now full-text and ranked, with
  `fuzzy=true` (Levenshtein ≤ 2) and `phonetic=true` (Soundex). Blank
  `q` is still `400`; an unopenable index is `503` rather than an empty
  result, so a broken index cannot masquerade as "no matches". A query
  Tantivy's parser rejects falls back to an OR over its tokens.
- **`POST /api/organizations/check-duplicates` now blocks on the index**
  (fuzzy name + exact identifier + phonetic routes, ≤ 200 candidates)
  instead of scanning up to 1000 rows. This removes the scale cliff
  where record 1001 was unreachable however obvious a duplicate it was;
  in particular a record sharing only an LEI, under a completely
  different name, is now found (pinned by a request test).
- **Indexing is wired into `src/streaming.rs`**, the single seam both
  the native and the FHIR controllers write through: create/update
  replace the document in place, delete and the duplicate side of a
  merge remove it. It runs after the write is durable and is
  best-effort — a failed index write is logged at `ERROR` and never
  fails a request that already committed.
- **`cargo loco task search_reindex`** (`src/tasks/search.rs`) rebuilds
  the index from the database (paginated, clears first, skips and
  counts unreadable payloads), and an **empty index over a populated
  table is rebuilt automatically at boot** — so an upgrade or a lost
  index volume self-heals. `ORGANIZATION_SEARCH_BOOT_REINDEX=0` opts
  out.
- New environment variables: `ORGANIZATION_SEARCH_INDEX_PATH`
  (default `data/search-index`) and `ORGANIZATION_SEARCH_BOOT_REINDEX`
  (default on).
- Tests: 16 DB-free search unit pins and 6 DB-gated request tests
  (keyword hit, index follows update + delete, fuzzy/phonetic over the
  wire, identifier-only duplicate blocking, `search_reindex` rebuild,
  boot self-heal). The DB-gated suite is 22 tests and green against
  Postgres 18.

### Removed

- The Postgres `ILIKE '%q%'` name search (`Model::search`) and its
  `escape_like` wildcard guard (SEC-G4). This crate now issues no
  `LIKE` query at all, so leaving an unused escaper behind would only
  invite a future caller to assume it was still wired in. The sibling
  care-pathway / case services keep theirs — they still search with
  `ILIKE`.

### Added — batch dedup + stored review queue + decision endpoints (2026-07-19)

- `POST /api/organizations/deduplicate` — pairwise batch scan (up to the
  check-duplicates cap) that **persists** candidates in the new
  `review_queue` table (migration `m20260719_000001`; normalized-pair
  UNIQUE upsert — re-scans refresh scores, decided rows keep their
  decision, item ids stay stable) and reports the stored rows. Already
  destructive-classed under ABAC.
- `GET /api/organizations/review-queue[?status=&limit=]` — the stored
  queue, newest first (cap 500).
- `POST /api/organizations/review-queue/{id}/decision`
  (`{"status": "confirmed" | "rejected"}`) — first-writer-wins decision
  (`404`/`422` on unknown/already-decided); reviewer = verified bearer
  `sub`; writes a `review_decision` audit row.
- The Postgres-gated auth-gate request test now detects the process-wide
  `OnceLock` flag cache being poisoned by an earlier test and skips
  honestly (it previously failed the full `--ignored` suite run).

### Fixed

- 2026-07-18 — **Fresh-database `db migrate` failure.** The
  `…_000004_event_outbox` migration used the loco `create_table`
  helper, which pluralizes table names (`event_outbox` →
  `event_outboxes`); its own index DDL then failed and rolled back
  the entire fresh migrate (zero tables). Rewritten as explicit SQL
  creating exactly `event_outbox`; verified against a fresh
  Postgres 18 (all migrations apply, correct table names). Family-wide
  fix (case, care-pathway, organization, portfolio; patient-flow
  shipped with the explicit-SQL form).


### Security

- **SEC-M5: check-digit / format validation of deterministic identifiers.**
  The service stored any `identifiers[i].value` verbatim and validated only
  that it was non-blank — but LEI / DUNS / GLN / VAT drive the matcher's
  **deterministic short-circuit to `1.0`**, so a malformed value in one of
  those could be stored and produce a **false deterministic match**.
  `validation::problems` now validates the deterministic schemes before
  store (`identifier_problem`): **LEI** (ISO 17442 — 20 alphanumerics + ISO
  7064 MOD 97-10 check), **GLN** (13 digits + GS1 mod-10 check digit),
  **DUNS** (9 digits — no public check digit), and **VAT** (2-letter country
  prefix + 2–13 alphanumerics; per-country check digits deferred). A bad
  value is a field-scoped `422`. Non-deterministic schemes are unconstrained.
  Pure check-digit helpers unit-tested with hand-verifiable values.

- **SEC-M1: input-size caps on the `Organization` payload.** The service
  stores the matcher's `Organization` verbatim and scored it with only a
  blank-`name` check — a single multi-megabyte string field or a huge array
  could be used as a CPU/memory `DoS` against the matcher's O(n·m)
  Jaro-Winkler / Levenshtein / Jaccard scoring, amplified across the
  `check-duplicates` scan. A new `src/validation.rs` (`problems`, mirroring
  the case-service reference) now bounds every scalar text field
  (`MAX_TEXT_LEN = 1024` chars — incl. the nested `address.*` sub-fields),
  array cardinality (`MAX_ARRAY_LEN = 256`), and per-entry string length
  (`MAX_ITEM_LEN = 512`), and keeps the blank-`name` /
  non-blank-`identifiers[i].value` rules — all collected into one `422`
  *before* the record is stored or matched. The controller's `validate`
  delegates to it. Unit tests cover blank/oversized text, oversized array,
  oversized array item, nested address, multi-problem reporting, and a
  within-caps large record.

- **SEC-G6: trailing slash can no longer downgrade a destructive POST.**
  `derive_action` classified `/merge` / `/deduplicate` / `/import` via
  `path.ends_with`, so a trailing slash (`POST …/merge/`) fell through to
  `Write` — a non-admin `access=write` caller could reach a destructive op.
  The path is now `trim_end_matches('/')`-normalised first. Test extended.

- **SEC-B6: relay claims outbox rows with `FOR UPDATE SKIP LOCKED`.** The
  Phase-3 relay drained via a plain unlocked `SELECT … WHERE published_at IS
  NULL`, so with more than one instance every relay would **double-ship** the
  same rows. `drain_once` now runs in a transaction and `unpublished` claims
  rows with `FOR UPDATE SKIP LOCKED` (a second relay skips locked rows; the
  lock releases on commit). Delivery stays at-least-once (consumers dedupe on
  `event_id`).

### Changed — event bus: audit now joins the outbox transaction (2026-07-09)

- Under the `outbox` transport, the `audit_logs` write now rides the
  **same transaction** as the entity mutation and its `event_outbox` row
  (`agents/share/event-bus.md` §3 — the three "can never disagree"). It
  was previously a best-effort side channel written *after* the
  transaction committed, so a crash or audit failure could leave a
  committed change + event with no audit row. `AuditModel::record` is now
  generic over `ConnectionTrait`; the `create/update/delete/merge_and_emit`
  functions own the audit write (strict/in-txn under `outbox`, best-effort
  logged under `memory`), and both the native and FHIR controllers no
  longer audit separately. New DB-gated `tests/outbox_audit.rs` drives
  `create_and_emit` under `outbox` and asserts entity + event + audit all
  commit together.

### Added — authz: ABAC policy authorization inside the blanket guard (2026-07-05)

- ABAC authorization landed (supersedes the earlier per-crate
  roles/RBAC sketch; family contract:
  `agents/share/authorization-attributes.md`). When
  `ORGANIZATION_REQUIRE_AUTH` is on, a verified PASETO token is
  further checked by the shared policy engine in
  `authentication-verifier` 0.3: the request's action is derived from
  the HTTP method plus the crate's destructive named POSTs
  (`auth::DESTRUCTIVE_POST_SUFFIXES` — `/merge`, `/deduplicate`,
  `/import`), and the policy is evaluated over the token's new `attrs`
  claim, first-match-wins, defaulting to allow-read / deny-mutation.
- New env vars `ORGANIZATION_ABAC_POLICY` (inline JSON) and
  `ORGANIZATION_ABAC_POLICY_FILE` (path); unset or unparsable ⇒
  `tracing::warn!` + the built-in default policy (`svc=true` ⇒
  everything; `access=admin` ⇒ destructive+write; `access=write` ⇒
  write) — the service always boots.
- `auth::enforce` now takes the HTTP method and the policy and returns
  `403` (deciding-rule reason) for a valid token the policy denies;
  `401` remains missing/bad credential. DB-free unit tests pin the
  family §7 matrix. Flag off ⇒ behaviour-neutral.

### Added

- **Boot-time PASETO key-set fetch over HTTP.** New env var
  `ORGANIZATION_PASETO_KEYS_URL`: when set, the service fetches the
  auth-service's published Ed25519 key set once at boot
  (`Verifier::from_paseto_keys_url`, `authentication-verifier` `fetch`
  feature) from `App::after_routes` via the new `auth::init_from_env`,
  seeding the process-wide verifier before serving. The fetched key set
  wins over `ORGANIZATION_PASETO_KEYS` (`tracing::info!`); any fetch
  failure logs a warning and falls back to the env key set, so the
  service always boots. Unset/blank URL keeps the prior env-injection
  behaviour exactly. Fetch-once only — a periodic refresh loop on key
  rotation is tracked as a future spec item (spec §16). Tests: a local
  ephemeral-port HTTP listener serving the test key set (the fetch-built
  verifier accepts a token signed by that key), a fast-failing-URL
  fallback pin (no panic), and a no-URL env-path pin. (Spec §7 env
  table + §13 fetch follow-up.)

### Fixed

- **`cargo fmt` drift.** Reformatted `src/auth.rs` and
  `tests/requests/organizations.rs` so `cargo fmt --check` passes again
  (no behavioural change).

### Changed

- **Auth pivot — sessions + PASETO (spec-level; code follow-up pending).**
  The family is moving off RS256 JWT + JWKS access tokens to server-side
  cookie sessions plus short-lived **PASETO v4.public** tokens verified
  offline against the authentication-service's published **Ed25519** key;
  the `authentication-verifier` becomes a PASETO verifier and RS256/JWKS
  is decommissioned. Front-ends adopt a BFF + httpOnly cookie + CSRF (the
  browser holds no token). The `ORGANIZATION_REQUIRE_AUTH` flag and
  blanket-enforcement semantics are unchanged — only the verified
  credential changes. Human-facing docs (README/AGENTS/index) updated to
  describe the new model; runtime code follow-up is tracked in spec §13.
  Source of truth:
  [agents/share/authentication-sessions.md](../../agents/share/authentication-sessions.md).

### Added

- **Doc/test harmonization pass.** Request-level tests added for the
  audit endpoints (`/audit/recent` + `/{pid}/audit` record CRUD actions;
  invalid pid ⇒ `400`) and for the plain-CRUD `created`/`updated`/
  `deleted` events on `/events/recent` (frozen `EventView` projection).
  `index.md` worked-flow extended to cover search / merge / audit /
  events / whoami / metrics, with a worked merge example; `README.md`
  gained worked merge, `whoami`, and `/metrics.prom` examples and a
  corrected Status section. `AGENTS.md` deferred list corrected (blanket
  `/api/*` enforcement is implemented; only JWKS-over-HTTP fetch
  remains). Crate `spec/index.md` and `AGENTS.md` now cross-link the
  entity umbrella spec (`../spec/index.md`) where the `R-DUP`/`T-7`/
  `T-9`/`T-12` task IDs the source comments cite are defined. Umbrella
  spec §13 T-9 follow-up marked blanket-enforcement done; cap-boundary
  truncation test and request-level whoami-200 test recorded as open
  tasks.

- **Prometheus metrics — `GET /metrics.prom`.** A new process-wide
  `prometheus::Registry` (`src/metrics.rs`, behind a `OnceLock` reached
  via `Metrics::global()`, mirroring `auth::verifier`) is served at the
  application **root** path `/metrics.prom` in text-exposition format
  (`text/plain; version=0.0.4`) by a loco controller
  (`src/controllers/metrics.rs`, mounted at root like the docs). The
  metric set: `organization_created_total`, `organization_updated_total`,
  `organization_deleted_total`, `organization_merged_total` (plain
  counters, incremented one per success path in the CRUD/merge
  controller handlers) plus a labelled `http_requests_total`
  (`path`/`status`) declared for a future request middleware. The path
  is added to `auth::is_public_path`, so it stays public under blanket
  JWT enforcement (no bearer token needed to scrape). New DB-free tests:
  registry render + counter increment (`metrics::tests`), the
  `/metrics.prom` OpenAPI path (`openapi::tests`), and `/metrics.prom`
  in the `enforce` public-path matrix (`auth::tests`). Brings parity
  with the older Axum services, which already expose Prometheus metrics.
- **Durable event bus — Phase 1 (in-memory envelope + `EventPublisher`
  seam).** `src/streaming.rs` now carries the canonical versioned
  `Envelope` (`event_id` UUID dedup key, `schema_version` = 1,
  `entity` = `"organization"`, `kind`, `pid`, `seq`, `actor`, `name`),
  an `EventPublisher` trait, and an `InMemoryPublisher` ring buffer
  (process-wide `OnceLock`) implementing it — replacing the flat
  `OrgEvent` free-function buffer. `occurred_at`/`data` are deferred to
  the outbox stage (Phase 2; the in-memory envelope is kept minimal).
  CRUD/merge call
  sites now stamp the bearer `actor` via `publish_with_actor`
  (`publish` kept as a `None`-actor shim). Pure refactor: behaviour is
  identical and the `GET /api/organizations/events/recent` wire shape is
  **frozen** — it returns the flat `EventView { kind, pid, name, seq }`
  projection of the envelope, byte-identical to before (front-end safe).
  Phases 2–3 (transactional outbox + Fluvio relay) remain infra-gated
  roadmap per [`agents/share/event-bus.md`](../../agents/share/event-bus.md).
- **Blanket `/api/*` JWT enforcement (default-off).** A new
  `ORGANIZATION_REQUIRE_AUTH` env flag (lenient bool — `1`/`true`/`yes`/
  `on`) gates an `axum::middleware::from_fn` layer wired in
  `App::after_routes`. When on, every route except the public health/ping
  + OpenAPI/Swagger paths requires a valid bearer token (`401`
  otherwise); when off (the default) behaviour is unchanged. The decision
  is the pure, unit-tested `auth::enforce` (plus `auth::require_auth`,
  `auth::parse_bool`, `is_public_path`). New `auth::tests` cover the
  matrix; a `#[serial]`/`#[ignore]` request test pins un-authed `GET
  /api/organizations` ⇒ `401` with the public OpenAPI doc still `200`.
  Implements the family contract in `agents/share/jwt-enforcement.md`.
- **Request-level integration tests.** `tests/requests/organizations.rs`
  (loco testing harness + `serial_test`): create round-trip
  (snake_case wire), blank-name `422` on create + update, unknown-pid
  `404`, search (+ blank-`q` `400`), check-duplicates ranking.
  `#[ignore]`-gated so the default `cargo test` stays green without
  Postgres; run with `cargo test -- --ignored`.

### Changed

- **Validation failures now return `422 Unprocessable Entity`** (was
  `400`): blank `name` on create and on replace (`PUT`), per the
  family convention. A DB-free unit test pins the mapping; OpenAPI
  updated.
- **Unknown `pid` now returns `404`** on get/replace/delete (loco's
  default `ModelError::EntityNotFound` mapping produced a `500`,
  breaking the documented contract).
- Docs (`README.md`, `index.md`, `AGENTS.md`) now describe the wire
  format as snake_case (`legal_name`, `same_as`, `founding_date`, …)
  matching the actual DTO serialization — entity spec OQ-1 resolved:
  no serde rename; snake_case is canonical.

### Removed

- loco scaffolding leftovers: `src/workers/downloader.rs` (TODO stub)
  and its worker registration, plus the empty `src/data/` and
  `src/tasks/` modules.

- **Audit log + event streaming.** `audit_logs` table records every
  create/update/delete (with a JSONB snapshot); a process-global
  in-memory event stream publishes Created/Updated/Deleted events.
  Endpoints: `GET /api/organizations/audit/recent`, `/{pid}/audit`,
  `/events/recent`.
- **Name search.** `GET /api/organizations/search?q=` — case-insensitive
  Postgres `ILIKE` on the denormalised name (Tantivy full-text remains a
  §13 follow-up).
- **OpenAPI + Swagger UI.** Hand-authored OpenAPI 3 spec at
  `/api-docs/openapi.json` (accurately typed `Organization` schema, since
  the matcher crate is `utoipa`-free) and a Swagger UI page at
  `/swagger-ui`.

- **Inaugural scaffold (v0.1.0).** loco.rs organization-identity
  registry (schema.org/Organization).
  - Generated via `loco new` (loco-rs 0.16) and stripped of the auth
    starter; auth is centralized in the authentication-service.
  - `organizations` table (`pid`, denormalised `name`, full
    `Organization` payload as JSONB `data`, `active`, soft-delete) +
    `sea-orm-migration` migrator.
  - CRUD controller: create / list / get / update / soft-delete, plus
    `POST /match` (rank a `{query, candidates}` set) and
    `POST /check-duplicates` (match a query against stored records).
  - **Embeds `organization-matcher` directly**: the API DTO *is*
    `organization_matcher::Organization`, stored verbatim and matched
    with the canonical engine — no separate model or adapter.
  - DB-free tests (`tests/matching.rs`): matcher embedding + JSON
    storage round-trip. Green `cargo build`, clippy clean.

### Notes

- The inaugural v0.1.0 scope was CRUD + matching; the entries above
  (in this Unreleased section) extend it with name search (`ILIKE`),
  audit + event streaming, record merge, OpenAPI/Swagger, Prometheus
  metrics, JWT verification (+ default-off blanket enforcement), and
  request-level tests. Still deferred (spec §13): Tantivy full-text
  search, per-field privacy/GDPR export, JWKS-over-HTTP fetch at boot,
  and richer validation.
