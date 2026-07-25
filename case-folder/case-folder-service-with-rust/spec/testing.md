# Testing strategy

> Part of the [Loco edition specification](index.md). Cross-cutting
> principles + stub mode: [root testing](../../spec/testing.md).

| Layer         | Tool                                                | Status                                                             |
| ------------- | --------------------------------------------------- | ------------------------------------------------------------------ |
| Type check    | `cargo check`                                       | required green                                                     |
| Lint          | `cargo clippy -- -D warnings`                       | required green                                                     |
| Format        | `cargo fmt --check`                                 | required green                                                     |
| Unit          | `cargo test --lib` (in-crate `#[cfg(test)]`)        | **14 in repo (nhs + geofence)**                                   |
| Request tests | `cargo test --test requests` (Loco testing harness) | **50 in repo** (use `StubClient`s — no real Patient/Worker needed) |

## Unit tests in repo

### `src/nhs.rs` — Modulus-11 (8 tests)

- `normalises_to_digits`
- `formats_full_numbers`, `formats_partial_inputs`
- `validates_known_good` (six Modulus-11-valid numbers)
- `accepts_grouped_and_bare_forms_identically`
- `rejects_bad_check_digit`, `rejects_check_digit_of_ten`, `rejects_wrong_length`

### `src/controllers/alerts.rs` — geofence breach derivation (6 tests)

The `detect_geofence_breaches` pure function and its `cabinet_buildings`
hierarchy resolver are tested directly, covering every branch of the
boundary-crossing rule:

- `cross_building_move_is_a_breach`, `same_building_move_is_not_a_breach`
- `move_with_missing_endpoint_cabinet_is_not_a_breach` (in-transit /
  created-in-place — a `None` endpoint)
- `move_via_unresolvable_cabinet_is_not_a_breach` (unknown cabinet id)
- `cabinet_under_orphan_room_is_unresolved` (room whose building is absent)
- `only_breaching_moves_are_returned_from_a_mixed_log`

## Request tests in repo (`tests/requests/*.rs`)

The 50 request tests are **`#[ignore]`-gated**, matching every sibling
service: a plain `cargo test` reports them as ignored rather than
failing on a machine with no `case_folder_test` database. Run them with

```sh
DATABASE_URL=postgres://…/case_folder_test cargo test -- --ignored
```

Each test marks `#[serial]` and calls `clean_db()` at
the top (which resets all five stub services), so it starts from an
empty world. Each test asserts a status code + JSON shape — no HTML
markup. The table below is a representative subset of the 50 tests;
`auth.rs`, `alerts.rs`, and `volumes.rs` add the remainder (auth
request/verify/me-guard/logout, geofence cross-building vs
same-building, and the volume CRUD + move flow). `auth.rs`'s
`logout_clears_the_session_cookie` pins `POST /api/auth/logout` → `204`
with a `Set-Cookie` that clears `cts_session` (`Max-Age=0`).

