# Delivered Tasks — Full Acceptance Criteria

This file archives the per-task acceptance-criteria detail for every delivered task from `spec.md` §23.2. The summary list and §23.1 carry-over are in [`delivered-tasks.md`](./delivered-tasks.md).

Status legend: `[x]` done.


**T-1 — Serialisable config.** Delivered.
- [x] Derive `Serialize, Deserialize` for `MatchConfig`, `SimilarityAlgorithm`, and `NicknameTable`.
- [x] `MatchConfig` carries `#[serde(default)]` so partial JSON documents merge over `MatchConfig::default()`.
- **Acceptance:** `MatchConfig::default()`, `strict()`, and `lenient()` round-trip through `serde_json` with all values preserved; partial JSON inherits defaults; end-to-end test confirms an engine built from a deserialised config matches the original byte-for-byte. Met by `tests/integration_tests.rs` §18 and `src/matcher.rs::tests`.

**T-2 — `Confidence` in `MatchResult`.** Delivered.
- [x] Add public `Confidence` enum (`High`/`Medium`/`Low`) in `src/matcher.rs`, re-exported from the crate root.
- [x] Add `pub confidence: Confidence` to `MatchResult` populated by `Confidence::from_score(score)` on every `match_persons` call.
- [x] Boundaries: `≥ 0.90 → High`, `≥ 0.75 → Medium`, else `Low` (inclusive on the low side).
- [x] Confidence is **independent of `match_threshold`** — a score of `0.92` is `High` under strict, default, and lenient presets alike. `is_match` remains the authoritative go/no-go signal.
- [x] `confidence` is `#[serde(default = "default_confidence")]` so legacy JSON payloads lacking the field deserialise to `Low` (interpretable as "needs re-scoring").
- **Acceptance:** Unit tests pin band boundaries, threshold independence, and serde round-trip. Integration tests pin: High for exact clones, Low for completely different persons, threshold-independence under strict vs lenient, and legacy JSON deserialisation. Met by `tests/integration_tests.rs` §17 and `src/matcher.rs::tests`.

**T-3 — Address sub-score correction.** Delivered.
- [x] Resolve §22 OQ-4. Implemented option (b): the address sub-score now accumulates `weighted_sum` and `total_weight` independently and divides at the end (`Σ(score × weight) / Σ(weight)`), so postcode (`0.5`) dominates as documented.
- [x] Neutral fallback (`0.5`) preserved when no sub-component fires.
- **Acceptance:** 4 unit tests + 2 integration tests in §26 pin the behaviour: exact postcode + slight street typo clears `0.7`, postcode-only match collapses to `1.0`, postcode-match + line1-mismatch is dominated by postcode, and the empty-address neutral fallback still returns `0.5`.

**T-4 — `strict_mode` enforcement.** Delivered.
- [x] Resolve §22 OQ-5: under `strict_mode = true`, set `is_match = (score >= threshold) && deterministic_match(...)`.
- [x] Probabilistic `score` and `confidence` remain unchanged across modes.
- **Acceptance:** Existing strict-mode integration tests continue to pass; new tests verify (a) a fuzzy match clearing a lowered strict threshold but lacking a deterministic anchor is rejected; (b) a deterministic match clearing the strict threshold is accepted; (c) the non-strict default still accepts fuzzy matches above the default threshold. Met by `src/matcher.rs::tests` and `tests/integration_tests.rs` §7.

**T-5 — Benchmarks.** Delivered.
- [x] Add `benches/match_pair.rs` using `criterion` (HTML reports enabled via the `html_reports` feature).
- [x] Four bench groups cover the hot paths: `match_pair` (identical / fuzzy / unrelated), `deterministic_match_identifier_hit`, `rank_one_to_many` (`n ∈ {10, 100, 1000}`, with criterion throughput reporting per candidate), and `config_variants` (default vs strict vs nickname-table-loaded).
- **Acceptance:** `cargo bench` compiles and runs end-to-end. Indicative single-machine numbers (2024 Apple Silicon, `--quick`): `match_pair / fuzzy_near_match ≈ 4 µs`, `deterministic_match_identifier_hit ≈ 160 ns`, `rank_one_to_many @ n=1000 ≈ 3 ms` (~3 µs/element). All well under the §17 budget of `< 50 µs` per pair.

**T-6 — Property tests.** Delivered.
- [x] Add `proptest` dev-dependency and properties listed in §18.4.
- [x] Eleven properties in `tests/property_tests.rs` covering normalisation idempotency, score bounds, self-match, symmetry (probabilistic and deterministic), confidence monotonicity, and serde round-trips for `Person` and `MatchConfig`.
- **Acceptance:** `cargo test --test property_tests` runs 1000 cases per property with zero failures. `tests/property_tests.proptest-regressions` is checked in so historical shrunk seeds are re-tried on every run.

