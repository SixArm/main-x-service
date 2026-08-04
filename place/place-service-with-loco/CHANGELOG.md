# Changelog

All notable changes to this crate are documented here.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/);
versioning: [SemVer](https://semver.org/spec/v2.0.0.html). See also:
[`index.md`](./index.md), [`spec.md`](./spec/index.md), [`README.md`](./README.md).

## [Unreleased]
### Fixed — `POST /api/places` demanded server-owned fields (QA-SERVER-FIELDS)

- **The JSON extractor required `id`, `is_deleted`, `created_at`,
  `updated_at`, `keywords`, `identifiers`, `amenity_features`, and
  `opening_hours` on the wire**, even though the server owns every one
  of them. A hand-written create body (the way a real API client
  writes one) omitting any of them was refused with `422 missing field
  …` before the handler, validation, or the repository ever ran —
  demanding a value it then ignored or discarded. Same defect, same
  fix, as the event service's `created_at`/`updated_at` fix
  (2026-08-01): every server-managed field is now `#[serde(default)]`.
  `name` — the one field the server does not own — is also
  `#[serde(default)]` now, so an omitted `name` reaches the normal
  `validate_place` path (`422 validation_error`) instead of being
  turned away by the extractor's generic "missing field" error.
- **Making the fields optional alone would have been a new bug**: an
  omitted `id` defaults to the nil UUID, and the repository previously
  persisted whatever `id`/`created_at`/`updated_at` the domain value
  carried verbatim rather than overwriting them — so a second
  hand-written create would have collided on the same nil primary key,
  and an omitted timestamp would have stored the Unix epoch. Fixed
  alongside: `create_place` mints a fresh id whenever the wire value is
  nil (mirroring the event service's existing pattern), and the
  repository now stamps `created_at`/`updated_at` to "now" on insert
  (preserving `created_at` and refreshing `updated_at` on update)
  rather than trusting the passed-in domain value.
- New DB-gated regression suite `tests/api_integration_test.rs`: a
  minimal hand-built create body succeeds and reads back a fresh id,
  ~now timestamps, and empty collections; two consecutive hand-written
  creates mint distinct ids rather than colliding; an omitted `name`
  still fails, but via `validation_error`, not the JSON extractor.
  Found while writing the auth activation proof
  (`tests/enforcement.rs`), whose payload is built by serializing
  `Place::new` rather than by hand for precisely this reason — that
  comment is now updated to reflect the fix.

  Suite: 3/3 new + 9/9 crate-wide green vs Postgres 18 (`scripts/ci-check.sh test-db`).

### Added — Durable event bus, real-broker sink (BUS-3, 2026-08-03)

Ports the case-service reference (BUS-1) onto this crate's relay:

- **`FluvioSink`** (`src/relay.rs`) — the Phase-3 relay's real-broker
  `EventSink`, behind this crate's own `fluvio` Cargo feature (off by
  default; `fluvio` 0.50). One producer per topic, partitioned by record
  `pid` per `agents/share/event-bus.md` §7. New env vars:
  `PLACE_FLUVIO_ENDPOINT` (unset ⇒ unchanged `LoggingSink` default) and
  `PLACE_EVENT_TOPIC` (default `mxi.place.events`).
- **No silent fallback.** An endpoint configured without the `fluvio`
  feature refuses to start the relay rather than silently falling back
  to `LoggingSink` — that fallback would mark outbox rows
  `published_at` without ever reaching the broker the operator asked
  for. The initial connection retries indefinitely instead of falling
  back, for the same reason.
- `compose.fluvio.yaml` + `Dockerfile.fluvio-cli` provision a local
  SC+SPU broker for opt-in manual runs (not part of any automated CI
  stage; identical layout to case-service's, container names
  `mxi-place-fluvio-*`).
- `tests/fluvio_relay.rs` is a feature-gated, `#[ignore]`d live-broker
  round-trip, verified by compiling under `--features fluvio` rather
  than an actual execution (no broker is stood up in this repo's CI).
- No behavioural change to a default build: `cargo build --lib` and
  `cargo test --lib` pass counts are unchanged; only `--features
  fluvio` adds the new sink to the dependency tree and build output.

### Changed — loco-rs 1.0.1 (2026-08-02)

- **loco-rs 0.16.4 → 1.0.1**: sea-orm 1.1 → 2.0, sea-orm-migration →
  2.0, sea-query → 1.0. Feature renames applied: `auth_jwt` → `auth`,
  `bg_pg` → `worker`.
- **4 raw `Statement` call sites** in `src/db/review_queue.rs` move to
  the `_raw` variants — this crate's only hand-rolled SQL; unlike
  person/worker there's no separate audit-chain or erasure module.
- **A pre-existing missing `EntityTrait` import surfaced in three
  `db/models.rs` submodules** (`place_merge_records` and two others),
  same class of latent bug as person-service: sea-orm 1.1's
  `DeriveEntityModel` expansion tolerated the gap, 2.0's doesn't.
- A `useless_conversion` in `src/db/outbox.rs` from a now-redundant
  `.into()`.
- No `BigDecimal`, no `DatabaseConnection::Disconnected`, and this
  crate's tables already key on `i64` (not loco's `PkAuto` DSL), so
  none of person/worker's other fixes were needed here.
- No behavioural change; verified with the full DB-gated suite (3
  tests, unchanged count) against a freshly migrated Postgres 18.

### Added — key rotation and policy hot-reload without a restart (2026-08-01)

AU-1, following the person service (the axum-style reference).

- **One reloadable verifier.** The PASETO verifier left `AppState` for a
  process-wide `ReloadableVerifier` that the blanket guard **and** the
  `AuthUser` / `MaybeAuthUser` extractors read per request. `EnforcementState` no longer snapshots the verifier or the policy either; it carries only the flag.
- **`spawn_key_refresh`** re-fetches `PLACE_PASETO_KEYS_URL` every
  `PLACE_PASETO_KEYS_REFRESH_SECS` (default 3600; `0` disables; a no-op
  when the URL is unset), so a key rotation needs no restart. A failed
  fetch **keeps the current key set** — a transient auth-service outage
  must not lock every caller out.
- **`policy()` is a `ReloadablePolicy`**, with `reload_policy()` and
  **`spawn_policy_watcher`** polling `PLACE_ABAC_POLICY_FILE`'s mtime
  every 15 s. A malformed edit falls back to the built-in default rather
  than leaving the service unprotected.
- **`tests/enforcement.rs`** — the activation proof, in its own binary
  because the auth `OnceLock`s are process-wide. With
  `PLACE_REQUIRE_AUTH=1` over the real router: public paths stay open, a
  protected read and write without a token are `401`, a malformed bearer
  is `401` (not a 500), a valid token with no attributes reads `200` and
  writes `403` — the 401/403 split the ABAC contract requires — and
  `access=write` creates. This crate had no HTTP test harness at all — its `tests/` are library tests over pure functions — so the proof brings a minimal one that builds the **production** router, which is the thing whose wiring is in question.
- New environment variable: `PLACE_PASETO_KEYS_REFRESH_SECS`.


### Changed — `Config::from_env` gained a testable seam and more variables (2026-07-23)

- The env overlay moved into a pure `Config::from_source(lookup)`;
  `from_env` is now a two-line delegation to it. This makes the
  variable-to-field mapping unit-testable without mutating the process
  environment — which matters because `std::env::set_var` is `unsafe`
  in the 2024 edition (this crate forbids `unsafe`) and process env is
  global state that makes parallel tests flaky.
- Added variables: `SEARCH_CACHE_SIZE_MB`, `STREAMING_BROKER_URL`,
  `STREAMING_TOPIC` (the previously-unreachable config fields).
- A blank or whitespace-only value now counts as **unset** rather than
  overwriting the default with an empty string, and typed values
  tolerate surrounding whitespace (a `.env` line like `SERVER_PORT = 9090 `).
- Pinned by five unit tests; behaviour is otherwise unchanged.

### Added — stored review queue + decision endpoints (2026-07-19)

- `review_queue` table (migration `m20260719_000001_create_review_queue`):
  the batch-dedup scan persists its candidate pairs (normalized pair
  order, UNIQUE upsert — re-scans refresh scores, decided rows keep
  their decision, ids stay stable) and the scan response now reports
  the **stored** rows.
- `GET /api/places/review-queue[?status=&limit=]` — list the stored
  queue (newest first, cap 500).
- `POST /api/places/review-queue/{id}/decision`
  (`{"status": "confirmed" | "rejected"}`) — decide a `pending` item;
  first-writer-wins in SQL, `404`/`422` on unknown/already-decided.

### Fixed

- 2026-07-19 — dedup-report drift: `POST /api/places/deduplicate` now returns the family's
  person/worker-shaped report: `auto_merged` (always 0),
  `queued_for_review`, and `review_items[]` with pair ids, score,
  quality band, `detection_method`, lowercase `status` wire tokens,
  and `created_at` — previously the response carried only counts,
  so the front-end's declared report shape was aspirational.
  Serde wire-token pin added in-file.

### Security

- **SEC-M1: input-size caps on the `Place` payload.** The validator
  enforced format/range rules but capped no field's *size*, so a single
  multi-megabyte text field or a huge array could be a CPU/memory `DoS`
  against the matcher's O(n·m) string / Jaccard scoring, amplified across
  the `check-duplicates` / `deduplicate` scan. `validate_place` now also
  bounds every scalar text field (`MAX_TEXT_LEN = 1024`: `name`,
  `alternate_name`, `description`, `telephone`, `fax_number`, `url`,
  `branch_code`, and the nested `address.*`), string-array cardinality +
  per-entry length (`MAX_ARRAY_LEN = 256` / `MAX_ITEM_LEN = 512`:
  `keywords`), and the inner text + cardinality of the struct arrays
  (`identifiers`, `amenity_features`, `opening_hours`) — field-scoped
  `422`s *before* persist/match. `global_location_number` (GLN 13-digit)
  and `opening_hours` times (5-char) keep their stricter existing bounds;
  geo lat/lon range checks are untouched. Factored into `place_size_caps`
  / `cap_*` helpers. Unit tested.

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

### Added — authz: ABAC policy authorization inside the blanket guard

- ABAC authorization landed (spec §13 T-8, the final sub-item —
  supersedes the earlier roles/RBAC sketch; family contract:
  `agents/share/authorization-attributes.md`). When
  `PLACE_REQUIRE_AUTH` is on, a verified PASETO token is further
  checked by the shared policy engine in `authentication-verifier`
  0.3: the request's action is derived from the HTTP method plus the
  crate's destructive named POSTs (`auth::DESTRUCTIVE_POST_SUFFIXES`
  — `/merge`, `/deduplicate`, `/import`), and the policy is evaluated
  over the token's new `attrs` claim, first-match-wins, defaulting to
  allow-read / deny-mutation.
