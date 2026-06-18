## 5. Algorithm overview

```text
match_courses(A, B):
  if deterministic_match(A, B):
      return Score 1.0 with deterministic_match=true.

  components = [
    (name_score(A, B),               name_weight),
    (course_code_score(A, B),        course_code_weight),
    (provider_score(A, B),           provider_weight),
    (educational_level_score(A, B),  educational_level_weight),
    (set_jaccard(A.keywords, B.k),   keywords_weight),
    (set_jaccard(A.teaches, B.t),    teaches_weight),
    (relationships_score(A, B),      relationships_weight),
    (set_jaccard(A.tags, B.tags),    tags_weight),
  ]

  score = weighted_average(components)   # ignores None entries
  is_match = score >= threshold
  return MatchResult { score, is_match, confidence, breakdown }
```

### 5.1 Relationships — `relationships_score`

Typed-set **Jaccard** over the `(relation, course_id)` pairs:

```text
A_set = { (r.relation, r.course_id) for r in a.relationships }
B_set = { (r.relation, r.course_id) for r in b.relationships }
score = |A_set ∩ B_set| / |A_set ∪ B_set|
```

So a `SimilarTo` reference only agrees with a `SimilarTo` reference to the
**same** course id — the relation kind is part of the key. Returns `None`
(does not participate in the weighted average) when **either** side has no
relationships; otherwise a value in `[0.0, 1.0]`.

A **supporting** signal, not an identifying field on its own: two records
that reference the same related courses are more likely the same course, but
shared references never single-handedly establish a match. Weighted
`relationships_weight` (§7, default 0.05) and renormalised over the present
components.

The matcher does **not** resolve, invert, or transitively close the
references (it has no registry) — `HigherLevelThan` and `LowerLevelThan` are
compared as opaque, distinct relation kinds. The consuming service owns the
inverse-consistency and acyclicity invariants (course-entity spec §5.5).

### 5.2 Tags — `tags_score`

Plain set **Jaccard** over the case-insensitively normalised tag sets —
identical to how `keywords` (§13) is scored:

```text
A_set = fold_set(a.tags)
B_set = fold_set(b.tags)
score = |A_set ∩ B_set| / |A_set ∪ B_set|
```

Returns `None` (does not participate in the weighted average) when
**either** side has an empty tag set; otherwise a value in `[0.0, 1.0]`.

A **supporting** signal, not an identifying field on its own: tags are
operator-applied operational labels (grouping, triage, workflow), so two
records that share tags are somewhat more likely the same course, but
shared tags never single-handedly establish a match. Weighted `tags_weight`
(§7, default 0.05) and renormalised over the present components.

