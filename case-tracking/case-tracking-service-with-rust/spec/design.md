# Design (Loco edition)

> Part of the [Loco edition specification](index.md). System-level
> decisions: [root design](../../spec/design.md). This file records the
> **Loco-specific** decisions that satisfy [requirements.md](requirements.md).

## LD-1 — Client trait + Http + Stub per upstream (AR-8, AR-9)

Each of the five Main-X-Services is fronted by a `Client` trait with two
impls: a `reqwest`-backed `HttpClient` and an in-process `StubClient`. A
private `RoutingClient` reads a `Mutex<Option<Arc<dyn Client>>>` slot at
request time, so tests (and stub mode) inject a stub while the live app
falls through to HTTP. _Satisfies:_ testability + the no-local-data rule.

## LD-2 — Typed response structs, no ad-hoc JSON (AR-4)

Every endpoint serialises through a `Serialize` struct in
`src/responses/mod.rs` (or a controller-local composition of them).
Controllers never build `serde_json::json!({...})` blobs. _Satisfies:_
schema-stable wire output.

## LD-3 — Snapshot at the controller boundary (AR-2, AR-9)

When a controller creates a folder or records a move, it calls
`label_path` (Place) and `find_by_id` (Worker/Thing) once and writes the
returned labels into the Thing `keywords`/`identifiers` and the Event
`keywords`. Snapshots are never refreshed afterwards.

## LD-4 — Vestigial Postgres (AR-9)

`Migrator::migrations()` returns `vec![]`; Postgres exists only because
`create_app::<App, Migrator>` requires an `AppContext.db`. No schema is
created or read. _Satisfies:_ the aggregator decision without fighting
Loco's boot contract.

## LD-5 — Stub-mode initializer reuses the seed path (AR-8)

`USE_UPSTREAM_STUBS=1` registers stub clients and runs the **same**
`run_seed` the `Seed` task uses, so stub data shape == real data shape.
Stub mode is a real feature, not test scaffolding — the Svelte e2e suite
depends on it.

## LD-6 — Soft-fail only where partial beats nothing (AR-6, AR-7)

Default `503` on upstream failure. Two explicit exceptions return partial
data: `/api/stats` (zeros for the dead slice) and `/api/patients/{nhs}`
(snapshot fallback). Implemented with the `responses::service_unavailable`
helper everywhere else.

## LD-7 — `/api` prefix, version via `Accept` (AR-1)

All routes sit under `/api` with no version segment; versioning is an
`Accept` mediatype concern. `/healthz` is the sole root exception.

## Requirement → design trace

| Requirement | Satisfied by   |
| ----------- | -------------- |
| AR-1, AR-4  | LD-2, LD-7     |
| AR-2, AR-3  | LD-3           |
| AR-5        | LD-2 + `nhs.rs`|
| AR-6, AR-7  | LD-6           |
| AR-8        | LD-1, LD-5     |
| AR-9        | LD-1, LD-3, LD-4 |
