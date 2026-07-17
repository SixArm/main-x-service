# AGENTS.md — working agreements

A pocket guide for human and AI collaborators working in this
subproject. Read this **before** opening a PR.

## What this project is

A **back-end JSON API**, written in Rust on [Loco](https://loco.rs)
(Axum + SeaORM + PostgreSQL), for hospital patient flow and bed
management: wards, bays, beds and their live state machine, inpatient
stays from admission to discharge, bed requests and rule-checked
allocation, infection-control flags, virtual wards, and the derived
whiteboard / at-a-glance / capacity reads. There is no built-in UI —
the [Svelte sibling](../patient-flow-front-end-with-svelte/) is the
whiteboard client.

**Domain ownership.** Patient Flow **owns operational state** (its
own tables) but **references identities**: patients are
person-service records, staff worker-service, sites place-service,
the trust organization-service — always as EntityRef URNs
(`person:<uuid>`), never duplicated demographics. See the
cross-cutting spec's [scope boundary](../spec/scope.md).

> ⚠️ Demo software, not a regulated medical record. See
> [regulatory](../spec/regulatory.md).

## Ground rules

1. **Spec first.** The cross-cutting spec at
   [`../spec/`](../spec/index.md) is the single source of truth;
   this subproject's `spec/` adds stack detail only. A behavioural
   change is spec edit + code + tests in one PR. The live task queue
   is [`../spec/tasks.md`](../spec/tasks.md) (PF-T* ids, traced to
   PF-D*/PF-R*).
2. **Family conventions.** Loco-idiomatic layout
   (`src/controllers/`), `#![forbid(unsafe_code)]`, thiserror,
   tracing + OTLP, OpenAPI/Swagger, header API versioning
   (`Accepts-version`), Podman not Docker, PostgreSQL not SQLite,
   `bg_pg` jobs, in-memory loco cache. See
   [rust-loco-stack](../../agents/share/rust-loco-stack.md).
3. **Pure core.** Bed state machine, allocation rules, Red2Green,
   DTOC clock live in `src/flow/` as DB-free pure functions with
   exhaustive unit tests; controllers stay thin (PF-D4).
4. **Transactional integrity.** State change + audit row + event
   outbox in one transaction; placement paths lock bed rows
   (PF-D9). Never break the one-occupant-per-bed invariant.
5. **Stub-first upstreams.** Every upstream client has a `stub`
   implementation; the service must boot, seed, and pass tests with
   no siblings running (PF-D11). Upstream reads never block writes.
6. **Auth posture.** Offline PASETO + ABAC via
   `authentication-verifier`; blanket `PATIENT_FLOW_REQUIRE_AUTH`
   defaults off (family activation gate); patient display names obey
   the `mask` obligation on whiteboard/locate reads. Sensitive reads
   (locate, stay detail) are audited.
7. **Synthetic data only** in seeds and tests.

## Layout (target — see [../spec/architecture.md](../spec/architecture.md))

```
src/app.rs · src/controllers/ · src/models/ (+_entities/) ·
src/clients/ (person|worker|place|organization × http|stub) ·
src/flow/ (pure core) · src/auth.rs · src/streaming.rs ·
src/validation.rs · src/openapi.rs · migration/ · tests/requests/
```

## Running

```bash
cargo run                 # API server (stub mode by default in dev)
cargo test                # unit + request tests
cargo loco task seed      # synthetic demo hospital
```
