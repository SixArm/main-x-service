## 10. Component Specifications

- **§10.1 `Normalizer`** (`normalizer.rs`) — static utility struct with `pub fn (input: &str) -> ...` methods: `normalize_name`, `normalize_postcode`, `normalize_phone`, `normalize_phone_e164`, `normalize_email`, `expand_street_abbreviations`, `normalize_address_line`, `parse_address_line`, `phonetic_code`. Algorithms in §14 / [AGENTS/normalization.md](../AGENTS/normalization.md).
- **§10.1a `identifiers`** (`identifiers.rs`) — free-function module: 42 per-scheme parsers + 9 passport-format validators, each `pub fn (&str) -> Option<String>`. See [AGENTS/national-person-identifiers.md](../AGENTS/national-person-identifiers.md). No IO.
- **§10.2 `Scorer`** (`scorer.rs`) — similarity primitives in `[0.0, 1.0]`: `jaro_winkler_similarity` (wraps `strsim::jaro_winkler`), `levenshtein_similarity` (`1 − distance / max_len`), `exact_match`, `combined_similarity` (`0.7 × jw + 0.3 × lev`), `optional_field_score`. Empty-input convention: both empty ⇒ 1.0, one empty ⇒ 0.0. `SimilarityAlgorithm` is a `Copy` enum: `JaroWinkler | Levenshtein | Exact | Combined`.
- **§10.2a `NicknameTable`** (`nicknames.rs`) — equivalence-class lookup. Public API: `empty()`, `english()` (built-in nicknames; exact contents NOT part of the public contract), `with_class(names)` (normalises via `normalize_name`; classes with fewer than two distinct normalised entries are silently dropped), `are_equivalent(a, b)`, `is_empty()`, `len()`.
- **§10.3 `MatchingEngine`** (`matcher.rs`) — holds an immutable `MatchConfig`. Public methods: `new(config)` / `default_config()`; `match_persons(&p1, &p2) -> MatchResult`; `deterministic_match(&p1, &p2) -> bool`; `match_one_to_many(&query, &[Person]) -> Vec<MatchResult>`; `rank_one_to_many(&query, &[Person]) -> Vec<(usize, MatchResult)>`.

---

