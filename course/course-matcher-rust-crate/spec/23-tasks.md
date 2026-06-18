## 23. Tasks

- [x] T-1: Scaffold (Cargo.toml, src/, spec, AGENTS, README, CHANGELOG).
- [x] T-2: Implement `match_courses` per §5 with all per-component fns.
- [x] T-3: `MatchConfig` + presets per §7.
- [x] T-4: `normalize::{fold, course_code, fold_set}` per §8.
- [x] T-5: Unit tests covering deterministic short-circuits + probabilistic ordering.
- [x] T-6: Phonetic (Soundex) bonus on `name` component — `src/phonetic.rs` + `+0.05` bonus applied inside `name_score` when both names produce the same Soundex code and Jaro-Winkler is `< 0.95`. Capped at `0.95` so a phonetic hit nudges Medium-band scores but never single-handedly mints High confidence.
- [x] T-7: `course-service`-side adapter + bridge test —
      [`course-service-with-loco/src/matching/adapter.rs`](../../course-service-with-loco/src/matching/adapter.rs)
      projects the service domain `Course` down to this crate's slim
      shape; [`course-service-with-loco/tests/duplicate_detection.rs`](../../course-service-with-loco/tests/duplicate_detection.rs)
      pins the contract with 14 assertions.
- [x] T-8: Criterion benches — service-side at
      [`course-service-with-loco/benches/matching_bench.rs`](../../course-service-with-loco/benches/matching_bench.rs)
      cover `match_courses` on a populated pair, the deterministic
      short-circuit path, and `find_matches` ranking 100 candidates.
- [x] T-9: `IdentifierScheme` doc comment per-variant — every variant
      now carries a one-line example + a deterministic-vs-provider-
      scoped tag in [`src/course.rs`](../src/course.rs).
- [x] T-10: `MatchingEngine::match_one_to_many(query, candidates)`
      returns `Vec<MatchResult>` in input order (no rank, no filter),
      matching the sibling `person_matcher::MatchingEngine` shape so
      callers that work across the matcher family share one call
      signature.
- [ ] T-11: Relationships component (mirrors `person-matcher`). Add the
      `Course::relationships: Vec<RelationshipRef>` field +
      `RelationshipRef { relation: RelationKind, course_id: String }` +
      `RelationKind { SimilarTo, HigherLevelThan, LowerLevelThan }`
      (re-exported from `lib.rs`); add `relationships_score(&a, &b)`
      doing typed-set Jaccard over `(relation, course_id)` pairs per
      §5.1 (`None` when either side empty); add `relationships_score`
      to `MatchBreakdown` (§6.2); add `relationships_weight` (default
      0.05) to `MatchConfig` per §7 and include it in the renormalised
      weighted average (§5, §17). Update `CHANGELOG.md` under
      "Unreleased" and the detailed-algorithm doc
      [`AGENTS/matching-algorithm.md`](../AGENTS/matching-algorithm.md)
      (new probabilistic-components row). Extend the service-side
      bridge test in
      [`course-service-with-loco/tests/duplicate_detection.rs`](../../course-service-with-loco/tests/duplicate_detection.rs)
      to pin the `relationships[]` routing.
- [ ] T-12: Tags component (mirrors `person-matcher` / `event-matcher`).
      Add the `Course::tags: Vec<String>` field (default empty); add
      `tags_score(&a, &b)` doing plain set Jaccard over the
      case-insensitively normalised tag sets per §5.2 / §13a (reusing
      `normalize::fold_set`; `None` when either side empty); add
      `tags_score` to `MatchBreakdown` (§6.2); add `tags_weight` (default
      0.05, supporting-signal cluster) to `MatchConfig` per §7 and
      include it in the renormalised weighted average (§5, §17). Update
      `CHANGELOG.md` under "Unreleased" and the detailed-algorithm doc
      [`AGENTS/matching-algorithm.md`](../AGENTS/matching-algorithm.md)
      (new probabilistic-components row). Extend the service-side bridge
      test in
      [`course-service-with-loco/tests/duplicate_detection.rs`](../../course-service-with-loco/tests/duplicate_detection.rs)
      to pin the `tags[]` routing.

