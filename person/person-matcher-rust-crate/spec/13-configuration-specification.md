## 13. Configuration Specification

### 13.1 Default Configuration

Threshold and `strict_mode` vary by preset; everything else is identical across presets.

- `match_threshold`: default **0.85**, strict **0.95**, lenient **0.75**.
- `strict_mode`: default `false`, strict **`true`**, lenient `false`.

Weights (all renormalised against participating fields):

- `0.30` — 42 per-scheme identifier weights (`<cc>_<scheme>_weight`, names = `Person` identifier-field names + `_weight`); `passport_book_weight`.
- `0.20` — `family_name_weight`, `date_of_birth_weight`.
- `0.15` — `given_name_weight`.
- `0.10` — `death_date_weight`.
- `0.05` — `gender_weight`, `blood_type_weight`, `multiple_birth_weight`, `address_weight`, `birth_place_weight`, `death_place_weight`, `phone_weight`, `email_weight`.
- **Planned, not yet implemented** (§23.2 T-33 / T-34): `relationships_weight`, `tags_weight` — proposed default `0.05` each once the corresponding fields land; neither exists on `MatchConfig` in `src/matcher.rs` today.

Other defaults: `use_phonetic_matching = true`; `name_algorithm = Combined`; `nickname_table = NicknameTable::empty()`; `gmail_dot_folding = false`; `phone_default_country = Some("GB")`.

### 13.2 `strict_mode` Semantics

Strict mode computes identical `score` / `confidence` / `MatchBreakdown` but tightens the binary `is_match` decision: `is_match = (score >= match_threshold) && deterministic_match(p1, p2)`. A fuzzy match clearing the threshold but lacking a deterministic anchor is rejected (FR-47).

---

