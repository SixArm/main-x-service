# Contact Relationship Management — Loco JSON API

A back-end **JSON API** for CRM: contacts and accounts with full
relationship timelines, lead capture and explainable rule-based
scoring, deal pipelines with Kanban ordering and stage-weighted
forecasting, consent-first email campaigns / segments / nurture
sequences, support tickets with SLA tracking, a versioned knowledge
base, and derived dashboards (win rate, pipeline value, campaign
ROI, CLV, SLA health). Implemented in Rust on
[Loco](https://loco.rs) (Axum + SeaORM + PostgreSQL). No built-in
UI — the
[Svelte sibling](../contact-relationship-management-front-end-with-svelte/)
provides the sales, marketing, and support client.

> ⚠️ **Demo software.** Not a production CRM; no real email is sent
> (delivery is simulated); synthetic data only. See
> [spec/regulatory](../spec/regulatory.md).

**Status: implemented (CRM-T1–T16, T19, T20; 2026-07-18 through
2026-07-20).** Builds; 64 DB-free unit tests + 7 request tests + the
enforcement matrix (8 total under `cargo test -- --ignored`) pass
against Postgres 18; clippy-pedantic clean; live smoke verified
(seed → forecast → ETag 304 → win rate). Remaining in
[../spec/tasks.md](../spec/tasks.md): the CRM-G1/CRM-G2 production
gates (auth activation + GDPR/PECR review) before any non-demo
exposure — not a code gap.

## Quick start

```bash
export DATABASE_URL=postgres://loco:loco@localhost:5432/contact_relationship_management_service_development
cargo run -- db migrate && cargo run -- task seed && cargo run -- start
curl localhost:5150/api/forecast | jq .
```

## What it answers

- *Which leads should I call first?* — score-sorted queue with the
  per-rule breakdown
- *Where is every deal, and what will this quarter close?* — Kanban
  board + stage-weighted forecast (derived, never typed)
- *Did that campaign pay for itself?* — funnel + ROI from
  attributed won revenue
- *Are we keeping our support promises?* — SLA deadlines + breach
  flags
- *What is this account worth?* — CLV from won deals

## Surface

Contacts / accounts / activities + timelines · leads + scoring ·
pipelines / stages / deals + Kanban · forecast + snapshots ·
consent + segments + campaigns + nurture · tickets + SLA policies ·
articles · dashboards · seven `/insights/*` derived views
(stale-deals, follow-ups, pipeline-hygiene, executive pack,
forecast-trends, SLA, DPO) · engagement/partnership data
(stakeholder typing, activity sentiment, partnerships, memberships,
working groups) + nine more derived views · audits ·
`/events/recent` · OpenAPI + Swagger · `/metrics.prom`.

Auth enforcement defaults **off** (`CRM_REQUIRE_AUTH` is the family
activation gate); upstream lookups will default to **stub mode**;
events default to the in-memory transport.
