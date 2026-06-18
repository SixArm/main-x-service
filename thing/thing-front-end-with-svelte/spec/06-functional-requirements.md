## 6. Functional Requirements

| ID | Requirement |
| --- | --- |
| FR-1 | List page MUST issue `GET /api/things/search?q=…` on mount and on search submission. |
| FR-2 | List page MUST expose toggles for `fuzzy` and `phonetic`. |
| FR-3 | Create page MUST capture HTTP 409 from `POST /api/things` and render the match candidates from `error.details`. |
| FR-4 | Create page MUST surface inline field-level validation (`name` required; URL-valued fields — `url`, `additional_type`, `main_entity_of_page`, `subject_of` — must be `http://` or `https://`). |
| FR-5 | Detail page MUST render identifiers, alternate names, same-as URLs, and images when present. |
| FR-6 | Edit page MUST PUT the full Thing record. |
| FR-7 | Soft-delete MUST be confirmed via `confirm()` before issuing DELETE. |
| FR-8 | Match page MUST POST to `/api/things/match` and render quality + score breakdown. |
| FR-9 | Merge page MUST make a per-ID GET preview available (a "Load preview" action) before POST `/api/things/merge`, and MUST guard the merge so both IDs are present and distinct (with a `confirm()` step) before issuing the POST. Preview is optional, not mandatory, before merging. |
| FR-10 | All pages MUST render the layout shell even when the API is unreachable; API errors render as inline banners. |
| FR-11 | The layout shell MUST present global navigation as a full-width **top bar** (header) with a **hamburger** toggle on narrow viewports — NOT a left sidebar — and the main content area MUST be full-width. |

