# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

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
