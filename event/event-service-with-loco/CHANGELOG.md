# Changelog

All notable changes to this crate are documented here.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/);
versioning: [SemVer](https://semver.org/spec/v2.0.0.html). See also:
[`index.md`](./index.md), [`spec.md`](./spec/index.md), [`README.md`](./README.md).

## [Unreleased]

### Changed — geo coordinates are exact decimals, not floats

`Place::latitude` / `Place::longitude` move from `f64` to `BigDecimal`,
and `event_locations.latitude` / `.longitude` from `DOUBLE PRECISION` to
`NUMERIC` (migration `m20260822_000001_location_coordinates_to_numeric`).

A coordinate is a decimal quantity. `DOUBLE PRECISION` cannot hold
`37.87` — it holds `37.869999999999997` — and cannot tell `37.87` from
`37.8700000000000001` at all. Coordinates now round-trip as the digits
the caller sent.

The immediate trigger was a real failure, not a tidy-up. The repository
adopted `serde_json`'s `arbitrary_precision` feature
(`spec/serde-json-float-roundtrip-arbitrary-precision`), under which
every number is represented internally as a one-key map. `Location` is an
internally-tagged enum (`#[serde(tag = "kind")]`), so serde buffers the
variant's fields through `Content` — and an `f64` read back from that
buffer fails with `invalid type: map, expected f64`. Concretely,
`POST /api/events` with real coordinates stopped deserializing;
`models::event::tests::roundtrip_serde` was the only thing in the tree
that caught it. An exact decimal has no such problem.

**The wire format is unchanged.** `BigDecimal`'s default serde impl emits
a quoted string, which would have broken every client; these fields opt
into `bigdecimal::impl_serde::arbitrary_precision_option` instead, so the
JSON stays `"latitude":37.87` (and `null` when absent), exactly as the
`f64` produced. The SvelteKit front-end types these as `number | null`
and needs no change. OpenAPI continues to advertise a number
(`#[schema(value_type = Option<f64>)]`).

Matching is unaffected in substance: the matcher scores geo distance with
Haversine, which is floating-point, so the adapter converts at that
boundary. A coordinate not representable as `f64` is dropped from scoring
rather than approximated into a wrong position — exactness is a property
of what is stored and returned, not of the distance score.

Also added, because the type change removes a bound that used to exist by
accident: `MAX_COORDINATE_SCALE` (10 decimal places, ~10 µm). An `f64`
capped the digit count implicitly at ~17 significant digits; an exact
decimal does not, so without this a caller could post a latitude with
thousands of fraction digits and have every one stored. Nothing a client
could previously send is newly rejected.

The migration widens exactly — every double has a `NUMERIC` form, so no
stored value is lost. Existing rows keep the float artefacts they were
written with (`37.869999999999997` stays); only values written from here
on are exact. Back-filling a rounder number would be inventing precision
the caller never sent. Rolling back is lossy by nature, which is the
argument for the direction taken.

Three parts, per this crate's SDD discipline: spec (§5.2.1, §5.3
invariants, §10.1), code, and tests — seven new cases pinning the JSON
number representation, the tagged-enum round-trip that regressed, exact
decimal round-tripping, `null`/absent handling, inclusive range bounds,
and the scale cap, plus one on the event-stream envelope (`EventEvent` is
also internally tagged and carries a whole `Event`, so bus consumers had
the identical latent break with nothing covering it).
`cargo test --all-targets` 159/159,
`cargo clippy --all-targets -- -D warnings` clean, `cargo fmt --check`
clean.

### Added — declared MSRV (Rust 1.95)

- `Cargo.toml` now declares `rust-version = "1.95"`, the repository's
  **current stable minus three** floor
  (`spec/rust-msrv-n-minus-3.md`). Sourced from `ci/msrv.txt` and
  enforced by `scripts/ci-check.sh msrv`, which asserts the declared
  value matches that file and then compiles the crate — `--all-targets`,
  so benches and tests count — against the 1.95 toolchain. Behaviour is
  unchanged; what changes is that the floor is now a checked claim
  rather than an unstated assumption.

