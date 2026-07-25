# Architecture

```
 rep / manager / marketing / support browser
        │  (cookie session; no token in JS)
        ▼
 contact-relationship-management-front-end-with-svelte  (SvelteKit BFF)
        │  Authorization: Bearer v4.public.…
        ▼
 contact-relationship-management-service-with-rust  (Loco: Axum + SeaORM + PostgreSQL)
        │  EntityRef lookups (read-only, cached, stub-able)
        ▼
 person / organization / worker / authentication services
```

## Service edition

Loco-idiomatic layout (the patient-flow / WPM shape):

```
src/
├── app.rs                loco Hooks (+ bg_pg workers: nurture
│                         scheduler, SLA sweep, campaign send)
├── controllers/          relationships (contacts/accounts/activities),
│                         sales (leads/pipelines/deals/forecast),
│                         marketing (campaigns/segments/nurture/consent),
│                         support (tickets/sla/articles),
│                         dashboards, audits, docs, metrics
├── models/               helpers + _entities/ (SeaORM)
├── clients.rs            stub-first upstream display-name lookups
├── rules/                pure core: lifecycle machines (lead, deal,
│                         campaign, ticket, article), lead scoring
│                         + breakdown, forecast arithmetic, ROI,
│                         CLV, SLA deadline/breach derivation,
│                         segment filter evaluation
├── auth.rs               offline PASETO + ABAC + personas + mask
├── streaming.rs          envelope + memory/outbox transports
├── validation.rs         caps + URN shapes + money → 422
└── openapi.rs            OpenAPI 3 doc
migration/                sea-orm-migration (crate root)
```

Key decisions (numbered in [design.md](design.md)): normalized
relational schema; every lifecycle and every KPI derivation in the
DB-free pure core; minor-unit money, per-currency reporting;
consent gating in the send path, not the UI; `bg_pg` jobs for
nurture/SLA/campaign (no external broker); ETag-conditional
dashboards; family fixtures (`#![forbid(unsafe_code)]`,
clippy-pedantic, OTLP, `Accepts-version`, Podman). **All-plural
table names** (the loco `create_table` pluralization lesson);
`404` mapping at `find_by_pid` call sites; enforcement tests in
their own binary.

## Front-end edition

SvelteKit 2 + Svelte 5 runes SPA + same-origin BFF proxy,
13-locale i18n from the start. Views per module: contact/account
timeline, lead queue (score-sorted with breakdown), deal Kanban
board, forecast table, campaign funnel + ROI, nurture editor,
ticket queue with SLA countdowns, KB editor, and the dashboards.
