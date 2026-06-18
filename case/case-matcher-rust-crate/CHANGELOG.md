# Changelog

All notable changes to this crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md) — single source of truth;
> [README.md](./README.md) — user-facing intro; [AGENTS.md](./AGENTS.md) — agent guide.

## [Unreleased]

### Changed

- **Docs/CI harmonization pass** (no behavioural change):
  - Fixed `Cargo.toml` `repository` URL to the nested, valid GitHub tree
    path `…/tree/main/case/case-matcher-rust-crate`.
  - Quoted the clippy gate as `cargo clippy --all-targets --all-features
    -- -D warnings` in spec §24, `AGENTS.md`, `AGENTS/testing.md`, and
    `README.md` to match CI and the repo-wide harmonization.
  - Classified `in_language` as a data-only field (spec §6/§14/§22,
    `AGENTS.md`, `README.md`, `AGENTS/matching-algorithm.md`,
    `AGENTS/spec-driven-development.md`): carried but never scored.
  - Corrected `../spec.md` link display text to `../spec/index.md` in
    `AGENTS/index.md` and `AGENTS/spec-driven-development.md`.

### Added

- Tests for previously-uncovered spec'd behaviour: the Soundex `+0.05`
  title bonus integration (§9), `alternate_titles` contribution and
  symmetry (§9), the `keywords` Jaccard component end-to-end (§13), and
  the documented serde wire shape of unit vs `Custom` enum variants (§6).
  Lib unit tests 42 → 46; public-API integration tests 12 → 13.
- Worked examples for the Soundex phonetic title bonus, renormalisation
  over partial components, and the strict/lenient threshold presets in
  `index.md` and `AGENTS/matching-algorithm.md`.

## [0.1.0] - 2026-06-13

### Added

- **Inaugural release.** Pairwise governmental case-management record
  matching, copy-adapted from the care-pathway-matcher template.
  - Domain model: `Case` (title / alternate titles / case number /
    agency / case type / status / priority / opened date / subjects /
    keywords / identifiers / sameAs / in language), `CaseType`,
    `CaseStatus`, `Priority`, `CaseIdentifier`, `IdentifierScheme`.
  - **Deterministic short-circuits**: R-0 globally-unique identifiers
    (`Docket`, `ExternalCaseId`, URI, UUID); R-1 same-agency case
    number; R-2 `same_as` URL overlap. Agency-scoped
    (`AgencyCaseNumber`/`LocalId`) and `Custom` never short-circuit.
  - **Probabilistic components**: title (Jaro-Winkler + Soundex bonus,
    0.30), subjects (Jaccard, 0.25), agency-scoped case number (0.15),
    case type (exact, 0.10), status (exact, 0.05), keywords (Jaccard,
    0.15); renormalised over present components. `priority` and
    `opened_date` are carried but never scored.
  - Normalisation: `fold`, `case_number` (alphanumeric-only), `url`,
    `fold_set`.
  - 42 embedded unit tests + a 12-test public-API integration suite + 7
    doctests. Green `cargo build`, clippy clean, rustfmt formatted.
