# Changelog

All notable changes to this crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md) — single source of truth;
> [README.md](./README.md) — user-facing intro; [AGENTS.md](./AGENTS.md) — agent guide.

## [Unreleased]

## [0.1.0] - 2026-06-18

### Added

- **Inaugural release.** Specification + doc-set **and the implemented
  crate** for pairwise work-item (Portfolio / Project / Product /
  Program) record matching, copy-adapted from the plan-matcher /
  care-pathway-matcher / case-matcher template. `spec/index.md` is the
  single source of truth. The crate builds and is fully tested
  (55 unit + 10 integration + 7 doctests; `clippy --all-targets
  --all-features -- -D warnings` clean; `cargo fmt` clean; zero
  `#[allow]`). Modules: `work_item`, `matcher`, `scoring`, `config`,
  `normalize` (incl. `url` + ISO-date `iso_date_to_days`), `phonetic`,
  `error`; plus a `main.rs` demo binary. `MatchBreakdown` carries a
  `kind_gate_blocked` flag alongside `deterministic_match`.
  - Domain model: `WorkItem` (kind / name / alternate_names / code /
    owner_org_id / owner_org_name / lead_ref / portfolio_ref / status /
    goals / start_date / target_date / keywords / tags / identifiers /
    sameAs / in_language / relationships), `WorkItemKind` (closed set —
    Portfolio / Project / Product / Program, no `Custom`), `Goal` /
    `GoalStatus`, `WorkItemStatus`, `WorkItemIdentifier` /
    `IdentifierScheme`, `WorkItemRelationship` / `RelationKind`. The
    crate's `WorkItem` type is the API DTO + persisted JSONB payload +
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
    0.05 (typed-set Jaccard over `(relation, work_item_id)` pairs), tags
    0.05 (set Jaccard); renormalised over present components.
  - Normalisation: `fold`, `code` (alphanumeric-only), `fold_set`.
  - Classification: `High` ≥ 0.95, `Medium` ≥ 0.70, else `Low`; default
    threshold 0.85 (`strict()` 0.95, `lenient()` 0.70).
