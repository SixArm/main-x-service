# Testing strategy & guide — Course Service

## Test categories

### Unit tests

Embedded in source files via `#[cfg(test)] mod tests`. Run with
`cargo test --lib` — 125 tests today (123 run + 2 DB-gated `#[ignore]`;
run it for the live count).

| Module | Tests | What's covered |
|---|---|---|
| `db` (`src/db/mod.rs`) | 3 | `CourseStatus` / `LinkType` round-trip through the enum-string helper; `to_course_active` field carrying |
| `db::outbox` (`src/db/outbox.rs`, T-21) | 4 | `OutboxInsert::from_envelope` maps every column / a `deleted` event / a `merged` event / rejects a non-UUID `pid` |
| `db::outbox_atomicity_tests` (`src/db/mod.rs`, T-21) | 2 (`#[ignore]`, DB-gated) | create → `created` outbox row; merge → `merged` row with `merged_from` + a `deleted` row for the duplicate, all in one transaction |
| `matching` (`src/matching/mod.rs`) | 3 | Identical-records score 1.0, DOI deterministic short-circuit, `find_matches` rank ordering |
| `matching::adapter` (`src/matching/adapter.rs`) | 3 | `provider_id` UUID → matcher `String`; `IdentifierType` 1:1 routing; `EducationalLevel` 1:1 routing |
| `search::index` (`src/search/index.rs`) | 2 | Empty-index has 0 docs; `create_or_open` round-trips |
| `search` (`src/search/mod.rs`) | 5 | Index + exact search, fuzzy search tolerates typo, provider-scoped blocking query, delete removes from index, `tokenise` handles underscores |
| `validation` (`src/validation/mod.rs`) | 16 | Every FR-21..FR-28 branch, the nested-instance `instances[i].field` path-prefix invariant, and the SEC-M1 input-size caps (oversized text / array / array-item / `instances` array, within-caps large record accepted) |
| `streaming` (`src/streaming/mod.rs`) | 2 | `InMemoryEventPublisher` publish/observe; `EventKind` PascalCase serialisation |
| `streaming::envelope` (`src/streaming/envelope.rs`, T-21) | 9 | `Envelope::for_event`/`for_merge` construction, `EventView` projection, `EventKind` wire-token round-trip, monotonic `seq`, `EventTransport` parse (defaults to `memory`, reads once) |
| `privacy` (`src/privacy/mod.rs`) | 4 | Mask clears `provider_id` + instructor refs; mask leaves non-sensitive fields; mask doesn't mutate input; export envelope shape |
| `compliance::record_integrity` (`src/compliance/record_integrity.rs`, T-24) | 6 | A content change changes every digest; an unhashed row isn't reported as a mismatch; every stored digest is verified; the version tag leads the pre-image; the `active` flag is bound in |
| `compliance::audit_integrity` (`src/compliance/audit_integrity.rs`, T-24) | 3 | Every field is bound into the pre-image; field boundaries are unambiguous; the version tag leads the pre-image |
| `compliance::mac` (`src/compliance/mac.rs`, T-24) | 2 | `all()` lists every domain; every domain has a distinct label |
| `config` (`src/config/mod.rs`) | 5 | Empty environment yields defaults; every variable overrides its field; blank values count as unset; malformed values are refused by name; typed values tolerate surrounding whitespace |
| `relay` (`src/relay.rs`, T-22) | 3 | `LoggingSink` sends ok; a capturing sink records `(entity, key)`; config defaults are safe (relay off, interval 5, retention 7) |
| `fhir` (`src/fhir/mod.rs`, T-20) | 5 | DTO ↔ `Basic` round-trip preserves core fields; a custom educational level round-trips; the identifier scheme ↔ FHIR `system` map round-trips; an unknown system becomes `Custom`; a missing name is rejected |
| `fhir::search` (`src/fhir/search.rs`, T-20) | 5 | Empty params match everything; `_id` must match exactly; `identifier` token matches course code + scheme; `code` matches the course coding; `name` matches an alias case-insensitively |
| `metrics` (`src/metrics.rs`) | 2 | Registry render includes the declared counters + HELP/TYPE banners; an increment is reflected in the rendered exposition (T-16) |
| `api::rest` (`src/api/rest/mod.rs`) | 3 | `metrics_routes()` is mounted at root (no `/api` prefix); `ApiDoc::openapi()` builds with the expected path surface (incl. `/metrics.prom`); a **live** `GET /metrics.prom` via `tower::oneshot` returns `200` + `text/plain; version=0.0.4` and reflects a counter increment (T-16) |
| `api::rest::handlers` (`src/api/rest/handlers.rs`) | 4 | `fold_duplicate_into_main` unions collections + dedupes identifiers; fold doesn't mutate inputs; `canonical_pair` is order-independent; batch-dedup (FR-9) threshold-band classification (skip / review-queue / auto-merge boundary semantics) |
| `api::rest::auth` (`src/api/rest/auth.rs`, T-15/AU-2) | 25 | Bearer parsing (missing / non-bearer / tampered / expired header → 401); the full ABAC matrix (empty attrs read-only, `access=write`, `access=admin`, `svc=true`, first-match deny, 401-vs-403 distinction); `derive_action` per-method + destructive-suffix routing; enforce on/off across public, FHIR, and out-of-prefix paths; boot-time verifier fetch success/fallback |
| `api::rest::state` (`src/api/rest/state.rs`) | 2 | An empty verifier builds with zero keys; `env_or` falls back when unset |
| `api::rest::fhir` (`src/api/rest/fhir.rs`, T-20) | 2 | The FHIR routes mount under `/fhir`; the `CapabilityStatement` matches the mounted routes and is labelled non-standard |
| `api::rest::version` (`src/api/rest/version.rs`, T-25) | 5 | No header resolves to the current version; a bare major aliases its current minor; exact + case-insensitive matches; only `/api` paths are versioned; an unsupported version is an error |