### Removed — dead `EventRepository::search` SQL method (QA-CUST-SQL)

Audited for the same MySQL-placeholder footgun fixed in
`authentication-service` (`Expr::cust_with_values("LOWER(email) = ?", …)`
— a `?` placeholder Postgres rejects). This crate's `search()`
(`src/db/repositories.rs`) already spelled `"LOWER(name) LIKE $1"` —
Postgres-style — so the specific defect did not apply. But `grep -rn`
across the crate (handlers, tests, benches — this crate has no bulk
module) found **zero callers**: `/api/events/search` goes through
Tantivy, not this method, and no test ever exercised it either. Rather
than leave a plausible-looking, DB-syntax-unverified method sitting
unregarded, it is removed — along with its now-orphaned `escape_like`
SEC-G4 helper and `Expr`/`cust_with_values` import, which existed only
to support it. Verified: `scripts/ci-check.sh test-db` still green
(7/7), `cargo fmt --check`, and
`cargo clippy --all-targets -- -D warnings` clean.

### Added — Durable event bus, real-broker sink (BUS-3, 2026-08-03)

`FluvioSink` (`src/relay.rs`) — the Phase-3 relay's real-broker
`EventSink`, behind this crate's own `fluvio` Cargo feature (off by
default; `fluvio` 0.50). Ported from case-service's BUS-1 reference
implementation. One producer per topic, partitioned by record `pid`
per `agents/share/event-bus.md` §7. New env vars:
`EVENT_FLUVIO_ENDPOINT` (unset ⇒ unchanged `LoggingSink` default) and
`EVENT_EVENT_TOPIC` (default `mxi.event.events`, matching this crate's
existing doubled `EVENT_EVENT_*` naming for domain-event settings). An
endpoint configured without the `fluvio` feature refuses to start the
relay rather than silently falling back to `LoggingSink` — that
fallback would mark outbox rows `published_at` without ever reaching
the broker the operator asked for. `compose.fluvio.yaml` +
`Dockerfile.fluvio-cli` provision a local SC+SPU broker (ports
9203/9210/9211) for opt-in manual runs (not part of any automated CI
stage); `tests/fluvio_relay.rs` is a feature-gated, `#[ignore]`d
live-broker round-trip, verified by compiling under `--features
fluvio` rather than an actual execution (no broker is stood up in this
repo's CI) — it drives `SeaOrmEventRepository` directly under
`EventTransport::Outbox` rather than case's loco `request::<App, _,
_>` helper, since this crate keeps the older hand-rolled
`AppState`/repository layout. `cargo build`/`clippy --all-targets -D
warnings`/`fmt --check`/`test --lib` all clean under both default
features and `--features fluvio` (152 tests passed, 1 ignored,
identical count both ways); full DB-gated suite green, zero
regressions. `cargo deny check` carries the same single pre-existing
`RUSTSEC-2023-0071` (via `loco-rs` → `jsonwebtoken` → `rsa`) with and
without the feature — confirmed by diff, so `fluvio` introduces no new
advisory. This crate has no `compliance/soup.tsv`, so no SOUP register
update applies (unlike case's BUS-1 landing).

### Changed — loco-rs 1.0.1 (2026-08-02)

- **loco-rs 0.16.4 → 1.0.1**: sea-orm 1.1 → 2.0, sea-orm-migration →
  2.0, sea-query → 1.0. Feature renames applied: `auth_jwt` → `auth`,
  `bg_pg` → `worker`.
- **`with-bigdecimal`** added to the `sea-orm` feature list — offer
  `price` is stored as `bigdecimal::BigDecimal`, same fix as person
  and worker's match-score columns.
- A `useless_conversion` in `src/db/outbox.rs` from a now-redundant
  `.into()`.
- No raw `Statement` calls anywhere in this crate (no review-queue
  module here), and no `EntityTrait`-import gap — `db/models.rs`
  glob-imports the sea-orm prelude per submodule.
- This crate is the last of the six person-style services to migrate
  (course, person, worker, place, thing, event) — closes out that half
  of the family-wide rollout.
- No behavioural change; verified with the full DB-gated suite (7
  tests, unchanged count) against a freshly migrated Postgres 18.

### Added — key rotation and policy hot-reload without a restart (2026-08-01)

AU-1, completing the five axum-style services (person was the reference).

- **One reloadable verifier, one reloadable policy.** Both left
  `AppState` for process-wide holders that the blanket guard **and** the
  `AuthUser` / `MaybeAuthUser` extractors read per request. Snapshotting
  them in the state meant a rotation or a policy edit could reach one
  path and not the other; only the `require_auth` flag is still a boot
  value, because turning enforcement on or off mid-flight is not
  something to do without a restart.
- **`spawn_key_refresh`** re-fetches `EVENT_PASETO_KEYS_URL` every
  `EVENT_PASETO_KEYS_REFRESH_SECS` (default 3600; `0` disables; a no-op
  when the URL is unset). A failed fetch **keeps the current key set** —
  a transient auth-service outage must not lock every caller out.
- **`policy()` is a `ReloadablePolicy`**, with `reload_policy()` and
  **`spawn_policy_watcher`** polling `EVENT_ABAC_POLICY_FILE`'s mtime
  every 15 s; a malformed edit falls back to the built-in default rather
  than leaving the service unprotected.
- **`tests/enforcement.rs`** — the activation proof, in its own binary
  because the auth `OnceLock`s are process-wide, and carrying its own
  minimal router builder because this crate has no HTTP test harness.
  With `EVENT_REQUIRE_AUTH=1` over the **production** router: public paths
  stay open, a protected read and write without a token are `401`, a
  malformed bearer is `401` (not a 500), a valid token with no
  attributes reads `200` and writes `403`, and `access=write` creates.
- New environment variable: `EVENT_PASETO_KEYS_REFRESH_SECS`. New
  dev-dependencies: `serial_test`, `tower`.

### Fixed — the DB-gated suite ran for the first time (2026-08-01)

- **`POST /api/events` demanded values it then ignored.** `created_at`
  and `updated_at` are server-managed — the repository stamps them on
  insert and refreshes them on update — but they were required on the
  wire, so an otherwise valid create was refused with `422 missing field
  created_at`. Both are now `#[serde(default)]`.
- The create round-trip test now reads the response body **before**
  asserting the status, so a refusal reports the server's reason instead
  of `422 != 201`. That is how the above was diagnosed in one run.

  Suite: 6/6 green vs Postgres 18; crate enrolled in
  [`ci/db-suites.txt`](../../ci/db-suites.txt).

### Added — row-level integrity digests + verify endpoints (2026-07-28)

*Landed but never recorded here until this DOC-2 docs pass
(2026-08-04) found the gap — shipped, tested, and reachable, but with
no `spec/13` task, no `spec/14` row, no `spec/12` compliance-table row,
no `agents/restful.md` endpoint entry, and no `CHANGELOG.md` entry.*

- **`src/compliance/mac.rs`** — this crate's binding to the shared
  `integrity-mac` crate: SHA-256, SHA3-256, and (when a key is
  configured) a keyed HMAC-SHA256 MAC, HKDF-domain-separated per
  (service, domain).
- **`src/compliance/record_integrity.rs`** — hashes the **assembled**
  `Event` record (the same value `GET /api/events/{id}` returns), not
  just the root `events` row, since an event's identifiers/location/
  parties live in child tables and are exactly the kind of field worth
  editing quietly.
- **`src/compliance/audit_integrity.rs`** — the same digest/MAC
  treatment for `audit_log` rows.
- **`GET /api/records/verify`** and **`GET /api/audit/verify`** —
  guarded like every other `/api` route.
- **Default off**: with no `EVENT_INTEGRITY_MAC_KEY` (or
  `EVENT_INTEGRITY_MAC_KEY_FILE`, which takes precedence) configured,
  no MAC is written and affected rows report `mac_absent` rather than
  a mismatch. Other env vars: `EVENT_INTEGRITY_MAC_KEY_ID`,
  `EVENT_INTEGRITY_MAC_KEYS_RETIRED`.
- Unlike person / worker / care-pathway / case, this crate has no hash
  chain (`prev_hash`/`hash`) and takes no external-witness checkpoint —
  a MAC proves a row's content is unchanged since it was written, and
  says nothing about a row deleted wholesale.

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

### Security

- **SEC-M1: input-size caps on the `Event` payload.** The validator
  enforced format/time-window rules but capped no field's *size*, so a
  single multi-megabyte text field or a huge array could be a CPU/memory
  `DoS` against the matcher's O(n·m) string / Jaccard scoring, amplified
  across the `check-duplicates` / `deduplicate` scan. `validate_event` now
  also bounds every scalar text field (`MAX_TEXT_LEN = 1024`: `name`,
  `description`, `disambiguating_description`, `url`, `duration`,
  `time_zone`, `typical_age_range`), string-array cardinality + per-entry
  length (`MAX_ARRAY_LEN = 256` / `MAX_ITEM_LEN = 512`: `alternate_names`,
  `image`, `same_as`, `keywords`), and the inner text + cardinality of the
  nested object arrays (`identifiers`, the `location` union, the six party
  lists, `about`/`works` references, `offers`, `sub_events`, `links`) —
  field-scoped `422`s *before* persist/match. `offers[i].price_currency`
  (3-char) and `in_language` (2-char) keep their stricter existing bounds;
  time-window checks are untouched. Factored into `event_size_caps` /
  per-group `cap_*` helpers. Unit tested.

- **SEC-G5: switch the blanket auth guard from prefix-gate to guard-all
  (deny-unless-public).** `enforce` previously returned `Ok` (bypassing auth)
  for any path **not** under `/api` or `/fhir`, so a route mounted outside
  those prefixes was silently unguarded (allow-unless-in-prefix). It now
  denies unless the path is in an explicit public allow-list (new
  `is_public_path`: `/_health`, `/_ping`, `/api-docs/openapi.json`,
  `/swagger-ui*`, `/metrics.prom`, `/api/health`, `/fhir/metadata`). Removed
  the now-unused `is_api_path` / `is_fhir_path` / `PUBLIC_API_PATHS` /
  `API_PREFIX` / `FHIR_PREFIX`. New test
  `test_enforce_on_guards_non_api_non_public_paths_is_401` pins `/`,
  `/admin`, `/secret`, `/foo/bar` ⇒ `401` when enforcement is on.
- **SEC-G6: normalise a trailing slash in `derive_action`.** A destructive
  named POST was classified via `path.ends_with(suffix)`, so
  `POST /api/events/merge/` (trailing slash) fell through to `Write`, letting
  an `access=write` (non-admin) caller reach a destructive op. `derive_action`
  now `trim_end_matches('/')`s the path first, so `/merge/` and `/merge//`
  still classify as `Destructive`. Pinned in `test_derive_action_matrix`.
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

### Added — authz: ABAC policy authorization inside the blanket guard (2026-07-05)

- ABAC authorization landed (spec §13 authorization item — supersedes
  the earlier RBAC scheduler / admin / read-only / service sketch;
  family contract: `agents/share/authorization-attributes.md`). When
  `EVENT_REQUIRE_AUTH` is on, a verified PASETO token is further
  checked by the shared policy engine in `authentication-verifier`
  0.3: the request's action is derived from the HTTP method plus the
  crate's destructive named POSTs (`auth::DESTRUCTIVE_POST_SUFFIXES`
  — `/merge`, `/deduplicate`, `/import`), and the policy is evaluated
  over the token's new `attrs` claim, first-match-wins, defaulting to
  allow-read / deny-mutation.
