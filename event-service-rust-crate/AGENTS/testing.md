# Testing Strategy & Guide

Three layers:

- **Unit tests** in `#[cfg(test)] mod tests { ... }` at the bottom of source files. No external dependencies.
- **Integration tests** in `tests/` against a real PostgreSQL via `DATABASE_URL`.
- **Benchmarks** in `benches/` using Criterion.

## Unit tests

```bash
cargo test --lib
```

Coverage by module (62 tests as of the schema.org/Event rewrite):

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

Or via Docker Compose:

```bash
docker-compose -f docker-compose.test.yml up
```

Current tests:

- `health_check_returns_healthy`
- `create_event_round_trip`
- `validation_rejects_missing_name`

Shared helpers live in `tests/common/mod.rs` — `create_test_app_state()` (async) builds an `AppState` with a per-test tempdir search index.

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
