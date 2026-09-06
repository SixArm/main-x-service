## 23. Tasks and Acceptance Criteria

Tasks tagged `T-NN`; status `[ ]` open, `[~]` in progress, `[x]` done. Delivered tasks with full acceptance criteria are archived in [`agents/delivered-tasks.md`](../agents/delivered-tasks.md) (summary) and [`agents/delivered-tasks-detail.md`](../agents/delivered-tasks-detail.md). This section keeps only currently-open tasks.

### 23.1 Done (carried over from CHANGELOG)

Full list in [`agents/delivered-tasks.md`](../agents/delivered-tasks.md); covers the core engine (T-1..T-8 / T-13 / T-15), 42 identifier schemes + 9 passport-format validators (T-16 / T-21 / T-23 / T-27 / T-28 / T-17.1), 39-jurisdiction phone E.164 (T-18 / T-19), address parsing + `previous_addresses` (T-20 / T-24), nickname / middle-name / DOB-transposition / email scoring (T-10 / T-25 / T-22 / T-11), passport books / blood type / multi-birth / birth+death (T-26 / T-29 / T-30 / T-31 / T-32), benchmarks / property tests / drift CI / doc harmonisation (T-5 / T-6 / T-7 / T-12), and the T-9 / T-14 / T-17 / T-19 research spike outcomes.

### 23.2 Open tasks

**T-9.1 — Phonetic encoder enum (implementation follow-up to T-9).**
- [ ] Add `rphonetic` as an optional dep behind the `phonetic-rphonetic` Cargo feature flag.
- [ ] Add `PhoneticEncoder` enum (`Soundex` default + `DoubleMetaphone` + `DaitchMokotoff`) and `MatchConfig::phonetic_encoder` field; default preserves current behaviour.
- [ ] Refactor `Normalizer::phonetic_code(name)` → `phonetic_code(name, encoder)` (additive overload).
- [ ] Wire `MatchingEngine::score_phonetic_names` to honour `config.phonetic_encoder`.
- [ ] Test multi-code semantics for Daitch-Mokotoff: non-empty code-set intersection → `1.0`, single-name match → `0.5`, disjoint → `0.0`.
- **Acceptance:** default-config behaviour and existing tests unchanged; new unit tests cover Double Metaphone (`"Stephen"`/`"Steven"`) and Daitch-Mokotoff (`"Schwarz"`/`"Shvarts"`); documented as opt-in only until T-9's corpus methodology is run.

**T-33 — Relationships as a weighted component (§8.1 / §8.6a / §12.2 / §13.1).**
- [ ] Add `relationships: Vec<RelationshipRef>` to `Person` and the `RelationshipRef` / `RelationKind` types (re-export from crate root).
- [ ] Score relationships by typed-set Jaccard over `(relation, person_id)` pairs; `None` when either side is empty; add `relationships_score` to `MatchBreakdown`.
- [ ] Add `relationships_weight` (default `0.05`) and include the field in the probabilistic weighted average (§12.3); update `agents/matching-algorithm.md` detail tables + `CHANGELOG.md`.
- [ ] Add an FR in §6 for relationship matching and cross-reference it from §8.1.
- **Acceptance:** two records sharing related-person ids score higher with a documented `relationships_score`; empty relationships do not participate; default weights renormalise correctly; `cargo test` + `cargo clippy --all-targets -- -D warnings` clean.

**T-34 — Tags as a weighted component (§8.1 / §8.5 / §12.2 / §13.1).**
- [ ] Add `tags: Vec<String>` to `Person` (default empty); normalise tags case-insensitively (trim + lowercase, de-duplicated) at score time.
- [ ] Score tags by plain set Jaccard over the normalised tag sets; `None` when either side is empty; add `tags_score` to `MatchBreakdown`.
- [ ] Add `tags_weight` (default `0.05`, supporting-signal cluster) and include the field in the probabilistic weighted average (§12.3); update `agents/matching-algorithm.md` detail tables + `CHANGELOG.md`.
- **Acceptance:** two records sharing tags score higher with a documented `tags_score`; empty tags do not participate; default weights renormalise correctly; `cargo test` + `cargo clippy --all-targets -- -D warnings` clean.