- New env vars `EVENT_ABAC_POLICY` (inline JSON) and
  `EVENT_ABAC_POLICY_FILE` (path), read once at `AppState`
  construction (restart to change); unset or unparsable ⇒
  `tracing::warn!` + the built-in default policy (`svc=true` ⇒
  everything; `access=admin` ⇒ destructive+write; `access=write` ⇒
  write) — the service always boots.
- `auth::enforce` now takes the HTTP method and the policy and
  returns `403` (with the deciding-rule reason) for a valid token the
  policy denies; `401` remains missing/bad credential.
- DB-free unit tests pin the family §7 matrix: action derivation,
  empty-`attrs` read-only default, `access=write` / `access=admin` /
  `svc=true` tiers, deny-beats-later-allow, 401-vs-403, bad-policy
  fallback.
- Flag off ⇒ behaviour-neutral: no authn and no authz, exactly as
  before.

### Added — boot-time HTTP fetch of the PASETO key set (2026-07-04)

- Boot-time key-set fetch landed (the fetch part of spec §13 T-8;
  family contract shared with the sibling services). New env var
  `EVENT_PASETO_KEYS_URL`: when set (non-blank), `App::after_routes`
  calls the new `state::boot_verifier` (async), which fetches the
  key-set JSON once via `Verifier::from_paseto_keys_url` — the
  `authentication-verifier` path dep now enables its `fetch` feature.
  A successful fetch **wins** over any `EVENT_PASETO_KEYS` env value
  (`tracing::info!` records the source URL and key count); any fetch
  failure `tracing::warn!`s and falls back to the env path (else the
  empty reject-all key set) — the service **always boots**.
  Unset/blank URL ⇒ previous behaviour exactly.
