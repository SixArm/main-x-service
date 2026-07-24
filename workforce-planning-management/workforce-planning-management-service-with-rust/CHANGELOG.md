# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added — learning & development (HCM-T20, 2026-07-20)

- Skills framework (catalog + declared employee proficiency 1-5 with
  optional target), learning paths (ordered course steps +
  per-employee enrolment with honest progress from completed training
  enrolments), and mentorships (proposed->active->completed lifecycle
  + session log). Derived views: the per-department skills matrix +
  gaps, training analytics (completion ratio + cert expiry), and the
  mentorship overview (load, unmatched, stale). Migration
  `m20260720_000008_learning`.

### Added

- 2026-07-18 — HCM-T1–T17 implementation round: full Loco service
  (copy-adapted from patient-flow). 7 migrations / 25 tables, pure
  `rules/` core (lifecycle machines for employee / requisition /
  application / leave / review / payroll; leave balances; overtime;
  shift conflicts; org-chart cycle check; payslip arithmetic with
  the `net = gross − Σ deductions` persist gate and overflow
  refusal; benchmark flags), five pillar controllers (hr_core /
  acquisition / workforce / development / payroll) + audits / docs /
  metrics, offline PASETO + ABAC with `resource.person` `$sub`
  ownership and salary/payslip `mask` obligations, sensitive-read
  audits, event seam (memory/outbox), OpenAPI (57 paths) + Swagger,
  `Accepts-version` negotiation, Prometheus gauges, seed task
  (synthetic 40-employee org). 71 unit + 7 request + 1 enforcement
  tests green against Postgres 18; clippy-pedantic clean.

- 2026-07-18 — HCM-T0 specification round: cross-cutting spec
  (`../spec/`) with the five-pillar domain, SDD trio
  (requirements HCM-R1–R17, design HCM-D1–D12, tasks HCM-T*), and
  this edition's doc scaffold. No code yet.
