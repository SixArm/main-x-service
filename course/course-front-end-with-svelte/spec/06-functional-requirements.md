## 6. Functional Requirements

| ID | Requirement |
| --- | --- |
| FR-1 | List page MUST issue `GET /api/courses/search?q=…` on mount and on search submission. |
| FR-2 | List page MUST expose a `fuzzy` toggle. (`phonetic` is intentionally omitted: the Course Service accepts a `phonetic` search param for API parity but documents it as a no-op, so the inert checkbox was removed in CHANGELOG v0.2.0. Re-add only when the service grows a real Soundex search path — tracked as §13 T-22.) |
| FR-3 | Create page MUST capture HTTP 409 from `POST /api/courses` and render the match candidates from `error.details`. |
| FR-4 | Create page MUST surface inline field-level validation (name required, URL fields must start with http(s)://, course code max 100 chars, credits >= 0). |
| FR-5 | Detail page MUST render identifiers, teaches, keywords, alternate names, same-as links, and instances when present. |
| FR-6 | Edit page MUST PUT the full Course record. |
| FR-7 | Soft-delete MUST be confirmed via `confirm()` before issuing DELETE. |
| FR-8 | Match page MUST POST to `/api/courses/match` and render quality + score breakdown. |
| FR-9 | Merge page MUST offer a per-ID GET preview (a "Load preview" action fetches the main + duplicate courses for confirmation). Preview is available, not a precondition: `doMerge()` validates both IDs are present and differ, then `confirm()`s before POST `/api/courses/merge`. |
| FR-10 | All pages MUST render the layout shell even when the API is unreachable; API errors render as inline banners. |
| FR-11 | The layout shell MUST present global navigation as a full-width **top bar** (header) with a **hamburger** toggle on narrow viewports — NOT a left sidebar — and the main content area MUST be full-width. |

