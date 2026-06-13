# Tasks (Loco edition delivery)

> Part of the [Loco edition specification](index.md). Cross-edition
> board: [root tasks](../../spec/tasks.md). Tasks trace to
> [requirements.md](requirements.md) (AR-) and [design.md](design.md) (LD-).

## Status legend

- [x] done · [~] in progress · [ ] not started

## Delivered (in repo)

- [x] **LT-1** Five `Client` traits + `Http` + `Stub` impls + RoutingClient (LD-1) — AR-8, AR-9
- [x] **LT-2** Typed `responses::` structs + 404/422/503 helpers (LD-2) — AR-4
- [x] **LT-3** Controllers: healthz, stats, folders, moves, patients, places, workers (LD-2) — AR-1
- [x] **LT-4** `POST /api/moves` with worker snapshot + free-text fallback (LD-3) — AR-2
- [x] **LT-5** `POST /api/folders` find-or-create patient + initial event (LD-3) — AR-3
- [x] **LT-6** `nhs.rs` Modulus 11 validator + 6 unit tests (—) — AR-5
- [x] **LT-7** Soft-fail stats + patient lookup; 503 elsewhere (LD-6) — AR-6, AR-7
- [x] **LT-8** Stub mode + seed task (3 buildings / 4 rooms / 5 cabinets / 6 patients / 9 folders) (LD-5) — AR-8
- [x] **LT-9** 29 request tests against stub upstreams (LD-1) — AR-8
- [x] **LT-10** Vestigial Postgres / empty Migrator (LD-4) — AR-9

## Active / next

- [ ] **LT-11** OpenAPI / JSON Schema document for every endpoint (P1) — AR-4
- [ ] **LT-12** SSE on `/api/moves` (P1)
- [ ] **LT-13** Soft-delete cabinets, refusing while occupied (P2)
- [ ] **LT-14** CSV / FHIR export of the audit log (P2)
- [ ] **LT-15** NHS Spine PDS lookup on folder registration (P3)

## Production gates

- [ ] **LT-G1** Auth (CIS2 / OIDC) + RBAC via Loco auth layer (P0)
- [ ] **LT-G2** Append-only chained-signature audit storage on a worker (P0)
- [ ] **LT-G3** API versioning via `Accept` mediatype (P0)
- [ ] **LT-G4** `auto_migrate: false`, no dangerous truncate/recreate, `--release`, secrets manager, reverse-proxy TLS/HSTS — see [regulatory.md](regulatory.md)

## Recipe: add an endpoint

1. Add route + method to [routes.md](routes.md); add a use case to
   [root requirements](../../spec/requirements.md) if new.
2. Add a `Serialize` struct in `src/responses/mod.rs` if a new shape.
3. Create/extend the controller; register it in `mod.rs` + `app.rs`.
4. Add request tests in `tests/requests/<resource>.rs` (happy + one failure).
5. Run the CI gates in [testing.md](testing.md).