**T-7 — Spec/code drift CI check.** Delivered.
- [x] Workflow: `.github/workflows/spec-drift.yml` runs on every pull request targeting `main`. It fetches full history (`fetch-depth: 0`) so the diff against the base ref is accurate, then invokes `scripts/spec-drift-check.sh` with the GitHub-provided base ref and head SHA.
- [x] Check script: `scripts/spec-drift-check.sh` (POSIX bash, no external dependencies beyond `git`). Resolves the base ref (`origin/<ref>` if available, else local `<ref>`), computes the changed-file set via `git merge-base` + `git diff --name-only`, then enforces: if any file matching the watched pattern (initially `^src/matcher\.rs$`) changed, `spec.md` MUST also have changed. Path patterns in `.spec-allow` (extended regex, blank / `#`-prefixed lines ignored) override the requirement for genuinely spec-irrelevant paths.
- [x] Allowlist: `.spec-allow` ships empty (modulo header comment), so the discipline starts maximally strict; reviewers add patterns as concrete need arises.
- [x] PR template: `.github/pull_request_template.md` references the spec-drift check, lists the spec / allowlist / CHANGELOG checkboxes, and prompts contributors for a test plan.
- [x] Script also runs cleanly from a contributor's machine pre-push (no GitHub-specific assumptions) and exits 0 gracefully when the base ref cannot be resolved (avoids spurious failures in fork CI).
- [x] Verified the script's pass paths against historical commits (`bash scripts/spec-drift-check.sh <older> <newer>` for commits that touched both matcher.rs and spec.md returns `OK`).
- **Acceptance:** Met. CI green on initial introduction because this PR ships `spec.md` updates alongside its source changes. Future PRs that touch `src/matcher.rs` without `spec.md` will fail the `spec-drift` check unless the changed paths match `.spec-allow`. The PR template references the check by name.

**T-8 — Mark `Person` and `Address` `#[non_exhaustive]`.** Delivered.
- [x] Add `#[non_exhaustive]` to both struct definitions in `src/models.rs`.
- [x] `Person::builder()` is the canonical constructor; `Address::new()` + field assignment + new `with_*` fluent setters cover ergonomic external construction.
- [x] Field-assignment syntax on `Address` (`a.line1 = Some(...)`) continues to work because `#[non_exhaustive]` does not block individual field access.
- **Acceptance:** Crate compiles unchanged; tests / examples / doctests all pass (the crate-internal struct-literal use in `Address::new()` is allowed inside the defining crate). External struct-literal construction is now a compile error pointing consumers at the builder. Met by passing test suite at 524 tests after the attribute is added.

**T-9 — Locale-aware phonetic encoder (research spike).** Delivered (recommendation; implementation deferred to T-9.1).
- [x] Surveyed Soundex (status quo), Double Metaphone, NYSIIS, Daitch-Mokotoff Soundex, Beider-Morse, locale-specific encoders (Kölner Phonetik etc.), and a custom encoder. Decision matrix and rationale in `AGENTS/roadmap-research.md`.
- [x] Sample size and methodology documented: the corpus shape (≥ 10k triples per jurisdiction across English-majority + Romance + Germanic + Slavic + Nordic populations) and metrics (TPR at the FR-23 `> 0.9` threshold, FPR, AUC, per-jurisdiction breakdown) a future empirical evaluation should use.
- [x] Recommendation: **stay with Soundex as the default**, add `MatchConfig::phonetic_encoder: PhoneticEncoder` enum with `Soundex` / `DoubleMetaphone` / `DaitchMokotoff` variants behind a `phonetic-rphonetic` Cargo feature flag, defer the default-switch decision until an empirical corpus exists.
- **Acceptance:** Met — written recommendation in `AGENTS/roadmap-research.md` with sample-size proposal, corpus specification, and evaluation methodology.

**T-10 — Nickname dictionary.** Delivered.
- [x] Public `NicknameTable` type in `src/nicknames.rs` exposing `empty()`, `english()`, `with_class()`, `are_equivalent()`, `is_empty()`, `len()`.
- [x] `MatchConfig::nickname_table: NicknameTable` defaults to `NicknameTable::empty()`; the feature is opt-in.
- [x] `MatchingEngine`'s `score_name` lifts the per-name component score to `max(score, 0.9)` when the table considers the pair equivalent. The boost never lowers a score.
- [x] Built-in English dictionary covers ≥40 common classes including the acceptance set.
- **Acceptance:** `Mike`↔`Michael`, `Liz`↔`Elizabeth`, `Bob`↔`Robert` lift the given-name score to ≥ 0.9. Met by `tests/integration_tests.rs` §15 and `src/nicknames.rs::tests`.

