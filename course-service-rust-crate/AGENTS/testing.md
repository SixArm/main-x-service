# Testing strategy & guide — Course Service

## Test categories

### Unit tests

Embedded in source files via `#[cfg(test)] mod tests`. Run with
`cargo test --lib` — 35 tests total today.

| Module | Tests | What's covered |
|---|---|---|
| `db` (`src/db/mod.rs`) | 3 | `CourseStatus` / `LinkType` round-trip through the enum-string helper; `to_course_active` field carrying |
| `matching` (`src/matching/mod.rs`) | 3 | Identical-records score 1.0, DOI deterministic short-circuit, `find_matches` rank ordering |
| `matching::adapter` (`src/matching/adapter.rs`) | 3 | `provider_id` UUID → matcher `String`; `IdentifierType` 1:1 routing; `EducationalLevel` 1:1 routing |
| `search::index` (`src/search/index.rs`) | 2 | Empty-index has 0 docs; `create_or_open` round-trips |
| `search` (`src/search/mod.rs`) | 5 | Index + exact search, fuzzy search tolerates typo, provider-scoped blocking query, delete removes from index, `tokenise` handles underscores |
| `validation` (`src/validation/mod.rs`) | 11 | Every FR-21..FR-28 branch plus the nested-instance `instances[i].field` path-prefix invariant |
| `streaming` (`src/streaming/mod.rs`) | 2 | `InMemoryEventPublisher` publish/observe; `EventKind` PascalCase serialisation |
| `privacy` (`src/privacy/mod.rs`) | 4 | Mask clears `provider_id` + instructor refs; mask leaves non-sensitive fields; mask doesn't mutate input; export envelope shape |
| `api::rest::handlers` (`src/api/rest/handlers.rs`) | 2 | `fold_duplicate_into_main` unions collections + dedupes identifiers; fold doesn't mutate inputs |

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

### Benchmark tests

`benches/` — 3 criterion benches. Run with `cargo bench`.

| File | What's measured |
|---|---|
| `benches/matching_bench.rs` | `match_courses` on populated pair, deterministic short-circuit, `find_matches` rank-of-100 |
| `benches/search_bench.rs` | `index_course`, exact `search`, `fuzzy_search`, `search_by_name_and_provider` (all against a 100-row index) |
| `benches/validation_bench.rs` | `validate_course` on a record exercising every FR-21..FR-28 branch |

## Running tests

```bash
cargo test --lib                              # 35 unit tests, no DB needed
cargo test --test duplicate_detection         # 14 bridge tests, no DB needed
cargo test --test api_integration_test -- --ignored   # 12 integration tests, DB required
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
