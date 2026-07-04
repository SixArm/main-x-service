# Changelog

All notable changes to this crate are documented here.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/);
versioning: [SemVer](https://semver.org/spec/v2.0.0.html). See also:
[`index.md`](./index.md), [`spec.md`](./spec/index.md), [`README.md`](./README.md).

## [Unreleased]

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
