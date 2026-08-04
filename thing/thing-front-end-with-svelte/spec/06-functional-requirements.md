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
| FR-12 | Review page MUST list the stored duplicate-review queue via `GET /api/things/review-queue`, with a status filter sent as `?status=` (omitted entirely for "all", since the endpoint has no such token and answers `422 INVALID_STATUS`) and a page-size control sent as `?limit=`. There is no `offset`, so page size is the whole of the pagination story. |
| FR-13 | Review page MUST offer a keyboard-reachable path to every action — a `Compare` button on each queue row and explicit `Confirm` / `Reject` buttons in the comparison panel — **in addition to**, not instead of, the board's drag-to-decide, which cannot be driven from a keyboard. |
| FR-14 | The comparison panel MUST load both sides of the pair with two parallel `GET /api/things/{id}` calls (there is no combined pair endpoint) and render id, name, additional type, description, url, owner, primary identifier and primary same-as URL side by side, plus `match_score`, `match_quality` and `detection_method`. One side failing to load MUST still render the other, with a notice. |
| FR-15 | The panel MUST render `score_breakdown` (when the service supplies it) as a labelled component / weight / score table, showing only the components actually present, plus any `true` boolean flags (`phonetic_match` / `deterministic_match`); an absent breakdown MUST render an explicit note rather than an empty table. The service's wire `ReviewQueueItem` does not serialize `score_breakdown` today (verified against `thing-service-with-loco/src/api/rest/handlers.rs`), so in the current deployment this note is the queue's permanent state — a backend follow-up, not a front-end gap. |
| FR-16 | Decision buttons MUST be disabled for any item that is not `pending` (the service refuses the transition with `422 INVALID_REVIEW_TRANSITION`), and a `confirmed` item MUST offer a deep link to `/things/merge?main=…&duplicate=…` in **either** survivor order — confirming records the verdict only and does not merge. |
| FR-17 | Merge page MUST seed its main / duplicate id inputs from `?main=` / `?duplicate=` when present, and MUST leave both editable. |
| FR-18 | The queue table and board cards MUST surface `detection_method` as the pair's "how found" signal. The service's `review_queue` schema has no separate `provenance` column (verified against `thing-service-with-loco/src/db/review_queue.rs`, unlike person / worker / place / organization), so this front end does not fabricate one. |

