## 10. Component Specifications

- **`Normalizer`** (`normalizer.rs`) — `normalize_name`, `normalize_postcode`, `normalize_phone`, `normalize_phone_e164`, `normalize_email`, `normalize_address_line`, `parse_address_line`, `phonetic_code`. Algorithms in §14 / [`AGENTS/normalization.md`](../AGENTS/normalization.md).
- **`identifiers`** (`identifiers.rs`) — one parser per scheme (42 total) + 9 passport-format validators (FR-77). Each parser is `&str → Option<String>`. Catalogue in [`AGENTS/national-person-identifiers.md`](../AGENTS/national-person-identifiers.md); algorithms in [`AGENTS/normalization.md`](../AGENTS/normalization.md) §14.5. No IO.
- **`Scorer`** (`scorer.rs`) — similarity primitives in `[0.0, 1.0]`: `jaro_winkler_similarity`, `levenshtein_similarity` (`1 − distance / max_len`), `exact_match`, `combined_similarity` (`0.7 × jw + 0.3 × lev`), `optional_field_score(opt1, opt2, algorithm)`. Empty-string: both empty ⇒ 1.0; one empty ⇒ 0.0. `SimilarityAlgorithm` is a `Copy` enum (`JaroWinkler | Levenshtein | Exact | Combined`).
- **`NicknameTable`** (`nicknames.rs`) — equivalence-class lookup: `empty()`, `english()`, `with_class(names)` (normalised via `Normalizer::normalize_name`; <2-distinct classes dropped), `are_equivalent(a, b)`, `is_empty()` / `len()`. Default English dictionary contents are NOT a public contract.
- **`MatchingEngine`** (`matcher.rs`) — immutable wrapper around `MatchConfig`. Public: `new(config)` / `default_config()`; `match_workers` (§12.3); `deterministic_match` (§12.1); `match_one_to_many` / `rank_one_to_many` (§12.6). Private helpers compute component scores + the address sub-score (`compare_addresses`).

---

