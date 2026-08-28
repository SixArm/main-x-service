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

## Edition-specific implementation notes (as landed)

- **Layout**: `src/{app,auth,clients,metrics,openapi,streaming,
  validation,version}.rs`, `src/rules/` (pure core: tokens,
  lifecycle, scoring, analytics, sla, segment, engagement),
  `src/models/` (+`_entities/`, 23 tables), `src/controllers/
  {relationships,sales,marketing,support,dashboards,insights,
  engagement,audits,docs,metrics}.rs`, `src/tasks/seed.rs`,
  crate-root `migration/` (7, explicit SQL — the 7th,
  `m20260720_000007_engagement`, added CRM-T20's stakeholder/
  partnership/membership/working-group tables).
- **Deal stages are data**, not a token table: `pipeline_stages`
  rows with probabilities + terminal flags; stage moves validate
  pipeline membership on the locked deal row.
- **Consent** is enforced in the segment evaluator (structural
  AND-gate) *and* re-checked at send time by both send paths; the
  `consent_events` table is append-only (no `deleted_at`).
- **Jobs**: `POST /api/nurture/advance` (idempotent per enrolment ×
  step) and `POST /api/sla/sweep` (once per breach) are the v1
  scheduler surface; a periodic `bg_pg` worker is the roadmap seam.
- **Dashboards** are ETag-conditional (weak tag over the payload
  minus `as_of`); ratios carry numerator/denominator and go `null`
  on a zero denominator; currencies never merge.
- **Enforcement tests** live in `tests/enforcement.rs` (own process,
  OnceLock lesson); request tests are `#[ignore]`d — run with
  `cargo test -- --ignored`.

## Delivery

<!-- PRO-H8, 2026-08-28: was stale at "CRM-T1-T16 delivered 2026-07-18" only.
     spec/tasks.md shows four further service-side rounds landed since:
     CRM-T19 (insights) + CRM-T20 (engagement), both 2026-07-20; CRM-T21
     (subject rights/retention) + CRM-T22 (auth activation surface), both
     2026-08-28. Verified against tasks.md and `cargo test --lib -- --list`
     (67 unit tests, matching CRM-T21/T22's stated count). -->
The queue is [../../spec/tasks.md](../../spec/tasks.md): CRM-T1–T16
**delivered 2026-07-18**; CRM-T19 (insight views) and CRM-T20
(engagement/partnership/confederation) **delivered 2026-07-20**;
CRM-T21 (subject rights & retention) and CRM-T22 (auth activation
surface) **delivered 2026-08-28**. Production gates CRM-G1/CRM-G2
remain partially open — see tasks.md for what each gate's code side
did and did not land. Tests per
[../../spec/testing.md](../../spec/testing.md).
