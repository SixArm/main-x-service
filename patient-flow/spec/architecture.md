# Architecture

## Editions

```
 ward touchscreen / desktop / mobile browser
        │  (cookie session; no token in JS)
        ▼
 patient-flow-front-end-with-svelte  (SvelteKit BFF)
        │  Authorization: Bearer v4.public.…  (short-lived PASETO)
        ▼
 patient-flow-service-with-rust  (Loco: Axum + SeaORM + PostgreSQL)
        │  EntityRef lookups (read-only, cached, stub-able)
        ▼
 person / worker / place / organization / authentication services
```

## Service edition (loco-style)

Follows the family's **loco-idiomatic** layout
([architecture.md](../../agents/share/architecture.md)) — the shape
organization/care-pathway/case/portfolio use — because Patient Flow
is new code with no legacy surface:

```
src/
├── app.rs                 loco Hooks (routes, boot, workers)
├── controllers/           wards, bays, beds, stays, bed_requests,
│                          whiteboard, at_a_glance, locate, capacity,
│                          audits, docs, metrics
├── models/                CRUD helpers + _entities/ (SeaORM)
├── clients.rs             one generic EntityRef-keyed display-name
│                          resolver — http|stub mode (not a
│                          per-service trait/module split)
├── flow/                  pure domain logic: bed state machine,
│                          allocation rules, red2green, DTOC clock
├── auth.rs                offline PASETO verify + ABAC (+ masking)
├── streaming.rs           Envelope + EventPublisher seam
├── validation.rs          payload validation → 422
└── openapi.rs             OpenAPI 3 doc
migration/                 sea-orm-migration crate (crate root)
```

Key decisions (numbered in [design.md](design.md)):

- **Normalized relational schema**, not matcher-DTO-as-JSONB: Patient
  Flow has no matcher and its value is transitional state +
  constraints; wards/bays/beds/stays are proper tables with FKs and
  state columns. (`alerts`, `requirements`, `delay_reasons` are the
  JSONB-typed leaves.)
- **Pure `flow/` core**: the bed state machine, allocation
  eligibility, and Red2Green/DTOC rules are pure functions —
  unit-testable without a database, same posture as the matcher
  crates.
- **Transactional writes**: state transition + audit + event outbox
  in one transaction; bed rows locked during placement.
- **Stub-first upstream clients**: boot and test with no siblings
  running (case-folder precedent).
- Standard family fixtures: `#![forbid(unsafe_code)]`, thiserror
  error enum, tracing + OTLP, `/metrics.prom`, OpenAPI/Swagger,
  header API versioning (`Accepts-version`), Podman packaging.

## Front-end edition

SvelteKit 2 + Svelte 5 runes + TypeScript strict, per family
front-end conventions (drift-accepted, copy-adapt from a sibling —
portfolio's operational views are the closest source). BFF pattern
per [auth.md](auth.md). Touchscreen mode is a CSS/layout concern
(large hit targets, kiosk route without chrome), not a separate app.

## Data & availability

PostgreSQL 18, SeaORM migrations, connection pooling, health
endpoints, stateless horizontal scaling (whiteboard reads are the
hot path and are plain indexed queries). In-memory loco cache for
upstream display-name lookups.
