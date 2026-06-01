# Thing matcher — documentation index

A Rust crate for pairwise matching of geographic-place records (deterministic and probabilistic) for de-duplication and record linkage.

> **Crate:** `thing-matcher` v0.4.0 &nbsp;·&nbsp; **Edition:** Rust 2024 &nbsp;·&nbsp; **Licence:** MIT OR Apache-2.0 OR GPL-2.0 OR GPL-3.0 OR BSD-3-Clause
>
> **Repository:** https://github.com/sixarm/thing-matcher-rust-crate

---

## Where to start

| If you are… | Read this first |
|---|---|
| A new user of the crate | [README.md](./README.md) |
| Looking at one worked example end-to-end | [examples/basic_usage.rs](./examples/basic_usage.rs) |
| A contributor (human or AI) | [AGENTS.md](./AGENTS.md) |
| A reviewer who needs the authoritative behaviour | [spec.md](./spec.md) |
| Tracking what changed when | [CHANGELOG.md](./CHANGELOG.md) |

---

## At a glance

The "Spec section" column points to the authoritative definition in [`spec.md`](./spec.md).

| Capability | Where it lives | Spec section |
|---|---|---|
| Probabilistic match — `MatchingEngine::match_places` returning `MatchResult` with per-field `MatchBreakdown` and `Confidence` band | `src/matcher.rs` | §5.2, §3.7 |
| Deterministic match — `MatchingEngine::deterministic_match` returning `bool` | `src/matcher.rs` | §5.1 |
| Batch screening — `match_one_to_many`, `rank_one_to_many` | `src/matcher.rs` | §5.3 |
| Place data model — `Place`, `Address`, `PlaceCategory`, `PlaceId`, `PlaceIdScheme` | `src/models.rs` | §3.1–§3.5 |
| External place IDs — Google, OSM (Node / Way / Relation), GeoNames, Wikidata, Foursquare, Here, Mapbox, `Other` | `src/models.rs` | §3.5 |
| 36-variant `PlaceCategory` enum (`#[non_exhaustive]`) | `src/models.rs` | §3.4 |
| 15-field `Place` struct (10 scored, 5 data-only) | `src/models.rs` | §3.1.1 |
| Geographic primitives — `Scorer::haversine_metres`, `Scorer::coordinates_score` (Gaussian decay) | `src/scorer.rs` | §6.3 |
| String similarity — Jaro-Winkler, Levenshtein, Combined, Exact via `SimilarityAlgorithm` | `src/scorer.rs` | §6.1 |
| Name handling — diacritics, punctuation, alternate-name cartesian product, optional Soundex bonus | `src/normalizer.rs`, `src/matcher.rs` | §4.1, §6.1, §6.2 |
| International phone normalisation (E.164) — 39 jurisdictions via `COUNTRY_PHONE_TABLE` | `src/normalizer.rs` | §4.3 |
| Address parsing — house number / unit / street with English abbreviation expansion | `src/normalizer.rs` | §4.5 |
| Email normalisation — trim + lowercase + structural validation; optional Gmail dot- and `+tag`-folding | `src/normalizer.rs` | §4.4, §6.9 |
| Phonetic encoding — American Soundex | `src/normalizer.rs` | §4.6 |
| Config from JSON — `MatchConfig` derives serde with `#[serde(default)]`; tunings can live in a file | `src/matcher.rs` | §7 |
| Configuration presets — `default()`, `strict()`, `lenient()` | `src/matcher.rs` | §7.1–§7.3 |
| Determinism, no-IO, no-unsafe, `Send + Sync`, `#![forbid(unsafe_code)]`, `#![deny(missing_docs)]` | `src/lib.rs` | §8 |

---

## Core documents

### [spec.md](./spec.md) — living specification (SSOT)
The single source of truth. Scope, terminology, data model, normalisation rules, match strategies, per-field scoring, configuration, determinism guarantees, error model, SemVer policy, open questions, worked examples, glossary.

### [README.md](./README.md) — user-facing overview
Quick-start examples, the public API at a glance, configuration presets, batch scoring, limitations.

### [CHANGELOG.md](./CHANGELOG.md) — version history
Dated, per-version log of additions, behaviour changes, removals, and dependency churn. Kept in lockstep with `Cargo.toml` versions and `spec.md` updates.

### [AGENTS.md](./AGENTS.md) — agent / contributor guide
The rules of the road. Quick orientation, golden rules, workflow, what not to do, file layout.

---

## Agent topic guides ([AGENTS/](./AGENTS/))

Each topic guide cross-references the authoritative section of [`spec.md`](./spec.md).

