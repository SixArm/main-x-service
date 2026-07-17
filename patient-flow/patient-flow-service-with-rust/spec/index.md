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

## Delivery

The queue is [../../spec/tasks.md](../../spec/tasks.md): PF-T1–T4
skeleton/topology, PF-T5–T8 bed states + journey, PF-T9–T11 demand +
reads + capacity, PF-T12–T13 IPC + virtual ward, PF-T14 auth
surface. Tests per [../../spec/testing.md](../../spec/testing.md).
