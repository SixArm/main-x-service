## 6. Functional Requirements

| ID | Requirement |
| --- | --- |
| FR-1 | List page MUST issue `GET /api/events/search?q=…` on mount and on search submission. |
| FR-2 | List page MUST expose a `fuzzy` toggle wired into the search query. (Phonetic search is **not** a service search parameter — Soundex is internal to the matcher only — so no phonetic toggle is implemented; deferred as §13 T-22.) |
| FR-3 | Create page MUST capture HTTP 409 from `POST /api/events` and render the match candidates from `error.details`. |
| FR-4 | Create page MUST surface inline field-level validation (name + start date required, end date >= start date, door time <= start date). |
| FR-5 | Detail page MUST render locations, organizers, performers, identifiers, offers when present. |
| FR-6 | Edit page MUST PUT the full Event record. |
| FR-7 | Soft-delete MUST be confirmed via `confirm()` before issuing DELETE. |
| FR-8 | Match page MUST POST to `/api/events/match` and render quality + score breakdown. |
| FR-9 | Merge page MUST issue a per-ID GET to render preview before POST `/api/events/merge`. |
| FR-10 | All pages MUST render the layout shell even when the API is unreachable; API errors render as inline banners. |
| FR-11 | The layout shell MUST present global navigation as a full-width **top bar** (header) with a **hamburger** toggle on narrow viewports — NOT a left sidebar — and the main content area MUST be full-width. |

