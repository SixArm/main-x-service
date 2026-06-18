# Changelog

All notable changes to this crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md) — single source of truth;
> [README.md](./README.md) — user-facing intro; [AGENTS.md](./AGENTS.md) — agent guide.

## [Unreleased]

## [0.1.0] - 2026-06-17

### Added

- **Inaugural release (spec-only).** Specification + doc-set for
  pairwise plan (project / product / programme / initiative / portfolio
  / epic) record matching, copy-adapted from the care-pathway-matcher
  template. Code is not yet written; `spec/index.md` is the single
  source of truth and the build queue lives in spec §23.
  - Domain model: `Plan` (name / alternate_names / plan_type /
    plan_code / owner_org_id / owner_org_name / lead_ref / status /
    goals / start_date / target_date / keywords / tags / identifiers /
    sameAs / in_language / relationships), `Goal` / `GoalStatus`,
    `PlanType`, `PlanStatus`, `PlanIdentifier` / `IdentifierScheme`,
    `PlanRelationship` / `RelationKind`. The crate's `Plan` type is the
    API DTO + persisted payload + match input (no adapter).
  - **Deterministic short-circuits**: R-0 globally-unique identifiers
    (URI, UUID, Jira project key, Asana GID, Trello board id, MS Project
    id, GitHub project id, Linear id); R-1 same-owner plan code; R-2
    `same_as` URL overlap. Owner-scoped (`PlanCode`/`LocalId`) and
    `Custom` never short-circuit.
  - **Probabilistic components**: name 0.30 (Jaro-Winkler + Soundex
    bonus), goals 0.15 (Jaccard over folded goal titles), plan code 0.15
    (owner-scoped), owner org 0.10 (case-folded exact), plan type 0.08
    (exact enum), timeframe 0.07 (date proximity, Gaussian decay),
    keywords 0.05 (Jaccard), relationships 0.05 (typed-set Jaccard over
    `(relation, plan_id)` pairs), tags 0.05 (set Jaccard); renormalised
    over present components.
  - Normalisation: `fold`, `plan_code` (alphanumeric-only), `fold_set`.
  - Classification: `High` ≥ 0.95, `Medium` ≥ 0.70, else `Low`; default
    threshold 0.85 (`strict()` 0.95, `lenient()` 0.70).
