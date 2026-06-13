## 2. Scope

**In Scope.** Pairwise matching of two `Person` records (single-pair and batch via `match_one_to_many` / `rank_one_to_many`). Deterministic matching on any of 42 national identifiers, passport-book agreement, or full demographic tuples. Probabilistic matching with weighted per-field similarity (one independent score per scheme). String similarity (Jaro-Winkler, Levenshtein, Combined, Exact), Soundex phonetics, DOB transposition heuristic, nickname-table boost, middle-name blend. Normalisation of names, postcodes, phones (legacy + E.164 across 39 jurisdictions), email (opt-in Gmail folding), structured address lines, per-scheme identifiers. Address structural comparison; place-of-birth / death-place / death-date scoring. `serde` (JSON-first) round-trip. Configurable via `MatchConfig` (default / strict / lenient presets).

**Out of Scope (Today).** Blocking / candidate generation across large datasets; persistent master person indices; population-scale record linkage (Fellegi-Sunter EM training); external postal-address standardisation (declined per T-14); cross-scheme identity resolution.

---

