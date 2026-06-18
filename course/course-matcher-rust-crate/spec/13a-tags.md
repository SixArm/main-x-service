## 13a. Tags

`fold_set(a.tags)` ∩ `fold_set(b.tags)` → Jaccard, identical to
keywords (§13). Returns `None` if either side is empty; otherwise a
value in `[0.0, 1.0]`. A **supporting** signal, weighted `tags_weight`
(§7); see §5.2 for the full rationale.
