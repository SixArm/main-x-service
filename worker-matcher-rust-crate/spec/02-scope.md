## 2. Scope

**In scope.** Pairwise matching of two `Worker` records. Deterministic on any of the 42 national identifiers, the passport-book branch, or the demographic-tuple branch. Probabilistic with weighted per-field similarity (one independent score per identifier scheme). String similarity (Jaro-Winkler, Levenshtein, Combined, Exact). Phonetic (Soundex) for names. Normalisation of names, alphanumeric postcodes, phone, email, address lines, per-scheme identifiers. Address structural comparison (postcode, city, line 1). `serde` JSON-first serialisation. Configurable weights / thresholds / algorithm choice.

**Out of scope (today).** Blocking / candidate generation; persistent master worker indices; population-scale Fellegi-Sunter EM training; external postal-address standardisation (declined per T-14); cross-scheme identity resolution; non-Latin-script-specific phonetic encoders (defer to T-9.1 opt-in).

---