**T-35 — Fuzz target for national-identifier parsing (§12.1 / §14 / `src/identifiers.rs`).** *(resolved 2026-09-05.)*
- [x] Add `fuzz/fuzz_targets/identifiers.rs` exercising every one of the 42 national-identifier parsers + the 9 passport-format validators in `src/identifiers.rs` with raw fuzzer bytes (never-panic + no cross-scheme false-equality).
- [x] Register the new `[[bin]]` in `fuzz/Cargo.toml` alongside `match_persons` / `normalizer` / `scorer`.
- [x] Document the target in `fuzz/README.md`.
- **Acceptance:** `cargo +nightly fuzz run identifiers` runs clean for the CI smoke duration (`FUZZ_SECONDS`, default 30); no panic/overflow on any parser; existing `identifiers.rs` unit tests unaffected (verified: `fuzz/fuzz_targets/` today holds only `scorer.rs` / `normalizer.rs` / `match_persons.rs` — no target touches `src/identifiers.rs`, the module carrying all 42 scheme parsers + 9 passport validators and the most string-parsing-heavy attack surface in the crate).
  - **Resolved.** The new target calls all 42 personal-identifier parsers
    (`parse_united_kingdom_national_health_service_number` through
    `parse_za_id`) plus the 9 passport validators
    (`parse_cy_passport` through `parse_sk_passport`) with the same
    arbitrary `&str`, asserting nothing beyond "does not panic" — each
    parser is a pure `&str -> Option<String>` with no comparison surface
    of its own, so cross-scheme false-equality is a matcher-level property
    (already pinned by the `match_persons` fuzz target and the proptest
    suite), not one an individual parser can exhibit in isolation. Smoke
    run clean: `cargo +nightly fuzz run identifiers -- -max_total_time=30`
    → ~907k executions, no crash. `cargo test --lib` (417 passed),
    `cargo clippy --all-targets -- -D warnings`, and `cargo fmt --check`
    all clean and unaffected (the `fuzz/` crate is standalone, per the
    family convention).

