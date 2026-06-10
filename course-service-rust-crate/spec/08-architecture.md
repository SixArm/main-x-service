## 8. Architecture

### 8.1 Module layout

```
src/
├── main.rs                  # binary entry — Config → AppState → api::rest::serve
├── api/
│   ├── mod.rs               # ApiResponse, ApiError
│   └── rest/                # REST API (Axum)
├── models/                  # Course, CourseInstance, Provider, identifier, …
├── db/                      # SeaORM entities + repository trait + audit
├── matching/                # service-side adapter onto course_matcher::MatchingEngine
├── search/                  # Tantivy index + query
├── config/                  # env loading + Config struct
└── error.rs
```

### 8.2 Boot sequence

`Config::from_env` → `db::create_connection` → `SearchEngine::new` →
`matching::CourseMatcher::new` → `AppState::new` →
`api::rest::serve`. Identical shape to the
[person-service](../../person-service-rust-crate/) binary.

### 8.3 Layering rules

1. `models/` MAY NOT depend on `db/`, `api/`, `search/`.
2. `api/` MAY depend on every other module; nothing depends on `api/`.
3. `matching/` is a thin adapter over the canonical
   [`course-matcher`](../../course-matcher-rust-crate/) crate.
4. `search/` MAY depend on `models/` only.

### 8.4 Data flow

**Create:** HTTP POST → validate → duplicate-detection (search +
matcher) → on duplicate return `409 MatchResult[]`; on success
repository INSERT → search index → audit log → event publish →
response.

**Match:** HTTP POST → blocker (`search_by_name_and_provider`) →
load candidates → `CourseMatcher::find_matches` → renormalised
weighted score → response.

**Merge:** HTTP POST → fetch both → fold identifiers / instances /
syllabus / links into main → update main → soft-delete duplicate →
update index → audit log + `CourseMerged` event → response.

