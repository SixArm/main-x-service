# Changelog

All notable changes to this crate are documented here.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/);
versioning: [SemVer](https://semver.org/spec/v2.0.0.html). See also:
[`index.md`](./index.md), [`spec.md`](./spec/index.md), [`README.md`](./README.md).

## [Unreleased]

### Security

- **SEC-G5: blanket guard is now guard-all (deny-unless-public).** The
  `enforce` decision previously only gated paths under `/api` or `/fhir`,
  silently allowing any out-of-prefix route (e.g. `/`, `/admin`) through
  with no token when enforcement was on. It now denies every path except an
  explicit public allow-list (`is_public_path`: `/_health`, `/_ping`,
  `/api-docs/openapi.json`, `/swagger-ui*`, `/metrics.prom`, `/api/health`,
  `/fhir/metadata`). The now-unused prefix helpers (`API_PREFIX`,
  `FHIR_PREFIX`, `GUARDED_PREFIXES`, `is_guarded_path`, `PUBLIC_API_PATHS`)
  were removed.
- **SEC-G6: trailing-slash normalisation in `derive_action`.** A trailing
  slash (`/api/things/merge/`, `//`) no longer downgrades a destructive
  named POST (`/merge`, `/deduplicate`, `/import`) to `Write`, which would
  have let a non-admin `access=write` caller reach a destructive op; the
  path is `trim_end_matches('/')`-normalised before the suffix check so it
  stays `Destructive`.

- **SEC-B6: relay claims outbox rows with `FOR UPDATE SKIP LOCKED`.** The
  Phase-3 relay drained via a plain unlocked `SELECT … WHERE published_at IS
  NULL`, so with more than one instance every relay would **double-ship** the
  same rows. `drain_once` now runs in a transaction and `unpublished` claims
  rows with `FOR UPDATE SKIP LOCKED` (a second relay skips locked rows; the
  lock releases on commit). Delivery stays at-least-once (consumers dedupe on
  `event_id`).

### Added — authz: ABAC policy authorization inside the blanket guard

- ABAC authorization landed (spec §13 T-4, the final sub-item —
  supersedes the earlier roles sketch; family contract:
  `agents/share/authorization-attributes.md`). When
  `THING_REQUIRE_AUTH` is on, a verified PASETO token is further
  checked by the shared policy engine in `authentication-verifier`
  0.3: the request's action is derived from the HTTP method plus the
  crate's destructive named POSTs (`auth::DESTRUCTIVE_POST_SUFFIXES`
  — `/merge`, `/deduplicate`, `/import`), and the policy is evaluated
  over the token's new `attrs` claim, first-match-wins, defaulting to
  allow-read / deny-mutation.
- New env vars `THING_ABAC_POLICY` (inline JSON) and
  `THING_ABAC_POLICY_FILE` (path), read once at `AppState`
  construction by the new `auth::policy_from_env` (restart to
  change); unset or unparsable ⇒ `tracing::warn!` + the built-in
  default policy (`svc=true` ⇒ everything; `access=admin` ⇒
  destructive+write; `access=write` ⇒ write) — the service always
  boots.
- `auth::enforce` now takes the HTTP method and the policy and
  returns `403` (with the deciding-rule reason) for a valid token the
  policy denies; `401` remains missing/bad credential. `AppState`
  carries the policy (`policy: Arc<Policy>`) alongside the verifier
  and the `require_auth` flag.
- DB-free unit tests pin the family §7 matrix: action derivation,
  empty-`attrs` read-only default, `access=write` / `access=admin` /
  `svc=true` tiers, deny-beats-later-allow, 401-vs-403, bad-policy
  fallback.
- Flag off ⇒ behaviour-neutral: no authn and no authz, exactly as
  before.

### Added — boot-time HTTP fetch of the PASETO key set (2026-07-04)

- Boot-time key-set fetch landed (the fetch part of spec §13 T-4;
  family contract shared with the sibling services). New env var
  `THING_PASETO_KEYS_URL`: when set (non-blank), `App::after_routes`
  calls the new `state::boot_verifier` (async), which fetches the
  key-set JSON once via `Verifier::from_paseto_keys_url` — the
  `authentication-verifier` path dep now enables its `fetch` feature.
  A successful fetch **wins** over any `THING_PASETO_KEYS` env value
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

### Added — offline PASETO v4.public bearer verification

- Peer-side bearer-token verification landed (the verification part of
  spec §13 T-4), per the family-wide design in
  `agents/share/authentication-sessions.md` §5: a new `AuthUser`
  extractor and `GET /api/whoami` endpoint (`src/api/rest/auth.rs`)
  verify **PASETO `v4.public`** (Ed25519) bearer tokens offline —
  signature, footer `kid`, `iss`, `aud`, `exp` — via the monorepo
  `authentication-verifier` 0.2 path dependency. No shared secret, no
  per-request introspection hop.
- The verifier is built from the environment at boot:
  `THING_PASETO_KEYS` (the Ed25519 key set the auth service publishes
  at `/.well-known/paseto-keys`), `THING_TOKEN_ISSUER` (default
  `authentication-service`), `THING_TOKEN_AUDIENCE` (default
  `main-x-service`). Absent/blank/unparseable key set ⇒ empty key set:
  every token is rejected but the service still boots.
  `AppState::with_verifier` swaps in a replacement (e.g. one built from
  a freshly fetched key set).
