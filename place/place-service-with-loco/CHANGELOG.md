# Changelog

All notable changes to this crate are documented here.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/);
versioning: [SemVer](https://semver.org/spec/v2.0.0.html). See also:
[`index.md`](./index.md), [`spec.md`](./spec/index.md), [`README.md`](./README.md).

## [Unreleased]

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