**T-11 — Email and `local_id` scoring (per OQ-2).** Delivered.
- [x] Implement `Normalizer::normalize_email(email, gmail_dot_folding) -> Option<String>` with trim + lowercase + structural validation, and opt-in Gmail dot/plus-tag folding for `gmail.com` / `googlemail.com`.
- [x] Add `MatchConfig::email_weight: f64` (default 0.05) and `MatchConfig::gmail_dot_folding: bool` (default false).
- [x] Add `MatchBreakdown::email_score: Option<f64>` populated from the canonical form.
- [x] `local_id` is deliberately not scored (cross-organisation collision risk); document explicitly.
- **Acceptance:** Unit tests cover case/whitespace canonicalisation, Gmail dot-folding (on/off), `+tag` stripping, non-Gmail untouched, malformed input → `None`, idempotence. Integration tests cover exact match, mismatch, missing on one side, unparseable yields `None`, and the dot-folding opt-in toggle. Met by `tests/integration_tests.rs` §16 and `src/normalizer.rs::tests`.

**T-12 — Documentation harmonisation.** Delivered.
- [x] Every top-level doc file now points to `index.md` as the entry point — verified by `rg index\.md` finding all 7 top-level markdown files (README, AGENTS, spec, CHANGELOG, CONTRIBUTING, CODE_OF_CONDUCT, IMPLEMENTATION_SUMMARY) plus `AGENTS/spec-driven-development.md`.
- [x] The previously-orphaned `AGENTS/national-person-identifiers.md` (35-scheme reference table) is now linked from both `AGENTS.md` and `index.md`.
- [x] `IMPLEMENTATION_SUMMARY.md` carries an explicit "superseded by `spec.md`" banner so readers do not mistake the historical snapshot for current behaviour.
- **Acceptance:** All intra-repo links resolve (manual sweep against the 17 referenced files). Every top-level doc + the spec-driven-development guide points to `index.md`. No orphaned guides under `AGENTS/`.

**T-13 — Remove or wire-up unused error variants.** Delivered.
- [x] Resolved §22 OQ-6. Removed `InvalidData`, `InvalidNhsNumber`, `InvalidDate`, and `ConfigError`: none was returned from any code path in 0.3.0 — identifier parsers return `Option<String>` instead of `Result`, `MatchConfig` builders are infallible, and the crate does not parse date strings.
- [x] Marked `MatchingError` as `#[non_exhaustive]` so future fallible code paths can introduce variants without breaking SemVer for downstream pattern-matches.
- **Acceptance:** `MissingField` is the sole surviving variant and is exercised by `Person::validate`'s test in `src/models.rs::tests` and by the `missing_field_display` test in `src/error.rs::tests`.

**T-14 — Address parser exploration (research spike).** Delivered (recommendation: **decline**).
- [x] Surveyed libpostal (C dep + multi-GB runtime model; ruled out by §17 portability axiom), Rust-native parsers (no incremental value over the existing `parse_address_line` from T-20), national reference datasets (jurisdiction-locked, fail the multinational scope), and commercial APIs (network IO; ruled out by §17 / §20 PII egress constraints).
- [x] Recommendation recorded in `AGENTS/roadmap-research.md`. Verdict: do **not** integrate an external postal-address reference at the person-matcher layer; consumers should standardise upstream in their ingest pipeline. The line-1 sub-component is only 0.2 of the 0.05-weighted address sub-score — the cost-benefit doesn't justify a heavyweight dependency.
- [x] Two incremental in-house improvements identified as potential follow-ups if recall data later demands it: locale-aware street-type vocabulary (FR `rue`, DE `straße`, IT `via`, ES `calle`, NL `straat`, …) and an optional `uprn`-style property identifier on `Address` scored like a national-identifier scheme. Neither is in scope for T-14; both are additive and unblocked.
- **Acceptance:** Met — written recommendation now lives at `AGENTS/roadmap-research.md`.

**T-15 — Batch API.** Delivered.
- [x] `MatchingEngine::match_one_to_many(query, candidates) -> Vec<MatchResult>` parallel to the input slice.
- [x] `MatchingEngine::rank_one_to_many(query, candidates) -> Vec<(usize, MatchResult)>` sorted by descending score with deterministic ascending-index tiebreak.
- [x] Blocking is a consumer concern; the crate stays a pure scoring library and the API surface is deliberately minimal.
- **Acceptance:** Unit tests pin empty-candidates, order preservation, individual-equivalence, ranking ordering, tie-break determinism, and call-to-call determinism. Integration tests pin filtered consumption, confidence-band carry-through, and `Arc`-shared threadsafe batch scoring. Met by `tests/integration_tests.rs` §20 and `src/matcher.rs::tests`.

