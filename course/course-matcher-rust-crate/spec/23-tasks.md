## 23. Tasks

- [x] T-1: Scaffold (Cargo.toml, src/, spec, AGENTS, README, CHANGELOG).
- [x] T-2: Implement `match_courses` per §5 with all per-component fns.
- [x] T-3: `MatchConfig` + presets per §7.
- [x] T-4: `normalize::{fold, course_code, fold_set}` per §8.
- [x] T-5: Unit tests covering deterministic short-circuits + probabilistic ordering.
- [x] T-6: Phonetic (Soundex) bonus on `name` component — `src/phonetic.rs` + `+0.05` bonus applied inside `name_score` when both names produce the same Soundex code and Jaro-Winkler is `< 0.95`. Capped at `0.95` so a phonetic hit nudges Medium-band scores but never single-handedly mints High confidence.
- [x] T-7: `course-service`-side adapter + bridge test —
      [`course-service-with-axum/src/matching/adapter.rs`](../../course-service-with-axum/src/matching/adapter.rs)
      projects the service domain `Course` down to this crate's slim
      shape; [`course-service-with-axum/tests/duplicate_detection.rs`](../../course-service-with-axum/tests/duplicate_detection.rs)
      pins the contract with 14 assertions.
- [x] T-8: Criterion benches — service-side at
      [`course-service-with-axum/benches/matching_bench.rs`](../../course-service-with-axum/benches/matching_bench.rs)
      cover `match_courses` on a populated pair, the deterministic
      short-circuit path, and `find_matches` ranking 100 candidates.
      This crate also carries its own Criterion bench,
      [`benches/match_pair.rs`](../benches/match_pair.rs) (added in
      `[0.7.0]`, T-15) — see §24 for how the two relate.
- [x] T-9: `IdentifierScheme` doc comment per-variant — every variant
      now carries a one-line example + a deterministic-vs-provider-
      scoped tag in [`src/course.rs`](../src/course.rs).
- [x] T-10: `MatchingEngine::match_one_to_many(query, candidates)`
      returns `Vec<MatchResult>` in input order (no rank, no filter),
      matching the sibling `person_matcher::MatchingEngine` shape so
      callers that work across the matcher family share one call
      signature.
- [x] T-11: Relationships component (mirrors `person-matcher` /
      `worker-matcher`). Added `Course::relationships: Vec<RelationshipRef>`
      + `RelationshipRef { relation: RelationKind, course_id: String }` +
      `RelationKind { SimilarTo, HigherLevelThan, LowerLevelThan }`
      (re-exported from `lib.rs`); `relationships_score(&a, &b)` doing
      typed-set Jaccard over `(relation, course_id)` pairs per §5.1
      (`None` when either side empty); `relationships_score` on
      `MatchBreakdown` (§6.2); `relationships_weight` (default 0.05) on
      `MatchConfig` per §7, wired into the renormalised weighted average
      (§5, §17). `CHANGELOG.md` and
      [`agents/matching-algorithm.md`](../agents/matching-algorithm.md)
      updated. The service-side bridge test in
      `course-service-with-axum/tests/duplicate_detection.rs` still
      routes the slim matcher `Course`; extending it to pin
      `relationships[]` end-to-end is left to whichever PR wires the
      service-side domain field through the adapter (out of scope for
      this matcher-only task).
- [x] T-12: Tags component (mirrors `person-matcher` / `event-matcher` /
      `worker-matcher`). Added `Course::tags: Vec<String>` (default
      empty); `tags_score(&a, &b)` doing plain set Jaccard over the
      case-insensitively normalised tag sets per §5.2 / §13a (reusing
      `normalize::fold_set`; `None` when either side has no usable tags,
      before or after folding); `tags_score` on `MatchBreakdown` (§6.2);
      `tags_weight` (default 0.05, supporting-signal cluster) on
      `MatchConfig` per §7, wired into the renormalised weighted average
      (§5, §17). `CHANGELOG.md` and
      [`agents/matching-algorithm.md`](../agents/matching-algorithm.md)
      updated. Service-side bridge-test extension: same note as T-11.
- [x] T-13: *(resolved 2026-09-04, option (b))* Property-test coverage for negative `MatchConfig` weights,
      per spec §22's documented anti-pattern ("Setting weights to
      negative. Not validated today — caller contract."). Extend
      `tests/proptests.rs::score_is_bounded_unit_interval` (or add a
      sibling property) to generate a `MatchConfig` with an
      arbitrary-signed weight on one component and assert the
      documented `[0.0, 1.0]` bound in `weighted_average`'s doc
      comment still holds — or, if it does not, decide and record
      whether `weighted_average` should clamp/reject negative weights
      or the doc comment should instead state the bound only holds for
      non-negative weights. *(verified:
      `weighted_average(&[(Some(0.0), 1.0), (Some(1.0), -0.5)])`
      returns `-1.0`, outside the documented range, and no existing
      proptest constructs a non-default `MatchConfig`.)*
      - **Acceptance:** either (a) `weighted_average` is bounded for
        any finite weight and a proptest pins it, or (b) the doc
        comment on `weighted_average` and spec §22 are updated to
        state the `[0.0, 1.0]` guarantee assumes non-negative weights,
        with a regression test pinning the now-explicit contract
        either way.
      - **Resolved via (b), not (a).** §22 already records negative
        weights as a *deliberate* caller-contract decision, not an
        oversight — adding validation to `MatchConfig` (option (a))
        would silently change that documented contract rather than
        clarify it, contrary to this crate's "trust the spec, don't
        silently realign it" rule (`AGENTS.md`). Instead:
        `weighted_average`'s doc comment (`src/scoring.rs`) now states
        the `[0.0, 1.0]` guarantee explicitly assumes non-negative
        weights, §22 above records the consequence, and a new unit
        test (a proptest was considered but a fixed example is
        sufficient and simpler here, since the property being pinned
        is "this one documented edge case doesn't regress silently",
        not a general invariant over arbitrary inputs) —
        `weighted_average_negative_weight_breaks_the_unit_interval_bound`
        — pins the exact numbers from the verified case above.
