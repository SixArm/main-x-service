# Contact Relationship Management service — edition spec

Stack-specific specification for the Loco edition. The
**cross-cutting spec at [`../../spec/`](../../spec/index.md) is the
single source of truth** for the domain (model, modules, state
machines, auth posture); this file adds only what is specific to
this edition. It will grow topic files (routes, api-contract,
database, examples) as implementation phases land — each CRM-T* task
adds its spec detail here in the same PR as the code.

## Stack

Per [architecture](../../spec/architecture.md) and the family
[rust-loco-stack](../../../agents/share/rust-loco-stack.md):
Rust 2024, Loco (Axum + SeaORM), PostgreSQL 18, crate-root
`migration/`, loco-idiomatic `src/controllers/` layout, pure
`src/rules/` core, `bg_pg` jobs (nurture scheduler, SLA sweep,
campaign send), stub-first upstream clients, offline PASETO + ABAC,
OpenAPI/Swagger, `Accepts-version` header versioning, tracing +
OTLP, Podman.

## Edition-specific decisions (so far)

- **Config**: `config/{development,test,production}.yaml`;
  development and test default upstream clients to `stub` and
  `CRM_EVENT_TRANSPORT=memory`.
- **Env vars**: `CRM_REQUIRE_AUTH`, `CRM_PASETO_KEYS[_URL]`,
  `CRM_ABAC_POLICY[_FILE]`, `CRM_EVENT_TRANSPORT`, upstream base
  URLs (`CRM_PERSON_SERVICE_URL`, `CRM_ORGANIZATION_SERVICE_URL`,
  `CRM_WORKER_SERVICE_URL`), `CRM_UPSTREAM_MODE` (default `stub`),
  `CRM_STALLED_DAYS` (default 14), scoring weight overrides.
- **Identifiers**: public UUID `pid` on every owned record; EntityRef
  URNs for all upstream references.
- **Table names all-plural** (the loco `create_table` pluralization
  lesson); `event_outbox` as explicit SQL.

## Delivery

The queue is [../../spec/tasks.md](../../spec/tasks.md): CRM-T1–T16
for this edition. Tests per
[../../spec/testing.md](../../spec/testing.md). Nothing implemented
yet.