**T-16 — Multinational national-identifier support.** Delivered.
- [x] Add `identifiers` module with `parse_united_kingdom_national_health_service_number`, `parse_fr_nir`, `parse_es_tsi`, `parse_ie_ihi`, `parse_uk_hc_number`.
- [x] Extend `Person` with `united_kingdom_national_health_service_number`, `fr_nir`, `es_tsi`, `ie_ihi`, `uk_hc_number` (each `Option<String>`).
- [x] Extend `MatchConfig` with per-scheme weights (all default `0.30`).
- [x] Extend `MatchBreakdown` with per-scheme `Option<f64>` scores.
- [x] `deterministic_match` returns `true` on any same-scheme identifier equality; identifiers across schemes never cross-match.
- [x] Use ISO 3166-1 alpha-2 country-code prefix (`<cc>_<scheme>`) consistently across fields, weights, scores, and parser names.
- **Acceptance:** Each scheme has at least one integration test exercising both deterministic and probabilistic matching, plus rejection of cross-scheme collisions. Met by §12 of `tests/integration_tests.rs`.

**T-17 — Add more national identifier schemes (research spike).** Delivered (recommendation; implementation tracked as T-17.1).
- [x] The original T-17 candidate list (UK Scotland CHI, DE KVNR, IT Codice Fiscale, NL BSN, PL PESEL, SE Personnummer, AU IHI) **all shipped** under T-23 / T-27 / T-28 / T-30; total identifier-scheme coverage is now 35.
- [x] Survey of the next batch: `AGENTS/roadmap-research.md` identifies the **7 jurisdictions where the crate already parses phones but not identifiers** (BR CPF, CN RRN, IN Aadhaar, JP My Number, MX CURP, NZ NHI, ZA ID) as the recommended next tranche. Closes the symmetry between the phone surface (39 jurisdictions post-T-19) and the identifier surface (35 schemes).
- [x] Per-scheme parser sketch + check-digit algorithm in `AGENTS/roadmap-research.md` for each of the 7. Highlights: BR CPF uses two weighted Mod-11 digits, CN RRN combines weighted Mod-11 with date-substring validation, IN Aadhaar uses Verhoeff, MX CURP requires structural validation + date substring + Mod-10 check digit, ZA ID layers Luhn over a date-encoding 13-digit format.
- **Acceptance:** Met — recommendation in `AGENTS/roadmap-research.md` with per-scheme parser sketch and check-digit specification.

**T-17.1 — Next-batch national identifiers (implementation follow-up to T-17).** Delivered.
- [x] Added `parse_br_cpf`, `parse_cn_rrn`, `parse_in_aadhaar`, `parse_jp_my_number`, `parse_mx_curp`, `parse_nz_nhi`, `parse_za_id` to `src/identifiers.rs` (FR-85..FR-91). Each includes its check-digit algorithm and sentinel-data rejection per the research recommendation.
- [x] Extended `Person`, `PersonBuilder`, `MatchConfig` (default weight `0.30`), `MatchBreakdown` (with `#[serde(default)]`), `MatchingEngine` (`deterministic_match` branch + `calculate_breakdown` + `calculate_weighted_score`), and `Person::validate` for each.
- [x] Updated the 3 external `MatchConfig` struct-literal sites (matcher.rs docstring, examples/custom_config.rs ×2, tests/integration_tests.rs ×1).
- [x] 39 new unit tests + 14 new integration tests pin canonical forms, formatted inputs, check-digit rejection, length / character rejection, scheme-local isolation, breakdown wiring, serde round-trips, and legacy-payload defaulting.
- [x] Demo block added to `examples/basic_usage.rs` showing all 7 parsers canonicalising real inputs.
- [x] Sentinel-data rejection: BR CPF all-equal sequences rejected; IN Aadhaar `0xxxxxxxxxxx` and `1xxxxxxxxxxx` UIDAI-reserved prefixes rejected.
- [ ] TSV rows in `AGENTS/national-person-identifiers.tsv` deferred — the TSV is a manually-curated reference table and is out of scope for the immediate ship.
- **Acceptance:** Met. All seven schemes match deterministically and probabilistically within-scheme, never cross-match, and pass per-scheme integration tests. Total scheme count is now **42** (35 → 42). Test totals: 414 unit (was 374, +40), 233 integration (was 219, +14), 11 property, 176 doc (was 169, +7). Clippy + fmt clean.

