# AGENTS.md — working agreements

A pocket guide for human and AI collaborators working in this
subproject. Read this **before** opening a PR.

## What this project is

A **back-end JSON API**, written in Rust on
[Loco](https://loco.rs)/Axum, that tracks the physical location of
paper case-note folders for a UK NHS hospital setting. There is **no
built-in UI** — front-ends live elsewhere and consume this service
over HTTP.

The tracker is a **pure aggregator** — it owns no domain tables.
Patients, places, workers, folders, and move events all live in five
external "Main-X-Service" HTTP services. Every controller proxies
those services and snapshots labels onto the records it writes so
audit data survives upstream outages.

Full spec is in [`spec/`](spec/index.md). Prefer reading that to
inferring from the code.

## Repository orientation

| Where                          | What                                                      |
| ------------------------------ | --------------------------------------------------------- |
| `src/app.rs`                   | `impl Hooks for App` — initializers + route composition   |
| `src/controllers/`             | Axum handlers, one module per resource                    |
| `src/responses/mod.rs`         | Every wire struct + error helpers (404/422/503)           |
| `src/main_*_service/`          | Client trait + `http::HttpClient` + `stub::StubClient`    |
| `src/initializers/`            | Wires upstream clients into the Axum extension layer      |
| `src/nhs.rs`                   | Modulus 11 NHS Number validator + formatter               |
| `tests/requests/`              | Request-level integration tests (stub upstreams)          |
| `config/`                      | Dev / test / production YAML                              |
| `migration/`                   | Empty SeaORM migrator (kept so Loco's boot still works)   |

## Working rules

### 1. Keep the API JSON-only

No HTML, no templates, no static assets, no client-side JS, no CSS.
The `tera` dependency is **not** in `Cargo.toml`. If you find yourself
about to add view templates, stop — front-ends consume the API, they
do not live here.

### 2. Match the response style in `src/responses/mod.rs`

Every endpoint's response goes through a `Serialize` struct in
`responses::` (or a controller-local struct that composes them). Don't
build ad-hoc `serde_json::json!({...})` blobs in controllers —
strongly-typed structs are the wire contract and give clients
schema-stable output.

### 3. HTTP semantics

- `GET` list endpoints → `{ "items": [...], "query": "..." }` envelope.
- `GET` single resource → resource body, no envelope.
- `POST` create → `201 Created` + `Location: /api/.../{id}` + resource body.
- Validation failure → `422` + `{ "errors": { "field": "message" } }`.
- Resource not found → `404` + `{ "error": "..." }`.
- Upstream service unreachable → `503` + `{ "error": "..." }`.
- Soft-fail exceptions (intentional): `GET /api/stats` and
  `GET /api/patients/{nhs}` — see spec §11.

### 4. Snapshot every cross-service field at write time

The audit trail must keep working when an upstream renames, deletes,
or goes offline. When a controller writes a new folder / move event,
it must capture the patient name, NHS Number, cabinet path, worker
name, and worker role from whatever the upstream authoritatively
returned at that moment. Never refresh snapshots after the fact.

### 5. Routes go under `/api`, no version segment

API versioning happens via HTTP (`Accept` mediatype), not URL prefix.
`/healthz` is the only exception — it lives at the root for
convention.

### 6. Tests use stub clients only

`tests/requests.rs` registers in-process `StubClient` impls for all
five upstream services via the `set_test_client` slot pattern. Every
test:

1. Is marked `#[tokio::test]` + `#[serial]`.
2. Calls `super::clean_db(&ctx).await` first to reset the stubs.
3. Asserts a status code and parses the JSON body — **never** matches
   HTML strings.

If you add a controller, add a request test in the matching file.

### 7. Don't add `tera`, `tower-livereload`, asset middleware, etc.

These were intentionally removed. The `server.middlewares.static`
block in all three configs is set to `enable: false` for the same
reason.

### 8. Stub mode is a real feature, not test scaffolding

The `USE_UPSTREAM_STUBS` env var registers in-process `StubClient`s
for every Main-X-Service and runs the seed task against them. The
SvelteKit Playwright suite depends on this. Keep the
`bootstrap_stubs` initializer in lock-step with any change to the
seeding logic or the Client traits.

### 9. CI gates (run all four before PR)

```bash
cargo check
cargo clippy --all-targets -- -D warnings
cargo fmt --check
DATABASE_URL=postgres://postgres@localhost:5432/case_folder_test cargo test
```

Postgres only needs to be reachable — no tables exist.

## Common tasks

### Add a new endpoint

1. Decide route + method. Add it to
   [`spec/routes.md`](spec/routes.md) (use-case mapping + route table).
2. If new response shape needed: add a `Serialize` struct in
   `src/responses/mod.rs`.
3. Create / extend the controller in `src/controllers/<resource>.rs`.
4. Register the controller in `src/controllers/mod.rs` (if new file)
   and `src/app.rs` `routes()`.
5. Add request tests in `tests/requests/<resource>.rs`. At minimum:
   happy path + one failure (`404` or `422`).
6. Run the CI gates.

### Add a new upstream service

Don't, unless the spec really demands it. If you must:

1. New module `src/main_<name>_service/{mod.rs, http.rs, stub.rs}`
   following the existing five.
2. New initializer in `src/initializers/main_<name>_service_client.rs`
   with a `set_test_client` slot.
3. Register the initializer in `src/app.rs` and add a public
   `set_test_client` re-export so tests can inject a stub.

### Drop a route

1. Remove the controller route entry.
2. Remove from `src/controllers/mod.rs` if the file becomes empty.
3. Update the matching `spec/` topic files: [routes.md](spec/routes.md),
   [api-contract.md](spec/api-contract.md), [examples.md](spec/examples.md),
   [testing.md](spec/testing.md).
4. Remove the matching request tests.

## Style

- `cargo fmt` formats. Don't second-guess it.
- Inline doc comments (`///`) on every public type + function in
  `src/responses` and `src/controllers`. Keep them short.
- Module-level `//!` doc comments on every controller file explain
  what the routes do at a high level — keep them current.
- Don't add prose comments for what well-named code already says.
- One short line of `// reason: ...` is fine when the reasoning is
  non-obvious (a workaround, a hidden upstream constraint).

## Sibling projects

- [`../case-folder-front-end-with-svelte`](../case-folder-front-end-with-svelte)
  — same domain, SvelteKit + TypeScript front-end, and a **client of
  this API**. It calls this service under `/api/*`, and its Playwright
  e2e suite runs against this service booted in stub mode
  (`USE_UPSTREAM_STUBS=1`), so the API is fully functional with no
  external upstreams. Keep the JSON contract stable for it.
- The five upstream services live under `~/git/sixarm/main-x-service/`.
