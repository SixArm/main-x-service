# Changelog

All notable changes to this crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md) — single source of truth;
> [README.md](./README.md) — user-facing intro; [AGENTS.md](./AGENTS.md) — agent guide.

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
