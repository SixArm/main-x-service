## 6. Functional Requirements

| ID | Requirement |
| --- | --- |
| FR-1 | List page MUST issue `GET /api/workers/search?q=…` on mount and on search submission. |
| FR-2 | List page MUST expose toggles for `fuzzy` and `phonetic`. |
| FR-3 | Create page MUST capture HTTP 409 from `POST /api/workers` and render the match candidates from `error.details`. |
| FR-4 | Create page MUST surface inline field-level validation (family + given required, birth date not in future). |
| FR-5 | Detail page MUST render identifiers, addresses, telecom, emergency contacts when present. |
| FR-6 | Edit page MUST PUT the full Worker record. |
| FR-7 | Soft-delete MUST be confirmed via `confirm()` before issuing DELETE. |
| FR-8 | Match page MUST POST to `/api/workers/match` and render quality + score breakdown. |
| FR-9 | Merge page MUST issue a per-ID GET to render preview before POST `/api/workers/merge`. |
| FR-10 | All pages MUST render the layout shell even when the API is unreachable; API errors render as inline banners. |
| FR-11 | The layout shell MUST present global navigation as a full-width **top bar** (header) with a **hamburger** toggle on narrow viewports — NOT a left sidebar — and the main content area MUST be full-width. |
| FR-14 | Review page MUST list the stored duplicate-review queue via `GET /api/workers/review-queue`, with a status filter sent as `?status=` (omitted entirely for "all", since the endpoint has no such token and answers `422 INVALID_STATUS`) and a page-size control sent as `?limit=`. There is no `offset`, so page size is the whole of the pagination story. |
| FR-15 | Review page MUST offer a keyboard-reachable path to every action — a `Compare` button on each queue row and explicit `Confirm` / `Reject` buttons in the comparison panel — **in addition to**, not instead of, the board's drag-to-decide, which cannot be driven from a keyboard. |
| FR-16 | The comparison panel MUST load both sides of the pair with two parallel `GET /api/workers/{id}` calls (there is no combined pair endpoint) and render name, birth date, gender, primary address and primary contact side by side, plus `match_score`, `match_quality` and `detection_method`. One side failing to load MUST still render the other, with a notice. |
| FR-17 | The panel MUST render `score_breakdown` as a labelled component / weight / score table, showing only the components actually present; a `null` breakdown MUST render an explicit note rather than an empty table. |
| FR-18 | Decision buttons MUST be disabled for any item that is not `pending` (the service refuses the transition with `422 INVALID_REVIEW_TRANSITION`), and a `confirmed` item MUST offer a deep link to `/workers/merge?main=…&duplicate=…` in **either** survivor order — confirming records the verdict only and does not merge. |
| FR-19 | `provenance` MUST be visible on the board cards and in the queue table, not only inside the comparison panel, **once the service sends it**. As of this writing `worker-service-with-loco`'s `review_queue` table carries no `provenance` column (unlike the person service — see `spec/13-tasks.md` T-25), so this requirement is currently unmet by the service, not by the front-end. |
| FR-20 | Merge page MUST seed its main / duplicate id inputs from `?main=` / `?duplicate=` when present, and MUST leave both editable. |

