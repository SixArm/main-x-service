# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added — insight views (CRM-T19, 2026-07-20)

- `controllers/insights.rs`: stale-deal aging (from stage-change
  audits), follow-ups (overdue + 30-day horizon), pipeline-hygiene
  findings, the period executive pack (per-currency won value never
  merged), the stored forecast-trend series (no interpolation), the
  SLA breach register + per-assignee workload, and the DPO view
  (consent coverage + withdrawals + per-source counts +
  duplicate-contact hygiene). All ETag-conditional with `as_of`.

### Added

- 2026-07-18 — CRM-T1–T16 implementation round: full Loco service
  (copy-adapted from the HCM service). 6 migrations / 19 tables,
  pure `rules/` core (lifecycle machines, deterministic lead scoring
  with breakdown, forecast/ROI/CLV/win-rate with per-currency
  honesty, SLA derivation, consent-gated segment evaluation), five
  module controllers (relationships / sales / marketing / support /
  dashboards) + audits/docs/metrics, offline PASETO + ABAC with
  `resource.owner` `$sub` ownership + amount masking,
  ETag-conditional dashboards, idempotent nurture advance + SLA
  sweep, simulated campaign send (consent re-checked at send time),
  seed task. 62 unit + 5 request + 1 enforcement tests green against
  Postgres 18; clippy-pedantic clean; live smoke verified.

- 2026-07-18 — CRM-T0 specification round: cross-cutting spec
  (`../spec/`) with the four-module domain, SDD trio
  (requirements CRM-R1–R17, design CRM-D1–D12, tasks CRM-T*), and
  this edition's doc scaffold. No code yet.