| File          | Test                                                                     | Asserts                                                               |
| ------------- | ------------------------------------------------------------------------ | --------------------------------------------------------------------- |
| `home.rs`     | `healthz_returns_ok`                                                     | `GET /healthz` → 200 + `{"status":"ok"}`                              |
| `home.rs`     | `stats_reports_zeros_on_empty_world`                                     | `GET /api/stats` → 200 + all zero counters                            |
| `home.rs`     | `stats_counts_folders_and_places`                                        | `GET /api/stats` → patients/folders/places > 0 after seeding          |
| `folders.rs`  | `list_filters_by_query`                                                  | `GET /api/folders?q=...` returns matching items + `query` echo        |
| `folders.rs`  | `list_filters_by_nhs_number`                                             | `GET /api/folders?nhs_number=...` returns only that patient's folders |
| `folders.rs`  | `create_folder_creates_patient_via_main_patient_service_and_returns_201` | `POST /api/folders` → 201 + Location + patient seeded upstream        |
| `folders.rs`  | `create_folder_attaches_to_existing_main_patient_service_patient`        | `patient_name`/`date_of_birth` are optional when patient exists       |
| `folders.rs`  | `create_folder_with_invalid_nhs_returns_422`                             | `POST /api/folders` Modulus-11 failure → 422 + errors body            |
| `folders.rs`  | `show_unknown_folder_returns_404`                                        | `GET /api/folders/{unknown}` → 404 + `{"error":"Folder not found"}`   |
| `folders.rs`  | `folder_history_lists_events_for_folder`                                 | `GET /api/folders/{id}/history` → 200 + items envelope                |
| `moves.rs`    | `folder_lookup_by_nhs_returns_matching_folders`                          | `GET /api/folders?nhs_number=...` is the live-lookup endpoint         |
| `moves.rs`    | `folder_lookup_by_unknown_nhs_returns_empty_list`                        | unknown NHS Number → 200 + empty list                                 |
| `moves.rs`    | `workers_list_returns_workers_from_main_worker_service`                  | `GET /api/workers` returns the stub Main Worker Service rows          |
| `moves.rs`    | `create_move_records_worker_snapshot`                                    | `POST /api/moves` with `worker_id` snapshots name + role              |
| `moves.rs`    | `create_move_falls_back_to_free_text_when_no_worker_id`                  | blank `worker_id` + `moved_by` text → free-text snapshot              |
| `moves.rs`    | `create_move_with_invalid_folder_id_returns_422`                         | malformed UUID → 422 + errors body                                    |
| `moves.rs`    | `create_move_with_unknown_folder_returns_404`                            | unknown folder UUID → 404                                             |
| `moves.rs`    | `moves_list_returns_audit_log`                                           | `GET /api/moves` returns the move event we just posted                |
| `patients.rs` | `list_returns_patients_from_folder_snapshots`                            | `GET /api/patients` derives the list from Main Thing Service folders  |
| `patients.rs` | `show_lists_folders_for_main_patient_service_patient`                    | `GET /api/patients/{nhs}` → `patient_service_match: true` + folders   |
| `patients.rs` | `show_falls_back_to_snapshot_when_main_patient_service_has_no_record`    | unknown patient → `patient_service_match: false` + snapshot folders   |
| `places.rs`   | `index_lists_buildings_rooms_and_cabinets`                               | `GET /api/places` returns the three grouped arrays                    |
| `places.rs`   | `index_kind_filter_hides_other_groups`                                   | `GET /api/places?kind=cabinet` empties the other arrays               |
| `places.rs`   | `show_returns_place_details_and_folders_inside_cabinet`                  | `GET /api/places/{cabinet-id}` includes folders parked inside         |
| `places.rs`   | `show_unknown_place_returns_404`                                         | unknown UUID → 404                                                    |
| `places.rs`   | `create_registers_a_new_place_in_the_main_place_service`                 | `POST /api/places` → 201 + Location + upstream row                    |
| `places.rs`   | `create_with_missing_name_returns_422`                                   | empty name → 422 + errors body                                        |
| `workers.rs`  | `list_returns_workers_from_stub_service`                                 | `GET /api/workers` returns stub worker rows + roles                   |
| `workers.rs`  | `list_filters_by_query`                                                  | `GET /api/workers?q=Bob` filters by name                              |

## CI gates (run all four before PR)

```bash
cargo check
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo test                               # unit tests; request tests report as ignored
DATABASE_URL=postgres://postgres@localhost:5432/case_folder_test \
  cargo test -- --ignored                # the 50 request tests
```

Postgres only needs to be reachable — no tables exist.

## Dev, build, deploy

```bash
# First-time setup
createdb case_folder_development
export DATABASE_URL=postgres://postgres:postgres@localhost:5432/case_folder_development

# Iterate
cargo run -- start                       # dev server (JSON API on :5150)
cargo run -- routes                      # print route table
cargo test                               # unit tests (request tests are #[ignore]d)
DATABASE_URL=… cargo test -- --ignored   # the DB-backed request tests
cargo clippy --all-targets -- -D warnings
cargo fmt --check

# Production build
cargo build --release
./target/release/case-folder-service-with-rust-cli start
```

### Required before merging

1. `cargo check` is green.
2. `cargo clippy -- -D warnings` is green.
3. `cargo fmt --check` is green.
4. `cargo test` is green, and `cargo test -- --ignored` is green
   against a reachable `case_folder_test` database.
5. `cargo run -- start` boots and serves `GET /healthz` with HTTP 200,
   and `POST /api/folders` + `POST /api/moves` round-trip end-to-end
   (verify with `curl`).
6. Any new use case is documented in [routes.md](routes.md).
7. Any new response shape is added to `src/responses/mod.rs` and echoed
   in [api-contract.md](api-contract.md) / [examples.md](examples.md).
