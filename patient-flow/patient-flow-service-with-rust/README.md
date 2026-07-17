# Patient Flow — Loco JSON API

A back-end **JSON API** for hospital patient flow and bed management
in a UK NHS hospital setting: live bed states, inpatient stays from
admission to discharge, bed requests and allocation, infection
control, virtual wards, and 'hospital at a glance' capacity views.
Implemented in Rust on [Loco](https://loco.rs) (Axum + SeaORM +
PostgreSQL). No built-in UI — the
[Svelte sibling](../patient-flow-front-end-with-svelte/) provides
the ward whiteboard and touchscreen client.

> ⚠️ **Demo software.** Not a regulated medical record, not a
> medical device, not assured for clinical use. Synthetic data only.
> See [spec/regulatory](../spec/regulatory.md).

**Status: implemented (PF-T1–T14, 2026-07-18).** Builds, 64 DB-free
unit tests pass, clippy-pedantic clean, verified end-to-end against
Postgres 18. Remaining in [../spec/tasks.md](../spec/tasks.md):
PF-T17 (request-test suite) and the front-end (PF-T15/T16).

## Quick start

```bash
# Postgres 18 with a loco user, then:
export DATABASE_URL=postgres://loco:loco@localhost:5432/patient_flow_service_development
cargo run -- db migrate       # create the schema
cargo run -- task seed        # synthetic demo hospital (5 wards, 76 beds)
cargo run -- start            # serve on :5150
curl localhost:5150/api/at-a-glance | jq .site_tiles
```

Upstream lookups default to **stub mode** (no sibling services
needed); events default to the in-memory transport; auth enforcement
defaults **off** (`PATIENT_FLOW_REQUIRE_AUTH` is the activation gate).

## What it answers

- *Where is patient X right now?* — `GET /api/locate/{person_ref}`
- *Which beds are free, cleaning, or closed?* — bed state machine +
  `GET /api/whiteboard/{ward}` + `GET /api/at-a-glance`
- *Who can go home today, and who is delayed and why?* — EDD/CCD,
  discharge-ready, Red2Green delay reasons, DTOC counts

## Route cheat-sheet (target surface)

```
/api/sites /api/wards /api/bays /api/beds        topology CRUD
POST /api/beds/{pid}/state                       bed transitions
/api/bed-requests (+ /allocate)                  demand queue
/api/stays (+ /transfer /discharge-ready /discharge)
/api/stays/{pid}/red-green                       day journal
/api/stays/{pid}/infection-flags                 IPC flags
/api/whiteboard/{ward}  /api/at-a-glance         derived reads
/api/locate/{person_ref}  /api/capacity/metrics
/api/audits?ward=&since=                         handover trail
```

## Relationship to the Main X Index family

Patient Flow is a **consumer application**: patients, staff, places,
and the trust are records in the sibling person / worker / place /
organization services, referenced by EntityRef URN. Auth is the
central authentication-service (offline PASETO verification; blanket
guard `PATIENT_FLOW_REQUIRE_AUTH`, default off).

## Docs

- [index.md](index.md) — documentation index
- [../spec/](../spec/index.md) — the cross-cutting specification
  (single source of truth)
- [spec/](spec/index.md) — this edition's stack-specific spec
- [AGENTS.md](AGENTS.md) — working agreements
