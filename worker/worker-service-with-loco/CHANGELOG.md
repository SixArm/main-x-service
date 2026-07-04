# Changelog

All notable changes to this crate are documented here.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/);
versioning: [SemVer](https://semver.org/spec/v2.0.0.html). See also:
[`index.md`](./index.md), [`spec.md`](./spec/index.md), [`README.md`](./README.md).

## [Unreleased]

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
- Blanket `/api/*` enforcement stays open as spec §13 T-1b.

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