- Boot order matters and is now explicit: the fetched verifier is
  installed with `AppState::with_verifier` **before** the shared-store
  insert and before `require_auth_mw` captures the state, so both
  router surfaces (the loco router's enforcement middleware and the
  `FromRef` handler extraction) consult the fetched key set.
- Fetch happens once at boot; there is deliberately no refresh loop —
  key-rotation re-fetch is a roadmap item (spec §15).
- New DB-free tokio tests in `src/api/rest/auth.rs`: a local
  ephemeral-port axum listener serves the in-process test key set and
  the fetch-built verifier accepts a token signed by that key; a
  fast-failing URL (`http://127.0.0.1:1/`) falls back to the env/empty
  path without panicking.

### Added — offline PASETO v4.public bearer verification (2026-07-04)

- New `src/api/rest/auth.rs`: an `AuthUser` Axum extractor plus
  `GET /api/v1/whoami` verify PASETO **`v4.public`** (Ed25519) bearer
  tokens **offline** — signature, footer `kid`, `iss`, `aud`, `exp` —
  via the monorepo `authentication-verifier` 0.2 path dependency, per
  the family-wide design in `agents/share/authentication-sessions.md`
  (§5, §9 step 4; spec §13 T-8, verification part). Handlers opt in by
  taking an `AuthUser` argument; blanket `/api/v1/*` enforcement has
  since landed default-off (see the next section) — roles +
  published-key HTTP fetch remain open T-8 items.
