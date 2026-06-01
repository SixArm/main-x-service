# Delivered Tasks Archive

This file archives the delivered (`[x]` / "✅ Delivered") tasks that were previously inlined in `spec.md §23`. The spec retains the still-open queue (`[ ]` / `[~]`) plus a pointer here. See [`spec.md`](../spec.md) §23 for the live work queue.

Status legend: `[x]` done.

## 23.1 Done (carried over from CHANGELOG)

- [x] Initial `Worker` model with builder.
- [x] `MatchingEngine` with configurable weights and thresholds.
- [x] Deterministic matching on national identifiers / demographics.
- [x] Probabilistic matching with weighted average.
- [x] Jaro-Winkler, Levenshtein, Exact, Combined string similarity.
- [x] Soundex-based phonetic matching.
- [x] Name, postcode, phone, identifier normalisation.
- [x] Address comparison (postcode + city + line 1).
- [x] Diacritic handling via NFKD decomposition.
- [x] Three pre-defined configs: default / strict / lenient.
- [x] Unit tests and integration tests per §18.
- [x] `serde` support for `Worker`, `Address`, `Gender`, `MatchResult`, `MatchBreakdown`.
- [x] Multinational national-identifier support (T-16): UK NHS Number, France NIR, España TSI, Éire IHI, UK NI H&C Number. Country-prefixed naming (`<cc>_<scheme>`) applied across `Worker` fields, `WorkerBuilder` setters, `MatchConfig` weights, and `MatchBreakdown` scores. `identifiers` module exposes one parser per scheme.
- [x] United States Social Security Number (T-21): `identifiers::parse_us_ssn` with full structural validation (`000` / `666` / `900..=999` area, `00` group, `0000` serial); `us_ssn` field on `Worker` and `WorkerBuilder`; `us_ssn_weight` on `MatchConfig`; `us_ssn_score` on `MatchBreakdown`; deterministic-match path.
- [x] Sophisticated address parsing (T-20): `Normalizer::expand_street_abbreviations`, `normalize_address_line`, `parse_address_line`, and `ParsedAddressLine`. The matcher's line-1 comparison now uses abbreviation expansion plus a structural house-number sub-component.
- [x] International phone-number support (T-18): `Normalizer::normalize_phone_e164` returns the E.164 canonical form for ~25 supported countries (all six identifier jurisdictions plus the major worker-mobility partners), `MatchConfig::phone_default_country` controls the assumed jurisdiction for national-format inputs, and `MatchingEngine` prefers the E.164 form with a fallback to the legacy national-significant comparison.
- [x] Nickname dictionary (T-10): public `NicknameTable` (empty/english/with_class/are_equivalent) consulted by `score_name`; given/family-name component lifts to `≥ 0.9` when the table considers the pair equivalent; boost never lowers a score.
- [x] Email scoring (T-11): `Normalizer::normalize_email` (trim + lowercase + structural validation, opt-in Gmail dot/+-folding); `MatchConfig::email_weight`, `MatchConfig::gmail_dot_folding`, and `MatchBreakdown::email_score`. `local_id` deliberately not scored (resolves OQ-2).
- [x] `Confidence` enum (T-2): `MatchResult::confidence` populated by `Confidence::from_score(score)`; band boundaries `≥ 0.90 / ≥ 0.75 / else`; independent of `match_threshold`; serde-derived with `#[serde(default)]` for legacy payloads.
- [x] Serialisable config (T-1): `MatchConfig`, `SimilarityAlgorithm`, and `NicknameTable` derive `Serialize + Deserialize`; `MatchConfig` carries `#[serde(default)]` for partial-document config files.
- [x] Date-of-birth transposition heuristic (T-22): probabilistic DOB sub-score returns `0.5` when one side is a day/month transposition of the other (same year, valid swapped date); `deterministic_match` is unchanged. Catches the common DD/MM ↔ MM/DD data-entry bug.
- [x] Batch API (T-15): `MatchingEngine::match_one_to_many(query, candidates)` and `MatchingEngine::rank_one_to_many(query, candidates)`. The engine remains immutable and `Send + Sync`, so consumers can layer parallelism (rayon, tokio) without changes to this crate.
- [x] `strict_mode` enforcement (T-4 / resolves OQ-5): under `strict_mode = true`, `is_match` requires both `score >= match_threshold` AND `deterministic_match(p1, p2)`. Probabilistic score and confidence are unchanged.
- [x] `previous_addresses` best-of scoring (T-24): the address sub-score is the highest score across the cartesian product of `(current ∪ previous_addresses)` on both sides. Catches the "worker moved house" failure mode without dragging down strong current-vs-current matches.
- [x] Middle-name scoring (T-25 / resolves OQ-1): when both sides have a `middle_name`, the given-name component blends `0.95 × given + 0.05 × middle` using the same `name_algorithm` and nickname-table boost as the given-name path.
- [x] Passport books (T-26): public `PassportBook { country, number, issued, expires }` type and `Worker::passport_books: Vec<PassportBook>` field model multi-country dual-citizenship, multi-book historical / current accumulation, and time-varying book numbers. Matching treats any shared `(country, number)` pair as a deterministic match; cross-country values with the same number never cross-match.
- [x] `#[non_exhaustive]` on `Worker` and `Address` (T-8 / resolves OQ-3): formalises that struct-literal construction is reserved for the defining crate. External consumers use `Worker::builder()` or `Address::new()` with the new `with_*` fluent setters. Future field additions can ship as minor releases without breaking downstream code.
- [x] Eighteen additional national personal identifiers (T-27): Belgium NN, Bulgaria EGN, Czech RČ, Denmark CPR, Estonia *Isikukood*, Spain DNI/NIE, Finland HETU, Croatia OIB, Iceland *Kennitala*, Lithuania *Asmens kodas*, Latvia *Personas kods*, Malta National ID, Norway *Fødselsnummer*, Poland PESEL, Romania CNP, Slovenia EMŠO, Slovakia RČ, UK NINO. Total schemes supported: 30. Each scheme-local with its own parser, builder setter, weight, breakdown score, and deterministic-match branch.
- [x] Five further personal identifiers + nine passport-number format validators (T-28) driven by `AGENTS/national-worker-identifiers.tsv`: Greece DSS, Liechtenstein National ID, Netherlands National ID, Poland NIP, Portugal NIF as full `Worker` fields (35 schemes total); Cyprus / Czech / Liechtenstein / Lithuania / Malta / Netherlands / Portugal / Romania / Slovakia passport format validators as standalone parsers feeding `PassportBook`.
- [x] Blood-type scoring (T-29): public `BloodType` enum (8 ABO+RhD variants) with a lenient `parse` accepting canonical, word-form, and zero-to-O variants; `Worker::blood_type: Option<BloodType>` plus `MatchConfig::blood_type_weight` (default 0.05) and `MatchBreakdown::blood_type_score`. Strong negative signal (stable for life) at a low default weight; deliberately excluded from `deterministic_match` and from `Worker::validate`'s identifying-field set.
- [x] Place-of-birth scoring (T-30): `Worker::birth_place: Option<Address>` reusing the existing Address type for FHIR `Patient.birthPlace` parity. Dedicated city + country sub-score (`0.7 × Jaro-Winkler(city) + 0.3 × exact(country)` blend); `MatchConfig::birth_place_weight` (default 0.05); `MatchBreakdown::birth_place_score`. Diacritic-tolerant via the shared name-normalisation pipeline.
- [x] Multiple-birth scoring (T-31): `Worker::multiple_birth: Option<u8>` (FHIR `Patient.multipleBirth`, 1-indexed birth order); `MatchConfig::multiple_birth_weight` (default 0.05); `MatchBreakdown::multiple_birth_score`. Primary use: disambiguating identical twins who otherwise share name, DOB, and demographic data. Not part of `deterministic_match` or `validate`'s identifying-field set.
- [x] Spec/code drift CI check (T-7): first CI workflow in the repo. `.github/workflows/spec-drift.yml` invokes `scripts/spec-drift-check.sh` on every pull request to `main`. The check fails if `src/matcher.rs` changes without `spec.md` changing in the same PR, modulo path-pattern exceptions in `.spec-allow`. PR template at `.github/pull_request_template.md` references the check. POSIX bash, runs locally as well as in CI.
- [x] Seven next-batch national identifier schemes (T-17.1): `parse_br_cpf` (BR CPF, 11 digits, two Mod-11 check digits), `parse_cn_rrn` (CN Resident Identity Card, 18 chars, weighted Mod-11 + date substring), `parse_in_aadhaar` (IN Aadhaar, 12 digits, Verhoeff), `parse_jp_my_number` (JP My Number, 12 digits, weighted Mod-11), `parse_mx_curp` (MX CURP, 18 alphanumeric chars, structural + Mod-10 weighted), `parse_nz_nhi` (NZ NHI original 7-char form, 3 letters + 4 digits, Mod-11 weighted with letter-to-int lookup excluding I/O), `parse_za_id` (ZA ID, 13 digits, Luhn + date substring). Each scheme is scheme-local (no cross-matching) and gets `Worker` field, builder setter, `MatchConfig` weight (0.30), `MatchBreakdown` score, `deterministic_match` branch, and `Worker::validate` inclusion. Sentinel-data rejection per §21.4 (BR CPF all-equal sequences, IN Aadhaar `0`/`1` prefixes). Total scheme count: 35 → **42**.
- [x] More-national-identifiers spike (T-17): the original T-17 candidate list (CHI, KVNR, Codice Fiscale, BSN, PESEL, Personnummer, IHI) all shipped under T-23 / T-27 / T-28; total coverage now 35 schemes. The follow-up §21.4 recommendation identifies the **7 phone-table-covered jurisdictions without an identifier parser** (BR CPF, CN RRN, IN Aadhaar, JP My Number, MX CURP, NZ NHI, ZA ID) as the next batch — each with a per-scheme parser sketch and check-digit algorithm. Implementation tracked as T-17.1.
- [x] Locale-aware phonetic encoder spike (T-9): surveyed Soundex, Double Metaphone, NYSIIS, Daitch-Mokotoff, Beider-Morse, locale-specific encoders, and a custom encoder. Recommendation in `AGENTS/roadmap-research.md`: keep Soundex as the default (no breaking change), expose an opt-in `MatchConfig::phonetic_encoder` enum via the `phonetic-rphonetic` Cargo feature flag, defer the default-switch decision until an empirical multinational worker corpus is available. The asymmetric `0.05`-weighted bonus design (FR-22/FR-23) caps the worst-case false-positive risk of any opt-in encoder. Implementation follow-up tracked as T-9.1.
- [x] Broader phone country table (T-19): expanded `COUNTRY_PHONE_TABLE` from 26 to 39 jurisdictions, covering every country the crate parses a national identifier for (added BG, CZ, EE, GR, HR, IS, LI, LT, LV, MT, RO, SI, SK). Refactored `has_trunk_prefix: bool` → `trunk_prefix: Option<&'static str>` to support Lithuania's `8` trunk prefix. 6 new e164 unit tests. Declined: full ~250-territory ITU-T expansion, `phonenumber` crate dependency, and per-country mobile/landline prefix validation. Recommendation matrix in `AGENTS/roadmap-research.md`.
- [x] Address-parser-exploration research spike (T-14): surveyed libpostal, Rust-native parsers, national reference datasets, and commercial APIs; recommendation recorded in `AGENTS/roadmap-research.md` is to **decline** external standardisation at this layer — adding it would violate the IO-free, pure-Rust, multinational axioms (§17, §20) for a fractional contribution (line 1 is only 0.2 of the 0.05-weighted address sub-score). Consumers SHOULD standardise upstream in their ingest pipeline. Two additive follow-ups identified (locale-aware street vocab; optional UPRN-style property identifier) but neither is in scope.
- [x] Documentation harmonisation (T-12): every top-level doc (`README.md`, `AGENTS.md`, `spec.md`, `CHANGELOG.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `IMPLEMENTATION_SUMMARY.md`) now points to `index.md` as the entry point. The previously-orphaned `AGENTS/national-worker-identifiers.md` reference table is linked from `AGENTS.md` and `index.md`. `IMPLEMENTATION_SUMMARY.md` carries a "superseded by `spec.md`" banner. All 17 referenced intra-repo doc paths verified to exist.
- [x] Error-model cleanup (T-13 / resolves OQ-6): removed the four `MatchingError` variants that no code path returned in 0.3.0 (`InvalidData`, `InvalidNhsNumber`, `InvalidDate`, `ConfigError`); marked `MatchingError` `#[non_exhaustive]` so future fallible code paths can add variants without breaking SemVer. Only `MissingField` (returned by `Worker::validate`) remains.
- [x] Property tests (T-6): `tests/property_tests.rs` exercises 11 invariants via `proptest` at 1000 cases each — `normalize_name` idempotency and shape, `score ∈ [0.0, 1.0]`, self-match positivity + `Confidence::High`, probabilistic and deterministic symmetry, DOB sub-score symmetry, `Confidence::from_score` monotonicity, and JSON round-trips for `MatchConfig::default()` and arbitrary `Worker` records. Historical failure seeds persisted in `tests/property_tests.proptest-regressions`.
- [x] Criterion benchmarks (T-5): `benches/match_pair.rs` covers `match_pair` (identical / fuzzy / unrelated), `deterministic_match_identifier_hit`, `rank_one_to_many` (`n ∈ {10, 100, 1000}` with throughput reporting), and `config_variants` (default / strict / English nickname table). Confirms the §17 performance budget (`< 50 µs` per pair) on a 2024 Apple Silicon machine: single-pair fuzzy match ~4 µs, deterministic ID hit ~160 ns, batch ranking ~3 µs/element.
- [x] Address sub-score weighted-average arithmetic (T-3 / resolves OQ-4): `compare_addresses` now accumulates `weighted_sum` and `total_weight` independently and returns `weighted_sum / total_weight`, so postcode (weight `0.5`) dominates city (`0.3`) and line 1 (`0.2`) as the spec already documented. Exact postcode + slightly different street clears `0.7`. Neutral `0.5` fallback when no sub-component fires is preserved.
- [x] Date-of-death and place-of-death scoring (T-32): `Worker::death_date: Option<NaiveDate>` (FHIR `Patient.deceasedDateTime`) reuses the DOB transposition heuristic via the shared `score_dob_pair` helper; `Worker::death_place: Option<Address>` parallels `birth_place` and reuses the new `score_named_place` free helper (extracted from the prior `score_birth_place` body). `MatchConfig::death_date_weight` defaults to `0.10`; `MatchConfig::death_place_weight` defaults to `0.05`. Both `MatchBreakdown` scores are independent from `date_of_birth_score` / `birth_place_score`. Neither field contributes to `deterministic_match` or to `validate`'s identifying-field set.
- [x] Six additional national identifiers (T-23): Australia IHI (`parse_au_ihi`), Germany KVNR (`parse_de_kvnr`), Italy *Codice Fiscale* (`parse_it_cf`), Netherlands BSN (`parse_nl_bsn`), Sweden *Personnummer* (`parse_se_personnummer`), and UK Scotland CHI Number (`parse_uk_chi_number`). Each with its own check-digit algorithm, `Worker` field, builder setter, `MatchConfig` weight (default 0.30), and independent `MatchBreakdown` score. Total schemes supported: 12.

## 23.2 Delivered tasks (with full acceptance criteria)

**T-1 — Serialisable config.** ✅ Delivered.
- [x] Derive `Serialize, Deserialize` for `MatchConfig`, `SimilarityAlgorithm`, and `NicknameTable`.
- [x] `MatchConfig` carries `#[serde(default)]` so partial JSON documents merge over `MatchConfig::default()`.
- **Acceptance:** `MatchConfig::default()`, `strict()`, and `lenient()` round-trip through `serde_json` with all values preserved; partial JSON inherits defaults; end-to-end test confirms an engine built from a deserialised config matches the original byte-for-byte. Met by `tests/integration_tests.rs` §18 and `src/matcher.rs::tests`.

**T-2 — `Confidence` in `MatchResult`.** ✅ Delivered.
- [x] Add public `Confidence` enum (`High`/`Medium`/`Low`) in `src/matcher.rs`, re-exported from the crate root.
- [x] Add `pub confidence: Confidence` to `MatchResult` populated by `Confidence::from_score(score)` on every `match_workers` call.
- [x] Boundaries: `≥ 0.90 → High`, `≥ 0.75 → Medium`, else `Low` (inclusive on the low side).
- [x] Confidence is **independent of `match_threshold`** — a score of `0.92` is `High` under strict, default, and lenient presets alike. `is_match` remains the authoritative go/no-go signal.
- [x] `confidence` is `#[serde(default = "default_confidence")]` so legacy JSON payloads lacking the field deserialise to `Low` (interpretable as "needs re-scoring").
- **Acceptance:** Unit tests pin band boundaries, threshold independence, and serde round-trip. Integration tests pin: High for exact clones, Low for completely different workers, threshold-independence under strict vs lenient, and legacy JSON deserialisation. Met by `tests/integration_tests.rs` §17 and `src/matcher.rs::tests`.

**T-3 — Address sub-score correction.** ✅ Delivered.
- [x] Resolve §22 OQ-4. Implemented option (b): the address sub-score now accumulates `weighted_sum` and `total_weight` independently and divides at the end (`Σ(score × weight) / Σ(weight)`), so postcode (`0.5`) dominates as documented.
- [x] Neutral fallback (`0.5`) preserved when no sub-component fires.
- **Acceptance:** 4 unit tests + 2 integration tests in §26 pin the behaviour: exact postcode + slight street typo clears `0.7`, postcode-only match collapses to `1.0`, postcode-match + line1-mismatch is dominated by postcode, and the empty-address neutral fallback still returns `0.5`.

**T-4 — `strict_mode` enforcement.** ✅ Delivered.
- [x] Resolve §22 OQ-5: under `strict_mode = true`, set `is_match = (score >= threshold) && deterministic_match(...)`.
- [x] Probabilistic `score` and `confidence` remain unchanged across modes.
- **Acceptance:** Existing strict-mode integration tests continue to pass; new tests verify (a) a fuzzy match clearing a lowered strict threshold but lacking a deterministic anchor is rejected; (b) a deterministic match clearing the strict threshold is accepted; (c) the non-strict default still accepts fuzzy matches above the default threshold. Met by `src/matcher.rs::tests` and `tests/integration_tests.rs` §7.

**T-5 — Benchmarks.** ✅ Delivered.
- [x] Add `benches/match_pair.rs` using `criterion` (HTML reports enabled via the `html_reports` feature).
- [x] Four bench groups cover the hot paths: `match_pair` (identical / fuzzy / unrelated), `deterministic_match_identifier_hit`, `rank_one_to_many` (`n ∈ {10, 100, 1000}`, with criterion throughput reporting per candidate), and `config_variants` (default vs strict vs nickname-table-loaded).
- **Acceptance:** `cargo bench` compiles and runs end-to-end. Indicative single-machine numbers (2024 Apple Silicon, `--quick`): `match_pair / fuzzy_near_match ≈ 4 µs`, `deterministic_match_identifier_hit ≈ 160 ns`, `rank_one_to_many @ n=1000 ≈ 3 ms` (~3 µs/element). All well under the §17 budget of `< 50 µs` per pair.

**T-6 — Property tests.** ✅ Delivered.
- [x] Add `proptest` dev-dependency and properties listed in §18.4.
- [x] Eleven properties in `tests/property_tests.rs` covering normalisation idempotency, score bounds, self-match, symmetry (probabilistic and deterministic), confidence monotonicity, and serde round-trips for `Worker` and `MatchConfig`.
- **Acceptance:** `cargo test --test property_tests` runs 1000 cases per property with zero failures. `tests/property_tests.proptest-regressions` is checked in so historical shrunk seeds are re-tried on every run.

**T-7 — Spec/code drift CI check.** ✅ Delivered.
- [x] Workflow: `.github/workflows/spec-drift.yml` runs on every pull request targeting `main`. It fetches full history (`fetch-depth: 0`) so the diff against the base ref is accurate, then invokes `scripts/spec-drift-check.sh` with the GitHub-provided base ref and head SHA.
- [x] Check script: `scripts/spec-drift-check.sh` (POSIX bash, no external dependencies beyond `git`). Resolves the base ref (`origin/<ref>` if available, else local `<ref>`), computes the changed-file set via `git merge-base` + `git diff --name-only`, then enforces: if any file matching the watched pattern (initially `^src/matcher\.rs$`) changed, `spec.md` MUST also have changed. Path patterns in `.spec-allow` (extended regex, blank / `#`-prefixed lines ignored) override the requirement for genuinely spec-irrelevant paths.
- [x] Allowlist: `.spec-allow` ships empty (modulo header comment), so the discipline starts maximally strict; reviewers add patterns as concrete need arises.
- [x] PR template: `.github/pull_request_template.md` references the spec-drift check, lists the spec / allowlist / CHANGELOG checkboxes, and prompts contributors for a test plan.
- [x] Script also runs cleanly from a contributor's machine pre-push (no GitHub-specific assumptions) and exits 0 gracefully when the base ref cannot be resolved (avoids spurious failures in fork CI).
- [x] Verified the script's pass paths against historical commits (`bash scripts/spec-drift-check.sh <older> <newer>` for commits that touched both matcher.rs and spec.md returns `OK`).
- **Acceptance:** Met. CI green on initial introduction because this PR ships `spec.md` updates alongside its source changes. Future PRs that touch `src/matcher.rs` without `spec.md` will fail the `spec-drift` check unless the changed paths match `.spec-allow`. The PR template references the check by name.

**T-8 — Mark `Worker` and `Address` `#[non_exhaustive]`.** ✅ Delivered.
- [x] Add `#[non_exhaustive]` to both struct definitions in `src/models.rs`.
- [x] `Worker::builder()` is the canonical constructor; `Address::new()` + field assignment + new `with_*` fluent setters cover ergonomic external construction.
- [x] Field-assignment syntax on `Address` (`a.line1 = Some(...)`) continues to work because `#[non_exhaustive]` does not block individual field access.
- **Acceptance:** Crate compiles unchanged; tests / examples / doctests all pass (the crate-internal struct-literal use in `Address::new()` is allowed inside the defining crate). External struct-literal construction is now a compile error pointing consumers at the builder. Met by passing test suite at 524 tests after the attribute is added.

**T-9 — Locale-aware phonetic encoder (research spike).** ✅ Delivered (recommendation; implementation deferred to T-9.1).
- [x] Surveyed Soundex (status quo), Double Metaphone, NYSIIS, Daitch-Mokotoff Soundex, Beider-Morse, locale-specific encoders (Kölner Phonetik etc.), and a custom encoder. Decision matrix and rationale in `AGENTS/roadmap-research.md`.
- [x] Sample size and methodology documented: corpus shape (≥ 10k triples per jurisdiction across English-majority + Romance + Germanic + Slavic + Nordic populations) and metrics (TPR at the FR-23 `> 0.9` threshold, FPR, AUC, per-jurisdiction breakdown) a future empirical evaluation should use.
- [x] Recommendation: **stay with Soundex as the default**, add `MatchConfig::phonetic_encoder: PhoneticEncoder` enum with `Soundex` / `DoubleMetaphone` / `DaitchMokotoff` variants behind a `phonetic-rphonetic` Cargo feature flag, defer the default-switch decision until an empirical corpus exists.
- **Acceptance:** Met — written recommendation in `AGENTS/roadmap-research.md` with sample-size proposal, corpus specification, and evaluation methodology.

**T-10 — Nickname dictionary.** ✅ Delivered.
- [x] Public `NicknameTable` type in `src/nicknames.rs` exposing `empty()`, `english()`, `with_class()`, `are_equivalent()`, `is_empty()`, `len()`.
- [x] `MatchConfig::nickname_table: NicknameTable` defaults to `NicknameTable::empty()`; the feature is opt-in.
- [x] `MatchingEngine`'s `score_name` lifts the per-name component score to `max(score, 0.9)` when the table considers the pair equivalent. The boost never lowers a score.
- [x] Built-in English dictionary covers ≥40 common classes including the acceptance set.
- **Acceptance:** `Mike`↔`Michael`, `Liz`↔`Elizabeth`, `Bob`↔`Robert` lift the given-name score to ≥ 0.9. Met by `tests/integration_tests.rs` §15 and `src/nicknames.rs::tests`.

**T-11 — Email and `local_id` scoring (per OQ-2).** ✅ Delivered.
- [x] Implement `Normalizer::normalize_email(email, gmail_dot_folding) -> Option<String>` with trim + lowercase + structural validation, and opt-in Gmail dot/plus-tag folding for `gmail.com` / `googlemail.com`.
- [x] Add `MatchConfig::email_weight: f64` (default 0.05) and `MatchConfig::gmail_dot_folding: bool` (default false).
- [x] Add `MatchBreakdown::email_score: Option<f64>` populated from the canonical form.
- [x] `local_id` is deliberately not scored (cross-organisation collision risk); document explicitly.
- **Acceptance:** Unit tests cover case/whitespace canonicalisation, Gmail dot-folding (on/off), `+tag` stripping, non-Gmail untouched, malformed input → `None`, idempotence. Integration tests cover exact match, mismatch, missing on one side, unparseable yields `None`, and the dot-folding opt-in toggle. Met by `tests/integration_tests.rs` §16 and `src/normalizer.rs::tests`.

**T-12 — Documentation harmonisation.** ✅ Delivered.
- [x] Every top-level doc file now points to `index.md` as the entry point — verified by `rg index\.md` finding all 7 top-level markdown files (README, AGENTS, spec, CHANGELOG, CONTRIBUTING, CODE_OF_CONDUCT, IMPLEMENTATION_SUMMARY) plus `AGENTS/spec-driven-development.md`.
- [x] The previously-orphaned `AGENTS/national-worker-identifiers.md` (35-scheme reference table) is now linked from both `AGENTS.md` and `index.md`.
- [x] `IMPLEMENTATION_SUMMARY.md` carries an explicit "superseded by `spec.md`" banner so readers do not mistake the historical snapshot for current behaviour.
- **Acceptance:** All intra-repo links resolve (manual sweep against the 17 referenced files). Every top-level doc + the spec-driven-development guide points to `index.md`. No orphaned guides under `AGENTS/`.

**T-13 — Remove or wire-up unused error variants.** ✅ Delivered.
- [x] Resolved §22 OQ-6. Removed `InvalidData`, `InvalidNhsNumber`, `InvalidDate`, and `ConfigError`: none was returned from any code path in 0.3.0 — identifier parsers return `Option<String>` instead of `Result`, `MatchConfig` builders are infallible, and the crate does not parse date strings.
- [x] Marked `MatchingError` as `#[non_exhaustive]` so future fallible code paths can introduce variants without breaking SemVer for downstream pattern-matches.
- **Acceptance:** `MissingField` is the sole surviving variant and is exercised by `Worker::validate`'s test in `src/models.rs::tests` and by the `missing_field_display` test in `src/error.rs::tests`.

**T-14 — Address parser exploration (research spike).** ✅ Delivered (recommendation: **decline**).
- [x] Surveyed libpostal (C dep + multi-GB runtime model; ruled out by §17 portability axiom), Rust-native parsers (no incremental value over the existing `parse_address_line` from T-20), national reference datasets (jurisdiction-locked, fail the multinational scope), and commercial APIs (network IO; ruled out by §17 / §20 PII egress constraints).
- [x] Recommendation recorded in `AGENTS/roadmap-research.md`. Verdict: do **not** integrate an external postal-address reference at the worker-matcher layer; consumers should standardise upstream in their ingest pipeline. The line-1 sub-component is only 0.2 of the 0.05-weighted address sub-score — the cost-benefit doesn't justify a heavyweight dependency.
- [x] Two incremental in-house improvements identified as potential follow-ups if recall data later demands it: locale-aware street-type vocabulary (FR `rue`, DE `straße`, IT `via`, ES `calle`, NL `straat`, …) and an optional `uprn`-style property identifier on `Address` scored like a national-identifier scheme. Neither is in scope for T-14; both are additive and unblocked.
- **Acceptance:** Met — written recommendation now lives at `AGENTS/roadmap-research.md`.

**T-15 — Batch API.** ✅ Delivered.
- [x] `MatchingEngine::match_one_to_many(query, candidates) -> Vec<MatchResult>` parallel to the input slice.
- [x] `MatchingEngine::rank_one_to_many(query, candidates) -> Vec<(usize, MatchResult)>` sorted by descending score with deterministic ascending-index tiebreak.
- [x] Blocking is a consumer concern; the crate stays a pure scoring library and the API surface is deliberately minimal.
- **Acceptance:** Unit tests pin empty-candidates, order preservation, individual-equivalence, ranking ordering, tie-break determinism, and call-to-call determinism. Integration tests pin filtered consumption, confidence-band carry-through, and `Arc`-shared threadsafe batch scoring. Met by `tests/integration_tests.rs` §20 and `src/matcher.rs::tests`.

**T-16 — Multinational national-identifier support.** ✅ Delivered.
- [x] Add `identifiers` module with `parse_uk_nhs_number`, `parse_fr_nir`, `parse_es_tsi`, `parse_ie_ihi`, `parse_uk_hc_number`.
- [x] Extend `Worker` with `uk_nhs_number`, `fr_nir`, `es_tsi`, `ie_ihi`, `uk_hc_number` (each `Option<String>`).
- [x] Extend `MatchConfig` with per-scheme weights (all default `0.30`).
- [x] Extend `MatchBreakdown` with per-scheme `Option<f64>` scores.
- [x] `deterministic_match` returns `true` on any same-scheme identifier equality; identifiers across schemes never cross-match.
- [x] Use ISO 3166-1 alpha-2 country-code prefix (`<cc>_<scheme>`) consistently across fields, weights, scores, and parser names.
- **Acceptance:** Each scheme has at least one integration test exercising both deterministic and probabilistic matching, plus rejection of cross-scheme collisions. Met by §12 of `tests/integration_tests.rs`.


Continued in [`delivered-tasks-2.md`](delivered-tasks-2.md) for tasks T-17 through T-32 and the project-level acceptance criteria.