**T-18 — International phone-number support.** Delivered.
- [x] Add `Normalizer::normalize_phone_e164(phone, default_country)` returning `Some("+CCNNN…")` for inputs that parse against the supported country table, else `None`.
- [x] Add `MatchConfig::phone_default_country: Option<String>` defaulting to `Some("GB")`.
- [x] Update `MatchingEngine::score_phone` to prefer the E.164 comparison and fall back to the legacy national-significant form when either input fails to parse.
- [x] Cover all six identifier jurisdictions (UK, FR, ES, IE, plus UK NI via GB dial code, plus US for SSN) and the major person-mobility partners (CA, DE, IT, NL, BE, PT, CH, AT, SE, NO, DK, FI, PL, AU, NZ, JP, CN, IN, BR, MX, ZA).
- **Acceptance:** §14.3 documents the algorithm and table; integration tests pin the within-country, cross-country, and fallback behaviour. Met by `tests/integration_tests.rs` §13.

**T-19 — Broader phone country table.** Delivered (tactical expansion; declined the heavyweight `phonenumber` dependency and per-country mobile-prefix validation).
- [x] Surveyed: status quo (26 countries), tactical expansion (39 covering every identifier-scheme jurisdiction), full ITU-T (~250 territories), `phonenumber` crate dependency, and mobile/landline prefix validation. Recommendation + decision matrix in `AGENTS/roadmap-research.md`.
- [x] Refactored `CountryPhoneInfo::has_trunk_prefix: bool` → `trunk_prefix: Option<&'static str>` so non-`0` trunk conventions work cleanly (Lithuania uses `8`).
- [x] Added 13 new entries to `COUNTRY_PHONE_TABLE` (BG, CZ, EE, GR, HR, IS, LI, LT, LV, MT, RO, SI, SK), bringing total coverage to 39 jurisdictions — one for every national identifier scheme the crate parses.
- [x] Added 6 new e164 unit tests pinning Lithuania's `8` trunk, Greece's no-trunk form, Romania's `0` trunk, Czech canonical form, Iceland's 7-digit NSN, and Croatia/Slovenia overlapping-3-digit-code disambiguation.
- [x] Declined: per-country mobile/landline prefix validation (doesn't help matching recall — the matcher canonicalises, doesn't classify). Declined: `phonenumber` dependency (marginal recall lift for compile-time / dep-surface cost). Consumers with global person bases SHOULD standardise upstream (same pattern as T-14).
- **Acceptance:** Met — recommendation in `AGENTS/roadmap-research.md` with sample size (13-jurisdiction gap), decision matrix (6 options), and concrete code shipped (13 entries + 6 tests + struct refactor).

**T-20 — Sophisticated address parsing.** Delivered.
- [x] Add `Normalizer::expand_street_abbreviations` covering street-type (St/Rd/Ave/Blvd/Ln/Dr/Ct/Pl/Sq/Ter/Hwy/Pkwy/Mt/Cres/Gdns/Gr/Cl/Pk/Plz/Expy/Trl) and directional (N/S/E/W/NE/NW/SE/SW) abbreviations.
- [x] Add `Normalizer::normalize_address_line` (abbreviation expansion + name-normalisation pipeline).
- [x] Add `Normalizer::parse_address_line` returning `ParsedAddressLine { house_number, unit, street }`. Public struct, serde-derived, re-exported from the crate root.
- [x] Update `MatchingEngine::compare_addresses` to combine the abbreviation-aware street similarity with an exact house-number sub-component, preserving the existing `count`-based aggregation.
- **Acceptance:** Unit tests cover abbreviation expansion, directional expansion, house-number extraction (including alphanumeric suffix and non-greedy stop), unit prefix recognition for `Flat`/`Apt`/`Apartment`/`Unit`/`Suite`/`Ste`/`Room`/`Rm`, idempotence, and serde round-trip. Integration tests pin: (a) `"123 High St"` vs `"123 High Street"` matches; (b) `"45 N Park Ave"` vs `"45 North Park Avenue"` matches; (c) `"10 Downing St"` outscores `"20 Downing St"`; (d) unit prefix on one side does not block the structured match. Met by `tests/integration_tests.rs` §14.

