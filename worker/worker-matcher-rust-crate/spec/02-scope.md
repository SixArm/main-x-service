## 2. Scope

**In scope.** Pairwise matching of two `Worker` records. Deterministic on any of the 42 national identifiers, the passport-book branch, or the demographic-tuple branch. Probabilistic with weighted per-field similarity (one independent score per identifier scheme). String similarity (Jaro-Winkler, Levenshtein, Combined, Exact). Phonetic (Soundex) for names. Normalisation of names, alphanumeric postcodes, phone, email, address lines, per-scheme identifiers. Address structural comparison (postcode, city, line 1). `serde` JSON-first serialisation. Configurable weights / thresholds / algorithm choice.

**Out of scope (today).** Blocking / candidate generation; persistent main worker indices; population-scale Fellegi-Sunter EM training; external postal-address standardisation (declined per T-14); cross-scheme identity resolution; non-Latin-script-specific phonetic encoders (defer to T-9.1 opt-in).

**Out of scope (permanently): organisation-level identifiers.** Every scored identifier slot is a *person-level* national scheme (the 42 of §6 plus the passport branch); each one is an exact-match short-circuit precisely because it identifies one human. Codes that identify an *organisation, site, or practice* — e.g. the UK NHS ODS organisation code, a GLN, or any employer/department code — are deliberately never scored and never gain a slot. Two workers at the same practice share the same organisation code, so an exact-match short-circuit on it would declare colleagues to be the same person. Such a value, if carried at all, belongs in the unscored `local_id` field (§8, OQ-2). Embedding services (e.g. `worker-service`) therefore drop these codes at their adapter rather than route them here; this is the matcher-side half of `worker-service` entity task T-7.

---

