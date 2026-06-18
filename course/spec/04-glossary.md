## 4. Glossary

Entity-level terms. Per-subproject vocabularies:
service [spec §4](../course-service-with-loco/spec/04-glossary.md),
matcher [spec §3](../course-matcher-rust-crate/spec/03-glossary.md),
front-end [spec §4](../course-front-end-with-svelte/spec/04-glossary.md).

| Term | Meaning |
|---|---|
| **Entity** | One domain concept (here: Course) delivered as a trio of subprojects in one directory |
| **Trio** | The three subprojects: service crate, matcher crate, front-end project |
| **Entity-level spec** | This document set — source of truth for the cross-subproject contract |
| **Crate spec** | A subproject's own `spec/` — source of truth for that subproject's internals |
| **Course** | The course **template** (`schema.org/Course`): name, course code, provider, educational level, keywords, teaches, syllabus, identifiers |
| **CourseInstance** | A specific **offering** of a course (`schema.org/CourseInstance`): schedule, mode, instructors, location, capacity, enrollment window — a sub-resource of Course |
| **Provider** | The issuing organisation; `course_code` is only meaningful **within** a provider |
| **Provider-scoped course code** | `CS101` identifies a course only inside one `provider_id` — never matched across providers |
| **Service model** | The service's full schema.org-shaped `Course` (`src/models/`) — what the REST API serves |
| **Matcher model** | The matcher's slim `Course` shape — only the properties carrying identity signal |
| **Adapter** | `src/matching/adapter.rs` in the service — the lossy projection service model → matcher model (§5.3) |
| **Canonical algorithm** | The matcher crate's scoring — the reference the service embeds via `course_matcher::MatchingEngine` |
| **Deterministic short-circuit** | An identifier-scheme match (DOI / Wikidata / LOM / OER / URI / UUID), same-provider course code, or `same_as` URL overlap that pins the score to 1.0 |
| **Envelope** | The REST response wrapper `{ "success": bool, "data": …, "error": … }` shared by service and front-end |
| **Match** | A comparison between two courses yielding a 0.00–1.00 score plus per-component breakdown |
| **Merge** | Transfers a duplicate's identifiers / instances / syllabus / links onto a surviving record, soft-deletes the duplicate, snapshots the transfer |
| **Review queue** | Persisted candidate duplicate pairs: `Pending` / `Confirmed` / `Rejected` / `AutoMerged` |
| **Soft delete** | Retention with `deleted_at` set; never `DELETE FROM` — the entity-wide erasure mechanism |
| **Idiomatic loco controllers** | REST handlers registered as native loco `Routes` in `App::routes` (not a merged side-router); this service is the family's reference for the pattern |
| **SSO** | Single sign-on via the [authentication entity](../../authentication/): magic-link, RS256 JWT + JWKS |
| **Bridge test** | Service-side test (`tests/duplicate_detection.rs`) that pins both the adapter and the matcher output |
| **Drift policy** | Front-ends keep per-project copies of types/client/forms; no shared package (repo decision 2026-06-02) |