- The verifier is built from the environment at boot and carried on
  `AppState`: `EVENT_PASETO_KEYS` (the Ed25519 key set the auth service
  publishes at `/.well-known/paseto-keys`), `EVENT_TOKEN_ISSUER`
  (default `authentication-service`), `EVENT_TOKEN_AUDIENCE` (default
  `main-x-service`). Absent/blank/unparseable key set ⇒ empty key set:
  every token is rejected but the service still boots.
  `AppState::with_verifier` swaps in a replacement (e.g. one built from
  a freshly fetched key set).
- New DB-free unit tests in `src/api/rest/auth.rs` mint `v4.public`
  tokens in-process (throwaway Ed25519 key via `rusty_paseto` +
  `ed25519-dalek` dev-deps) and pin valid / missing / non-bearer /
  expired / tampered / no-key outcomes.

### Added — blanket `/api/v1/*` auth enforcement (default-off, 2026-07-04)

- The enforcement remainder of spec §13 T-8, per the family contract
  in `agents/share/jwt-enforcement.md`: a pure `auth::enforce`
  decision plus the `auth::require_auth_mw` Axum middleware require a
  valid PASETO `v4.public` bearer token on every `/api/v1/*` route
  when `EVENT_REQUIRE_AUTH` is truthy (`1`/`true`/`yes`/`on`,
  case-insensitive via `auth::parse_bool`; anything else including
  unset/blank ⇒ off — the default, so behaviour is unchanged until a
  deployment opts in). The flag is read once at `AppState`
  construction (`auth::require_auth_from_env`, carried as
  `AppState::require_auth`); restart to change.
