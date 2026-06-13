## 14. Implementation Status

Honest status per subproject, rolled up from each crate's own §14.
Aspirational items live in §15, not here.

### course-service-rust-crate

| Area | Status |
|---|---|
| Loco boot + idiomatic controllers (family reference) | ✅ — code shipped; crate spec/docs lag behind (§13 T-2) |
| Course + instance CRUD, search, match, merge, dedup | ✅ FR-1..FR-13 |
| Audit + event streaming | ✅ in-memory MVP; Fluvio adapter pending |
| Privacy (masked view + GDPR export) | ✅ |
| OpenAPI / Swagger UI | ✅ |
| Tests | ✅ 35 unit + 14 bridge + 12 `#[ignore]` integration + 3 benches |
| JWT auth | ❌ (crate T-15) |
| Syllabus-section API | ❌ JSONB only, no read/write endpoints |

### course-matcher-rust-crate

| Area | Status |
|---|---|
| Probabilistic components (name / code / provider / level / keywords / teaches) | ✅ |
| Deterministic short-circuits (DOI / Wikidata / LOM / OER / URI / UUID, R-1, R-2) | ✅ |
| Soundex bonus, renormalisation, confidence classification | ✅ |
| Config presets (`strict` / `default` / `lenient`) | ✅ |
| `match_one_to_many` family-shape API | ✅ |
| Crate task queue | ✅ all T-1..T-10 closed (crate §23) |

### course-front-end-with-svelte

| Area | Status |
|---|---|
| List / search / create-with-409 / detail / edit / delete / match / merge / audit | ✅ |
| Tests | ✅ 9 Vitest + 5 Playwright |
| Instance / syllabus edit UI | ❌ read-only (crate T-15) |
| check-duplicates preview, masked-view toggle, export download, dedup-scan UI | ❌ queued (crate T-13..T-20) |
| Live integration walkthrough against a running service | ❌ pending |
| Localization | ❌ English-only |

### Entity-level composition

| Contract | Status |
|---|---|
| Front-end ↔ service wire contract (`/api`, envelope, `{ items, total }`) | ✅ pinned by front-end unit tests |
| Service ↔ matcher adapter contract (§5.3) | ✅ pinned by 14 bridge tests |
| SSO across the trio | ❌ roadmap |
| Entity spec ↔ crate specs consistency | ✅ service spec aligned with the loco conversion and post-nesting links repaired 2026-06-13 (§13 T-2, T-3, T-4) |
