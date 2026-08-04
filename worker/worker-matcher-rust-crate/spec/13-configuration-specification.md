## 13. Configuration Specification

### 13.1 Default Configuration

Default thresholds: `match_threshold` = **0.85** (strict 0.95 / lenient 0.75). All 42 national-identifier weights + `passport_book_weight` default to `0.30` (unchanged across presets). Demographic weights: `given_name_weight` = `0.15`; `family_name_weight` and `date_of_birth_weight` = `0.20`; `death_date_weight` = `0.10`; `gender_weight`, `blood_type_weight`, `multiple_birth_weight`, `address_weight`, `birth_place_weight`, `death_place_weight`, `phone_weight`, `email_weight` = `0.05` (all unchanged across presets). Other defaults: `use_phonetic_matching = true`; `name_algorithm = Combined`; `strict_mode = false` (true under strict); `nickname_table = empty()`; `gmail_dot_folding = false`; `phone_default_country = Some("GB")`. Weights are renormalised against participating fields. `MatchConfig` in `src/matcher.rs` is the authoritative source. **Planned, not yet implemented (§23 T-33/T-34):** `relationships_weight` and `tags_weight` would join the supporting-signal cluster at `0.05` each once relationships/tags land on `Worker`.

### 13.2 `strict_mode` Semantics

When `strict_mode` is `true`, `is_match = (score >= match_threshold) && deterministic_match(p1, p2)`. Probabilistic `score` and `confidence` are unchanged across modes — strict mode tightens only the binary `is_match` decision, rejecting fuzzy matches that lack a deterministic anchor.

---

