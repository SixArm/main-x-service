# Changelog

All notable changes to this crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md) — single source of truth;
> [README.md](./README.md) — user-facing intro; [AGENTS.md](./AGENTS.md) — agent guide.

## [Unreleased]

### Documented — the caller must bound `goals`/`keywords`/`relationships`/`tags` array sizes

Each is compared by Jaccard over every element — an unbounded O(n·m)
operation — and this crate has no length cap of its own on any of the
four. The family's SEC-M1 caps live only in
`project-portfolio-management-service`'s validation layer, which runs
before this crate is ever called; a standalone integrator has no
equivalent. Ported case-matcher's identical fix: added a "the caller
must bound array sizes" section to the crate-root docs and a matching
note on `MatchingEngine::match_plans`'s rustdoc, a new `AGENTS.md`
golden rule, and a §22 anti-pattern line, all pointing at the service's
`MAX_ARRAY_LEN` (256) / `MAX_ITEM_LEN` (512) as the reference cap. No
code change; see spec/index.md §23.

### Changed
- MSRV raised to Rust 1.96 (N-2 policy tightened from N-3; see spec/rust-msrv-n-minus-2/index.md).

## [0.2.0] - 2026-08-26

### Added — `Plan.phase` + `PlanPhase` (wire-format addition; never a matcher signal)

- 2026-08-25 — **`Plan` gains an optional `phase` field and the
  `PlanPhase` enum** (`Initiating` / `Planning` / `Executing` /
  `Controlling` / `Closing` — the classic five process groups), following
  the `PlanStatus` precedent exactly: `#[serde(default)]`, so every
  stored payload and old peer round-trips unchanged, and
  **informational-only — never scored**. Two records of one initiative
  routinely sit in different phases, and the phase is precisely the
  field most likely to differ between two systems describing one plan;
  scoring it would make the matcher worse exactly where records are
  hardest to reconcile. Pinned by the `phase_is_not_scored` test, the
  same pin `status` carries.
- `PlanPhase` ships `ALL` (process order), `ordinal()`, `token()` (the
  wire tokens `initiating` … `closing`), and `parse()` — unknown input is
  `None`, refused by the caller rather than coerced, so a typo can never
  silently place a plan in `Initiating`. Re-exported from `lib.rs`
  alongside the other domain types.
- No weights, thresholds, deterministic rules, or scoring behaviour
  changed. The bump to 0.2 marks the DTO wire-format addition, not any
  matching change.

### Added — Criterion benchmarks

- `benches/match_pair.rs`, matching the harness the other matcher crates
  carry. Four groups over the paths a downstream integrator actually
  exercises: single-pair scoring in three regimes (identical clone,
  fuzzy near-match through the full pipeline, unrelated pair), the
  deterministic short-circuits (a shared Jira project key, and a shared owner-scoped code), `rank` at 10 / 100 / 1000
  candidates with `Throughput::Elements` set so per-candidate cost and
  any super-linear scaling are visible directly, and the same fuzzy pair
  under each shipped config preset.
- Fixtures are built from `Plan::new` and derived deterministically from
  an index, so a candidate list of any size is reproducible without
  randomness.

### Added — declared MSRV (Rust 1.95)

- `Cargo.toml` now declares `rust-version = "1.95"`, the repository's
  **current stable minus three** floor
  (`spec/rust-msrv-n-minus-3/index.md`). Sourced from `ci/msrv.txt` and
  enforced by `scripts/ci-check.sh msrv`, which asserts the declared
  value matches that file and then compiles the crate — `--all-targets`,
  so benches and tests count — against the 1.95 toolchain. Behaviour is
  unchanged; what changes is that the floor is now a checked claim
  rather than an unstated assumption.

### Added

- 2026-07-22 — **Five new `PlanKind` labels: `Practice`, `Process`,
  `Purpose`, `Pathway`, `Proposal`.** Additive on the closed set that
  already held `Portfolio` / `Project` / `Product` / `Program`; `kind`
  remains **optional descriptive metadata** and is still never a match
  gate, so the new labels change no scoring behaviour. A stored payload
  with one of the new labels round-trips through serde; old payloads are
  unaffected.