- [x] T-14: Pin the documented trailing-slash `same_as` behaviour *(resolved 2026-09-05.)*
      (spec §16: "we do NOT strip trailing slashes"). Add a unit test
      alongside `same_as_url_overlap_short_circuits` (`src/matcher.rs`)
      asserting that `same_as = ["https://x.org/c/"]` vs `same_as =
      ["https://x.org/c"]` does **not** deterministic-match (R-2
      misses), and that `same_as = ["https://x.org/c/"]` vs `same_as =
      ["https://X.ORG/c/"]` (case only) **does** match. *(verified:
      `grep -n "trailing_slash\|trailing slash" src/*.rs tests/*.rs`
      returns nothing, and the sole existing `same_as` test only
      varies case/whitespace, never the trailing-slash distinction §16
      documents.)*
      - **Acceptance:** the new test(s) pass under current
        `normalize::fold` and fail if a future change makes `fold`
        strip trailing slashes, so the spec's documented "we
        deliberately preserve it" decision is enforced, not just
        asserted in prose.
      - **Resolved.** Two new unit tests alongside
        `same_as_url_overlap_short_circuits` (`src/matcher.rs`):
        `same_as_trailing_slash_difference_does_not_short_circuit`
        (`.../c/` vs `.../c` ⇒ no deterministic match) and
        `same_as_case_only_difference_still_short_circuits` (`.../c/`
        vs `.../C/`, slash present on both ⇒ still matches) — split
        into two functions rather than one to keep each under
        clippy's `many_single_char_names` threshold. `cargo test
        --lib`: 95 passed (up from 92), 0 failed; `cargo build`/
        `clippy --all-targets -- -D warnings` clean.
- [x] T-15: Reconcile `spec/24-testing-strategy.md` and this file's
      T-8 entry with the crate-local Criterion bench added in
      `[0.7.0]`. *(Resolved.)* §24's "Benchmarks live at" line now
      names this crate's own `benches/match_pair.rs` (confirmed: four
      groups — single-pair scoring, deterministic short-circuits,
      `rank` throughput at N=10/100/1000, per-preset cost) alongside
      the service-side bench, with a note on how the two relate; T-8
      above cross-references it. *(verified: `CHANGELOG.md` `[0.7.0]`
      "Added — Criterion benchmarks" entry describes
      `benches/match_pair.rs`, which exists at `benches/match_pair.rs`;
      `spec/24-testing-strategy.md` and T-8's body previously described
      only the service-side bench.)*
      - **Acceptance (met):** §24 and the T-8 note above both name
        `benches/match_pair.rs` alongside the service-side bench; no
        code change required.
- [x] T-16: Match evidence (`agents/share/match-evidence.md`) *(resolved
      2026-11-09)*. New `src/evidence.rs` with this crate's m/u table
      (§18a), `MatchResult.evidence` (`#[serde(default)]`) and
      `MatchConfig::proof_bits` (default 30). Identifier bits on a shared
      DOI / Wikidata / OER / LOM / URI / UUID, an overlapping `same_as`
      URL, or a same-provider course code; contradictions per disjoint
      unique scheme and per same-provider differing course code. Evidence
      is computed on the short-circuit path too. `score`, `is_match`,
      `confidence` and `breakdown` unchanged.
      - **Acceptance (met):** unit tests in `src/evidence.rs` (missing
        component adds 0; identical sparse ⇒ `Consistent`; shared
        identifier / `same_as` / same-provider code ⇒ `Proven`;
        conflicting identifiers or same-provider codes ⇒ `Contradicted`;
        richest identifier-free record ≈ 27.6 bits, pinned below the
        default bar; clear disagreement < 0 bits; pre-field JSON
        deserialises) and the `evidence_is_finite_and_strength_consistent`
        property in `tests/proptests.rs`.
- [x] T-17: Correlated groups, value frequencies and per-term evidence
      (`agents/share/match-evidence.md` §6–§8) *(resolved 2026-11-09)*.
      `CORRELATED_GROUPS` = {keywords, teaches, tags} and {provider,
      educational_level} at `CORRELATED_DISCOUNT` 0.5;
      `FREQUENCY_COMPONENTS` = `["name"]` with `frequency_key` (primary
      name, folded); `ValueFrequencies`, `NoFrequencies`,
      `FrequencyTable`, `MIN_SHARE`, `frequency_bits`;
      `MatchingEngine::match_courses_with_frequencies`; and
      `Evidence.components` (`#[serde(default)]`), whose terms sum to
      `weight_bits`. `score`, `is_match`, `confidence` unchanged. The
      richest identifier-free record falls from ≈ 27.6 to ≈ 23.62 bits.
      - **Acceptance (met):** unit tests in `src/evidence.rs` (group sum
        below the plain sum and order-independent, ties included;
        uncorrelated fields still sum; correlated disagreements
        discounted; common name (0.01) below and rare (1e-6) above the
        default; share ≥ m gives 0; no adjustment on partial agreement or
        disagreement; `NoFrequencies` equals the plain call; identifier
        and contradiction terms listed) and the
        `evidence_terms_sum_to_weight_bits` property in
        `tests/proptests.rs`.
