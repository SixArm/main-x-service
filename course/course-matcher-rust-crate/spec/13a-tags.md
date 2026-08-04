## 13a. Tags (planned, §23 T-12 — not yet implemented)

`fold_set(a.tags)` ∩ `fold_set(b.tags)` → Jaccard, identical to
keywords (§13). Returns `None` if either side is empty; otherwise a
value in `[0.0, 1.0]`. A **supporting** signal, weighted `tags_weight`
(§7); see §5.2 for the full rationale.

This section describes the design `Course::tags` will follow once T-12
lands; the field, `tags_score`, and `tags_weight` do not exist in the
shipped code today (see §6, §7).
