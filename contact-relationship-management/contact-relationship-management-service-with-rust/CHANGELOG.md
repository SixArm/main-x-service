# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed — record-level ABAC wired into the contact/deal handlers (CRM-T24)

`auth::deal_resource_attrs`/`auth::contact_resource_attrs` were defined
and unit-tested but `authorize_record` was called only from
`controllers/privacy.rs` (subject-access/erase) — the deal and contact
read/write handlers ran no record-level check at all. `get_contact` and
`list_contacts` (via a new `readable_contacts` helper, omitting a
denied row per invariant 5) now call `authorize_record` and, on a
`mask` obligation, redact `preferred_channel` through a new
`auth::mask_text_json` (a text-placeholder sibling of the existing
amount-nulling `auth::mask_json`); `repoint_contact` gates its write
the same way. `list_deals` (via a new `readable_deals` helper) and
`deal_stage`/`reopen_deal` do the equivalent for deals, redacting
`amount_minor`. New unit test `mask_text_json_redacts_text_keeps_structure`;
`tests/enforcement.rs` gained an owner-vs-non-owner,
masked-vs-unmasked scenario for one deal and one contact endpoint.
`get_contact`'s bundled activities/deals/tickets rows, and the
forecast/ROI fields CRM-T22 also named, are explicitly left out of
this change (documented, not silent). See spec/tasks.md CRM-T24.

### Added — `require_ref` test coverage for Worker and Organization (CRM-T25)

`src/validation.rs`'s `ref_rules` unit test only ever exercised
`EntityType::Person`, even though `controllers/sales.rs`,
`support.rs`, and `relationships.rs` all call the shared `require_ref`
helper with `EntityType::Worker`/`::Organization` too. New
`ref_rules_wrong_type_worker_and_organization` test pins both the
wrong-type rejection and the matching-type acceptance for each. See
`../spec/tasks.md` CRM-T25.

### Added — declared MSRV (Rust 1.95)

- `Cargo.toml` now declares `rust-version = "1.95"`, the repository's
  **current stable minus three** floor
  (`spec/rust-msrv-n-minus-3/index.md`). Sourced from `ci/msrv.txt` and
  enforced by `scripts/ci-check.sh msrv`, which asserts the declared
  value matches that file and then compiles the crate — `--all-targets`,
  so benches and tests count — against the 1.95 toolchain. Behaviour is
  unchanged; what changes is that the floor is now a checked claim
  rather than an unstated assumption.
  *(Corrected 2026-09-06: `Cargo.toml` now declares `rust-version =
  "1.96"`, matching `ci/msrv.txt` and the repository's current **N-2**
  policy (`spec/rust-msrv-n-minus-2/index.md`) — the policy tightened
  from N-3 to N-2 after this entry was written. No behaviour change;
  `Cargo.toml`, `ci/msrv.txt`, and `scripts/ci-check.sh msrv` already
  agreed on 1.96 before this correction.)*

### Changed — loco-rs 1.0.1 (2026-08-02)

- **loco-rs 0.16 → 1.0.1**: sea-orm 1.1 → 2.0, sea-orm-migration →
  2.0, sea-query → 1.0. No feature-list changes (default feature set).
- **`ColType::PkAuto` now generates a 64-bit primary key.** Of this
  crate's ~23 tables, exactly one (`audit_logs`) goes through loco's
  schema DSL and moves from `i32` to `i64`; the relationships, sales,
  marketing, support, engagement, and `event_outbox` tables are all
  created with raw SQL and stay `i32` — same split as the sibling
  consumer apps.
- A `useless_conversion` in `src/models/event_outbox.rs` and a
  pre-existing `needless_borrows_for_generic_args` in
  `src/controllers/mod.rs`, both surfaced by the same clippy run.
- No behavioural change; verified with the full DB-gated suite (8
  tests, unchanged count) against a freshly migrated Postgres 18.

### Added — engagement / partnerships / confederation (CRM-T20, 2026-07-20)

- Declared stakeholder typing (role + power–interest 1–5 on contacts;
  role on accounts), recorded activity sentiment, the forward-only
  partnership lifecycle, per-account memberships, and working groups
  with derived activity feeds (`controllers/engagement.rs`).
- Nine new derived views (`controllers/insights.rs`): cadence,
  engagement workload, the audit-derived pipeline funnel, member
  health, consent-by-account, the stakeholder register + grid, the
  partnership register, membership renewals; follow-ups gains a
  `kind` filter (renewals convention).

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
  (copy-adapted from the WPM service). 6 migrations / 19 tables,
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
