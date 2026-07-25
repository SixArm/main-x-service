# Workforce Planning Management — Loco JSON API

A back-end **JSON API** for workforce planning management across the full
employee lifecycle: requisitions and applicant tracking, onboarding,
employee records and org charts, time & attendance, leave, shift
scheduling with working-time guardrails, benefits, wellbeing &
benefits-awareness prompts, the anonymous pulse, performance reviews
and 360° multi-rater appraisals with in-app notifications, training,
skills / learning paths / mentorships, assessments (aptitude /
personality / psychometric / selection / cognitive), upskilling and
reskilling plans, talent pipelines, apprenticeships and internships,
succession planning, workforce intelligence, ergonomic (DSE)
workstation assessments, reasonable adjustments, subject rights
(access / erasure / retention), payroll runs with payslips, and
salary benchmarking.
Implemented in Rust on [Loco](https://loco.rs) (Axum + SeaORM +
PostgreSQL). No built-in UI — the
[Svelte sibling](../workforce-planning-management-front-end-with-svelte/)
provides the HR, manager, and employee self-service client.

> ⚠️ **Demo software.** Not a production HR or payroll system;
> statutory calculations are illustrative stubs; synthetic data
> only. See [spec/regulatory](../spec/regulatory.md).

**Status: implemented (WPM-T1–T36, 2026-07-18 → 2026-07-25).** 139
DB-free unit tests + 19 request suites + the enforcement persona
matrix (mounted on the shipped reference policy) pass against
Postgres 18; clippy-pedantic clean. Both production gates' **code
sides are done** (WPM-G1 reference policy + runbook; WPM-G2 subject
rights + retention); what remains on them is operational and legal
work — see [../spec/tasks.md](../spec/tasks.md).

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
- _Who's eligible for the shingles jab, and did the prompts work?_ —
  wellbeing entitlement rules + aggregate-only uptake and enrolment
  conversion
- _How is the team actually doing?_ — the anonymous pulse, k-floored
  so no small cell can identify anyone
- _What does a full circle say about this person?_ — 360° appraisals
  with group-floored reports and rater self-service
- _Is anyone's rota heading into unlawful territory?_ — advisory
  48-hour and 11-hour-rest working-time flags
- _What's wrong with the workstations?_ — DSE checklists and the
  department issues report (equipment facts, never symptoms)
- _What change would help you do your job?_ — reasonable-adjustment
  requests: barrier / impact / change, no diagnosis needed or storable
- _What do we hold about this person, and can we forget them?_ — the
  subject-access export, erasure as anonymisation, the retention sweep

## Surface

Requisitions / candidates / applications / interviews · onboarding
items · employees + org-chart · time entries · leave entitlements +
requests · shifts + assignments · working-time guardrails · benefit
plans + enrollments · wellbeing entitlements + acknowledgements +
uptake · pulse surveys + k-floored results · review cycles / reviews /
goals / feedback · 360° appraisals + nominations + responses +
reports + rater requests · in-app notifications · training
enrollments · skills + learning paths + mentorships · assessment
instruments / sittings / results + profiles (5 categories incl.
cognitive) · development plans (upskill / reskill) · talent
pipelines · early-career programmes + placements · succession plans ·
workforce intelligence · ergonomic assessments + items + issues ·
adjustment requests + decisions · subject-access / erase / retention ·
payroll runs + payslips · benchmarks · audits · `/events/recent` ·
OpenAPI + Swagger · `/metrics.prom`.

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

Interactive docs: `http://localhost:5150/swagger-ui/` (OpenAPI at
`/api-docs/openapi.json`).

## Tutorial — a worked tour

All examples assume the server on `:5150` and `jq`. Every endpoint
also negotiates `Accepts-version: 1.0` (optional today).

**1. Hire someone.**

```bash
ORG="organization:$(uuidgen)"
EMP=$(curl -s localhost:5150/api/employees -H 'content-type: application/json' -d '{
  "person_ref": "person:'$(uuidgen)'", "organization_ref": "'$ORG'",
  "employee_number": "E-1001", "display_name": "Ada Lovelace",
  "employment_type": "permanent", "department": "engineering",
  "job_title": "Engineer", "salary_minor": 3600000,
  "salary_currency": "GBP", "hired_on": "2026-01-05" }' | jq -r .pid)
curl -s localhost:5150/api/employees/$EMP/status \
  -H 'content-type: application/json' -d '{"to":"active"}' | jq .status
```

**2. Wellbeing prompts.** A department-scoped rule; Ada sees it,
acknowledges it, HR sees only counts:

```bash
RULE=$(curl -s localhost:5150/api/wellbeing-entitlements -H 'content-type: application/json' -d '{
  "name": "Seasonal flu vaccination", "kind": "health",
  "description": "Free NHS flu jab for frontline staff.",
  "departments": ["engineering"], "doses": 2 }' | jq -r .pid)
curl -s localhost:5150/api/employees/$EMP/wellbeing-prompts | jq '.prompts[].name'
curl -s localhost:5150/api/employees/$EMP/wellbeing-acknowledgements \
  -H 'content-type: application/json' \
  -d '{"entitlement_pid":"'$RULE'","response":"booked"}' | jq .response
curl -s localhost:5150/api/wellbeing/uptake | jq '.entitlements[0].uptake_rate'
```

**3. A 360°.** Draft → nominate 3+ raters → collect (raters are
notified) → respond → share → the group-floored report:

```bash
A=$(curl -s localhost:5150/api/employees/$EMP/appraisals \
  -H 'content-type: application/json' \
  -d '{"competencies":["communication","delivery"]}' | jq -r .pid)
# … nominate manager/peer raters (POST /api/appraisals/$A/nominations),
# move to collecting, then each rater:
#   POST /api/appraisals/$A/responses {"rater_pid":…,"scores":{…}}
# and their own pending list is GET /api/employees/{pid}/appraisal-requests
curl -s localhost:5150/api/appraisals/$A | jq '.nominations[] | {display_name, group, responded}'
```

**4. Subject rights.** Everything WPM holds, in one document, with
its exclusions named:

```bash
curl -s localhost:5150/api/employees/$EMP/subject-access | jq 'keys, .exclusions'
curl -s localhost:5150/api/retention | jq '{horizon_days, expired_consent_candidates}'
```

**5. Guardrails.** Advisory only — nothing is refused:

```bash
curl -s "localhost:5150/api/workforce/working-time?department=engineering" \
  | jq '{employees_checked, flagged: [.flagged[].display_name]}'
curl -s localhost:5150/api/ergonomics/issues | jq .by_department
```

## Auth activation (production)

The shipped default is **wide open** — activation is a release gate.
Follow the runbook in [../spec/auth.md](../spec/auth.md): mount a
policy (start from
[`config/abac-policy.reference.json`](config/abac-policy.reference.json)),
point at the PASETO keys, set `WPM_REQUIRE_AUTH=1`, and verify with
`cargo test --test enforcement -- --ignored`.
