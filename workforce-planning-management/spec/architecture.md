# Architecture

```
 employee / manager / HR browser
        │  (cookie session; no token in JS)
        ▼
 workforce-planning-management-front-end-with-svelte  (SvelteKit BFF)
        │  Authorization: Bearer v4.public.…
        ▼
 workforce-planning-management-service-with-rust  (Loco: Axum + SeaORM + PostgreSQL)
        │  EntityRef lookups (read-only, cached, stub-able)
        ▼
 person / worker / organization / course / authentication services
```

## Service edition

Loco-idiomatic layout (the patient-flow shape — the closest sibling):

```
src/
├── app.rs                loco Hooks
├── controllers/          acquisition, adjustments, appraisals,
│                         assessments, audits, development, docs,
│                         ergonomics, hr_core, intelligence,
│                         learning, metrics, notifications, payroll,
│                         privacy, talent, wellbeing, workforce
├── models/               helpers (+ notifications push) + _entities/
├── clients.rs            stub-first upstream lookups (display names,
│                         birth dates — cached, never stored)
├── rules/                pure core: every lifecycle machine
│                         (requisition, application, review, payroll
│                         run, mentorship, appraisal, adjustment,
│                         placement, …), leave/time/scheduling
│                         arithmetic, working-time guardrails,
│                         wellbeing eligibility + prompt machine,
│                         pulse k-floor, 360 group floor, assessment
│                         category↔scale map, DSE completion gate,
│                         erasure/retention rules, payslip
│                         arithmetic, org-chart cycle check
├── auth.rs               offline PASETO + ABAC + personas + mask
├── streaming.rs          envelope + memory/outbox transports
├── validation.rs         caps + tokens + URN shapes → 422
└── openapi.rs            OpenAPI 3 doc
migration/                sea-orm-migration (crate root, 17 sets)
config/abac-policy.reference.json   the shipped, matrix-verified
                                    persona policy (WPM-G1 runbook)
```

Key decisions (numbered in [design.md](design.md)): normalized
relational schema (constraints and lifecycles, not DTO-as-JSONB);
every state machine and money calculation in the DB-free pure core;
minor-unit money; ETag-conditional dashboards; family fixtures
(`#![forbid(unsafe_code)]`, clippy-pedantic, OTLP, `Accepts-version`,
Podman). **All-plural table names** (the loco `create_table`
pluralization gotcha is documented family knowledge).

## Front-end edition

SvelteKit 2 + Svelte 5 runes SPA + same-origin BFF proxy
(patient-flow/PPM pattern), dependency-light, 13-locale i18n from
the start (the PPM lesson: retrofitting costs more). Views per
pillar: requisition/application boards, onboarding tracker, team
calendar + rota + working-time and ergonomic-issue panels, the
employee profile (a self-service hub: wellbeing prompts, pulse,
notifications, 360s + "my 360 requests", ergonomics, reasonable
adjustments, subject-access download, erase action) + org chart,
review and enrollment panels, `/wellbeing` (entitlement rules,
uptake, pulse results), `/privacy` (retention report + sweep),
payroll run screen, benchmarking table, and the HR dashboard.
