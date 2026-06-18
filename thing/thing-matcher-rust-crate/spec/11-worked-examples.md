## 11. Worked examples

Worked examples are kept in `tests/integration_tests.rs` and in the crate-level doctests (`src/lib.rs`, `src/matcher.rs`). The summary below sketches each scenario; the test files are the runnable source. Each scenario names the `tests/integration_tests.rs` function that pins it.

### 11.1 Canonical probabilistic match

Two records of the same item with matching `url` and overlapping names produce a high score, `Confidence::High`, `is_match = true`.

```rust
use thing_matcher::{MatchingEngine, Thing};

let a = Thing::builder()
    .name("Pride and Prejudice")
    .add_alternate_name("Stolz und Vorurteil")
    .url("https://example.org/book/9780141439518")
    .build();
let b = Thing::builder()
    .name("Stolz und Vorurteil")
    .url("https://example.org/book/9780141439518")
    .build();

let result = MatchingEngine::default_config().match_things(&a, &b);
assert!(result.is_match);
```

Pinned by `test_perfect_match_all_fields` and `test_identical_minimal_record_scores_one`.

### 11.2 Deterministic match via shared identifier

Two records of the same book with different titles ("Pride and Prejudice" / "Stolz und Vorurteil") but sharing an `(isbn, 9780141439518)` pair produce `deterministic_match = true` regardless of probabilistic score.

Pinned by `test_deterministic_via_shared_identifier_returns_true`.

### 11.3 Deterministic match via shared `sameAs`

Both records carry `same_as = ["https://www.wikidata.org/wiki/Q170583"]` → `deterministic_match = true` even when names and URLs differ.

Pinned by `test_deterministic_via_shared_same_as_returns_true` (and `test_deterministic_via_canonical_url_returns_true` for the `url` variant).

### 11.4 Renormalisation: missing fields do not penalise

Both records carry only `name`; everything else is `None`. The renormalised score is computed against only the `name_weight`, so an exact name match scores `1.0` rather than `0.30 / (0.30 + …)`.

Pinned by `test_identical_minimal_record_scores_one` and `test_missing_optional_fields_yield_none_in_breakdown`.

### 11.5 Strict mode rejects fuzzy-only matches

Two records with high name similarity but no shared identifier / sameAs / url and `strict_mode = true` produce `is_match = false` even when `score > threshold`.

Pinned by `test_strict_preset_requires_deterministic_match`.

### 11.6 Phonetic bonus

`Normalizer::phonetic_code("Robert") == Normalizer::phonetic_code("Rupert") == "R163"`. With `use_phonetic_matching = true`, two records whose names produce the same Soundex code receive a small score uplift.

Pinned by `test_phonetic_bonus_raises_score` (the bonus lifts the overall score versus the same pair with phonetic matching off).

### 11.7 Batch ranking

`rank_one_to_many` against a candidate slice returns `(index, MatchResult)` tuples sorted by descending score with deterministic tie-breaking on the original index.

Pinned by `test_rank_one_to_many_orders_by_descending_score` and `test_match_one_to_many_returns_one_result_per_candidate`.

### 11.8 The `Combined` name blend is `0.7 / 0.3`

The default `SimilarityAlgorithm::Combined` blends Jaro-Winkler and Levenshtein as `0.7 · JW + 0.3 · Lev` (§6.1). For a known pair this yields a value distinguishable from any `0.6 / 0.4` blend.

Pinned by `test_combined_blend_is_seventy_thirty`.

---