- New DB-free unit tests in `src/api/rest/auth.rs` mint `v4.public`
  tokens in-process (throwaway Ed25519 key via `rusty_paseto` +
  `ed25519-dalek` dev-deps) and pin valid / missing / non-bearer /
  expired / tampered / no-key outcomes.
- Blanket `/api/*` enforcement has since landed default-off (see the
  next section); roles + published-key HTTP fetch remain open in spec
  §13 T-4.

### Added — blanket `/api/*` auth enforcement (default-off, 2026-07-04)

- The enforcement remainder of spec §13 T-4, per the family contract
  in `agents/share/jwt-enforcement.md`: a pure `auth::enforce`
  decision plus the `auth::require_auth_mw` Axum middleware require a
  valid PASETO `v4.public` bearer token on every `/api/*` route when
  `THING_REQUIRE_AUTH` is truthy (`1`/`true`/`yes`/`on`,
  case-insensitive via `auth::parse_bool`; anything else including
  unset/blank ⇒ off — the default, so behaviour is unchanged until a
  deployment opts in). The flag is read once at `AppState`
  construction (`auth::require_auth_from_env`, carried as
  `AppState::require_auth`); restart to change.
- Public allow-list (`auth::PUBLIC_API_PATHS`): `/api/health` stays
  public inside the enforced prefix; root-level `/_health`, `/_ping`,
  `/api-docs/openapi.json`, `/swagger-ui*`, and `/metrics.prom` sit
  outside the `/api` scope and are never gated (segment-aware prefix
  check, so `/api-docs` is not mistaken for `/api/...`).
- Wired on **both** router surfaces via
  `axum::middleware::from_fn_with_state` — the hand-written
  `create_router` and the loco router in `App::after_routes` — inside
  the CORS layer so preflight requests are still answered.
- New DB-free unit tests in `src/api/rest/auth.rs` pin the full
  matrix: off + no token ⇒ pass; on + public / out-of-scope paths ⇒
  pass; on + protected + no token ⇒ `401`; on + valid token ⇒ pass;
  on + expired / tampered ⇒ `401`; plus the lenient `parse_bool`
  flag-parser semantics.
- Still open in spec §13 T-4: roles (editor / read-only / service)
  and fetching the published Ed25519 key set over HTTP at boot.

### Added — matcher bridge

- New `src/matching/adapter.rs` exposing
  `to_matcher_thing(&service::Thing) -> thing_matcher::Thing`.
  Projects the schema.org-shaped service record into the matcher
  crate's builder shape: 1:1 mapping for `name`, `description`,
  `disambiguating_description`, `url`, `main_entity_of_page`,
  `owner`, `alternate_names`, `same_as`; singular `additional_type`
  / `subject_of` become first entries of the matcher's list fields;
  first `images` entry becomes the matcher's single `image`;
  identifiers map via `map_identifier_property` to schema.org
  canonical tokens (`doi`, `isbn`, `issn`, `gtin`, `sku`, `mpn`,
  `serialNumber`, `uri`, `uuid`; `Custom(s)` passes through
  verbatim); registry-only fields are dropped.
- `src/matching/mod.rs` now re-exports the sibling `thing-matcher` crate
  as `matcher_lib`, so callers can reach `MatchingEngine`,
  `MatchConfig`, `MatchResult`, `MatchBreakdown`, `Confidence`, and
  every public matcher type without taking a separate dependency.
- Field-routing rules are inline-documented in `adapter.rs` and
  pinned by `tests/duplicate_detection.rs`.

### Added — tests

- New `tests/duplicate_detection.rs`. Black-box bridge tests that
  drive service records through `to_matcher_thing` and assert on
  the canonical `MatchingEngine::match_things` output. Covers
  identical clones, name typos (Jaro-Winkler), deterministic
  short-circuits (DOI / ISBN / UUID), negative cases
  (unrelated records, same name + different ISBNs), per-adapter field
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
  (`thing_created_total` / `_updated_total` / `_deleted_total` /
  `_matched_total`, labeled `http_requests_total`) and histograms
  (`http_request_duration_seconds`, `thing_match_score`,
  `thing_search_duration_seconds`).
- **Prometheus metrics endpoint `GET /metrics.prom`.** The previously
  dead `src/metrics.rs` registry is now actually scraped: a new
  `handlers::metrics_prom` handler renders `thing::metrics::METRICS`
  in text-exposition format (`text/plain; version=0.0.4`), served at
  the application **root** path `/metrics.prom` (not under `/api`) so
  a default Prometheus scrape config (`metrics_path: /metrics.prom`)
  finds it. Wired both as a loco controller group
  (`api::rest::metrics_routes`, registered in `App::routes`) and on the
  hand-written Axum router (`create_router`), and added to the
  `OpenAPI` document under a new `observability` tag. The `thing_*`
  counters (`thing_created_total` / `_updated_total` / `_deleted_total`
  / `_matched_total`) and the latency/score histograms are now
  externally observable. New DB-free tests pin the `/metrics.prom`
  `OpenAPI` path and the root loco-route binding (`api::rest::tests`).
  Brings parity with the older Axum services, which already expose
  Prometheus metrics.

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

- The thing-matcher crates.io 0.3.0 API drift (Sweden personnummer
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
  task in `spec/13-tasks.md` T-4 now targets PASETO verification via
  the `authentication-verifier` crate.
