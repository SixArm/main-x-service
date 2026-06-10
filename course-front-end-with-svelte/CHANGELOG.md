# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec.md](./spec/index.md) — single source of truth (numbered §1–§18; live work queue in §13); [README.md](./README.md) — user-facing intro; [AGENTS.md](./AGENTS.md) — agent guide.

## [Unreleased]

### Fixed

- **`API_BASE_URL` default pointed at the wrong service.** The
  fallback was `http://localhost:8080` — the person-service slot in
  the Main X Index family. Course Service runs on host port 8084
  (docker-compose default + every README / index.md / AGENTS doc).
  A developer running `pnpm dev` without setting
  `PUBLIC_API_BASE_URL` silently routed every API call to the
  wrong service (or to nothing). Default updated to 8084 with a
  comment explaining the family-port-allocation gotcha.
- **README + `.env.example` carried the same 8080 mistake.** README
  prereq + configuration table both said default 8080; `.env.example`
  shipped `PUBLIC_API_BASE_URL=http://localhost:8080` AND a leftover
  "Person Service" comment from the sibling copy-adapt. All three
  realigned to 8084 with notes on the family port allocation.
- **spec.md §7 + §10 SSR pointers named T-7 instead of T-13.** T-7
  was "Detail / edit / soft-delete" (already shipped). The SSR /
  `event.fetch` follow-up lives at §13 T-13. Both pointers re-aimed.
- **spec.md §16 OQ-4 was stale.** Said "revisit when the third
  sibling front-end ships" — six entity front-ends have long since
  shipped (person / worker / place / thing / event / course). The
  trigger condition fired iterations ago. Marked resolved with a
  note that the drift policy held up: this session's shape-
  mismatch sweep (e.g. `ScoredCandidate` adding `name` +
  `course_code`) fixed cleanly per-project without needing a
  shared package. No `mxi-svelte-core`; copy-adapt per project.

- **spec.md §15 Roadmap was misaligned with the just-cut v0.2.0.**
  Said v0.2 would carry "SSR-safe load functions; Lily Dialog/
  Combobox integration; identifier/address edit UI" — none of
  those shipped in v0.2 (which was bug fixes + realignment). The
  v0.4 bullet talked about scaffolding sibling front-ends "for
  Worker / Place / Course / Event" — but this IS the Course
  front-end and the other five entity front-ends already exist
  in the family. Re-cut: v0.2 = shipped; v0.3 = SSR + Lily +
  instance/syllabus edit UI; v0.4 = auth; v0.5+ = batch dedup UI,
  masked toggle, GDPR download.

## [0.2.0] — 2026-06-05

### Fixed

- **Blank optional URL fields would 422 on create/edit.** The form
  bound `<input type="url">` to optional Course fields, so leaving
  `url` / `license` / `additional_type` blank shipped `""` on the
  wire. The service's FR-25 scheme check
  (`url.starts_with("http://")`) rejects empty strings — users who
  filled only the required `name` field got 422 from a benign
  blank URL. Added a `normalizeForWire` step on submit that maps
  blank strings on every optional text field
  (`description`, `disambiguating_description`, `url`, `license`,
  `additional_type`, `course_code`, `typical_age_range`,
  `time_required`, `version`, `audience`, `educational_use`) to
  `undefined` so the omitted-key branch of the service's serde
  default fires. The same normalisation runs over nested
  `identifiers[*].url` and `identifiers[*].name` — an identifier
  row whose URL was tabbed-through-but-left-blank had the same
  failure mode.

### Changed

- **spec §14 test counters were off** — unit tests claimed 8 (real
  is 9: 5 client + 4 courses), Playwright smoke claimed 6 (real is
  5). `pnpm install` / `pnpm test` "pending manual verification"
  ticks marked as verified — both have been running clean through
  every change in the recent session.
- **spec §13 T-15 named Person-shape sub-records.** Leftover from
  copy-adaption from a sibling project — `address` and
  `emergency-contact` aren't Course fields. Re-scoped to instance /
  syllabus-section edit UI which is the genuine remaining gap;
  identifier add/remove already shipped via `CourseIdentifierInput`.
- **README "Status: MVP scaffold"** updated to reflect that the
  routes are live and the testing harness is verified; only
  instance / syllabus-section edit UI and the operator walkthrough
  remain.
- **Stale comment about duplicate-detection response shape.** The
  create page's 409 handler still claimed the service wrapped
  candidates in `{ has_duplicates, potential_matches }`. The
  Course Service ships a flat `MatchResult[]` directly under
  `error.details`. Comment updated to reflect the actual contract;
  the wrapper-tolerant code stays as forward-compatibility for any
  future sibling-service shape change.
- **Courses list page returned zero hits on initial load.** The
  page sent `q: q || "*"` thinking `"*"` was a wildcard, but the
  service treats the query as a literal Tantivy term — the asterisk
  matched nothing and the grid loaded empty. The service falls
  back to `list` when `q` is empty / whitespace, which is the
  "show all" behaviour the page actually wants. Now sends
  `q: q.trim()` so the empty initial load hits the list path.
- **Phonetic-search checkbox was inert.** The service's
  `SearchQuery` accepts a `phonetic` parameter for API parity with
  sibling services but documents it as a no-op (search dispatches
  only on `fuzzy`). Removed the checkbox until the service grows a
  real Soundex search path; the matcher-level Soundex bonus
  (course-matcher T-6) continues to fire on `match` /
  `check-duplicates` independently.