- Public allow-list (`auth::PUBLIC_API_PATHS`): `/api/v1/health` stays
  public inside the enforced prefix; root-level `/_health`, `/_ping`,
  `/api-docs/openapi.json`, `/swagger-ui*`, `/metrics.prom`, and the
  `/fhir/*` `501 Not Implemented` stubs sit outside the `/api/v1`
  scope and are never gated (the FHIR surface mounts at `/fhir`, not
  under the enforced API prefix, so it deliberately stays public until
  it grows beyond stubs).
- Wired on **both** router surfaces via
  `axum::middleware::from_fn_with_state` — the hand-written
  `create_router` and the loco router in `App::after_routes` — inside
  the CORS layer so preflight requests are still answered.
- New DB-free unit tests in `src/api/rest/auth.rs` pin the full
  matrix: off + no token ⇒ pass; on + public / out-of-scope paths
  (incl. `/fhir/*`) ⇒ pass; on + protected + no token ⇒ `401`; on +
  valid token ⇒ pass; on + expired / tampered ⇒ `401`; plus the
  lenient `parse_bool` flag-parser semantics.
- Still open in spec §13 T-8: scheduler / admin / read-only / service
  roles and fetching the published Ed25519 key set over HTTP at boot.

### Added — matcher bridge

- New `src/matching/adapter.rs` exposing
  `to_matcher_event(&service::Event) -> event_matcher::Event`.
  Projects the schema.org/Event-shaped service record into the matcher
  crate's flat builder shape: `DateTime<Utc>` → RFC 3339 strings for
  the time fields (`start_date`, `end_date`, `door_time`,
  `previous_start_date`); `event_status` / `event_attendance_mode` /
  `event_type` → matcher enums (`map_event_type`, with operational
  subtypes flowing through as `Other(name)`); `Vec<Location>` → the
  first populated variant dispatched variant-aware (Place / virtual
  URL / PostalAddress / Text); `organizers[0].name` → matcher
  `organizer`; `performers` → `Vec<String>`; identifiers routed via
  `map_identifier_scheme` (system-URI hints — Eventbrite, Meetup,
  Ticketmaster, Songkick, Bandsintown, Facebook, Luma, Google
  Calendar, Wikidata, iCalendar UID — else `IdentifierType` enum →
  `Other(name)`). Language tags follow the ET-8 divergence: only the
  first `in_language` entry is projected, data-only (never scored).
- `src/matching/mod.rs` now re-exports the sibling `event-matcher` crate
  (crates.io `0.6.1`, per `Cargo.toml`) as `matcher_lib`, so callers
  can reach `MatchingEngine`, `MatchConfig`, `MatchResult`,
  `Confidence`, and every public matcher type without taking a
  separate dependency.
- Field-routing rules are inline-documented in `adapter.rs` and
  pinned by `tests/duplicate_detection.rs`.

