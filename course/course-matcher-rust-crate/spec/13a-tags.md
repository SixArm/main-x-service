## 13a. Tags (shipped — §23 T-12)

`fold_set(a.tags)` ∩ `fold_set(b.tags)` → Jaccard, computed the same
way as keywords (§13), but with different `None` semantics: returns
`None` if either side has no usable tags (empty `Vec`, or every entry
folds away to blank) rather than keywords' "both empty ⇒ `None`, one
empty ⇒ `Some(0.0)`" rule — mirroring the sibling matcher crates
(`worker-matcher` / `person-matcher` / `event-matcher`). Otherwise a
value in `[0.0, 1.0]`. A **supporting** signal, weighted `tags_weight`
(§7); see §5.2 for the full rationale.

`Course::tags`, `tags_score`, and `MatchConfig::tags_weight` are live in
`src/course.rs` / `src/matcher.rs` / `src/config.rs` (see §6, §7).