### Bridge tests

`tests/duplicate_detection.rs` — 14 tests. Drive service-side
`Course` records through `matching::adapter::to_matcher_course`
and assert on `course_matcher::MatchingEngine::match_courses`
output. The suite pins **both sides of the contract** (the
adapter's field-routing rules AND the matcher's scoring) so a
regression on either side fails here.

Run with: `cargo test --test duplicate_detection`

Coverage: identical-clone scoring (≥0.95, High band), name-typo
fuzzy match, all three deterministic short-circuits (DOI /
Wikidata / `same_as` URL / shared provider+code), negatives
(LMS-id alone, same code at different providers, unrelated
titles), per-enum routing (`provider_id`, `EducationalLevel`,
`LearningResourceType`, `Custom` label), strict-⊆-default config
preset invariant.

Add a new bridge test when:

- The adapter (`src/matching/adapter.rs`) gains a new routing rule.
- The course-matcher crate exposes a new scoring component the
  service needs to surface.
- A regression escapes the adapter's own `#[cfg(test)] mod tests`.

### Integration tests

`tests/api_integration_test.rs` — 12 tests, all `#[ignore]`-tagged
so `cargo test --lib` stays fast. Drive `tower::ServiceExt::oneshot`
against the full Axum router with real PostgreSQL + Tantivy + the
in-memory event publisher.

Run with:

```bash
# Bring up Postgres + apply migrations once (see README.md for the
# manual psql loop; auto-migrate is out of scope for MVP).
podman compose up -d postgres

DATABASE_URL=postgres://course_user:course_password@localhost:5434/course \
  cargo test --test api_integration_test -- --ignored
```

Coverage: health, full lifecycle (create + GET + PUT + soft-delete),
422 validation, search hit, check-duplicates, match, merge, batch
dedup response shape, instance sub-resource round-trip, audit log
records CREATE then UPDATE, masked view clears provider, GDPR export
envelope shape.

