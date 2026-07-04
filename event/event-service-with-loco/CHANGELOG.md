# Changelog

All notable changes to this crate are documented here.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/);
versioning: [SemVer](https://semver.org/spec/v2.0.0.html). See also:
[`index.md`](./index.md), [`spec.md`](./spec/index.md), [`README.md`](./README.md).

## [Unreleased]

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
  in `AGENTS/testing.md`.

### Removed

- The unused `jsonwebtoken` dependency (never referenced in `src/` or
  `tests/`). The family auth design has pivoted from RS256 JWT / JWKS
  to cookie sessions + short-lived PASETO v4.public tokens (see
  `agents/share/authentication-sessions.md`); the still-pending auth
  task in `spec/13-tasks.md` T-8 now targets PASETO verification via
  the `authentication-verifier` crate.
