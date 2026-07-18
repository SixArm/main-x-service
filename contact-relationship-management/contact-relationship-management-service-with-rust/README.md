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

**Status: not started.** The cross-cutting specification landed
2026-07-18; the delivery queue is
[../spec/tasks.md](../spec/tasks.md) (CRM-T1 onward).

## What it will answer

- *Which leads should I call first?* — score-sorted queue with the
  per-rule breakdown
- *Where is every deal, and what will this quarter close?* — Kanban
  board + stage-weighted forecast (derived, never typed)
- *Did that campaign pay for itself?* — funnel + ROI from
  attributed won revenue
- *Are we keeping our support promises?* — SLA deadlines + breach
  flags
- *What is this account worth?* — CLV from won deals

## Target surface (per the cross-cutting spec)

Contacts / accounts / activities + timelines · leads + scoring ·
pipelines / stages / deals + Kanban · forecast + snapshots ·
consent + segments + campaigns + nurture · tickets + SLA policies ·
articles · dashboards · audits · `/events/recent` · OpenAPI +
Swagger · `/metrics.prom`.

Auth enforcement defaults **off** (`CRM_REQUIRE_AUTH` is the family
activation gate); upstream lookups will default to **stub mode**;
events default to the in-memory transport.
