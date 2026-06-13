## 6. Functional Requirements

| ID | Requirement |
| --- | --- |
| FR-1 | List page MUST issue `GET /api/places/search?q=…` on mount and on search submission. |
| FR-2 | List page MUST expose toggles for `fuzzy` and `phonetic`. |
| FR-3 | Create page MUST capture HTTP 409 from `POST /api/places` and render the match candidates from `error.details`. |
| FR-4 | Create page MUST surface inline field-level validation (name required, latitude −90…90, longitude −180…180, GLN 13 digits). |
| FR-5 | Detail page MUST render address, geo coordinates, identifiers, opening hours, amenities when present. |
| FR-6 | Edit page MUST PUT the full Place record. |
| FR-7 | Soft-delete MUST be confirmed via `confirm()` before issuing DELETE. |
| FR-8 | Match page MUST POST to `/api/places/match` and render quality + score breakdown. |
| FR-9 | Merge page MUST issue a per-ID GET to render preview before POST `/api/places/merge`. |
| FR-10 | All pages MUST render the layout shell even when the API is unreachable; API errors render as inline banners. |