**T-21 — United States Social Security Number.** Delivered.
- [x] Add `identifiers::parse_us_ssn` enforcing the structural rules: exactly 9 ASCII digits, area not in `{000, 666, 900..=999}`, group not `00`, serial not `0000`.
- [x] Add `us_ssn: Option<String>` to `Person` and a `us_ssn(value)` setter on `PersonBuilder`.
- [x] Add `us_ssn_weight: f64` (default 0.30) to `MatchConfig`; add `us_ssn_score: Option<f64>` to `MatchBreakdown`.
- [x] Extend `deterministic_match` and `match_persons` to treat `us_ssn` as an independent scheme-local identifier.
- [x] Extend `Person::validate` to accept a solo `us_ssn`.
- **Acceptance:** Unit tests cover canonical and hyphenated layouts, boundary area numbers (`001`, `665`, `667`, `899`), invalid area / group / serial values, wrong length, letters, and arbitrary punctuation stripping. Integration tests pin deterministic and probabilistic match, mismatch, structurally-invalid-yields-None, and inclusion in `Person::validate`. Met by `tests/integration_tests.rs` §12 (US SSN block) and `src/identifiers.rs::tests`.

**T-22 — Date-of-birth transposition heuristic.** Delivered.
- [x] Extend `MatchingEngine`'s DOB component score to return `0.5` when one side is a day/month transposition of the other (same year, swapped form is a valid calendar date).
- [x] Leave `deterministic_match` unchanged — it still requires exact `NaiveDate` equality on the demographic-tuple branch.
- **Acceptance:** Unit tests pin the four outcomes (exact, transposition, same-year unrelated, cross-year). Integration tests pin: classic DD/MM ↔ MM/DD lift; cross-year non-fire; deterministic still rejects; partial credit lifts the overall score relative to a zero DOB; transposition alone is not enough to clear the default 0.85 threshold. Met by `tests/integration_tests.rs` §19 and `src/matcher.rs::tests`.

**T-23 — Six additional national identifier schemes.** Delivered.
- [x] Add `parse_au_ihi` (16-digit Luhn-checked Australian Individual Healthcare Identifier).
- [x] Add `parse_de_kvnr` (letter + 9 digits Mod-10 German *Krankenversichertennummer*).
- [x] Add `parse_it_cf` (16-character alphanumeric Mod-26 Italian *Codice Fiscale*).
- [x] Add `parse_nl_bsn` (9-digit 11-test Dutch *Burgerservicenummer*).
- [x] Add `parse_se_personnummer` (10- or 12-digit Luhn Swedish personal identity number).
- [x] Add `parse_uk_chi_number` (10-digit Mod-11 Scottish Community Health Index Number).
- [x] Extend `Person`, `PersonBuilder`, `MatchConfig` (per-scheme weight 0.30), `MatchBreakdown` (per-scheme `Option<f64>` with `#[serde(default)]`), `MatchingEngine` deterministic and breakdown paths, and `Person::validate`.
- **Acceptance:** 6 × 6 unit tests in `src/identifiers.rs` (canonical / wrong check / wrong length / wrong chars / format variants / empty); per-scheme integration tests covering deterministic match, mismatch, unparseable yields `None`, and breakdown carries each score; cross-scheme: AU IHI ↔ IE IHI scheme-local; UK CHI ↔ UK United Kingdom National Health Service Number and UK CHI ↔ UK NI H&C scheme-local. Met by `tests/integration_tests.rs` §12 (extended polyglot block) and `src/identifiers.rs::tests`.

**T-24 — `previous_addresses` best-of scoring.** Delivered.
- [x] Extend `score_address` to take the highest score across every pair drawn from `(p1.address ∪ p1.previous_addresses) × (p2.address ∪ p2.previous_addresses)`.
- [x] Returns `None` only when at least one side has no address data at all.
- **Acceptance:** Integration tests pin: (a) a matching historical pair lifts the score when currents differ; (b) only-historical-on-both-sides still produces a score; (c) no-data-on-one-side stays `None`; (d) an unrelated historical address does not lower a strong current-vs-current match (relative non-regression). Met by `tests/integration_tests.rs` §4.

**T-25 — Middle-name scoring.** Delivered.
- [x] Extend `score_given_name` to blend `0.95 × given_sim + 0.05 × middle_sim` when both persons carry a `middle_name`.
- [x] Reuse the existing `score_name` helper so the configured similarity algorithm and nickname-table boost apply to middle names.
- [x] One-sided middle-name data MUST leave the score unchanged (no penalty for asymmetric metadata).
- **Acceptance:** Integration tests pin (a) matching given + matching middle ≈ 1.0; (b) matching given + different middle drops modestly (≥ 0.93, < 1.0); (c) one-sided middle name leaves the score unchanged; (d) matching middle names lift the score relative to a no-middle comparison when given names are close but not equal.

