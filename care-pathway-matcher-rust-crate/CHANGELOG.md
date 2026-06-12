# Changelog

All notable changes to this crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md) — single source of truth;
> [README.md](./README.md) — user-facing intro; [AGENTS.md](./AGENTS.md) — agent guide.

## [Unreleased]

### Added

- **Inaugural release (v0.1.0).** Pairwise care-pathway (clinical
  pathway) record matching, copy-adapted from the course-matcher
  template.
  - Domain model: `CarePathway` (name / alternateName / pathway code /
    provider / care setting / condition codes / interventions / keywords
    / identifiers / sameAs), `ConditionCode`, `CodeSystem`,
    `CareSetting`, `PathwayIdentifier`, `IdentifierScheme`.
  - **Deterministic short-circuits**: R-0 globally-unique identifiers
    (DOI, Wikidata, guideline-registry id, URI, UUID); R-1
    same-provider pathway code; R-2 `same_as` URL overlap.
    Provider-scoped (`PathwayCode`/`LocalId`) and `Custom` never
    short-circuit.
  - **Probabilistic components**: name (Jaro-Winkler + Soundex bonus),
    target condition codes (Jaccard over `system:code` tokens),
    provider-scoped pathway code, care setting, interventions (Jaccard),
    keywords (Jaccard); renormalised over present components.
  - Normalisation: `fold`, `pathway_code` (alphanumeric-only), `fold_set`.
  - 38 embedded unit tests + a 10-test public-API integration suite + 7
    doctests. Green `cargo build`, clippy clean, rustfmt formatted.
