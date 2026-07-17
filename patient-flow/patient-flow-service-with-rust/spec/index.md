# Patient Flow service — edition spec

Stack-specific specification for the Loco edition. The
**cross-cutting spec at [`../../spec/`](../../spec/index.md) is the
single source of truth** for the domain (model, state machines,
journeys, views, auth posture); this file adds only what is specific
to this edition. It will grow topic files (routes, api-contract,
database, examples) as implementation phases land — each PF-T* task
adds its spec detail here in the same PR as the code.

## Stack

Per [architecture](../../spec/architecture.md) and the family
[rust-loco-stack](../../../agents/share/rust-loco-stack.md):
Rust 2024, Loco (Axum + SeaORM), PostgreSQL 18, crate-root
`migration/`, loco-idiomatic `src/controllers/` layout, pure
`src/flow/` core, stub-first upstream clients, offline PASETO +
ABAC, OpenAPI/Swagger, `Accepts-version` header versioning,
tracing + OTLP, Podman.

## Edition-specific decisions (so far)

- **Config**: `config/{development,test,production}.yaml`;
  development and test default all upstream clients to `stub` and
  `PATIENT_FLOW_EVENT_TRANSPORT=memory`.
- **Env vars**: `PATIENT_FLOW_REQUIRE_AUTH`,
  `PATIENT_FLOW_PASETO_KEYS[_URL]`, `PATIENT_FLOW_ABAC_POLICY[_FILE]`,
  `PATIENT_FLOW_EVENT_TRANSPORT`, upstream base URLs
  (`PATIENT_FLOW_PERSON_SERVICE_URL`, …), DTOC grace
  (`PATIENT_FLOW_DTOC_GRACE`, default `midnight`).
- **Identifiers**: public UUID `pid` on every owned record; EntityRef
  URNs for all upstream references.

## Edition-specific implementation notes

- **Layout**: `src/{app,auth,clients,metrics,openapi,streaming,
  validation,version}.rs`, `src/flow/` (pure core), `src/models/`
  (+`_entities/`), `src/controllers/{topology,stays,bed_requests,
  boards,audits,docs,metrics}.rs`, `src/tasks/seed.rs`, crate-root
  `migration/`.
- **Migration gotcha**: loco's `create_table` helper **pluralizes**
  table names via `cruet::to_plural` — every patient-flow table name
  is already plural so this is a no-op, except `event_outbox`, whose
  migration is written as explicit SQL to keep the exact name the
  SeaORM entity declares. (The sibling services' copied
  `event_outbox` migrations share this hazard.)
- **Clients**: one `src/clients.rs` module (mode-switched `stub` /
  `http` via `PATIENT_FLOW_UPSTREAM_MODE`, default `stub`) rather
  than per-service trait modules — patient-flow needs only
  display-name resolution.
- **`/events/recent`** reads the in-memory ring under the `memory`
  transport and the `event_outbox` table under `outbox`.

## Delivery

The queue is [../../spec/tasks.md](../../spec/tasks.md): PF-T1–T14
**delivered 2026-07-18** (see the phase note there); open: PF-T17
request-test suite, PF-T15/T16 front-end. Tests per
[../../spec/testing.md](../../spec/testing.md).