### Changed

- 2026-07-20 — **BREAKING: unified the four kinds into one recursive
  plan tree.** `Plan.kind` is now `Option<PlanKind>`
  (optional descriptive metadata) instead of a required discriminator,
  and the **hard kind gate (R-GATE) is removed** — any two plans
  may now match regardless of `kind`. `Plan::new` drops its `kind`
  argument (`new(name)`; kind defaults `None`). `parent_ref` is now a
  general containment link (any plan may contain any other), and the
  parent component (`parent_score`) applies to every plan rather
  than only the former child kinds. `MatchBreakdown.kind_gate_blocked` is
  retained but vestigial (always `false`). `PlanKind` loses its
  `Default` impl and `is_child()` method. Removed the kind-gate tests;
  added tests pinning that different kinds still match and that a shared
  deterministic id matches across kinds.

- 2026-07-18 — **Subproject renamed**: `portfolio` →
  `project-portfolio-management` (directory, crate/package name, lib
  ident, env-var prefix `PORTFOLIO_*` → `PROJECT_PORTFOLIO_MANAGEMENT_*`,
  database names). The **domain language is unchanged**: the plan
  kinds (portfolio / project / product / program), the `plans`
  table, the API routes, and the matcher's `Plan` type keep their
  names — the rename repositions the *subproject* as a project
  portfolio management (PPM) product; see the feature roadmap in
  `../spec/15-roadmap.md`.


### Added — cargo-fuzz harness (SEC-I2)

