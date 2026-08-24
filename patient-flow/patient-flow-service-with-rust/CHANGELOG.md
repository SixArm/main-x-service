# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added — the stitched-journey timeline (`GET /api/stays/{pid}/time-analysis`)

This service now serves the timeline contract `care-pathway-service`
follows across a `continues_as` link, so a journey that begins on a care
pathway and continues into an inpatient stay can be measured end to end
instead of stopping at the service boundary.

**A green Red2Green day is the value-adding time.** Nothing new had to
be invented: time-based analysis asks what share of an episode was *the
work*, and Red2Green already answers exactly that in the NHS's own
vocabulary — a green day moves the patient toward discharge, a red day
does not. Deriving it any other way would have meant this service making
a clinical judgement it is not entitled to make.

**Unclassified days count as non-value-adding**, matching the consuming
service's denominator rule: elapsed calendar time is the denominator and
unrecorded time counts against you, because the alternative rewards
recording less. The figure is therefore a **floor**, and the response
carries `coverage` and `confidence` — an unclassified stay and a
genuinely red one both report little value-adding time, and only the
confidence tells them apart. That distinction matters: one calls for
filling in the board, the other for fixing the delay.

Four edges are handled in the pure derivation rather than left to the
caller: a same-day stay is one day of care and its value time is capped
at the elapsed span (a whole green day would put the consuming ratio
above 1); a classification outside the stay's own span is ignored rather
than counted, which would push coverage past 1; a duplicated day counts
once; and a reversed clock reports zero rather than a negative duration
that would flow into a stitched journey's arithmetic.

The endpoint is a **sensitive read** like the MDT view — record-level
ABAC, audited — so a caller assembling a cross-service journey leaves
the same trail as one opening the record. The `mask` obligation is
deliberately not applied: the response carries no identifiers, only
durations, so there is nothing in it to redact.

**A coverage ceiling worth knowing about:** `red-green` classifies
*today* and takes no `day`, so a stay admitted before the board was in
use can never be fully classified retrospectively. Its coverage is
capped by when classification started, not by ward diligence — which is
why the confidence label matters more here than a bare percentage.



### Added — declared MSRV (Rust 1.95)

- `Cargo.toml` now declares `rust-version = "1.95"`, the repository's
  **current stable minus three** floor
  (`spec/rust-msrv-n-minus-3.md`). Sourced from `ci/msrv.txt` and
  enforced by `scripts/ci-check.sh msrv`, which asserts the declared
  value matches that file and then compiles the crate — `--all-targets`,
  so benches and tests count — against the 1.95 toolchain. Behaviour is
  unchanged; what changes is that the floor is now a checked claim
  rather than an unstated assumption.

### Changed — loco-rs 1.0.1 (2026-08-02)

- **loco-rs 0.16 → 1.0.1**: sea-orm 1.1 → 2.0, sea-orm-migration →
  2.0, sea-query → 1.0. No feature-list changes (this crate uses
  loco's default feature set, not an explicit list).
- **loco's `ColType::PkAuto` now generates a 64-bit primary key.** All
  10 domain tables move from `i32` to `i64`: `sites`, `wards`, `bays`,
  `beds`, `stays`, `transfers`, `bed_requests`, `red_green_days`,
  `infection_flags`, `audit_logs`. `event_outbox` stays `i32` — its
  migration writes raw SQL (`id SERIAL PRIMARY KEY`) rather than the
  loco schema DSL, the same pattern as every entity-registry crate in
  this migration.
- A `useless_conversion` in `src/models/event_outbox.rs` and a
  pre-existing `needless_borrows_for_generic_args` in
  `src/controllers/mod.rs`, both surfaced by the same clippy run.
- No behavioural change; verified with the full DB-gated suite (9
  tests, unchanged count) against a freshly migrated Postgres 18.

### Fixed

- 2026-07-18 — **Unknown-pid reads returned 500, not 404** (loco 0.16
  does not map `ModelError::EntityNotFound`): the shared
  `records::find_*` finders and `records::parse_pid` now return
  `Error::NotFound` directly, so every `GET /…/{pid}` contract is an
  honest 404. Pinned in the topology request test (ghost UUIDs +
  malformed pid). Part of a family-wide fix (case, care-pathway,
  project-portfolio-management; organization was already correct).


### Added

- 2026-07-18 — PF-T17: weak-**ETag conditional GETs** on the
  whiteboard and at-a-glance reads (`If-None-Match` ⇒ `304`; the tag
  covers everything except `as_of`), and the DB-gated request-test
  suite: `tests/requests/{topology,flows,boards}.rs` (topology CRUD +
  422s; the full request→allocate→admit→Red2Green→ready→discharge→
  deep-clean journey; the double-placement race — two concurrent
  admits, exactly one 200; board shapes + the ETag 200/304/refresh
  cycle; locate + its `locate_read` audit pin) plus
  `tests/enforcement.rs` (auth-activation matrix over live routes:
  401/403 splits, action gating, and the **masked whiteboard** under
  an allow-with-`mask` policy). All 9 verified against Postgres 18.
- 2026-07-18 — PF-T1–T14 implementation round: the full Loco service.
  Migrations for sites / wards / bays / beds / stays / transfers /
  bed_requests / red_green_days / infection_flags / audit_logs /
  event_outbox; the pure `src/flow/` core (bed state machine with
  deep-clean propagation, allocation rules 1–5 with overridable sex /
  ward-fit and side-room-conserving ranking, Red2Green validation,
  DTOC clock, LOS); controllers for topology CRUD, bed state
  transitions, the stay lifecycle (admit / update / transfer /
  discharge-ready / discharge), Red2Green, infection flags, bed
  requests + eligible + allocate + cancel, whiteboard, at-a-glance,
  capacity metrics, locate, audits + handover query, events;
  transactional audit + event emission (`memory` ring / `outbox`
  rows); offline PASETO + blanket `PATIENT_FLOW_REQUIRE_AUTH` guard +
  ABAC with ward-scoped record checks and the `mask` obligation;
  stub-first upstream display-name clients; the synthetic demo-
  hospital seed task; OpenAPI + Swagger UI; `Accepts-version`
  negotiation; Prometheus counters + capacity gauges. 64 DB-free unit
  tests; clippy-pedantic clean; verified end-to-end against Postgres
  18 (migrate → seed → full patient journey → deep-clean gate →
  locate → audits → events → 406 on `Accepts-version: 2.0`).
  Remaining: PF-T17 request-test suite; PF-T15/T16 front-end.
- 2026-07-17 — PF-T0 specification round: cross-cutting spec
  ([../spec/](../spec/index.md)) with domain model, bed state
  machine, patient journey (SAFER / Red2Green / DTOC), whiteboard,
  infection control, virtual ward, capacity, auth/audit posture, and
  the phased PF-T* task queue; this edition's doc scaffold. No code
  yet — implementation begins at PF-T1.
