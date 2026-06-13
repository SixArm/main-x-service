## 2. Scope

### 2.1 In scope

- Pairwise scoring (`match_courses`).
- One-to-many scoring (`match_one_to_many` — input order) and
  ranking (`rank`, `find_matches` — score order).
- Deterministic short-circuits on identifier schemes, same-provider
  course codes, and `same_as` URLs.
- Probabilistic weighted-average scoring over name, course_code,
  provider, educational level, keywords, teaches.
- Tunable `MatchConfig` (weights + threshold).
- Total functions — no panics on bad input.
- `serde` round-trip for `Course`, `MatchConfig`, `MatchResult`.

### 2.2 Out of scope

- Search / blocking. Callers (e.g. `course-service`) pre-filter
  candidates via Tantivy before calling into the matcher.
- Persistence.
- HTTP / gRPC.
- Cross-language matching. Set `same_as` for that.
- Stemming / synonym expansion.