- New env vars `PLACE_ABAC_POLICY` (inline JSON) and
  `PLACE_ABAC_POLICY_FILE` (path), read once at router construction
  by the new `auth::policy_from_env` (restart to change); unset or
  unparsable ⇒ `tracing::warn!` + the built-in default policy
  (`svc=true` ⇒ everything; `access=admin` ⇒ destructive+write;
  `access=write` ⇒ write) — the service always boots.
- `auth::enforce` now takes the HTTP method and the policy and
  returns `403` (with the deciding-rule reason) for a valid token the
  policy denies; `401` remains missing/bad credential.
  `EnforcementState` carries the policy alongside the verifier.
- DB-free unit tests pin the family §7 matrix: action derivation,
  empty-`attrs` read-only default, `access=write` / `access=admin` /
  `svc=true` tiers, deny-beats-later-allow, 401-vs-403, bad-policy
  fallback.
- Flag off ⇒ behaviour-neutral: no authn and no authz, exactly as
  before.

### Added — auth: boot-time HTTP fetch of the PASETO key set

- Boot-time key-set fetch landed (spec §13 T-8, the fetch sub-item;
  family contract shared with the sibling services). New env var
  `PLACE_PASETO_KEYS_URL`: when set (non-blank),
  `app.rs::after_routes` calls the new `state::boot_verifier` (async),
  which fetches the key-set JSON once via
  `Verifier::from_paseto_keys_url` — the `authentication-verifier`
  path dep now enables its `fetch` feature. A successful fetch **wins**
  over any `PLACE_PASETO_KEYS` env value (`tracing::info!` records the
  source URL and key count); any fetch failure `tracing::warn!`s and
  falls back to the env path (else the empty reject-all key set) — the
  service **always boots**. Unset/blank URL ⇒ previous behaviour
  exactly.
