# Architecture

```
 employee / manager / HR browser
        │  (cookie session; no token in JS)
        ▼
 human-capital-management-front-end-with-svelte  (SvelteKit BFF)
        │  Authorization: Bearer v4.public.…
        ▼
 human-capital-management-service-with-rust  (Loco: Axum + SeaORM + PostgreSQL)
        │  EntityRef lookups (read-only, cached, stub-able)
        ▼
 person / worker / organization / course / authentication services
```

## Service edition

Loco-idiomatic layout (the patient-flow shape — the closest sibling):

```
src/
├── app.rs                loco Hooks
├── controllers/          acquisition, workforce, hr_core,
│                         development, payroll, boards, audits,
│                         docs, metrics
├── models/               helpers + _entities/ (SeaORM)
├── clients.rs            stub-first upstream display-name lookups
├── rules/                pure core: pipelines (requisition,
│                         application, review, payroll-run), leave
│                         balances, overtime, shift conflicts,
│                         payslip arithmetic, org-chart cycle check
├── auth.rs               offline PASETO + ABAC + personas + mask
├── streaming.rs          envelope + memory/outbox transports
├── validation.rs         caps + tokens + URN shapes → 422
└── openapi.rs            OpenAPI 3 doc
migration/                sea-orm-migration (crate root)
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
calendar + rota, employee profile + org chart, review and
enrollment panels, payroll run screen, benchmarking table, and the
HR dashboard.