| Guide | Topic | Authoritative spec section |
|---|---|---|
| [architecture.md](./AGENTS/architecture.md) | Module layout, layering rules, dependency graph | §3, §8 |
| [coding-style.md](./AGENTS/coding-style.md) | Rust conventions: naming, idioms, error handling, doc comments | §8, §9 |
| [testing.md](./AGENTS/testing.md) | Test pyramid, fixtures, property tests, doctests | (all) |
| [matching-algorithm.md](./AGENTS/matching-algorithm.md) | Deterministic and probabilistic scoring, weights, coordinates | §5, §6, §7 |
| [normalization.md](./AGENTS/normalization.md) | Name, postcode, phone, email, address, phonetic rules | §4 |
| [security-and-privacy.md](./AGENTS/security-and-privacy.md) | Personal-data handling, no-IO posture, threat model | §8 |
| [release.md](./AGENTS/release.md) | Versioning, CHANGELOG, publishing checklist | §9 |
| [spec-driven-development.md](./AGENTS/spec-driven-development.md) | How `spec.md` is maintained as the SSOT | §10 |

---

## Supporting files

- [Cargo.toml](./Cargo.toml) — manifest, dependencies, crate metadata.
- [CITATION.cff](./CITATION.cff) — citation metadata.
- [CONTRIBUTING.md](./CONTRIBUTING.md) — how to contribute.
- [CODE_OF_CONDUCT.md](./CODE_OF_CONDUCT.md) — community standards.
- [LICENSE.md](./LICENSE.md) — multi-licence offering.
- [src/](./src/) — crate source.
- [tests/](./tests/) — integration tests and property tests.
- [examples/](./examples/) — runnable examples (`basic_usage.rs`, `custom_config.rs`).
- [benches/](./benches/) — criterion benchmarks (`match_pair.rs`).

---

## Common tasks

| Task | Command |
|---|---|
| Build | `cargo build` |
| Run unit + integration + property + doctest | `cargo test` |
| Run a single test | `cargo test test_name` |
| Show test stdout | `cargo test -- --nocapture` |
| Run property tests only | `cargo test --test property_tests` |
| Pin the public API | `cargo test --test adapter_contract` (downstream-service-adapter contract guardrail) |
| Lint | `cargo clippy --all-targets -- -D warnings` |
| Format | `cargo fmt` |
| Run the demo | `cargo run` |
| Run an example | `cargo run --example basic_usage` (or `--example custom_config`) |
| Run benchmarks | `cargo bench` (use `--quick` for a smoke run; HTML reports in `target/criterion/`) |
| Build API docs | `cargo doc --no-deps --open` |
| Pre-publish dry run | `cargo publish --dry-run` |

---

## Common questions

| Question | Answer |
|---|---|
| How do I match two places? | Build two `Place` values, then call `MatchingEngine::default_config().match_places(&a, &b)`. See [README.md](./README.md) and [`spec.md` §5.2](./spec.md). |
| What does the default config do? | `match_threshold = 0.80`, weighted sum across name (0.20), coordinates (0.30, Gaussian decay scale 50 m), address (0.10), category (0.10), country code (0.05), place IDs (0.15), phone (0.03), email (0.02). See [`spec.md` §7.1](./spec.md). |
| When should I use strict mode? | When false positives are more dangerous than false negatives — `MatchConfig::strict()` raises the threshold to 0.95 and additionally requires `deterministic_match` for `is_match = true`. See [`spec.md` §5.2.3, §7.2](./spec.md). |
| When should I use lenient mode? | When triaging large candidate sets where false negatives are worse than false positives — `MatchConfig::lenient()` lowers the threshold to 0.65 and turns on the phonetic bonus. See [`spec.md` §7.3](./spec.md). |
| Where are the algorithms documented? | Per-field scoring algorithms in [`spec.md` §6](./spec.md); pipeline overview in [AGENTS/matching-algorithm.md](./AGENTS/matching-algorithm.md). |
| Why are some breakdown fields `None`? | The renormalisation rule: missing fields neither contribute to nor penalise the score. See [`spec.md` §5.4](./spec.md). |
| How do `place_ids` short-circuit? | Two places sharing any one `(scheme, value)` pair are a `deterministic_match`. See [`spec.md` §5.1, §6.7](./spec.md). |
| What is "data-only" vs "scored"? | Five fields (`local_id`, `altitude_as_metre`, `elevation_as_metre`, `area_as_metre_2`, `maximum_capacity_count`) round-trip through serde for downstream consumers but are not consulted by the matcher. See [`spec.md` §3.1.1](./spec.md). |

---

## Project conventions at a glance

- **Specification-first.** Behavioural changes update [`spec.md`](./spec.md) in the same PR as the code. See [AGENTS/spec-driven-development.md](./AGENTS/spec-driven-development.md).
- **Pure library.** No IO, no logging, no global state — see [AGENTS/security-and-privacy.md](./AGENTS/security-and-privacy.md).
- **Deterministic.** Same inputs always produce the same outputs, byte-for-byte.
- **Diacritic-correct.** Unicode diacritics survive normalisation — see [AGENTS/normalization.md](./AGENTS/normalization.md).
- **Explainable.** Every probabilistic match returns a per-field breakdown. See [`spec.md` §3.7](./spec.md).
- **`#[non_exhaustive]` everywhere it matters.** `Place`, `Address`, `PlaceCategory`, `PlaceIdScheme`, `MatchingError` — adding fields / variants is non-breaking. See [`spec.md` §9.1](./spec.md).
