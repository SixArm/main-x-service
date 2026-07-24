# Workforce Planning Management — Loco JSON API

A back-end **JSON API** for workforce planning management across the full
employee lifecycle: requisitions and applicant tracking, onboarding,
employee records and org charts, time & attendance, leave, shift
scheduling, benefits, performance reviews, training, assessments
(aptitude / personality / psychometric / selection), upskilling and
reskilling plans, talent pipelines, apprenticeships and internships,
succession planning, workforce intelligence, payroll runs with
payslips, and salary benchmarking.
Implemented in Rust on [Loco](https://loco.rs) (Axum + SeaORM +
PostgreSQL). No built-in UI — the
[Svelte sibling](../workforce-planning-management-front-end-with-svelte/)
provides the HR, manager, and employee self-service client.

> ⚠️ **Demo software.** Not a production HR or payroll system;
> statutory calculations are illustrative stubs; synthetic data
> only. See [spec/regulatory](../spec/regulatory.md).

**Status: implemented (WPM-T1–T17, 2026-07-18).** Builds, 71 DB-free
unit tests + 7 request tests + the enforcement persona matrix pass
against Postgres 18, clippy-pedantic clean, live smoke verified
(migrate → seed → org chart → payroll → benchmarks). Remaining in
[../spec/tasks.md](../spec/tasks.md): the front-end (WPM-T18/T19).

## What it answers

- _Where is this vacancy in its pipeline?_ — requisition +
  application state machines
- _Can this employee take two weeks in August?_ — leave balances +
  rota conflicts
- _Who reports to whom?_ — the derived org chart
- _What does this month's payroll cost?_ — calculated runs with
  per-employee payslips (minor-unit arithmetic, stub tax tables)
- _Which critical roles have no ready successor?_ — the succession
  gap report and the single-points-of-failure list
- _How did this candidate do on the selection tests?_ — the
  assessment profile (reported, never a recommendation)
- _Is this person's development plan actually happening?_ — declared
  progress next to progress verified against declared proficiency
- _Has this apprentice met their off-the-job training hours?_ — the
  completion gate that refuses to say otherwise
- _Where is the workforce thin?_ — the workforce-intelligence views,
  every rate carrying its numerator and denominator

## Surface

Requisitions / candidates / applications / interviews · onboarding
items · employees + org-chart · time entries · leave entitlements +
requests · shifts + assignments · benefit plans + enrollments ·
review cycles / reviews / goals / feedback · training enrollments ·
assessment instruments / sittings / results + profiles · development
plans (upskill / reskill) · talent pipelines · early-career
programmes + placements · succession plans · workforce intelligence ·
payroll runs + payslips · benchmarks · audits ·
`/events/recent` · OpenAPI + Swagger · `/metrics.prom`.

Auth enforcement defaults **off** (`WPM_REQUIRE_AUTH` is the family
activation gate); upstream lookups default to **stub mode**; events
default to the in-memory transport.

## Upgrading across the 2026-07-23 rename

This project was renamed from *human capital management* (`HCM`) to
**workforce planning management** (`WPM`). Three of the four changes are
handled for you; one needs an operator action.

| Change | What happens |
|---|---|
| Env prefix `HCM_*` → `WPM_*` | **Automatic.** The old names still work and log a one-off deprecation warning naming the replacement (`src/compat.rs`). Rename them at your leisure; the fallback will be removed. |
| ABAC entity `"hcm"` → `"wpm"` | **Automatic.** A mounted policy whose rules key on `entity: "hcm"` is rewritten at load and warn-logged. This one matters: a stale entity condition would *silently stop matching* and fall through to the default decision. |
| Front-end theme / locale | **Automatic.** A returning user's `mxi.hcm.*` localStorage values are adopted under `mxi.wpm.*` on their next visit, so nobody's language or theme resets. |
| **Database name** | **Manual — see below.** |

The service's default database name changed
(`human_capital_management_service*` → `workforce_planning_management_service*`).
Nothing can rename an existing database for you, so either point
`DATABASE_URL` at the existing one (no rename needed), or rename it:

```sql
ALTER DATABASE human_capital_management_service            RENAME TO workforce_planning_management_service;
ALTER DATABASE human_capital_management_service_test       RENAME TO workforce_planning_management_service_test;
ALTER DATABASE human_capital_management_service_development RENAME TO workforce_planning_management_service_development;
```

A `DATABASE_URL` set explicitly (the usual deployment shape) overrides
the default and is unaffected either way.

## Quick start

```bash
# Postgres 18 with a loco user, then:
export DATABASE_URL=postgres://loco:loco@localhost:5432/workforce_planning_management_service_development
cargo run -- db migrate       # create the schema
cargo run -- task seed        # synthetic demo org (40 employees)
cargo run -- start            # serve on :5150
curl "localhost:5150/api/org-chart?organization=<org-urn>" | jq .
```
