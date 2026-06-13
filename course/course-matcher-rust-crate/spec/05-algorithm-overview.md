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
  ]

  score = weighted_average(components)   # ignores None entries
  is_match = score >= threshold
  return MatchResult { score, is_match, confidence, breakdown }
```