**T-26 — Passport books (multi-country, multi-book, time-varying).** Delivered.
- [x] Public `PassportBook { country, number, issued, expires }` type in `src/models.rs`; constructor canonicalises country (uppercased 2-letter ASCII) and number (whitespace stripped, uppercased); date fields are metadata only.
- [x] `Person::passport_books: Vec<PassportBook>` with `add_passport_book` and `passport_books` builder methods. `Person::validate` accepts a non-empty `passport_books` as a sufficient identifying field.
- [x] `MatchConfig::passport_book_weight` (default `0.30`); `MatchBreakdown::passport_book_score: Option<f64>` with `#[serde(default)]`.
- [x] `MatchingEngine` deterministic path: `true` when any `(country, number)` pair is shared across the two persons' lists. Cross-country values with the same `number` never cross-match.
- **Acceptance:** Unit tests in `src/models.rs::tests` pin constructor canonicalisation, invalid input rejection (bad country / empty number), date setters, serde round-trip (including legacy payloads without date fields). Integration tests in `tests/integration_tests.rs` §21 pin: single-pair deterministic match (with mixed case + whitespace inputs); multi-country any-pair match; same digits different country never match; historical-book pair still matches; one-side-empty → `None`; both-non-empty disjoint → `0.0`; dates are metadata; serde round-trip; legacy Person JSON deserialises with empty `passport_books`.

**T-27 — Eighteen additional national personal identifiers.** Delivered.
- [x] Add 18 new parsers to `src/identifiers.rs`: `parse_be_nn` (Belgium Mod-97), `parse_bg_egn` (Bulgaria weighted Mod-11), `parse_cz_rc` (Czech Mod-11 divisibility), `parse_dk_cpr` (Denmark format-only), `parse_ee_ik` (Estonia cascading Mod-11), `parse_es_dni` (Spain DNI/NIE Mod-23 letter), `parse_fi_hetu` (Finland Mod-31 letter), `parse_hr_oib` (Croatia ISO 7064 MOD 11,10), `parse_is_kt` (Iceland Mod-11), `parse_lt_ak` (Lithuania cascading Mod-11), `parse_lv_pk` (Latvia weighted Mod-11), `parse_mt_id` (Malta format + letter), `parse_no_fnr` (Norway dual Mod-11), `parse_pl_pesel` (Poland weighted Mod-10), `parse_ro_cnp` (Romania Mod-11), `parse_si_emso` (Slovenia Mod-11), `parse_sk_rc` (Slovakia Mod-11), `parse_uk_nino` (UK format with prefix blacklist).
- [x] Extend `Person`, `PersonBuilder`, `MatchConfig` (per-scheme weight 0.30), `MatchBreakdown` (per-scheme `Option<f64>` with `#[serde(default)]`), `MatchingEngine` deterministic and breakdown paths, and `Person::validate` to cover the 18 new schemes. Total: 30 schemes.
- **Acceptance:** ≥4 unit tests per parser pinning canonical / wrong-check / wrong-length / format-variant cases; integration tests pin deterministic match per scheme and verify three UK Mod-11 schemes (United Kingdom National Health Service Number / NI H&C / Scotland CHI) remain scheme-local plus NINO never cross-matches. Met by `src/identifiers.rs::tests` and `tests/integration_tests.rs` §21a.

**T-28 — Five further personal IDs + nine passport-format validators.** Delivered.
- [x] Drive from `AGENTS/national-person-identifiers.tsv`.
- [x] Add five Person-field identifiers: `gr_dss` (Greece DSS, format-only 10 digits), `li_id` (Liechtenstein National ID, 2 letters + 8–9 digits, format-only with renewal caveat), `nl_id` (Netherlands National ID, 9-char `[A-Z\O]{2}[A-Z0-9\O]{6}[0-9]`), `pl_nip` (Poland NIP, 10 digits weighted Mod-11), `pt_nif` (Portugal NIF, 9 digits weighted Mod-11). Total Person-field schemes: **35**.
- [x] Add nine passport-format validators in the `identifiers` module (Cyprus, Czech, Liechtenstein, Lithuania, Malta, Netherlands, Portugal, Romania, Slovakia). These are pure format validators with no Person field; passport data is canonically stored via `Person::passport_books: Vec<PassportBook>`.
- **Acceptance:** ≥3 unit tests per parser pinning canonical / variant / wrong-shape cases (43 new identifier tests); per-scheme integration tests cover deterministic match, mismatch, validate-accepts-solo, plus scheme-locality (NL ID ≠ NL BSN; PL NIP ≠ PL PESEL); composition test demonstrates `parse_<cc>_passport` feeding `PassportBook::new`. Met by `src/identifiers.rs::tests` and `tests/integration_tests.rs` §21b / §21c.

