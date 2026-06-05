# course-front-end-with-svelte — Living Specification

> **Source of truth.** When code and spec disagree, the spec wins. Open a task in §13 to bring code in line.
>
> **Three-part PRs.** A behavioural change is one PR: spec edit + code edit + test edit.

For the underlying service contract, see [`../course-service-rust-crate/spec.md`](../course-service-rust-crate/spec.md). For shared MXI guidance (REST conventions, observability, compliance), see [`../AGENTS.md`](../AGENTS.md) and [`../agents/share/`](../agents/share/).

## Table of contents

1. [Purpose and Vision](#1-purpose-and-vision)
2. [Scope](#2-scope)
3. [Stakeholders and Users](#3-stakeholders-and-users)
4. [Glossary](#4-glossary)
5. [Information Architecture](#5-information-architecture)
6. [Functional Requirements](#6-functional-requirements)
7. [Non-Functional Requirements](#7-non-functional-requirements)
8. [Architecture](#8-architecture)
9. [API Consumption](#9-api-consumption)
10. [Persistence](#10-persistence)
11. [Testing Strategy](#11-testing-strategy)
12. [Compliance](#12-compliance)
13. [Tasks](#13-tasks)
14. [Implementation Status](#14-implementation-status)
15. [Roadmap](#15-roadmap)
16. [Open Questions](#16-open-questions)
17. [References](#17-references)
18. [Change Control](#18-change-control)

## 1. Purpose and Vision

### 1.1 Purpose

Provide an operator-facing web UI for the Course Service that exercises the full duplicate-handling workflow: search, create with real-time duplicate detection, score-based match check, manual merge, and per-record audit review.

### 1.2 Vision

A web interface that:

- Surfaces every score-bearing decision (match quality, per-component breakdown) so operators can audit a merge before committing.
- Mirrors the service's REST surface 1:1 — no hidden business logic on the client.
- Stays terse and direct: a single primary action per page, no modals stacked on modals.
- Scales to the four sibling entities (Worker / Place / Course / Event) by copy-adapt of this scaffold.

### 1.3 Non-goals

- **Not** a public-facing portal — assumes authenticated operator users (auth itself out of scope until the service ships it).
- **Not** a substitute for direct API access — power users use Swagger UI / curl.
- **Not** a FHIR client — FHIR routes are out of scope (the service exposes them; this UI doesn't render them).

## 2. Scope

### 2.1 In scope (MVP)

- Dashboard with service-health + recent audit feed.
- Courses list with full-text / fuzzy / phonetic search and SVAR DataGrid.
- Create course with 409-duplicate handling that surfaces the match candidates inline.
- Detail view: identity, identifiers, addresses, telecom, emergency contacts.
- Edit form (full Course record).
- Soft-delete (with confirm).
- Match check page (score a hypothetical record against the index).
- Merge UI (preview + execute).
- Per-record audit log view.

### 2.2 Out of scope (MVP)

- Authentication / authorisation UI.
- FHIR R5 resource viewer.
- GDPR data-export download (the API exists; no UI yet).
- Consent management UI.
- Identity-document detail editing (read-only on detail page).
- Batch deduplication scan UI (API exists; defer until ops asks).
- i18n / locale switching.
- Theme switcher.

## 3. Stakeholders and Users

| Coursea | Need |
| --- | --- |
| Identity operator | Day-to-day CRUD, duplicate triage, manual merge. |
| Engineer | Verify production data without curl. |
| Compliance auditor | Read-only audit log access (deferred until auth lands). |

## 4. Glossary

| Term | Meaning |
| --- | --- |
| **Envelope** | `{ success, data, error }` JSON wrapper from the service. |
| **Match quality** | `definite` / `certain` / `probable` / `possible` / `unlikely` (per `course-service-rust-crate/AGENTS/matching.md`). |
| **Main / Duplicate** | Merge terminology: main survives, duplicate is soft-deleted. |

## 5. Information Architecture

| Route | Purpose |
| --- | --- |
| `/` | Dashboard |
| `/courses` | List + search |
| `/courses/new` | Create |
| `/courses/match` | Match check |
| `/courses/merge` | Merge |
| `/courses/[id]` | Detail |
| `/courses/[id]/edit` | Edit |
| `/courses/[id]/audit` | Audit log |

## 6. Functional Requirements

| ID | Requirement |
| --- | --- |
| FR-1 | List page MUST issue `GET /api/courses/search?q=…` on mount and on search submission. |
| FR-2 | List page MUST expose toggles for `fuzzy` and `phonetic`. |
| FR-3 | Create page MUST capture HTTP 409 from `POST /api/courses` and render the match candidates from `error.details`. |
| FR-4 | Create page MUST surface inline field-level validation (family + given required, birth date not in future). |
| FR-5 | Detail page MUST render identifiers, addresses, telecom, emergency contacts when present. |
| FR-6 | Edit page MUST PUT the full Course record. |
| FR-7 | Soft-delete MUST be confirmed via `confirm()` before issuing DELETE. |
| FR-8 | Match page MUST POST to `/api/courses/match` and render quality + score breakdown. |
| FR-9 | Merge page MUST issue a per-ID GET to render preview before POST `/api/courses/merge`. |
| FR-10 | All pages MUST render the layout shell even when the API is unreachable; API errors render as inline banners. |

## 7. Non-Functional Requirements

- **Bundle size**: keep the JS payload of the list page under 250 kB gzipped.
- **Time to first interaction**: under 1 s on a warm dev server.
- **Accessibility**: WAI-ARIA conformance for forms, focus management on navigation.
- **TypeScript**: strict + `noUncheckedIndexedAccess`.
- **No SSR data fetch in MVP**: pages mount and `fetch()` from the browser; SSR fetch is tracked in §13 T-13 (warm-cache wins for SEO-irrelevant routes).
- **Errors**: every `ApiError` rendered with its `code` and `message`; 422 `details` rendered field-by-field where possible.

## 8. Architecture

```
                +-----------------------------+
                |        Browser (SPA)        |
                |  +-----------------------+  |
                |  |  SvelteKit routes     |  |
                |  |  + Svelte 5 components|  |
                |  +----------+------------+  |
                |             |               |
                |             v               |
                |  +-----------------------+  |
                |  |  CourseRepository     |  |
                |  |  (lib/api/courses.ts) |  |
                |  +----------+------------+  |
                |             |               |
                |             v               |
                |  +-----------------------+  |
                |  |  ApiClient            |  |
                |  |  (lib/api/client.ts)  |  |
                |  +----------+------------+  |
                +-------------|---------------+
                              | HTTP JSON
                              v
                +-----------------------------+
                |   course-service-rust-crate |
                |   Axum + SeaORM + Tantivy   |
                +-----------------------------+
```

## 9. API Consumption

The front-end binds 1:1 to the Course Service REST surface (see [`course-service-rust-crate/AGENTS/restful.md`](../course-service-rust-crate/AGENTS/restful.md)):

| Endpoint | Used by |
| --- | --- |
| `GET /api/health` | Dashboard |
| `GET /api/courses/search` | `/courses` list |
| `GET /api/courses/{id}` | `/courses/[id]`, `/courses/[id]/edit`, merge preview |
| `POST /api/courses` | `/courses/new` |
| `PUT /api/courses/{id}` | `/courses/[id]/edit` |
| `DELETE /api/courses/{id}` | Detail page (soft-delete button) |
| `POST /api/courses/match` | `/courses/match` |
| `POST /api/courses/check-duplicates` | (available — not yet routed) |
| `POST /api/courses/merge` | `/courses/merge` |
| `POST /api/courses/deduplicate` | (available — not yet routed; deferred to roadmap) |
| `GET /api/courses/{id}/audit` | `/courses/[id]/audit` |
| `GET /api/audit/recent` | Dashboard |
| `GET /api/courses/{id}/masked` | (available — not yet routed) |
| `GET /api/courses/{id}/export` | (available — not yet routed) |

Envelope handling is centralised in `ApiClient`; per-endpoint methods on `CourseRepository` return unwrapped `data`.

## 10. Persistence

The front-end is stateless. No local DB, no client-side cache layer beyond Svelte component state. Page reloads re-fetch from the service.

(Roadmap: introduce SvelteKit `+page.ts` load functions with `event.fetch` for SSR hydration — §13 T-13.)

## 11. Testing Strategy

| Layer | Tool | Scope |
| --- | --- | --- |
| Unit | Vitest + jsdom | `ApiClient` envelope handling, `ApiError` mapping, `CourseRepository` wiring. |
| E2E smoke | Playwright | Page-shell rendering for every MVP route without requiring a live service. |
| Live integration | (manual) | Run `pnpm dev` against a running `course-service-rust-crate`; click through CRUD/match/merge. |

Run: `pnpm test`, `pnpm test:e2e`.

## 12. Compliance

- **No PII logging**: never `console.log` Course values.
- **Soft-delete confirm**: the UI requires confirm before `DELETE`; the service handles the soft-delete itself.
- **GDPR**: export endpoint exists in the service; UI deferred (roadmap).
- **HIPAA**: audit-log view exposes the trail; user-attribution requires auth (deferred).

## 13. Tasks

- [x] T-1: Scaffold SvelteKit project (config, app shell, CSS).
- [x] T-2: Wire TypeScript types matching `course-service-rust-crate/AGENTS/models.md`.
- [x] T-3: `ApiClient` + `CourseRepository`.
- [x] T-4: Form primitives (`LabeledField`, `FieldError`, `FieldRow`, `createForm`).
- [x] T-5: List route with SVAR DataGrid + search box.
- [x] T-6: Create route with 409-duplicate inline surfacing.
- [x] T-7: Detail / edit / soft-delete.
- [x] T-8: Audit log view.
- [x] T-9: Match check route.
- [x] T-10: Merge UI with preview.
- [x] T-11: Vitest unit tests for `ApiClient` + `CourseRepository`.
- [x] T-12: Playwright e2e smoke for every MVP route.
- [ ] T-13: SSR-safe load functions using `event.fetch` for SEO-irrelevant but warm-cache wins.
- [ ] T-14: Integrate Lily Headless components beyond Button (Dialog for merge confirm, Combobox for identifier system, Banner for error states).
- [ ] T-15: Instance / syllabus-section edit UI (currently the detail page renders instances read-only; the service ships full CRUD on `/api/courses/{id}/instances/{instance_id}` so it is purely a UI gap). Identifier add/remove already covered by `CourseIdentifierInput`.
- [ ] T-16: Theming tokens in `app.css` extracted to a small theme module.
- [ ] T-17: `check-duplicates` endpoint wired into create form (preview before commit).
- [ ] T-18: Batch deduplicate-scan results UI.
- [ ] T-19: Masked-view toggle on detail page.
- [ ] T-20: GDPR-export download button.
- [ ] T-21: Validate the SVAR licensing fit (free GPL-3.0 vs Pro) — see §16 OQ-1.

## 14. Implementation Status

| Area | Status |
| --- | --- |
| Project scaffold | ✅ |
| TypeScript types | ✅ |
| API client | ✅ (Vitest covered) |
| List + search | ✅ |
| Create + 409 surfacing | ✅ |
| Detail / Edit / Delete | ✅ |
| Audit view | ✅ |
| Match check | ✅ |
| Merge UI | ✅ |
| Unit tests | ✅ 9 tests (5 in `client.test.ts` + 4 in `courses.test.ts`) |
| E2E smoke | ✅ 5 Playwright tests in `e2e/courses.spec.ts` |
| `pnpm install` verified | ✅ |
| `pnpm test` verified | ✅ |
| Live integration | ❌ — pending operator walkthrough against a running service |

## 15. Roadmap

- **v0.2** (shipped 2026-06-05): bug-fix sweep against the now-real
  Course Service surface — `ScoredCandidate` alignment, blank-string
  URL normalisation, inert match-page threshold, dashboard health
  badge, empty-search `"*"` wildcard, inert phonetic checkbox,
  `API_BASE_URL` default + README/`.env.example` port realignment.
  Plus spec.md §13 / §14 counter realignment + CHANGELOG.
- **v0.3** (next): SSR-safe load functions using `event.fetch` for
  warm-cache SEO-irrelevant wins (T-13); Lily Dialog (merge
  confirm) + Combobox (identifier system) integration (T-14);
  instance / syllabus-section edit UI (T-15 — the genuine
  remaining sub-resource gap).
- **v0.4**: Auth integration once Course Service ships JWT (T-15 of
  service spec — blocked on the family-wide rollout).
- **v0.5+**: Batch dedup results UI (T-18), masked-view toggle on
  detail (T-19), GDPR-export download (T-20).

## 16. Open Questions

- **OQ-1**: SVAR DataGrid free tier is GPL-3.0. If this front-end ships in a commercial / proprietary deployment, what license tier do we need? Pro? Enterprise? Decision required before any production deploy.
- **OQ-2**: Should the create route call `check-duplicates` for an inline preview before the actual `create` POST, or rely solely on the 409 round-trip? Round-trip is simpler; preview is friendlier. Operator feedback needed.
- **OQ-3**: When the service returns `403`/`401` (post-auth), how should the UI redirect? Tied to whatever auth flow the service chooses (JWT vs session vs OAuth).
- **OQ-4**: Drift policy: per project decision 2026-06-02 we keep API client + types in each front-end project. Revisit when the third sibling front-end ships if drift becomes painful.

## 17. References

- [`../course-service-rust-crate/spec.md`](../course-service-rust-crate/spec.md)
- [`../course-service-rust-crate/AGENTS/restful.md`](../course-service-rust-crate/AGENTS/restful.md)
- [`../course-service-rust-crate/AGENTS/models.md`](../course-service-rust-crate/AGENTS/models.md)
- [`../course-service-rust-crate/AGENTS/matching.md`](../course-service-rust-crate/AGENTS/matching.md)
- [`../agents/share/restful.md`](../agents/share/restful.md)
- SVAR Svelte DataGrid — https://svar.dev/svelte/grid/
- Lily Design System Svelte Headless — `../../lilydesignsystem/lily-design-system/lily-design-system-svelte-headless/`

## 18. Change Control

- Major changes to the API surface require coordinated PR with the service crate.
- Major changes to the form / route shape require updating §5, §6, §13 in this spec.
- Roadmap items move from §15 to §13 when they become work-in-flight.