**T-36 — Input-size bounding for matcher inputs (§17 / §20).**
- [ ] Add `MAX_NAME_LEN` / `MAX_ARRAY_LEN` / `MAX_ITEM_LEN`-style constants (mirroring the family's `security.md` invariant 3 values) and apply them at the top of `MatchingEngine::match_persons` / `Scorer` entry points, returning a `MatchingError` variant rather than performing unbounded work.
- [ ] Add property/unit tests proving an oversized `Vec<String>` field (identifiers, `previous_addresses`, and — once T-33/T-34 land — `relationships`/`tags`) is rejected in bounded time rather than driving an O(n·m) Jaro-Winkler/Jaccard scan.
- [ ] Document the caps in §14 (normalisation) and §17 (quality attributes); note the crate-level cap alongside any caller-side cap a service layer may also apply.
- **Acceptance:** a crafted `Person` with a 100k-entry array scores/matches in bounded time (benchmarked) rather than timing out; `cargo test` + `cargo clippy --all-targets -- -D warnings` clean (verified: `grep -rn "MAX_TEXT_LEN\|MAX_ARRAY_LEN\|MAX_ITEM_LEN\|^const \|^pub const" src/*.rs` finds no such bound anywhere in this crate, unlike the family-wide SEC-M1 caps `agents/share/security.md` §2/§3 describes as already landed in the *services*; this standalone-usable library has none of its own).

**T-37 — `match_many_to_many` / blocking-key helper (§21.3 roadmap; promote to task).**
- [ ] Add a `blocking_key(person: &Person) -> String` helper (e.g. Soundex-of-surname + birth-year) and a `match_many_to_many(&self, left: &[Person], right: &[Person]) -> Vec<MatchResult>` entry point that pre-filters candidate pairs by shared blocking key before scoring, atop the existing `match_one_to_many`/`rank_one_to_many`.
- [ ] Bench the blocked vs. unblocked O(n·m) cost on a synthetic 10k×10k population (Criterion, `benches/`).
- [ ] Document the blocking-key strategy and its recall/precision trade-off in §12 and §17.
- **Acceptance:** `match_many_to_many` returns the same pairs `match_one_to_many` run pairwise would find above threshold, with measured sub-quadratic wall time on the benchmark population; `cargo test` clean (verified: `src/matcher.rs` line 879 only carries a doc comment — "For sparse / large populations consider blocking" — with no `blocking`/`match_many_to_many` function anywhere in `src/`; §21.3 already lists this as a longer-term 0.4.x–1.0 roadmap item with no §23 task tracking it).

**T-38 — Pin OQ-7: phonetic bonus's participation in `total_weight` (§22 OQ-7 / §12.3).** *(Resolved.)*
- [x] Add a unit test that directly asserts whether `total_weight` (the probabilistic-average denominator) includes the phonetic bonus's weight only when the bonus actually applied, vs. always — pinning today's implemented behaviour byte-for-byte.
- [x] Update §22 OQ-7 from "Open" to "Resolved (T-38)" with a one-line statement of the pinned rule, cross-referenced from `agents/matching-algorithm.md`.
- **Acceptance (met):** `matcher::tests::total_weight_includes_phonetic_bonus_only_when_bonus_applies` (`src/matcher.rs`) constructs two fixtures scoring on given-name + family-name only — one where the phonetic mean is `0.5` (`Some`, not `> 0.9`) and one where it's `1.0` — derives the expected score from each fixture's own real sub-scores (via the private `score_given_name`/`score_family_name`/`score_phonetic_names` helpers, a deliberate exception to "pin observable behaviour, not internals" since that's the whole point here), and asserts the actual `match_persons` score matches only the "bonus weight excluded unless it applies" formula. Verified to fail (temporarily changed the `&& score > 0.9` guard to always include the bonus, reran the test, saw it fail with a clear expected-vs-actual mismatch) and reverted. OQ-7 marked Resolved; cross-referenced from `agents/matching-algorithm.md`'s Probabilistic Scoring Pipeline. `cargo test` 418/418 (was 417); `cargo clippy --all-targets -- -D warnings` clean; `cargo fmt --check` clean.

**T-39 — Second national-identifier batch: HK / SG / KR / TR / RU / AR / CA-provincial (§21.3 roadmap spike, follow-on to T-17).**
- [ ] Run a T-17-style research spike (`agents/roadmap-research.md`) evaluating format/check-digit availability for Hong Kong HKID, Singapore NRIC, South Korea RRN, Turkey TC Kimlik No, Russia SNILS, Argentina DNI/CUIL, and Canadian provincial health-card numbers.
- [ ] Land the subset with a public, documented check-digit algorithm as new `Person` fields + `src/identifiers.rs` parsers + weights, following the T-17.1 batch's pattern (one field per scheme, never cross-matched).
- [ ] Update `agents/national-person-identifiers.md`'s reference table and the crate `Cargo.toml` description's scheme count.
- **Acceptance:** each landed scheme has a parser, a weight, deterministic short-circuit tests, and property tests for the never-cross-match invariant; `cargo test` + `cargo clippy --all-targets -- -D warnings` clean (verified: §21.3 explicitly names this exact jurisdiction list as "further national-identifier schemes beyond 42 … incremental per consumer demand" with no §23 task yet opened for it, unlike T-17's now-delivered spike).

### 23.3 Acceptance Criteria — Project-level

"1.0-ready" when all §21.1 tasks complete; spec and code agree (T-7 enforced); `Person` / `Address` `#[non_exhaustive]` (T-8); public API unchanged for two consecutive minor releases; coverage `≥ 90%` and `cargo test` in `< 5 s`.

---

