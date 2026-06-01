# Person matcher — Documentation Index

A Rust crate for pairwise person-record matching (deterministic and probabilistic) in healthcare information exchange scenarios.

> **Crate:** `person-matcher` v0.3.0 &nbsp;·&nbsp; **Edition:** Rust 2024 &nbsp;·&nbsp; **Licence:** MIT OR Apache-2.0 OR GPL-2.0 OR GPL-3.0 OR BSD-3-Clause
>
> **Repository:** https://github.com/sixarm/person-matcher-rust-crate

## At a Glance

| Capability | Where it lives |
|---|---|
| **Multinational national identifiers** (**42 schemes**) — UK United Kingdom National Health Service Number / FR NIR / ES TSI / IE IHI / UK NI H&C / US SSN / AU IHI / DE KVNR / IT CF / NL BSN / SE Personnummer / UK Scotland CHI / BE NN / BG EGN / CZ RČ / DK CPR / EE IK / ES DNI / FI HETU / HR OIB / IS KT / LT AK / LV PK / MT ID / NO FNR / PL PESEL / RO CNP / SI EMŠO / SK RČ / UK NINO / GR DSS / LI ID / NL ID / PL NIP / PT NIF / BR CPF / CN RRN / IN Aadhaar / JP My Number / MX CURP / NZ NHI / ZA ID | `src/identifiers.rs`, spec §6.4 / §14.5 |
| **Passport-number format validators** (9 jurisdictions) — CY, CZ, LI, LT, MT, NL, PT, RO, SK | `src/identifiers.rs`, spec §6.4a (`PassportBook`) |
| **Blood type** — `BloodType` enum (8 ABO+RhD variants); strong negative signal | `src/models.rs`, spec §8.2a / §12.2 |
| **Place of birth** — `Person::birth_place: Option<Address>` (FHIR parity); city + country sub-score | `src/matcher.rs::score_birth_place`, spec §12.4a |
| **Multiple birth** — FHIR `Patient.multipleBirth`; identical-twin disambiguation | `src/matcher.rs::score_multiple_birth`, spec FR-82 |
| **Date of death** — FHIR `Patient.deceasedDateTime`; DD/MM ↔ MM/DD transposition heuristic | `src/matcher.rs::score_death_date`, spec §12.4c / FR-83 |
| **Place of death** — `Person::death_place: Option<Address>`; reuses the shared `score_named_place` helper with place of birth | `src/matcher.rs::score_death_place`, spec §12.4b / FR-84 |
| **Passport books** — multi-country, multi-book, time-varying via `Vec<PassportBook>` | `src/models.rs`, spec §6.4a / §8.6 / §12.1 |
| **Probabilistic match** with weighted per-field [`MatchBreakdown`] and `Confidence` band | `src/matcher.rs`, spec §12.3 / §12.5 |
| **Deterministic match** (any-identifier or demographic-tuple agreement) | `src/matcher.rs`, spec §12.1 |
| **Batch API** — `match_one_to_many` and `rank_one_to_many` for MPI screening | `src/matcher.rs`, spec §12.6 |
| **Name handling** — diacritics, punctuation, phonetic (Soundex), nickname dictionary | `src/normalizer.rs`, `src/nicknames.rs`, spec §14.1 / §14.4 |
| **International phone (E.164)** — `+CC` / `00CC` / default-country, **39 countries** (one per national-identifier scheme jurisdiction) | `src/normalizer.rs`, spec §14.3.2 / §21.4 (T-19) |
| **Address parsing** — house number / unit / street with abbreviation expansion | `src/normalizer.rs`, spec §14.4a |
| **Email scoring** with optional Gmail dot/+-folding | `src/normalizer.rs`, spec §14.3a |
| **Config from JSON** — `MatchConfig`, `SimilarityAlgorithm`, `NicknameTable` are serde-derived; `MatchConfig` carries `#[serde(default)]` | `src/matcher.rs`, spec §16 |

---

## Where To Start

