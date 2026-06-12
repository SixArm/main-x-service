# Changelog

All notable changes to this crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md) — single source of truth;
> [README.md](./README.md) — user-facing intro; [AGENTS.md](./AGENTS.md) — agent guide.

## [Unreleased]

### Added

- **Inaugural release (v0.1.0).** Pairwise organization-record matching
  modelled on schema.org/Organization, copy-adapted from the
  course-matcher template.
  - Domain model: `Organization` (name / legalName / alternateName /
    identifiers / url / sameAs / address / jurisdiction / foundingDate /
    keywords), `OrgIdentifier`, `IdentifierScheme`, `PostalAddress`.
  - **Deterministic short-circuits**: R-0 globally-unique identifiers
    (LEI, DUNS, ISO 6523, GLN, Wikidata, ROR, ISNI, VAT); R-1
    same-jurisdiction tax id; R-2 `same_as` URL overlap. Classification
    codes (NAICS/ISIC/SIC) and `Custom` never short-circuit.
  - **Probabilistic components**: name (legal-suffix-aware Jaro-Winkler
    + Soundex bonus), postal address (weighted field-by-field), url /
    domain, jurisdiction, founding date (by year), keywords (Jaccard) —
    renormalised over present components.
  - `MatchConfig` weights (name 0.35, address 0.20, url 0.15,
    jurisdiction 0.10, founding_date 0.10, keywords 0.10; threshold
    0.85) with `strict()` / `lenient()` presets.
  - Normalisation: `fold`, `legal_name` (legal-suffix stripping),
    `domain` (URL→registered domain), `fold_set`.
  - 42 embedded unit tests + a 11-test public-API integration suite +
    7 doctests. Green `cargo build`, clippy clean, rustfmt formatted.
