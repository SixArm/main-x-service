# Event matcher — documentation index

A Rust crate for pairwise matching of event records (deterministic and probabilistic) modelled on [schema.org/Event](https://schema.org/Event), for de-duplication and record linkage.

> **Crate:** `event-matcher` v0.5.0 &nbsp;·&nbsp; **Edition:** Rust 2024 &nbsp;·&nbsp; **Licence:** MIT OR Apache-2.0 OR GPL-2.0 OR GPL-3.0 OR BSD-3-Clause
>
> **Repository:** https://github.com/sixarm/event-matcher-rust-crate

> **Domain change in 0.5.0.** Prior 0.4.x releases matched geographic *places*. From 0.5.0 the crate matches schema.org/Event records — festivals, conferences, concerts, sports fixtures, screenings, hackathons. See [CHANGELOG.md](./CHANGELOG.md) for the full rename / removal / addition list.

---

## Where to start

| If you are… | Read this first |
|---|---|
| A new user of the crate | [README.md](./README.md) |
| Looking at one worked example end-to-end | [examples/basic_usage.rs](./examples/basic_usage.rs) |
| A contributor (human or AI) | [AGENTS.md](./AGENTS.md) |
| A reviewer who needs the authoritative behaviour | [spec.md](./spec.md) — note: 0.4.x place-matcher spec; 0.5.0 event-matcher surface is documented inline in source and README. |
| Tracking what changed when | [CHANGELOG.md](./CHANGELOG.md) |

---

## At a glance

| Capability | Where it lives |
|---|---|
| Probabilistic match — `MatchingEngine::match_events` returning `MatchResult` with per-field `MatchBreakdown` and `Confidence` band | `src/matcher.rs` |
| Deterministic match — `MatchingEngine::deterministic_match` returning `bool` (shared `EventId` OR normalised name + same `start_date` instant) | `src/matcher.rs` |
| Batch screening — `match_one_to_many`, `rank_one_to_many` | `src/matcher.rs` |
| Event data model — `Event`, `EventBuilder`, `Address`, `Location`, `EventCategory`, `EventStatus`, `EventAttendanceMode`, `EventId`, `EventIdScheme` | `src/models.rs` |
| External event IDs — Wikidata, Eventbrite, Meetup, Ticketmaster, Songkick, Bandsintown, Facebook, Luma, GoogleCalendar, ICalendarUid, `Other` | `src/models.rs` |
| 24-variant `EventCategory` enum (`#[non_exhaustive]`, schema.org/Event subtypes) | `src/models.rs` |
| Schema.org-aligned `Event` struct covering identity, schedule, location, classification, capacity, people, hierarchy | `src/models.rs` |
| Temporal primitives — `Scorer::seconds_between`, `Scorer::start_date_score` (Gaussian decay) | `src/scorer.rs` |
| Geographic primitives — `Scorer::haversine_metres`, `Scorer::coordinates_score` (Gaussian decay) | `src/scorer.rs` |
| ISO 8601 parser — `Normalizer::parse_iso8601_unix_seconds` (date, naive datetime, Z, ±HH:MM, fractional seconds) | `src/normalizer.rs` |
| String similarity — Jaro-Winkler, Levenshtein, Combined, Exact via `SimilarityAlgorithm` | `src/scorer.rs` |
| Name handling — diacritics, punctuation, alternate-name cartesian product, optional Soundex bonus | `src/normalizer.rs`, `src/matcher.rs` |
| Address parsing — house number / unit / street with English abbreviation expansion | `src/normalizer.rs` |
| Config from JSON — `MatchConfig` derives serde with `#[serde(default)]` | `src/matcher.rs` |
| Configuration presets — `default()`, `strict()`, `lenient()` | `src/matcher.rs` |
| Determinism, no-IO, no-unsafe, `Send + Sync`, `#![forbid(unsafe_code)]`, `#![deny(missing_docs)]` | `src/lib.rs` |

---

## Core documents

### [spec.md](./spec.md) — living specification
The authoritative behaviour spec. Pre-0.5.0 the spec described place matcher; sections that overlap (string-similarity algorithms, normalisation, scoring pipeline, determinism guarantees, SemVer policy) remain accurate. The event-specific data model and weight table for 0.5.0 are documented inline in `src/`, README, and CHANGELOG until the spec is rewritten in a follow-up.

### [README.md](./README.md) — user-facing overview
Quick-start examples, the public API at a glance, configuration presets, batch scoring, limitations.

### [CHANGELOG.md](./CHANGELOG.md) — version history
Dated, per-version log of additions, behaviour changes, removals, and dependency churn.

### [AGENTS.md](./AGENTS.md) — agent / contributor guide
The rules of the road. Quick orientation, golden rules, workflow, file layout.

---

## Agent topic guides ([AGENTS/](./AGENTS/))

| Guide | Topic |
|---|---|
| [architecture.md](./AGENTS/architecture.md) | Module layout, layering rules, dependency graph |
| [coding-style.md](./AGENTS/coding-style.md) | Rust conventions: naming, idioms, error handling, doc comments |
| [testing.md](./AGENTS/testing.md) | Test pyramid, fixtures, property tests, doctests |
| [matching-algorithm.md](./AGENTS/matching-algorithm.md) | Deterministic and probabilistic scoring, weights, schedule + location |
| [normalization.md](./AGENTS/normalization.md) | Name, postcode, datetime, address, phonetic rules |
| [security-and-privacy.md](./AGENTS/security-and-privacy.md) | Personal-data handling, no-IO posture, threat model |
| [release.md](./AGENTS/release.md) | Versioning, CHANGELOG, publishing checklist |
| [spec-driven-development.md](./AGENTS/spec-driven-development.md) | How `spec.md` is maintained |

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
| How do I match two events? | Build two `Event` values, then call `MatchingEngine::default_config().match_events(&a, &b)`. See [README.md](./README.md). |
| What does the default config do? | `match_threshold = 0.80`, weighted sum across name (0.20), start_date (0.25, Gaussian scale 1 h), end_date (0.05), location (0.15), category (0.08), country code (0.04), event IDs (0.15), organizer (0.04), performers (0.02), URL (0.02). |
| When should I use strict mode? | When false positives are more dangerous than false negatives — `MatchConfig::strict()` raises the threshold to 0.95 and additionally requires `deterministic_match` for `is_match = true`. |
| When should I use lenient mode? | When triaging large candidate sets where false negatives are worse than false positives — `MatchConfig::lenient()` lowers the threshold to 0.65 and turns on the phonetic bonus. |
| Why are some breakdown fields `None`? | The renormalisation rule: missing fields neither contribute to nor penalise the score. |
| How do `event_ids` short-circuit? | Two events sharing any one `(scheme, value)` pair are a `deterministic_match`. |
| What ISO 8601 shapes does `start_date` accept? | `YYYY-MM-DD`, `YYYY-MM-DDTHH:MM[:SS]` (interpreted as UTC), and the same with `Z` / `±HH:MM` / fractional seconds. |

---

## Project conventions at a glance

- **Pure library.** No IO, no logging, no global state.
- **Deterministic.** Same inputs always produce the same outputs, byte-for-byte.
- **Diacritic-correct.** Unicode diacritics survive normalisation.
- **Explainable.** Every probabilistic match returns a per-field breakdown.
- **`#[non_exhaustive]` everywhere it matters.** `Event`, `Address`, `Location`, `EventCategory`, `EventIdScheme`, `EventStatus`, `EventAttendanceMode`, `MatchingError` — adding fields / variants is non-breaking.