`tests/common/mod.rs` builds `AppState` against env-configured
Postgres + a process-shared Tantivy `TempDir` (concurrent tests
share an index; unique timestamped names avoid collisions inside
the shared DB).

### Auth-activation test

`tests/enforcement.rs` — 1 test, `#[ignore]`-tagged, its **own binary**
(because `require_auth`/`policy`/`verifier` are process-wide
`OnceLock`s that a same-process off-suite would otherwise poison).
Boots the real router with `COURSE_REQUIRE_AUTH` on and asserts a real
request with/without a valid PASETO gets the right status — the
"activation proof" (AU-1/AU-2) that fails if the guard is ever
un-wired. Needs a database. Run with:

```bash
cargo test --test enforcement -- --ignored
```

### Fluvio round-trip test (feature-gated)

`tests/fluvio_relay.rs` — 1 test, gated on the `fluvio` Cargo feature
(`#![cfg(feature = "fluvio")]`, so a default build compiles a file with
zero tests) and `#[ignore]`-tagged (needs both Postgres and a live
Fluvio broker). Verified today only by compiling under the feature —
no automated run in this repo stands up a broker. Run against
`compose.fluvio.yaml`:

```bash
podman compose -f compose.fluvio.yaml up -d
DATABASE_URL=postgres://course:course@localhost:5434/course \
  COURSE_FLUVIO_ENDPOINT=127.0.0.1:9103 \
  cargo test --features fluvio --test fluvio_relay -- --ignored
podman compose -f compose.fluvio.yaml down -v
```

### Benchmark tests

`benches/` — 3 criterion benches. Run with `cargo bench`.

| File | What's measured |
|---|---|
| `benches/matching_bench.rs` | `match_courses` on populated pair, deterministic short-circuit, `find_matches` rank-of-100 |
| `benches/search_bench.rs` | `index_course`, exact `search`, `fuzzy_search`, `search_by_name_and_provider` (all against a 100-row index) |
| `benches/validation_bench.rs` | `validate_course` on a record exercising every FR-21..FR-28 branch |

## Running tests

```bash
cargo test --lib                              # 123 unit tests, no DB needed (+ 2 DB-gated #[ignore])
cargo test --test duplicate_detection         # 14 bridge tests, no DB needed
cargo test --test api_integration_test -- --ignored   # 12 integration tests, DB required
cargo test --test enforcement -- --ignored    # 1 auth-activation test, DB required
cargo test --features fluvio --test fluvio_relay -- --ignored   # 1 test, DB + Fluvio broker required
cargo bench                                   # 3 criterion benches
```

## Writing new tests

### Unit-test pattern

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptive_name() {
        // Arrange
        let course = Course::new("CS101");

        // Act
        let json = serde_json::to_string(&course).unwrap();
        let round_trip: Course = serde_json::from_str(&json).unwrap();

        // Assert
        assert_eq!(round_trip.name, "CS101");
    }
}
```

### Integration-test pattern

Use `tests/common/mod.rs::create_test_router` to build the full
router against env-configured Postgres + the shared Tantivy
tempdir. Each test creates its own records via
`common::course_json("Suffix")` which emits a UUID-zeros id +
unique timestamped name so re-runs are idempotent against the
shared DB.

```rust
#[tokio::test]
#[ignore]
async fn some_flow_works() {
    let app = common::create_test_router().await;
    let body = common::course_json("SomeFlow");
    let (status, env) = send(&app, Method::POST, "/api/courses", Some(body)).await;
    assert_eq!(status, StatusCode::CREATED);
}
```

## Test-data conventions

- Use well-known canonical courses for readability — "CS101
  Introduction to Computer Science", "MAT221 Linear Algebra".
- Use realistic schema.org-style URLs in `same_as` (Wikidata Q
  identifiers, OER repositories).
- Use `Course::new("name")` for the minimal test course; layer on
  fields only when the test requires them.
