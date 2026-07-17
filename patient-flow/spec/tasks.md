# Tasks — delivery checklist

Status legend: `[x]` done · `[~]` in progress · `[ ]` not started.
Every task traces to design (PF-D*) and requirement (PF-R*) ids.
Three-part rule applies: a behavioural change lands as spec edit +
code + tests in one PR.

## Phase 0 — specification

- [x] PF-T0 Cross-cutting spec round: topic files + SDD trio, both
  edition doc scaffolds, root AGENTS.md wiring. (all PF-D*, PF-R*)
  — landed 2026-07-17.

## Phase 1 — service skeleton & topology (PF-R1, PF-R15)

- [x] PF-T1 Scaffold `patient-flow-service-with-rust`: loco app,
  config, migration crate (CI workflows deferred — nested
  workflows don't run in the monorepo), family
  fixtures (forbid-unsafe, thiserror, tracing/OTLP, `/metrics.prom`,
  OpenAPI + Swagger, `Accepts-version` middleware, health routes).
  (PF-D12)
- [x] PF-T2 Migrations + models + CRUD controllers for Site / Ward /
  Bay / Bed with validation → 422, soft delete, audit + event
  emission seam (`PATIENT_FLOW_EVENT_TRANSPORT=memory`). (PF-D2,
  PF-D3, PF-D9; PF-R1, PF-R13)
- [x] PF-T3 Upstream client seam: person / worker / place /
  organization traits + `http` + `stub`, config-selected; stub-mode
  boot test; display-name cache. (PF-D11; PF-R15)
- [x] PF-T4 Seed task: synthetic demo hospital (2 sites, ~6 wards
  incl. virtual + escalation, ~120 beds). (PF-R15)

## Phase 2 — bed states & journey (PF-R2, PF-R4–R6)

- [x] PF-T5 Pure `flow/` core: bed state machine + transition table
  + deep-clean propagation; exhaustive unit tests. (PF-D4; PF-R2)
- [x] PF-T6 Bed state transition endpoints wiring the pure core:
  `FOR UPDATE` locking, `state_since`, audit + event in-tx (the
  double-placement race test moved to PF-T17). (PF-D9; PF-R2, PF-R13)
- [x] PF-T7 Stay lifecycle: admit / transfer / discharge-ready /
  discharge controllers + Transfer rows + LOS derivation (request
  tests moved to PF-T17). (PF-D5; PF-R4, PF-R5, PF-R6)
- [x] PF-T8 SAFER fields + Red2Green journal (day rows, ≤2 coded
  reasons, same-day edit window) + DTOC clock in the pure core.
  (PF-D4; PF-R6, PF-R7)

## Phase 3 — demand, reads & capacity (PF-R3, PF-R8–R10)

- [x] PF-T9 BedRequest queue + pure-core allocation eligibility +
  ranking + override-with-reason; eligible-count on open requests.
  (PF-D4, PF-D7; PF-R3)
- [x] PF-T10 Whiteboard + stay-detail + locate reads (`as_of` stamp;
  locate read-audit; ETag conditional GET deferred to PF-T17).
  (PF-D6; PF-R8, PF-R9, PF-R13)
- [x] PF-T11 At-a-glance + capacity metrics + Prometheus gauges +
  handover audit query (`?ward=&since=`). (PF-D6; PF-R10, PF-R13)

## Phase 4 — infection control & virtual ward (PF-R11, PF-R12)

- [x] PF-T12 InfectionFlag CRUD + transfer restriction + deep-clean
  gate + bay/ward `closed_to_admissions` + reopen precondition +
  IPC counts in capacity. (PF-R11)
- [x] PF-T13 Virtual ward: virtual slots, step-up / step-down /
  direct-admission flows, no-clean cycle, census. (PF-D8; PF-R12)

## Phase 5 — auth activation surface (PF-R14)

- [x] PF-T14 `auth.rs`: offline PASETO verify + blanket
  `PATIENT_FLOW_REQUIRE_AUTH` guard (guard-all / deny-unless-public)
  + ABAC + `resource.ward` record-level checks + `mask` obligation
  on whiteboard/locate; full family auth test matrix. (PF-D10)

> Phases 1–5 landed 2026-07-18 in one implementation round
> (`patient-flow-service-with-rust`): 24 src modules, 5 migrations,
> 64 DB-free unit tests (state machine, allocation rules, journey,
> auth matrix), clippy-pedantic clean, verified end-to-end against a
> live Postgres 18 (migrate → seed → admit → red-green → flag →
> discharge-ready → discharge → deep-clean gate → locate → audits →
> events → version negotiation → Prometheus). Notes: concurrency
> pinning relies on `FOR UPDATE` locking (dedicated race test is in
> PF-T17); `/events/recent` reads the ring or the outbox per
> transport.

- [ ] PF-T17 Request-test suite (`tests/requests/`, Postgres,
  `--ignored` per family convention): per-controller happy paths +
  422s, the double-placement race, masked whiteboard render, auth
  matrix over live routes; ETag/`updated_since` conditional GET on
  the board reads; CI workflows. (PF-D9; spec testing.md)

## Phase 6 — front-end (PF-R8 UI)

- [ ] PF-T15 Scaffold `patient-flow-front-end-with-svelte`
  (SvelteKit 2 + Svelte 5 runes + TS strict, BFF auth, copy-adapt
  from portfolio front-end), routes: whiteboard (+ kiosk/touch
  route), stay detail, at-a-glance, bed requests, locate, audits.
- [ ] PF-T16 vitest bed-card matrix + Playwright e2e against
  stub-mode service (admit→board→ready→discharge→clean walk).

## Production gates (P0 — design-only until a real deployment)

- [ ] PF-T-G1 Clinical safety case (DCB0129/0160) + named CSO.
- [ ] PF-T-G2 IG: DPIA, DSP Toolkit, retention schedule,
  subject-access slice.
- [ ] PF-T-G3 Security activation: REQUIRE_AUTH on, ABAC policy
  mounted, TLS, masked corridor mode verified.
- [ ] PF-T-G4 Resilience: HA deployment + failover test + paper
  fallback procedure.
