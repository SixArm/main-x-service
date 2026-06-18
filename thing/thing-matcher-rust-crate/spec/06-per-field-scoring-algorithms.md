## 6. Per-field scoring algorithms

This section pins the exact algorithm for each scored field. Detailed pseudocode and edge-case tables are in [`AGENTS/matching-algorithm.md`](../AGENTS/matching-algorithm.md).

### 6.1 `name_score`

Best-of cartesian product over `(primary name + alternate names)` on each side. For each `(n1, n2)` pair: apply `Normalizer::normalize_name` to both, then run the configured `name_algorithm`:

- `JaroWinkler` — `Scorer::jaro_winkler_similarity` over the normalised strings.
- `Levenshtein` — `Scorer::levenshtein_similarity` (edit distance normalised by max length).
- `Exact` — `1.0` if equal, `0.0` otherwise.
- `Combined` (default) — `0.7 · jaro_winkler + 0.3 · levenshtein`.

`name_score` is `None` iff either side has no non-empty name (after trimming).

### 6.2 `description_score`, `disambiguating_description_score`

`Scorer::combined_similarity` over the two strings after `Normalizer::normalize_text`. `None` if either side is absent.

### 6.3 `identifiers_score`

See §5.8.

### 6.4 `url_score`, `image_score`, `main_entity_of_page_score`

See §5.9 — exact equality after `Normalizer::normalize_url`. `None` if either side is absent.

### 6.5 `same_as_score`, `additional_types_score`

Jaccard set similarity (`|A ∩ B| / |A ∪ B|`) over the URL lists after `Normalizer::normalize_url`. `None` only when both lists are empty.

### 6.6 `relationships_score`

See §5.9.1 — typed-set Jaccard (`|A ∩ B| / |A ∪ B|`) over the `(relation, thing_id)` pairs, where `relation` is part of the key. `None` only when either side has no relationships. A supporting signal weighted `relationships_weight` (default `0.05`); not identifying on its own.

### 6.7 `name_phonetic_score`

See §5.7. `None` when `use_phonetic_matching` is `false` or either side has no names.

### 6.8 `tags_score`

See §5.9.2 — plain set Jaccard (`|A ∩ B| / |A ∪ B|`) over the tag sets after case-insensitive normalisation (trim + lowercase, de-duplicated). `None` when either side has an empty tag set. A supporting signal weighted `tags_weight` (default `0.05`); not identifying on its own.

---

