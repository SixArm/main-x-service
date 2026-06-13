## 14. Implementation Status

| Area | Status |
|---|---|
| Skeleton (compiles, binary runs end-to-end) | ✅ |
| loco.rs conversion (Hooks boot, native controllers, config/*.yaml, Postgres queue, loco Migrator) | ✅ family reference |
| SeaORM entities | ✅ 15 modules (providers, courses, identifiers, links, instances, syllabus_sections, audit_log, course_match_scores, course_merge_records + the normalized collection child tables) |
| Repository CRUD | ✅ courses + identifiers + links + instances + merge records; syllabus_sections still UI-only |
| Search engine | ✅ index / fuzzy / exact / blocking-query / delete |
| Validation | ✅ FR-21..FR-28 |
| Matching adapter | ✅ drives `course_matcher::MatchingEngine` (with T-6 Soundex bonus) |
| REST handlers | ✅ FR-1..FR-9 + FR-14..FR-18 (+ OpenAPI/Swagger UI) |
| Audit / streaming | ✅ in-memory MVP; Fluvio adapter under flag pending |
| Privacy | ✅ mask + GDPR export (FR-15, FR-16) |
| Tests | ✅ 35 unit + 14 bridge + 12 `#[ignore]` integration + 3 criterion benches |