- **Dashboard health badge never reported "down".** The check
  `h.status?.toLowerCase().includes("ok") || ... ? "ok" : "ok"` had
  identical branches, so the badge always showed healthy even when
  the service responded with a degraded status. The service also
  emits `"healthy"` (not `"ok"`/`"up"`), so the substring matcher
  fell through to the dead "ok" branch anyway. Now matches
  `healthy` / `ok` / `up` against an affirmative set and surfaces
  the reported status as a banner when the badge flips to "down".
- **Match-page threshold was inert.** `MatchRequest` carried
  `threshold` + `max_candidates` but the service's `/api/courses/match`
  handler accepts a `Course`-shaped body — those fields were
  silently dropped on the wire, so the threshold slider had no
  effect. Dropped both fields from `MatchRequest`, doc-commented the
  wire shape, and reapplied the threshold as a client-side
  `$derived` filter (`results = rawResults.filter(r => r.score >=
  threshold)`). Slider relabelled "Display threshold" so the UX is
  honest about where the cutoff is applied.
- **Match / dedup result rendering.** `MatchResult` was typed as
  `{ course: Course, score, confidence, breakdown }` but the service
  emits a flat `ScoredCandidate` with `course_id` + `name` +
  `course_code` at the top level. Pages backed by
  `/api/courses/match` and `/api/courses/check-duplicates` would
  have rendered blank names and broken detail links in production.
  Realigned `MatchResult` to the wire shape and updated
  `MatchResultsList.svelte` to read `r.name` / `r.course_code` /
  `r.course_id` directly. svelte-check 0/0, vitest 9/9.

## [0.1.0] — 2026-06-02

Initial scaffold for the Course Service front-end. SvelteKit 2 + Svelte 5 runes + SVAR Svelte DataGrid + Lily Design System Svelte Headless. Domain types follow [schema.org/Course](https://schema.org/Course).

### Added

- **Routes (MVP).** Dashboard with service-health + recent-audit feed; courses list with name / identifier / additional-type search and SVAR DataGrid (columns: ID, Name, schema.org Type, Primary identifier, URL); create with real-time 409 duplicate detection inline; detail view (identity, additional-type as schema.org URL, identifiers with deep links, alternate names, same-as URLs, images); edit; soft-delete with confirm; per-record audit log; match check (name + description + URL + identifiers + same-as); merge with two-ID preview.
- **API layer.** `ApiClient` (envelope + error normalisation) + `ApiError`; `CourseRepository` binding the [Course Service REST surface](../course-service-rust-crate/AGENTS/restful.md). **Note:** Course Service uses `POST /api/courses/duplicates` (not `/check-duplicates`).
- **TypeScript types.** Snake-case domain types mirroring [`course-service-rust-crate/AGENTS/models.md`](../course-service-rust-crate/AGENTS/models.md): `Course` with all 13 schema.org/Course canonical properties (`name`, `alternate_names`, `description`, `disambiguating_description`, `additional_type`, `url`, `identifiers`, `images`, `main_entity_of_page`, `owner`, `same_as`, `subject_of`, `potential_action`); `CourseIdentifier` with schema.org [`PropertyValue`](https://schema.org/PropertyValue) shape (`property_id`, `value`, optional `name`/`url`); `IdentifierType` (Doi/Isbn/Issn/Gtin/Sku/Mpn/SerialNumber/Uri/Uuid/`{Custom: string}`); `DETERMINISTIC_TYPES` constant lists identifier types that short-circuit matching to score 1.0 (Doi/Isbn/Issn/Gtin/Mpn/SerialNumber/Uuid — Sku/Uri/Custom excluded); `MatchResult` + `MatchConfidence` + `MatchBreakdown` (per-component: name / identifier / description / url / same_as / phonetic flag / deterministic flag); `MergeRequest`/`Record`/`Response`; `BatchDeduplicationRequest`/`Response`; `AuditEntry`.
- **Form primitives.** `LabeledField`, `FieldError`, `FieldRow`, `createForm` Svelte 5 rune-based store.
- **Components.** `SearchBox`, `CourseGrid` (SVAR `Grid` with `select` + `init`/`select-row`), `CourseIdentifierInput` (dynamic add/remove, Custom-type label sub-field, optional per-identifier URL), `CourseForm` (name + additional_type URL + description + disambiguating description + URL + owner + multi-line alternate names + multi-line same_as URLs + identifier list; client-side validation of HTTP(S) URL fields), `MatchResultsList` with breakdown surfacing name / identifier / description / URL / same-as / phonetic / deterministic short-circuit.
- **Tests.** 5 Vitest unit tests for `ApiClient`, 3 unit tests for `CourseRepository` (pins `/duplicates` not `/check-duplicates`), 5 Playwright smoke tests covering every MVP route shell.
- **SDD doc set.** `spec.md` (§1–§18; live work queue in §13; open questions in §16), `README.md`, `AGENTS.md`, `CLAUDE.md`.

### Configuration

- `PUBLIC_API_BASE_URL` env var (default `http://localhost:8080`).
- SPA-only (`src/routes/+layout.ts` exports `ssr = false; prerender = false;`).

### Cross-references

- Service spec: [`../course-service-rust-crate/spec.md`](../course-service-rust-crate/spec/index.md).
- Service REST contract: [`../course-service-rust-crate/AGENTS/restful.md`](../course-service-rust-crate/AGENTS/restful.md).
- Service model types: [`../course-service-rust-crate/AGENTS/models.md`](../course-service-rust-crate/AGENTS/models.md).
- Service matching reference: [`../course-service-rust-crate/AGENTS/matching.md`](../course-service-rust-crate/AGENTS/matching.md).
