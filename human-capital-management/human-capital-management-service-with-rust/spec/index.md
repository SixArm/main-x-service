# Human Capital Management service — edition spec

Stack-specific specification for the Loco edition. The
**cross-cutting spec at [`../../spec/`](../../spec/index.md) is the
single source of truth** for the domain (model, pillars, state
machines, auth posture); this file adds only what is specific to
this edition. It will grow topic files (routes, api-contract,
database, examples) as implementation phases land — each HCM-T* task
adds its spec detail here in the same PR as the code.

## Stack

Per [architecture](../../spec/architecture.md) and the family
[rust-loco-stack](../../../agents/share/rust-loco-stack.md):
Rust 2024, Loco (Axum + SeaORM), PostgreSQL 18, crate-root
`migration/`, loco-idiomatic `src/controllers/` layout, pure
`src/rules/` core, stub-first upstream clients, offline PASETO +
ABAC, OpenAPI/Swagger, `Accepts-version` header versioning,
tracing + OTLP, Podman.

## Edition-specific decisions (so far)

- **Config**: `config/{development,test,production}.yaml`;
  development and test default upstream clients to `stub` and
  `HCM_EVENT_TRANSPORT=memory`.
- **Env vars**: `HCM_REQUIRE_AUTH`, `HCM_PASETO_KEYS[_URL]`,
  `HCM_ABAC_POLICY[_FILE]`, `HCM_EVENT_TRANSPORT`, upstream base
  URLs (`HCM_PERSON_SERVICE_URL`, `HCM_WORKER_SERVICE_URL`,
  `HCM_ORGANIZATION_SERVICE_URL`, `HCM_COURSE_SERVICE_URL`),
  `HCM_UPSTREAM_MODE` (default `stub`).
- **Identifiers**: public UUID `pid` on every owned record; EntityRef
  URNs for all upstream references; employee number unique per
  organization.
- **Table names all-plural** (the loco `create_table` pluralization
  lesson); `event_outbox` as explicit SQL.

## Delivery

The queue is [../../spec/tasks.md](../../spec/tasks.md): HCM-T1–T17
for this edition. Tests per [../../spec/testing.md](../../spec/testing.md).
Nothing implemented yet.