### Added — tests

- New `tests/duplicate_detection.rs`. Black-box bridge tests that
  drive service records through `to_matcher_event` and assert on
  the canonical `MatchingEngine::match_events` output. Covers
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
  (`event_created_total` / `_updated_total` / `_deleted_total` /
  `_matched_total`, labeled `http_requests_total`) and histograms
  (`http_request_duration_seconds`, `event_match_score`,
  `event_search_duration_seconds`).
- New `GET /metrics.prom` route on the web router serving Prometheus
  text-exposition format (`text/plain; version=0.0.4`). This is the
  only metrics surface — there is no HTML dashboard (this is a
  backend-only loco service with no view tier); configure scrapers
  with `metrics_path: /metrics.prom`.

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
  practitioner framing across spec.md, AGENTS.md, agents/*, README,
  CLAUDE.md, and index.md. Preserved: FHIR R5 resource and field
  names (e.g. `Patient.birthPlace`, `Practitioner` resource),
  national-identifier proper nouns (United Kingdom National Health
  Service Number, Australia IHI), paper citations, the
  `compliance-for-healthcare.md` doc, and `HIPAA` / `NHS` / `PHI`
  as compliance regimes.
- `spec.md §11 Testing Strategy` now lists the bridge integration
  tests; `agents/testing.md` gained a `## Bridge Integration Tests`
  section; `agents/restful.md` gained adapter + Prometheus blocks;
  `index.md` gained a worked example showing the canonical bridge
  in action.

### Documentation + tests harmonization (2026-06-15)

- Rewrote `index.md` (and its `README.md` symlink) from the spec:
  replaced leftover Person-template content (date-of-birth / gender /
  address match components, `MRN`/`SSN`/national-ID identifiers,
  `birth_date`/`gender` JSON payloads, `MATCHING_GENDER_WEIGHT` /
  `MATCHING_DOB_WEIGHT` env vars, `/api/events/*` paths) with the
  event-shaped equivalents: event identifier types, the §6.2 match
  components, event-record create/match JSON, `/api/v1/` paths, and a
  worked bridge example (`to_matcher_event` → `MatchingEngine`).
- Corrected the FHIR claim from "partial compliance" to the spec
  §6.8 reality: `/fhir/Event/*` returns `501 Not Implemented`.
- Mounted the FHIR R5 stub routes (`/fhir/Event` + `/fhir/Event/{id}`)
  in both `create_router` and the loco-native `routes()` so an
  unmatched request yields `501` (not `404`), matching spec §6.8/§9.
- Corrected the matcher-bridge entry above (was Person/FHIR-shaped) and
  removed the `Added — UI` themes section and the "canonical `/metrics`
  HTML dashboard" phrasing — none of that exists in this backend-only
  service.
- Removed the stale `Fixed` note about event-matcher `0.3.0`
  national-identifier API drift (that was Person-matcher content;
  `Cargo.toml` pins `event-matcher = 0.6.1`).
- Added unit tests for the `Mixed` attendance-mode / location coupling
  (physical + virtual required) and the `remaining ≤ total` capacity
  invariant (`src/validation/mod.rs`); added a FHIR-501 integration
  test (`tests/api_integration_test.rs`). Refreshed stale test counts
  in `agents/testing.md`.

### Removed

- The unused `jsonwebtoken` dependency (never referenced in `src/` or
  `tests/`). The family auth design has pivoted from RS256 JWT / JWKS
  to cookie sessions + short-lived PASETO v4.public tokens (see
  `agents/share/authentication-sessions.md`); the still-pending auth
  task in `spec/13-tasks.md` T-8 now targets PASETO verification via
  the `authentication-verifier` crate.

### Fixed

- `tests/api_integration_test.rs` had rustfmt drift that broke the
  crate's `cargo fmt --check` gate. Reformatted with `cargo fmt`; no
  behavioural change, tests unchanged and green.
