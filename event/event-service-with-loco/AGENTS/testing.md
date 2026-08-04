# Testing Strategy & Guide

Three layers:

- **Unit tests** in `#[cfg(test)] mod tests { ... }` at the bottom of source files. No external dependencies.
- **Integration tests** in `tests/` against a real PostgreSQL via `DATABASE_URL`.
- **Benchmarks** in `benches/` using Criterion.

## Unit tests

```bash
cargo test --lib
```

Coverage by module (schema.org/Event rewrite; run `cargo test --lib`
for the live count):

| Module | What's covered |
|---|---|
| `models::event` | Defaults, serde round-trip, `Location` variants |
| `models::identifier` | Display, unknown type falls back to `Other` |
| `validation` | Required `name`, `end ≥ start`, `door ≤ start`, capacity breakdown, attendance-mode coherence, ISO duration / language codes, phone normalization, address standardization |
| `matching::algorithms` | Title fuzzy match, alt-name search, start-date exponential decay, window overlap, location dispatch, party matching, identifier formatting tolerance |
| `matching::scoring` | Probabilistic weighted sum, strong-identifier short-circuit, deterministic rules, quality classification |
| `matching::phonetic` | Soundex codes |
| `search::index` | Index lifecycle, schema fields |
| `search` | Index + search by name / organizer, fuzzy search, delete, bulk index |
| `privacy` | Mask value preserves separators, identifier + party email masking, GDPR export |

## Integration tests

```bash
DATABASE_URL=postgres://… cargo test --test api_integration_test
```

Or against the containerised Postgres (Podman):

```bash
scripts/test-db.sh up event/event-service-with-loco
scripts/ci-check.sh test-db event/event-service-with-loco
```

Current tests (`tests/api_integration_test.rs`):

- `health_check_returns_healthy`
- `create_event_round_trip`
- `fhir_appointment_surface_is_live`
- `validation_rejects_missing_name`

Shared helpers live in `tests/common/mod.rs` — `create_test_app_state()` (async) builds an `AppState` with a per-test tempdir search index.

### Other DB/broker-gated integration-test files

Two more files under `tests/` are DB- or broker-gated and run outside
the default `cargo test --lib` / `--test api_integration_test` pair:

- **`tests/enforcement.rs`** — the "activation proof" (AU-1): an
  end-to-end blanket-enforcement + ABAC check over the **real** router
  with `EVENT_REQUIRE_AUTH` on, as opposed to the DB-free pure-`enforce`
  unit tests in `src/api/rest/auth.rs`. Its own test binary (process-wide
  `OnceLock`s for `require_auth`/`policy`/`verifier` would otherwise leak
  across the off-by-default suite). `#[ignore]`d — the router opens a
  database connection. Run with `cargo test --test enforcement --
  --ignored`.
- **`tests/fluvio_relay.rs`** — round-trips the durable-bus relay
  against a real Fluvio broker (BUS-3). `#![cfg(feature = "fluvio")]`-gated
  (a default build compiles a file with zero tests) and `#[ignore]`d (needs
  Postgres + a live broker, neither stood up by any automated run in this
  repo). Run against `compose.fluvio.yaml`; see the file header for the
  exact commands.

## Benchmarks

```bash
cargo bench
```

Benches in `benches/`:

| File | Benchmarks |
|---|---|
| `matching_bench.rs` | Name match (exact + fuzzy), time match, location match, party match, identifier match, probabilistic match against 50 candidates, phonetic similarity |
| `search_bench.rs` | Index single event, full-text search over 500 docs, fuzzy search over 500 docs |
| `validation_bench.rs` | Simple validation, rich validation (place + virtual + organizer + keywords), phone normalization, address standardization |
| `bridge_bench.rs` | Service-side `Event` → `to_matcher_event` → `event_matcher::MatchingEngine::match_events`: adapter-only cost, end-to-end bridge cost, one-vs-100-candidates |

## Writing new tests

### Test event factory

```rust
use event_service::models::Event;
use chrono::{TimeZone, Utc};

let start = Utc.with_ymd_and_hms(2026, 3, 1, 9, 0, 0).unwrap();
let mut event = Event::new("Annual Conference", start);
event.event_type = EventType::Conference;
event.end_date = Some(start + chrono::Duration::hours(2));
```

### Tempdir search index

```rust
let tmp = tempfile::tempdir().unwrap();
let engine = SearchEngine::new(tmp.path()).unwrap();
```

## CI / hooks

Existing GitHub Actions:

| Workflow | Steps |
|---|---|
| `test.yml` | `cargo test --lib` + integration tests against a PostgreSQL service |
| `quality.yml` | `cargo fmt --check` + `cargo clippy` |
| `security.yml` | Security scans |

## Bridge Integration Tests

`tests/duplicate_detection.rs` is a black-box test that drives the
service-side domain model through [`matching::adapter::to_matcher_event`]
and asserts on `MatchingEngine::match_events` output. The suite pins
**both sides of the contract** — the adapter's field-routing rules and
the matcher's scoring algorithm — so a regression on either side fails
a test here.

Run with: `cargo test --test duplicate_detection`

### Coverage (18 tests)

| Category | What it pins |
|---|---|
| Identical / near-duplicate | identical-clone score ≥ 0.95, name-typo fuzzy match, ordering invariants (closer-evidence outscores farther) |
| Deterministic short-circuits | Eventbrite identifier via system URI, iCalendar UID routing, BookingNumber type-enum fallback, Virtual location URL match, Place location geo propagation, RFC 3339 start_date projection |
| Negative cases | unrelated records score low, common-name + divergent demographics not flagged as duplicate |
| Field-routing pinning | per-adapter mapping tests (telecom → phone/email, address field renames, identifier-system-URI routing) |
| Edge cases | sparse records, empty fields, config presets |

### Running

```bash
cargo test --test duplicate_detection                       # all bridge tests
cargo test --test duplicate_detection identical             # just the identical-clone tests
cargo test --test duplicate_detection -- --nocapture        # with stdout
```

### When to add a new test here

Add a bridge test when:

- The adapter (`src/matching/adapter.rs`) gains a new routing rule.
- The event-matcher crate exposes a new scoring component the service
  needs to surface.
- A regression escapes the adapter's own `#[cfg(test)] mod tests`.