- Boot order matters and is now explicit: the fetched verifier is
  installed with `AppState::with_verifier` **before**
  `EnforcementState::from_app_state` and the shared-store insert, so
  both router surfaces (the blanket-enforcement middleware and the
  `FromRef` handler extraction) consult the fetched key set.
- Fetch happens once at boot; there is deliberately no refresh loop —
  key-rotation re-fetch is a roadmap item (spec §15).
- New DB-free tokio tests in `src/api/rest/auth.rs`: a local
  ephemeral-port axum listener serves the in-process test key set and
  the fetch-built verifier accepts a token signed by that key; a
  fast-failing URL (`http://127.0.0.1:1/`) falls back to the env/empty
  path without panicking.

### Added — auth: offline PASETO v4.public bearer verification

- Peer bearer-token verification landed, per the family-wide design in
  `agents/share/authentication-sessions.md` (§5; spec §13 T-8, the
  verification half). New `src/api/rest/auth.rs`: an `AuthUser` Axum
  extractor plus `GET /api/whoami` verify **PASETO `v4.public`**
  (Ed25519) bearer tokens **offline** — signature, footer `kid`, `iss`,
  `aud`, `exp` — via the monorepo `authentication-verifier` 0.2 path
  dependency (`Verifier::from_paseto_keys_value`). No shared secret, no
  per-request introspection hop. Handlers opt in by taking an `AuthUser`
  argument; blanket `/api/*` enforcement landed the same day (next
  section).