| If you are… | Read this first |
|---|---|
| A new user of the crate | [README.md](./README.md) |
| A contributor (human or AI) | [AGENTS.md](./AGENTS.md) |
| A reviewer who needs the authoritative behaviour | [spec.md](./spec.md) |
| Tracking what changed when | [CHANGELOG.md](./CHANGELOG.md) |
| Curious about clinical / research background | [help/](./help/) + [spec.md §5](./spec.md#5-research-basis) |

---

## Core Documents

### [spec.md](./spec.md) — Living Specification
The single source of truth. Covers purpose, scope, requirements, domain model, architecture, algorithms, configuration, normalisation, error model, testing strategy, roadmap, open questions, and the task backlog. Any behavioural change must be reflected here.

### [README.md](./README.md) — User-Facing Overview
Quick-start, public-API examples, feature list, research references, and contribution pointers. Aimed at the consuming developer.

### [CHANGELOG.md](./CHANGELOG.md) — Version History
Dated, per-version log of additions, behaviour changes, and fixes. Keep in lockstep with `Cargo.toml` versions and `spec.md` updates.

### [AGENTS.md](./AGENTS.md) — Agent / Contributor Guide
The rules of the road. Workflow, golden rules, and links to topic-specific guides under `AGENTS/`.

---

## Agent Topic Guides ([AGENTS/](./AGENTS/))

- [architecture.md](./AGENTS/architecture.md) — Module layout and layering.
- [coding-style.md](./AGENTS/coding-style.md) — Rust conventions used here.
- [testing.md](./AGENTS/testing.md) — Test pyramid, naming, fixtures.
- [matching-algorithm.md](./AGENTS/matching-algorithm.md) — Deterministic and probabilistic scoring.
- [normalization.md](./AGENTS/normalization.md) — Name, postcode, phone, phonetic rules.
- [security-and-privacy.md](./AGENTS/security-and-privacy.md) — PII, data protection, clinical safety.
- [release.md](./AGENTS/release.md) — Versioning and publishing.
- [spec-driven-development.md](./AGENTS/spec-driven-development.md) — How `spec.md` is maintained as the single source of truth.
- [national-person-identifiers.md](./AGENTS/national-person-identifiers.md) — Reference table of the supported national personal identifier schemes (42 total post-T-17.1; the TSV / .md reference table covers the original 35 — the 7 T-17.1 schemes are documented in `src/identifiers.rs` and spec FR-85..FR-91).

---

## Supporting Files

- [Cargo.toml](./Cargo.toml) — Manifest, dependencies, crate metadata.
- [CITATION.cff](./CITATION.cff) — Citation metadata.
- [CONTRIBUTING.md](./CONTRIBUTING.md) — How to contribute.
- [CODE_OF_CONDUCT.md](./CODE_OF_CONDUCT.md) — Community standards.
- [IMPLEMENTATION_SUMMARY.md](./IMPLEMENTATION_SUMMARY.md) — Historical snapshot of the initial implementation. Superseded by `spec.md`; retained for context.
- [help/](./help/) — Source research PDFs.
- [src/](./src/) — Crate source.
- [tests/](./tests/) — Integration tests.
- [examples/](./examples/) — Runnable examples (`basic_usage.rs`, `custom_config.rs`).
- [benches/](./benches/) — Criterion benchmarks (`match_pair.rs`).
- [scripts/spec-drift-check.sh](./scripts/spec-drift-check.sh) — Spec/code drift enforcer (T-7); runs locally or in CI.
- [.github/workflows/spec-drift.yml](./.github/workflows/spec-drift.yml) — GitHub Actions workflow that invokes the spec-drift check on every pull request to `main`.
- [.spec-allow](./.spec-allow) — Path-pattern exceptions to the spec-drift requirement.

---

## Common Tasks

| Task | Command |
|---|---|
| Build | `cargo build` |
| Run tests | `cargo test` (includes `tests/property_tests.rs` — 1000 proptest cases per property) |
| Run benchmarks | `cargo bench` (use `--quick` for a smoke run; HTML reports land in `target/criterion/`) |
| Run property tests only | `cargo test --test property_tests` |
| Lint | `cargo clippy --all-targets -- -D warnings` |
| Format | `cargo fmt` |
| Run the demo | `cargo run` |
| Run an example | `cargo run --example basic_usage` |
| Build API docs | `cargo doc --no-deps --open` |

---

## Project Conventions At a Glance

- **Specification-first.** Behaviour changes update [`spec.md`](./spec.md) in the same PR as the code.
- **Pure library.** No IO, no logging, no global state — see [AGENTS/security-and-privacy.md](./AGENTS/security-and-privacy.md).
- **Deterministic.** Same inputs always produce the same outputs.
- **Diacritic-correct.** Unicode diacritics survive normalisation — see [AGENTS/normalization.md](./AGENTS/normalization.md).
- **Explainable.** Every probabilistic match returns a per-field breakdown.
