# Changelog

All notable changes to this crate are documented here.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/);
versioning: [SemVer](https://semver.org/spec/v2.0.0.html). See also:
[`index.md`](./index.md), [`spec.md`](./spec/index.md), [`README.md`](./README.md).

## [Unreleased]
### Fixed — cross-service link endpoints now use the uniform response envelope (2026-08-03)

`POST`/`GET`/`DELETE /api/workers/{pid}/links` previously returned bare
JSON bodies while every other worker REST endpoint wraps in
`{success,data,error}` (`ApiResponse<T>`) — a front-end client that
unwraps `.data` would have silently read these as `undefined`. Fixed;
the bulk aggregator endpoint (`GET /api/workers/links`) is unchanged
(still bare, for the link-graph aggregator's HTTP client). New DB-gated
regression test pins the wrapped shape end-to-end.

### Added — durable event bus Phase 3, `FluvioSink` (BUS-3, 2026-08-03)

Ported from the case-service BUS-1 reference implementation.

- **`FluvioSink`** (`src/relay.rs`), a real-broker `impl EventSink` behind
  a new, off-by-default `fluvio` Cargo feature — a default build's
  dependency tree and behaviour are unchanged. One producer per topic,
  held for the sink's lifetime, partitioned by record `pid`.
- **Sink selection in `relay::spawn`**: `WORKER_FLUVIO_ENDPOINT` unset ⇒
  `LoggingSink` (unchanged default); set ⇒ `FluvioSink` against
  `WORKER_EVENT_TOPIC` (default `mxi.worker.events`); set **without** the
  `fluvio` feature compiled in ⇒ the relay refuses to start at all
  (logged at `error`) rather than silently falling back to `LoggingSink`
  and marking rows published without reaching a real broker. The initial
  connection retries indefinitely rather than falling back.
- **`compose.fluvio.yaml` + `Dockerfile.fluvio-cli`**: a local Fluvio
  SC+SPU broker for opt-in manual testing only; not wired into any
  automated CI stage.
- **`tests/fluvio_relay.rs`**: a `#![cfg(feature = "fluvio")]`-gated,
  `#[ignore]`d live-broker round-trip test. Verified today by compiling
  under `--features fluvio` (no automated run in this repo stands up a
  live broker). Connects directly via `DATABASE_URL` rather than through
  `loco_rs::testing::prelude::request` — this crate's dev-dependencies do
  not enable loco's `testing` feature (a structural difference from
  case), so the test follows this crate's own existing DB-gated
  `src/db/repositories.rs::tests` pattern instead.
- SOUP register (`compliance/soup.tsv`) gained a `fluvio` row.
- No behavioural change to a default build: `cargo build --lib`,
  `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, and
  `cargo test --lib` are unaffected; the same checks pass under
  `--features fluvio` too, which is the actual proof the `fluvio` 0.50
  API is used correctly.

### Changed — loco-rs 1.0.1 (2026-08-02)

- **loco-rs 0.16.4 → 1.0.1**: sea-orm 1.1 → 2.0, sea-orm-migration →
  2.0, sea-query → 1.0. Feature renames applied: `auth_jwt` → `auth`,
  `bg_pg` → `worker`.
- **21 raw `Statement` call sites** across `src/db/audit.rs`,
  `src/compliance/erasure.rs`, `src/db/review_queue.rs`,
  `src/api/rest/handlers.rs`, and three test files move to the `_raw`
  variants.
- **`with-bigdecimal`** added to the `sea-orm` feature list — the same
  `BigDecimal` match-score columns as person-service, same fix.
- **`DatabaseConnection::Disconnected`**, removed in sea-orm 2.0, was
  `tests/common/mod.rs`'s stand-in for "a connection that errors if
  touched" in the no-DB test router. Replaced with an empty
  `MockDatabase`, added as a `mock`-feature dev-dependency.
- A `useless_conversion` in `src/db/outbox.rs` from a now-redundant
  `.into()`.
- No pre-existing `EntityTrait`-import gap here (unlike person-service)
  — this crate's `db/models.rs` submodules already glob-import the
  prelude per-module.
- No behavioural change; verified with the full DB-gated suite (33
  tests, unchanged count) against a freshly migrated Postgres 18.

### Added — key rotation and policy hot-reload without a restart (2026-08-01)

AU-1, following the person service (the axum-style reference).

- **One reloadable verifier.** The PASETO verifier left `AppState` for a
  process-wide `ReloadableVerifier` that the blanket guard **and** the
  `AuthUser` / `MaybeAuthUser` extractors read per request. This crate held it in **two** places — the state *and* a copy captured by the enforcement layer — so a rotation could have updated one and not the other.
- **`spawn_key_refresh`** re-fetches `WORKER_PASETO_KEYS_URL` every
  `WORKER_PASETO_KEYS_REFRESH_SECS` (default 3600; `0` disables; a no-op
  when the URL is unset), so a key rotation needs no restart. A failed
  fetch **keeps the current key set** — a transient auth-service outage
  must not lock every caller out.
- **`policy()` is a `ReloadablePolicy`**, with `reload_policy()` and
  **`spawn_policy_watcher`** polling `WORKER_ABAC_POLICY_FILE`'s mtime
  every 15 s. A malformed edit falls back to the built-in default rather
  than leaving the service unprotected.
- **`tests/enforcement.rs`** — the activation proof, in its own binary
  because the auth `OnceLock`s are process-wide. With
  `WORKER_REQUIRE_AUTH=1` over the real router: public paths stay open, a
  protected read and write without a token are `401`, a malformed bearer
  is `401` (not a 500), a valid token with no attributes reads `200` and
  writes `403` — the 401/403 split the ABAC contract requires — and
  `access=write` creates. Mutation-checked: forcing the flag off fails it.
- New environment variable: `WORKER_PASETO_KEYS_REFRESH_SECS`.


### Added — `Config::from_env` now loads the environment (2026-07-23)

- `Config::from_env` was a stub (`// TODO: Implement environment
  variable loading`) that returned `Config::default()` and ignored the
  process environment entirely — so `DATABASE_URL`, `SERVER_PORT`, and
  every other documented variable had **no effect**. It now layers the
  environment (and a best-effort `.env`) over the defaults.
- Variables: `DATABASE_URL`, `DATABASE_MAX_CONNECTIONS`,
  `DATABASE_MIN_CONNECTIONS`, `SERVER_HOST`, `SERVER_PORT`,
  `GRPC_PORT`, `SEARCH_INDEX_PATH`, `SEARCH_CACHE_SIZE_MB`,
  `MATCHING_THRESHOLD`, `OTLP_SERVICE_NAME`, `OTLP_ENDPOINT`,
  `RUST_LOG`, `STREAMING_BROKER_URL`, `STREAMING_TOPIC`.
- A blank or whitespace-only value counts as **unset** (an empty
  `SERVER_HOST` must not bind the server to nothing). A malformed typed
  value is **refused** with `Error::Config` naming the variable and its
  raw value, rather than silently falling back to a default the
  operator did not ask for.
- Pinned by five unit tests against a pure `Config::from_source` seam
  (defaults, every variable, blank-as-unset, malformed-by-name,
  whitespace tolerance) — no process-environment mutation, so they are
  parallel-safe and need no `unsafe` (`std::env::set_var` is `unsafe`
  in the 2024 edition, which this crate forbids).

### Fixed — `workers.gender` persisted in the wrong case (2026-07-23)

- `src/db/repositories.rs` wrote the bare `Debug` form of `Gender`
  (`"Male"`, `"Unknown"`) at all three write sites, but the `workers`
  table's CHECK constraint admits only
  `'male' | 'female' | 'other' | 'unknown'`. Against a constrained
  schema **every worker create and update failed** with
  `violates check constraint "workers_gender_check"`. Now lowercased at
  all three writers, matching what the search index and the FHIR
  surface already did (and what the sibling person-service already
  fixed).
- The read parser lowercases before matching, so rows written by the
  old path on an unconstrained deployment still round-trip rather than
  silently reading back as `Unknown`.
- Pinned DB-free by
  `db::repositories::tests::gender_is_persisted_as_a_constraint_legal_token`
  (every variant must persist as a constraint-legal *and* serde-canonical
  token). Unblocks the DB-gated outbox tests, which were red for this
  reason.
- **Data migration** `m20260723_000002_normalize_worker_gender_case`
  lowercases any legacy capitalized rows. Idempotent, and a no-op on a
  correctly-constrained schema (where such rows could never have been
  written); it exists for deployments whose `workers` table lacks the
  constraint. Values still outside the vocabulary after lowercasing are
  deliberately left untouched rather than silently rewritten — the
  `up.sql` carries the query to find them. `down` is a documented
  no-op. Pinned by the DB-gated `tests/gender_normalization_db.rs`,
  which runs the migration's real SQL and proves the repair by
  re-adding the constraint.

### Added — workforce assessments (2026-07-23)

Aptitude, personality, psychometric, and selection tests recorded
against a worker (spec §5.5 / §6.9 / §9.2 / §10.5, task T-10).

- **Domain model** `src/models/assessment.rs`: `AssessmentCategory`
  (aptitude / personality / psychometric / selection) × the 13
  `AssessmentScale` dimensions they measure — numerical and verbal
  reasoning, problem-solving, logical thinking; work style, team
  compatibility, introversion/extraversion; behavioural style, emotional
  intelligence, cognitive ability; job simulation, skills assessment,
  judgement test. `AssessmentCategory::permits` encodes the one
  deliberate overlap: a psychometric assessment also accepts aptitude
  and personality scales. Plus `ScoreBand` (the norm-referenced
  10/30/70/90 percentile split), the `AssessmentStatus` lifecycle
  machine, `is_valid_on`, `mean_percentile`, and `masked`.
- **`worker_assessments` table** (migration `m20260723_000001`) with the
  per-scale outcomes as a `results` JSONB array + soft delete, and
  `src/db/assessments.rs` (insert / worker-scoped list + find / update /
  soft-delete; a drifted stored token or malformed payload is a mapped
  error, never a panic).
- **Endpoints** under `/api/workers/{id}/assessments` (create, list with
  `?category=&status=&valid_on=`, fetch, update, withdraw) plus the
  derived `GET /api/workers/{id}/assessment-profile` — the current
  reading per scale in each category, the scales *not* assessed, and the
  selection-suitability mean, all from real scores only. Mounted on both
  router surfaces and in the OpenAPI document.
- **Validation** (`validate_assessment`): instrument required,
  scale-must-suit-category, one reading per scale, percentile ∈ [0, 100],
  `0 ≤ raw ≤ max`, expiry not before administration, completion
  requiring its date and results, plus SEC-M1 caps — reported as one
  complete `422`.
- **Sensitivity**: worker-level ABAC on every route; the `mask`
  obligation honoured on **every** read path (single, list, profile —
  bands survive, scores and narratives do not); audit rows on both reads
  and mutations.

### Added — stored review queue + decision endpoints (2026-07-19)

- `review_queue` table (migration `m20260719_000001_create_review_queue`):
  the batch-dedup scan persists its candidate pairs (normalized pair
  order, UNIQUE upsert — re-scans refresh scores, decided rows keep
  their decision, ids stay stable) and the scan response now reports
  the **stored** rows.
- `GET /api/workers/review-queue[?status=&limit=]` — list the stored
  queue (newest first, cap 500).
- `POST /api/workers/review-queue/{id}/decision`
  (`{"status": "confirmed" | "rejected"}`) — decide a `pending` item;
  first-writer-wins in SQL, `404`/`422` on unknown/already-decided.

### Added — matcher-partition guard test (cross-service-linking §7)

- A bridge test (`tests/duplicate_detection.rs::links_are_not_a_matcher_signal`)
  pins the partition rule: cross-service links are **never** a matcher
  signal. Cross-service `entity_links` are structurally excluded (their own
  table, never a field on the domain `Worker`, so they never reach
  `to_matcher_worker`), and the adapter also ignores the within-entity
  `Worker.links`. The test adds link data to a record and asserts its match
  score is unchanged — a regression guard so a future edit that routed any
  link into the matcher input fails here. Closes the spec §13 T-10 partition
  acceptance box.

### Added — cross-service `linked` / `unlinked` events (LNK-1)

- Worker now **emits** its cross-service link events on the durable event
  envelope (previously deferred), mirroring person. `EventKind` gained
  `Linked` / `Unlinked`, and `Envelope` gained an **additive**
  `data: Option<Value>` field (`skip_serializing_if = "Option::is_none"`,
  so the existing CRUD/merge wire shape is byte-identical) carrying the §4.2
  edge detail the link-graph aggregator deserialises into its `LinkedEvent`.
  - `POST /api/workers/{id}/links` emits `linked`; `DELETE …/{link_id}`
    emits `unlinked`. Under `WORKER_EVENT_TRANSPORT=outbox` the edge upsert
    (or soft-delete) and its event are enqueued in **one transaction** (the
    outbox guarantee); under `memory` (dev) the in-memory
    `WorkerEvent::Linked`/`Unlinked` is published as a lossy signal.
  - Tests: token, frozen-CRUD-shape, and `for_link` edge-detail (aggregator
    seam) unit tests, plus a DB-gated `linked_event_is_enqueued_to_the_outbox`.
    (Repo tasks.md LNK-1.)

### Added — cross-service `employed_by` affiliation edge (LNK-3)

- The worker link endpoints now originate the **`employed_by`** affiliation
  edge (worker → organization, temporal, carrying a `role` job title) in
  addition to the `same_identity` backbone. `validate_edge`'s permit set went
  from `same_identity`-only to `{same_identity, employed_by}`, relying on the
  shared `entity-ref` registry's `EdgeKind::permits` for the endpoint check —
  so `employed_by` requires an **organization** target and person-originated
  `works_at`/`member_of` + case-originated `subject_of` are still rejected on
  the worker side. No schema or endpoint change (the `entity_links` table +
  endpoints + bulk pull are unchanged and already generic over kind).
  Accept/reject matrix unit-tested (`accepts_employed_by_worker_to_org`,
  `rejects_employed_by_to_non_org`, `rejects_kinds_worker_does_not_originate`).
  (Repo tasks.md LNK-3.)

### Added — cross-service `same_identity` write-side (LNK-2)

- Worker now originates the **`same_identity`** cross-service edge
  (worker → person, the inverse direction of person's person → worker),
  mirroring the person reference
  ([cross-service-linking.md](../../../agents/share/cross-service-linking.md)
  §4.1/§4.2). New `entity_links` table (migration
  `2026071000000001_create_entity_links`, idempotent upsert on
  `(from_pid, kind, to_ref, valid_from) NULLS NOT DISTINCT`), persistence
  (`src/db/entity_links.rs`: upsert/list/find/bulk/soft-delete over
  `crate::db::models::entity_links`), and endpoints in
  `src/api/rest/links.rs` on **both** router surfaces:
  - `POST /api/workers/{id}/links` — validate + optimistic upsert (no
    cross-service call); `GET` lists a worker's active edges; `DELETE
    /api/workers/{id}/links/{link_id}` soft-deletes.
  - `GET /api/workers/links` — the aggregator's reconciliation pull
    (canonical §4.2 `EdgeDetail`, `{ "edges": [...] }`), gated as a
    **governed** read (`Action::Destructive`, SEC-G1) so a default
    read-only caller cannot dump every identity link.
  - `validate_edge` accepts **only** `same_identity` worker → person and
    rejects any other kind, a non-person target, or a malformed `to_ref`
    (pure, unit-tested matrix); depends on the shared `entity-ref` crate.
  - Record-level authz reuses `authorize_record` (no-op when
    `WORKER_REQUIRE_AUTH` is off); every mutation + bulk read writes a
    best-effort audit row (a new `AuditLogRepository::log_export` was added
    for the bulk surfacing). Cross-service `linked`/`unlinked` **event**
    emission is deferred (as on person); the bulk endpoint is the sync
    path. The link-graph aggregator adds worker to its reconcile list + a
    seam test in the same change.

### Security

- **SEC-M1: input-size caps on the `Worker` payload.** The validator
  enforced format/required rules but capped no field's *size*, so a single
  multi-megabyte text field or a huge array could be a CPU/memory `DoS`
  against the matcher's O(n·m) Jaro-Winkler / Levenshtein / Jaccard
  scoring, amplified across the `check-duplicates` / `deduplicate` scan.
  `validate_worker` now also bounds every scalar text field
  (`MAX_TEXT_LEN = 1024`), string-array cardinality + per-entry length
  (`MAX_ARRAY_LEN = 256` / `MAX_ITEM_LEN = 512`), and the inner text +
  cardinality of the nested collections (names + `additional_names`,
  `identifiers`, `addresses`, `telecom`, `documents`,
  `emergency_contacts` incl. their nested telecom/address, `photo`,
  `tax_id`, `marital_status`) — field-scoped `422`s *before*
  persist/match. Factored into `worker_size_caps` / `cap_*` helpers. Unit
  tested.

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
- **SEC-G4: escape `LIKE` wildcards in the repository name search.** The
  fallback `search` (`db/repositories.rs`) built its pattern as
  `format!("%{}%", query.to_lowercase())` with no escaping, so `%` matched
  every row and `_`×N forced expensive scans (wildcard injection / DoS;
  the value was already a bound parameter). It now escapes `\`/`%`/`_` via
  a new `escape_like` helper. Unit test `escape_like_neutralises_wildcards`.

### Changed — API versioning moved from URL to header (2026-07-07)

- REST URLs are now version-free (`/api/workers`, not `/api/v1/workers`).
  API versioning is selected with the `Accepts-version` request header
  (default `1.0`), per
  [`agents/share/api-versioning.md`](../../../agents/share/api-versioning.md).
  New `src/api/rest/version.rs` (`require_version_mw`) negotiates the
  version on `/api/*` (unsupported explicit version ⇒ `406`; resolved
  version echoed on the response), layered next to the auth guard on both
  the standalone Axum `create_router` and the loco `after_routes` surfaces.

### Added — authz: record-level resource attributes + obligations (2026-07-05)

- Record-level ABAC (verifier 0.3 → 0.6). Beyond the coarse blanket
  guard, `GET`/`PUT`/`DELETE /api/v1/workers/{id}` run a second, finer
  decision after loading the record: `auth::worker_resource_attrs`
  derives `resource.active` / `resource.deceased` / `resource.managing_org`
  and `auth::authorize_record` calls `Policy::evaluate_with_context`
  (gated on `WORKER_REQUIRE_AUTH`, a no-op when off). `PUT`/`DELETE`
  evaluate the **stored** record. Deployments can write e.g. "deny
  write on an inactive worker's record unless `access=admin`".
- Also supplies **environment attributes** (`env.hour` / `env.after_hours`,
  UTC) and honours the **`mask` obligation** on `GET` (returns
  `mask_worker`). New `auth::MaybeAuthUser` extractor + module-level
  `auth::policy()` / `require_auth()` accessors. DB-free tests for the
  resource-attribute mapping and the working-hours derivation.

### Added — authz: ABAC policy authorization inside the blanket guard

- ABAC authorization landed (spec §13 T-1b, the authorization sub-item
  — supersedes the earlier RBAC roles sketch of HR-admin /
  credentialing-officer / read-only / service; family contract:
  `agents/share/authorization-attributes.md`). When
  `WORKER_REQUIRE_AUTH` is on, a verified PASETO token is further
  checked by the shared policy engine in `authentication-verifier`
  0.3: the request's action is derived from the HTTP method plus the
  crate's destructive named POSTs (`auth::DESTRUCTIVE_POST_SUFFIXES`
  — `/merge`, `/deduplicate`, `/import`), and the policy is evaluated
  over the token's new `attrs` claim, first-match-wins, defaulting to
  allow-read / deny-mutation.
- New env vars `WORKER_ABAC_POLICY` (inline JSON) and
  `WORKER_ABAC_POLICY_FILE` (path), read once at router construction
  (restart to change); unset or unparsable ⇒ `tracing::warn!` + the
  built-in default policy (`svc=true` ⇒ everything; `access=admin` ⇒
  destructive+write; `access=write` ⇒ write) — the service always
  boots.
- `auth::enforce` now takes the HTTP method and the policy and
  returns `403` (with the deciding-rule reason) for a valid token the
  policy denies; `401` remains missing/bad credential.
- DB-free unit tests pin the family §7 matrix: action derivation,
  empty-`attrs` read-only default, `access=write` / `access=admin` /
  `svc=true` tiers, deny-beats-later-allow, 401-vs-403, bad-policy
  fallback.
- Flag off ⇒ behaviour-neutral: no authn and no authz, exactly as
  before.

### Added — boot-time PASETO key-set fetch (`WORKER_PASETO_KEYS_URL`; spec §13 T-1b fetch item)

- New `WORKER_PASETO_KEYS_URL` env var: when set, the auth-service
  published Ed25519 key set (`/.well-known/paseto-keys`) is fetched
  **once at boot** via `Verifier::from_paseto_keys_url` (the
  `authentication-verifier` `fetch` feature, now enabled in
  Cargo.toml). On success the fetched key set **wins** over
  `WORKER_PASETO_KEYS` (logged at `info`); on any fetch failure
  (network / HTTP / parse) a `warn` is logged and the verifier falls
  back to the `WORKER_PASETO_KEYS` env path — the service **always
  boots**; auth-service downtime never prevents startup. Unset/blank
  URL ⇒ prior behaviour exactly. One-shot fetch — no refresh loop
  (periodic refresh is a spec §15 roadmap note).
- Wired in `App::after_routes`: the verifier is resolved
  (`state::verifier_from_env_or_fetch`) and swapped into `AppState`
  via `with_verifier` **before** the enforcement middleware and the
  shared-store state are built, so both router surfaces (the
  `apply_enforcement` layer and the `AuthUser` extractor) verify
  against the fetched key set. Issuer/audience still come from
  `WORKER_TOKEN_ISSUER` / `WORKER_TOKEN_AUDIENCE` (same defaults).
- New DB-free tokio tests in `src/api/rest/auth.rs` (reusing the
  in-process PASETO minting helpers): a local ephemeral-port HTTP
  listener serves the key set and a token signed by that key verifies;
  a dead port falls back to the env path without panicking; URL-unset
  uses the env path (precedence).
- Authorization has since landed as ABAC (see the top entry), not
  RBAC — the spec §13 T-1b authorization item is complete.

### Added — blanket auth enforcement (default off; spec §13 T-1b)

- Blanket `/api/*` auth enforcement per the family contract in
  `agents/share/jwt-enforcement.md`: when `WORKER_REQUIRE_AUTH` is
  truthy (`1`/`true`/`yes`/`on`, case-insensitive; unset/blank/`0`/junk
  ⇒ off — the default), every route on **both** router surfaces (the
  standalone Axum `create_router` and the loco router built in
  `App::after_routes`) requires a valid PASETO `v4.public` bearer token
  and returns `401` otherwise.
- New in `src/api/rest/auth.rs`: pure, unit-testable `enforce(...)`
  decision; lenient `parse_bool` + `require_auth_from_env()` flag
  reader; `apply_enforcement(router, flag, verifier)` middleware layer.
  The flag and verifier are captured **at router construction** —
  changing `WORKER_REQUIRE_AUTH` requires a process restart. The layer
  sits beneath CORS so preflight `OPTIONS` is answered before
  enforcement.
- Public allow-list (`PUBLIC_PATHS` / `PUBLIC_PATH_PREFIXES`):
  `/_health`, `/_ping`, `/api/v1/health`, `/api-docs/openapi.json`,
  `/metrics.prom`, and `/swagger-ui*`. The `/fhir` surface is
  deliberately protected (worker PII).
- New DB-free unit tests pin the family test matrix: off + no token ⇒
  pass; on + public path ⇒ pass; on + protected + no token ⇒ `401`;
  on + valid token ⇒ pass; on + expired/tampered token ⇒ `401`; plus
  the flag-parser truthy/falsy table and a `/fhir`-is-protected pin.
- Boot-time key-set fetch has since landed (see the entry above);
  authorization has since landed as ABAC (top entry), completing
  spec §13 T-1b.

### Added — offline PASETO v4.public bearer verification

- New `src/api/rest/auth.rs`: `AuthUser` extractor + `GET /api/v1/whoami`
  verify **PASETO `v4.public`** (Ed25519) bearer tokens offline —
  signature, footer `kid`, `iss`, `aud`, `exp` — via `bearer_claims`,
  per the family-wide design in
  `agents/share/authentication-sessions.md` (§5; spec §13 T-1a).
  Authentication is opt-in per handler: any handler that takes an
  `AuthUser` argument requires a valid bearer token. Ported from the
  person-service implementation.
- New `authentication-verifier` monorepo path dependency (`0.2`,
  PASETO-only: `Verifier::from_paseto_keys_value`), carried on
  `AppState` as `verifier: Arc<Verifier>` (swap with
  `AppState::with_verifier`).
- The verifier is built from the environment at boot:
  `WORKER_PASETO_KEYS` (the Ed25519 key set the auth service publishes
  at `/.well-known/paseto-keys`), `WORKER_TOKEN_ISSUER` (default
  `authentication-service`), `WORKER_TOKEN_AUDIENCE` (default
  `main-x-service`). Absent/blank/unparseable key set ⇒ empty key set:
  every token is rejected but the service still boots.
- New DB-free unit tests in `src/api/rest/auth.rs` mint `v4.public`
  tokens in-process (throwaway Ed25519 key via `rusty_paseto` +
  `ed25519-dalek` dev-deps) and pin valid / missing / non-bearer /
  expired / tampered / no-key outcomes.
- Blanket `/api/*` enforcement landed in the same cycle (see the
  entry above); the spec §13 T-1b remainders were boot-time key-set
  fetch over HTTP and authorization — both since delivered
  (authorization as ABAC, top entry).

### Added — matcher bridge

- New `src/matching/adapter.rs` exposing
  `to_matcher_worker(&service::Worker) -> worker_matcher::Worker`.
  Projects the FHIR/schema.org-shaped service record into the matcher
  crate's builder shape: name flattening (`HumanName` → flat
  `given_name`/`family_name`/`middle_name`), telecom sampling
  (first phone / sms / email of each system),
  identifier routing by FHIR-style `system` URI (UK NHS via `https://fhir.nhs.uk/Id/nhs-number` → `uk_nhs_number`, US SSN, 40+ country slots
  with type-based fallbacks), NPI passthrough, ODS organisation codes and address field
  renaming (`state` → `county`, `postal_code` → `postcode`).
- `src/matching/mod.rs` now re-exports the sibling `worker-matcher` crate
  as `matcher_lib`, so callers can reach `MatchingEngine`,
  `MatchConfig`, `MatchResult`, `MatchBreakdown`, `Confidence`, and
  every public matcher type without taking a separate dependency.
- Field-routing rules are inline-documented in `adapter.rs` and
  pinned by `tests/duplicate_detection.rs`.
- Expanded identifier `system`-URI routing in `adapter.rs` to cover
  the remaining national schemes the matcher already scores
  deterministically: PL PESEL / PL NIP, RO CNP, UK NINO / UK CHI /
  UK H&C, IT Codice Fiscale, ES DNI, PT NIF, FI HETU, DK CPR, HR OIB,
  NO FNR, BG EGN, SI EMŠO, CN RRN, ZA ID, BE NN. Tokens are chosen
  not to collide (e.g. `nino` never overlaps `nir`); extraction into
  the `route_additional_scheme` helper keeps both functions within the
  pedantic line cap. A shared value in any covered scheme now drives a
  deterministic match instead of silently falling through. Pinned by
  new adapter unit tests and the new bridge test
  `shared_pesel_drives_match_via_pl_pesel_slot`.

### Added — tests

- New `tests/duplicate_detection.rs`. Black-box bridge tests that
  drive service records through `to_matcher_worker` and assert on
  the canonical `MatchingEngine::match_workers` output. Covers
  identical clones, name typos (Jaro-Winkler), deterministic
  short-circuits (national / strong identifiers), negative cases
  (unrelated records, divergent demographics), per-adapter field
  routing, and config-preset invariants (strict ⊆ lenient).

### Added — bridge benchmarks

- New `benches/bridge_bench.rs` (Criterion). Three groups:
  `bridge_adapter_only` (projection cost on minimal vs. rich
  records), `bridge_end_to_end` (adapter + engine call), and
  `bridge_one_to_many` (single query vs. 10 / 50 / 100 candidates).
  Regression guard for the duplicate-check hot path.

### Added — observability

- New `src/metrics.rs` exposing a process-wide `LazyLock<Metrics>`
  Prometheus registry. Standard counters
  (`worker_created_total` / `_updated_total` / `_deleted_total` /
  `_matched_total`, labeled `http_requests_total`) and histograms
  (`http_request_duration_seconds`, `worker_match_score`,
  `worker_search_duration_seconds`).
- New `GET /metrics.prom` route on the web router serving Prometheus
  text-exposition format (`text/plain; version=0.0.4`). The
  canonical `/metrics` continues to render the HTML dashboard;
  configure scrapers with `metrics_path: /metrics.prom`.

### Added — UI

- `assets/static/css/themes/` ships 39 standalone Lily Design System
  themes (light, dark, dracula, nord, cyberpunk, … + four
  United Kingdom NHS variants). The layout's theme picker now lists
  all 39; default is `light`. Selection swaps the `<link href>` of
  `<link id="lily-theme-css">` at runtime; persisted in
  `localStorage["lily-theme"]`. The command palette also lists all
  39 themes.

### Changed — Loco background jobs

- Dropped the `bg_redis` and `bg_sqlt` features from the `loco-rs`
  dependency. Background jobs are now backed exclusively by
  PostgreSQL (`bg_pg`), using the same database as application data
  — no external Redis broker. `config/development.yaml` and
  `config/production.yaml` updated to `queue.kind: Postgres` with
  `uri: DATABASE_URL`. Removes the `rusty-sidekiq` →
  `redis 0.22.3` future-incompat warning chain.

### Changed — documentation

- Reduced healthcare / clinical / patient / hospital / clinician /
  practitioner framing across spec.md, AGENTS.md, AGENTS/*, README,
  CLAUDE.md, and index.md. Preserved: FHIR R5 resource and field
  names (e.g. `Patient.birthPlace`, `Practitioner` resource),
  national-identifier proper nouns (United Kingdom National Health
  Service Number, Australia IHI), paper citations, the
  `compliance-for-healthcare.md` doc, and `HIPAA` / `NHS` / `PHI`
  as compliance regimes.
- `spec.md §11 Testing Strategy` now lists the bridge integration
  tests; `AGENTS/testing.md` gained a `## Bridge Integration Tests`
  section; `AGENTS/restful.md` gained adapter + Prometheus blocks;
  `index.md` gained a worked example showing the canonical bridge
  in action.

### Fixed

- The worker-matcher crates.io 0.3.0 API drift (Sweden personnummer
  renamed from `se_personnummer` to `se_workernummer`,
  `united_kingdom_national_health_service_number` shortened to
  `uk_nhs_number`) is now caught at the matcher level by each
  matcher's `tests/adapter_contract.rs` — see the matcher
  CHANGELOG.

### Removed

- The unused `jsonwebtoken` dependency (never referenced in `src/` or
  `tests/`). The family auth design has pivoted from RS256 JWT / JWKS
  to cookie sessions + short-lived PASETO v4.public tokens (see
  `agents/share/authentication-sessions.md`); the still-pending auth
  task in `spec/13-tasks.md` T-1 now targets PASETO verification via
  the `authentication-verifier` crate.
- The stray backup file `src/api/rest/handlers.rs.bak` (accidentally
  committed; the live `handlers.rs` is unchanged).