- The verifier rides on `AppState` and is built from the environment at
  boot: `PLACE_PASETO_KEYS` (the Ed25519 key set the auth service
  publishes at `/.well-known/paseto-keys`), `PLACE_TOKEN_ISSUER`
  (default `authentication-service`), `PLACE_TOKEN_AUDIENCE` (default
  `main-x-service`). Absent/blank/unparseable key set ⇒ empty key set:
  every token is rejected but the service still boots.
  `AppState::with_verifier` swaps in a replacement (e.g. one built from
  a freshly fetched key set).
- `GET /api/whoami` is registered in both routers (`create_router` and
  the loco `places_routes`) and in the OpenAPI document under a new
  `auth` tag, with a `bearer` (PASETO) security scheme added via a
  utoipa `SecurityAddon` modifier.
- New DB-free unit tests: `src/api/rest/auth.rs` mints `v4.public`
  tokens in-process (throwaway Ed25519 key via `rusty_paseto` +
  `ed25519-dalek` + `base64` dev-deps) and pins valid / missing /
  non-bearer / expired / tampered / no-key outcomes;
  `src/api/rest/state.rs` pins the empty-key-set fallback and the
  `env_or` default; `src/api/rest/mod.rs` pins that OpenAPI advertises
  `/api/whoami` and defines the `bearer` scheme.

### Added — auth: blanket `/api/*` enforcement (default-off)

- Blanket auth enforcement landed, per the family-wide contract in
  `agents/share/jwt-enforcement.md` (spec §13 T-8, the enforcement
  half). `src/api/rest/auth.rs` gains a pure `enforce(require_auth,
  path, headers, verifier)` decision — `Ok(())` lets the request
  through, `Err((401, msg))` rejects — plus `parse_bool` /
  `require_auth_from_env` (lenient flag parse: `1`/`true`/`yes`/`on`
  case-insensitive ⇒ on; unset/blank/`0`/junk ⇒ off) and a
  `require_auth_middleware` Axum middleware carrying an
  `EnforcementState { require_auth, verifier }`.
- **Off by default.** `PLACE_REQUIRE_AUTH` is read once at router
  construction (restart to change). When on, every route requires a
  valid PASETO `v4.public` bearer token except the public allow-list,
  documented in the `PUBLIC_PATHS` / `PUBLIC_PATH_PREFIXES` constants:
  `/api/health`, loco's `/_health` / `/_ping`,
  `/api-docs/openapi.json`, `/swagger-ui*` (prefix), and the
  root-mounted `/metrics.prom`. When off, behaviour is unchanged
  (opt-in `AuthUser` per handler).
- Wired on **both** router surfaces via
  `axum::middleware::from_fn_with_state`: the `create_router` Axum
  surface (`src/api/rest/mod.rs`) and the loco router
  (`src/app.rs::after_routes`), so the flag guards the service however
  it is mounted.
- New DB-free unit tests in `src/api/rest/auth.rs` reuse the in-process
  PASETO minting helpers and pin the required matrix: off + no token ⇒
  Ok; on + each public path ⇒ Ok; on + protected (incl. `/api/whoami`)
  + no token ⇒ `401`; on + protected + valid token ⇒ Ok; on +
  expired/tampered token ⇒ `401`; plus the `parse_bool`
  truthy/falsy parser pin.

### Added — observability