**T-29 — Blood-type scoring.** Delivered.
- [x] Add public `BloodType` enum (8 ABO+RhD variants) in `src/models.rs` with serde-rename to canonical short forms (`"A+"`, …).
- [x] Add `BloodType::parse(s)` accepting canonical, lowercase, word, `+VE`/`-VE`, separator, and zero-to-O variants.
- [x] Add `Person::blood_type: Option<BloodType>` with a builder setter and `#[serde(default)]` (legacy JSON deserialises with `None`).
- [x] Add `MatchConfig::blood_type_weight` (default 0.05) and `MatchBreakdown::blood_type_score` (`Some(1.0)`/`Some(0.0)`/`None`).
- [x] Blood type is deliberately **not** consulted by `deterministic_match` and **not** an identifying field for `Person::validate`.
- **Acceptance:** 11 unit tests pin canonical / lowercase / word / `+VE` / separator / zero-O variants plus serde round-trip and builder behaviour. 7 integration tests pin match, mismatch, missing, not-part-of-deterministic, parse-through-builder, serde round-trip, legacy-payload deserialisation. Met by `src/models.rs::tests` and `tests/integration_tests.rs` §22.

**T-30 — Place-of-birth scoring.** Delivered.
- [x] Add `Person::birth_place: Option<Address>` (with `#[serde(default)]`) reusing the existing `Address` type for FHIR `Patient.birthPlace` parity.
- [x] Add `PersonBuilder::birth_place(value)` setter.
- [x] Add `MatchConfig::birth_place_weight` (default 0.05) and `MatchBreakdown::birth_place_score` (with `#[serde(default)]`).
- [x] Dedicated `score_birth_place` helper that considers only `city` (Jaro-Winkler) and `country` (exact), blended `0.7 × city + 0.3 × country` when both present; single signal when only one; `None` when no comparable subset.
- [x] **Not** part of `deterministic_match` (too weak alone) and **not** part of `Person::validate`'s identifying-field set.
- **Acceptance:** 10 integration tests pin: identical-birth-place scores ~1.0; wildly-different scores low; same-city / different-country = 0.7; missing-on-one-side = None; country-only fallback; empty subfields = None; not-deterministic invariant; diacritic-tolerant city; serde round-trip with Person; legacy-payload deserialisation.

**T-31 — Multiple-birth scoring.** Delivered.
- [x] Add `Person::multiple_birth: Option<u8>` (FHIR `Patient.multipleBirth`, 1-indexed birth order) with `#[serde(default)]`.
- [x] Add `PersonBuilder::multiple_birth(value)` setter.
- [x] Add `MatchConfig::multiple_birth_weight` (default 0.05) and `MatchBreakdown::multiple_birth_score` (with `#[serde(default)]`).
- [x] `score_multiple_birth` helper: `Some(1.0)` for equal values, `Some(0.0)` for different, `None` when either side is missing.
- [x] Not part of `deterministic_match` (too weak alone) and not part of `Person::validate`'s identifying-field set.
- **Acceptance:** 6 integration tests pin: match, identical-twin disambiguation (the canonical clinical failure mode), missing-on-one-side `None`, not-part-of-deterministic invariant, serde round-trip carrying the field, legacy-payload deserialisation to `None`.

**T-32 — Date-of-death and place-of-death scoring.** Delivered.
- [x] Add `Person::death_date: Option<NaiveDate>` and `Person::death_place: Option<Address>` (both with `#[serde(default)]`).
- [x] Add `PersonBuilder::death_date(value)` and `PersonBuilder::death_place(value)` setters.
- [x] Add `MatchConfig::death_date_weight` (default 0.10) and `MatchConfig::death_place_weight` (default 0.05).
- [x] Add `MatchBreakdown::death_date_score` and `death_place_score` (both with `#[serde(default)]`).
- [x] Extract a free `score_named_place(&Address, &Address) -> Option<f64>` helper from the prior `score_birth_place` body; refactor `score_birth_place` to delegate to it; introduce `score_death_place` that delegates likewise. Death-place data goes through the same `0.7 × city + 0.3 × country` blend as birth-place data.
- [x] `score_death_date` delegates to the existing free `score_dob_pair` helper, so DD/MM ↔ MM/DD transpositions on death dates are also recognised as half-credit.
- [x] Neither field contributes to `deterministic_match` (weak alone) nor to `Person::validate`'s identifying-field set.
- **Acceptance:** 14 integration tests in `tests/integration_tests.rs` §25 plus 8 unit tests in `src/matcher.rs::tests` pin: exact match, day/month transposition, unrelated dates, missing-on-one-side `None`, independence from `date_of_birth_score`, place exact / different / city-only-partial / missing / independence from birth_place / not-part-of-deterministic, serde round-trip, legacy-payload deserialisation to `None`, composite-score non-regression, and free-helper edge cases.