- A `fuzz/` [`cargo-fuzz`](https://rust-fuzz.github.io/book/) crate adopting
  the person-matcher reference scaffolding, with two coverage-guided
  libFuzzer targets: `match_plans` (deserialize a JSON `[plan_a, plan_b]` tuple →
  `MatchingEngine::match_plans`; finite score in `[0,1]`, both orders) and
  `normalize` (the pure `normalize` free functions — fold / code / URL / fold-set / ISO date — over arbitrary
  UTF-8, never-panic). Two targets rather than the reference three because
  this crate exposes its similarity primitives only through the engine (the
  `scoring` module publishes no string-similarity functions). Run on nightly:
  `cargo +nightly fuzz run <target>` (see `fuzz/README.md`). The `fuzz/` crate
  is standalone (not a workspace member), so it never affects the crate’s
  normal stable build/test/clippy. Verified: `cargo +nightly fuzz build`
  compiles both targets and short campaigns run clean (millions of execs, no
  panics).

### Added

- **SEC-M6 — property-based tests.** Added `proptest` (dev-dependency) and
  `tests/property_tests.rs`, proving the matcher and its pure helpers are
  well-behaved on arbitrary input rather than only hand-picked examples:
  - `score_is_finite_and_bounded` — the engine never panics and every
    `score` is a real number in `[0.0, 1.0]` (never `NaN`).
  - `matching_is_symmetric_same_kind` — `match_plans(a, b)` equals
    `match_plans(b, a)` in score, `is_match`, and confidence for
    same-kind records.
  - `kind_gate_blocks_all_cross_kind_pairs` — at the time this property
    landed, the **kind gate**: any pair of different `kind` always
    scored `0.0`, never matched, and set `kind_gate_blocked` with every
    component `None`. **Superseded 2026-07-20** (see the `Changed`
    entry above): the kind gate was removed, and this property was
    rewritten as `different_kinds_are_not_gated` — the opposite
    assertion — in `tests/property_tests.rs`.
  - `identical_clone_matches_itself` — reflexivity: a clone of any
    well-formed record clears the threshold (`is_match`).
  - `pure_helpers_never_panic` / `iso_date_never_overflows` — `fold`,
    `code`, `url`, `fold_set`, `iso_date_to_days`, and `soundex` never panic
    on arbitrary strings; `iso_date_to_days` never overflows on adversarial
    long-year date strings (guards the SEC-M4 fix).

  Tests + dev-dependency only — no weights, thresholds, or matching
  behaviour changed.

### Security

- **SEC-M2 — a bare root `same_as` URL no longer forces a deterministic
  match.** `normalize::url("/")` returns `"/"` (non-empty by design), so
  two different plans sharing only `same_as=["/"]` short-circuited to
  `1.0`. The `R-2` `same_as` overlap in `src/matcher.rs` now skips a value
  that is empty-after-normalization or a bare `"/"` root, so such a
  placeholder is not treated as identity evidence. Added the
  `trivial_root_same_as_does_not_short_circuit` test (with a positive
  control that a real shared URL still matches). No weights, thresholds, or
  probabilistic behaviour changed.

### Fixed

- **Security (SEC-M4): year bound in `normalize::iso_date_to_days`.** The
  year was parsed as an unbounded `i64`, so a crafted date such as
  `"99999999999999-01-01"` overflowed the `era * 146_097` term in
  `days_from_civil` (panic in debug, wrap in release) when the timeframe
  component parsed an attacker-supplied `start_date` / `target_date`. The
  year is now bounded to the ISO-8601 `0..=9999` range; out-of-range years
  return `None` (treated as an absent date). Regression test
  `iso_date_year_is_bounded_and_never_overflows`. Upholds the crate's
  "no panic in library code" rule.

## [0.1.0] - 2026-06-18

### Added

- **Inaugural release.** Specification + doc-set **and the implemented
  crate** for pairwise plan (Portfolio / Project / Product /
  Program) record matching, copy-adapted from the plan-matcher /
  care-pathway-matcher / case-matcher template. `spec/index.md` is the
  single source of truth. The crate builds and is fully tested
  (55 unit + 10 integration + 7 doctests; `clippy --all-targets
  --all-features -- -D warnings` clean; `cargo fmt` clean; zero
  `#[allow]`). Modules: `plan`, `matcher`, `scoring`, `config`,
  `normalize` (incl. `url` + ISO-date `iso_date_to_days`), `phonetic`,
  `error`; plus a `main.rs` demo binary. `MatchBreakdown` carries a
  `kind_gate_blocked` flag alongside `deterministic_match`.
  - Domain model: `Plan` (kind / name / alternate_names / code /
    owner_org_id / owner_org_name / lead_ref / portfolio_ref / status /
    goals / start_date / target_date / keywords / tags / identifiers /
    sameAs / in_language / relationships), `PlanKind` (closed set —
    Portfolio / Project / Product / Program, no `Custom`), `Goal` /
    `GoalStatus`, `PlanStatus`, `PlanIdentifier` /
    `IdentifierScheme`, `PlanRelationship` / `RelationKind`. The
    crate's `Plan` type is the API DTO + persisted JSONB payload +
    match input (no adapter).
  - **Kind gate (R-GATE)**: `A.kind != B.kind` short-circuits to `0.0`
    before every other rule — matching is within-kind only (replaces the
    ancestor's `plan_type` weighted component).
  - **Deterministic short-circuits**: R-0 globally-unique identifiers
    (URI, UUID, Jira project key, Asana GID, Trello board id, MS Project
    id, GitHub project id, Linear id); R-1 same-owner code; R-2
    `same_as` URL overlap. Owner-scoped (`Code`/`LocalId`) and `Custom`
    never short-circuit.
  - **Probabilistic components**: name 0.30 (Jaro-Winkler + Soundex
    bonus), goals 0.15 (Jaccard over folded goal titles), code 0.15
    (owner-scoped), owner org 0.10 (case-folded exact), portfolio 0.08
    (same parent `portfolio_ref`, child kinds), timeframe 0.07 (date
    proximity, Gaussian decay), keywords 0.05 (Jaccard), relationships
    0.05 (typed-set Jaccard over `(relation, plan_id)` pairs), tags
    0.05 (set Jaccard); renormalised over present components.
  - Normalisation: `fold`, `code` (alphanumeric-only), `fold_set`.
  - Classification: `High` ≥ 0.95, `Medium` ≥ 0.70, else `Low`; default
    threshold 0.85 (`strict()` 0.95, `lenient()` 0.70).