- **Prometheus metrics endpoint — `GET /metrics.prom`.** The process-wide
  `prometheus::Registry` in [`src/metrics.rs`](./src/metrics.rs) (reached
  via the `METRICS` `LazyLock`) is now served at the application **root**
  path `/metrics.prom` in text-exposition format
  (`text/plain; version=0.0.4`) by the handler
  `api::rest::handlers::metrics_prom`, registered both in the loco route
  table via `api::rest::metrics_routes()` and in the `create_router` Axum
  surface — not under `/api`, so a default scraper
  (`metrics_path: /metrics.prom`) finds it. The metric set:
  `place_created_total`, `place_updated_total`, `place_deleted_total`,
  `place_matched_total` plus a labelled `http_requests_total`
  (`method`/`path`/`status`) and latency/score histograms. The path is
  added to the OpenAPI document under a new `observability` tag. Before,
  the registry was built but never exposed over HTTP, so the counters
  were dead. New DB-free test
  `api::rest::tests::openapi_includes_metrics_prom_path` (the existing
  `metrics::tests` already pins registry render + counter increment).
  Brings parity with the sibling person-service, which already exposes
  this endpoint.

### Changed — validation

- Opening-hours times are now validated. `validate_place` checks every
  `OpeningHoursSpecification.opens` / `.closes` against a real 24-hour
  `HH:MM` clock via the new public helper `validation::time_is_valid(&str)`
  (2 ASCII digits, colon, 2 ASCII digits; hours `00..=23`, minutes
  `00..=59`), reporting indexed field paths (`opening_hours[i].opens` /
  `.closes`). Previously these were free strings, so `"25:99"` / `"5pm"`
  were accepted. Closes the drift between `CLAUDE.md` (which already
  listed "Opening hours validation") and the code (which did none); spec
  §6.5 + §14.1 now list the check. Added unit + integration coverage.
- GLN validation now verifies the GS1 mod-10 check digit, not just the
  13-digit length. New public helper `validation::gln_is_valid(&str)`;
  `validate_place` rejects a GLN whose check digit is wrong with
  "GLN must be exactly 13 digits with a valid GS1 check digit". This
  closes the drift between spec §6 / §14 (which already promised a "GLN
  check digit") and the code (which only counted digits). Tests updated
  to use real GS1 GLNs (`0614141999996`, `4006381333931`); added
  check-digit unit + integration coverage.

### Added — matcher bridge

- New `src/matching/adapter.rs` exposing
  `to_matcher_place(&service::Place) -> place_matcher::Place`.
  Projects the FHIR/schema.org-shaped service record into the matcher
  crate's builder shape: name flattening (`HumanName` → flat
  `given_name`/`family_name`/`middle_name`), telecom sampling
  (first phone / sms / email of each system),
  identifier routing by FHIR-style `system` URI (schema.org / FHIR system URI → matcher scheme enum
  with type-based fallbacks), GLN, FIPS, GNIS, OSM IDs and address field
  renaming (`state` → `county`, `postal_code` → `postcode`).
- `src/matching/mod.rs` now re-exports the sibling `place-matcher` crate
  as `matcher_lib`, so callers can reach `MatchingEngine`,
  `MatchConfig`, `MatchResult`, `MatchBreakdown`, `Confidence`, and
  every public matcher type without taking a separate dependency.
- Field-routing rules are inline-documented in `adapter.rs` and
  pinned by `tests/duplicate_detection.rs`.

### Added — tests

- New `tests/duplicate_detection.rs`. Black-box bridge tests that
  drive service records through `to_matcher_place` and assert on
  the canonical `MatchingEngine::match_places` output. Covers
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
  (`place_created_total` / `_updated_total` / `_deleted_total` /
  `_matched_total`, labeled `http_requests_total`) and histograms
  (`http_request_duration_seconds`, `place_match_score`,
  `place_search_duration_seconds`).
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

- The place-matcher crates.io 0.3.0 API drift (Sweden personnummer
  renamed from `se_personnummer` to `se_workernummer`,
  `united_kingdom_national_health_service_number` shortened to
  `uk_nhs_number`) is now caught at the matcher level by each
  matcher's `tests/adapter_contract.rs` — see the matcher
  CHANGELOG.
- `tests/integration_geo_radius.rs` used `vec![]` for two fixed
  candidate collections that are only iterated, tripping
  `clippy::useless_vec` and breaking the crate's clippy-clean gate
  (`--all-targets -- -D warnings`). Now plain arrays, reformatted with
  `cargo fmt` (the same file also had pre-existing rustfmt drift); no
  behavioural change, tests unchanged and green.

### Removed

- The unused `jsonwebtoken` dependency (never referenced in `src/` or
  `tests/`). The family auth design has pivoted from RS256 JWT / JWKS
  to cookie sessions + short-lived PASETO v4.public tokens (see
  `agents/share/authentication-sessions.md`); `spec/13-tasks.md` T-8's
  PASETO-verification and blanket-enforcement halves are now delivered
  (see the *Added — auth* sections above); roles + boot-time HTTP key
  fetch remain open.
