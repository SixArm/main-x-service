## 6. Functional Requirements

| ID | Requirement |
| --- | --- |
| FR-1 | List page MUST issue `GET /api/persons/search?q=…` on mount and on search submission. |
| FR-2 | List page MUST expose toggles for `fuzzy` and `phonetic`. |
| FR-3 | Create page MUST capture HTTP 409 from `POST /api/persons` and render the match candidates from `error.details`. |
| FR-4 | Create page MUST surface inline field-level validation (family + given required, birth date not in future). |
| FR-5 | Detail page MUST render identifiers, addresses, telecom, emergency contacts when present. |
| FR-6 | Edit page MUST PUT the full Person record. |
| FR-7 | Soft-delete MUST be confirmed via `confirm()` before issuing DELETE. |
| FR-8 | Match page MUST POST to `/api/persons/match` and render quality + score breakdown. |
| FR-9 | Merge page MUST issue a per-ID GET to render preview before POST `/api/persons/merge`. |
| FR-10 | All pages MUST render the layout shell even when the API is unreachable; API errors render as inline banners. |
| FR-11 | The layout shell MUST render a Lily `ThemePicker` whose selection persists to `localStorage` (`lily-theme`) across reloads. |
| FR-12 | The layout shell MUST render a Lily `LocalePicker` whose selection persists to `localStorage` (`lily-locale`); it detects from the navigator on first load. (Picker only — translated copy is out of scope per §2.2.) |
| FR-13 | The layout shell MUST present global navigation as a full-width **top bar** (header) with a **hamburger** toggle on narrow viewports — NOT a left sidebar — and the main content area MUST be full-width. |

