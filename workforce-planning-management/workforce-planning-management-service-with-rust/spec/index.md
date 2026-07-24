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

## Edition-specific implementation notes (as landed)

- **Layout**: `src/{app,auth,clients,metrics,openapi,streaming,
  validation,version}.rs`, `src/rules/` (pure core), `src/models/`
  (+`_entities/`), `src/controllers/{hr_core,acquisition,workforce,
  development,payroll,audits,docs,metrics}.rs`, `src/tasks/seed.rs`,
  crate-root `migration/` (7 migrations, explicit SQL).
- **Masking**: `mask_employee` clears `salary_minor`+currency;
  `mask_payslip` zeroes amounts and drops the deduction lines;
  payslip reads authorize against the **owning employee's** resource
  attrs, so one policy masks both.
- **Ownership**: `resource.person` carries the bare person uuid so a
  `{"resource.person": ["$sub"]}` rule gives employees their own
  records (deployments map user pid = person uuid).
- **Payroll**: `calculate` replaces the run's payslips inside one
  transaction; only `approved` time entries feed overtime; benefit
  costs join only in the payslip currency; stub tax = 20% over a
  monthly allowance (documented demo stub).
- **Enforcement tests** live in `tests/enforcement.rs` (own process,
  OnceLock lesson); request tests in `tests/requests/` are
  `#[ignore]`d — run with `cargo test -- --ignored`.

## Delivery

The queue is [../../spec/tasks.md](../../spec/tasks.md): HCM-T1–T17
**delivered 2026-07-18**. Tests per
[../../spec/testing.md](../../spec/testing.md).
