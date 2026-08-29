# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec.md](./spec/index.md) — single source of truth (numbered §1–§18; live work queue in §13); [README.md](./README.md) — user-facing intro; [AGENTS.md](./AGENTS.md) — agent guide.

## [Unreleased]

### Added — real magic-link sign-in for the live-integration suite (T-27, PRO-P32)

- `tests/integration/golden-paths.spec.ts` could no longer complete
  any mutating flow once the page-visit auth guard (T-26) and CSRF
  check (T-22) landed — it never signed in. Added a Playwright `setup`
  project (`tests/integration/auth.setup.ts`) driving a **real**
  magic-link sign-in against a live authentication-service (not a
  bypass of the guard/CSRF check): `POST /api/auth/signup`, poll the
  service's own console log for the issued token (ANSI-stripped —
  its tracing output stays coloured through a plain pipe), then
  `page.goto` the real `/verify?token=…` and save the resulting
  `__Host-mxi_session` / `__Host-mxi_csrf` cookies as a `storageState`
  the `integration` project depends on. `smoke` is unaffected — this
  is a project dependency, not a top-level `globalSetup`.
- New family-reusable compose stack, `examples/compose/authentication-dev.yml`:
  authentication-service in `LOCO_ENV=development` (the only mode that
  logs the magic link — SEC-A3), with a container-start patch of
  `server.binding: localhost → 0.0.0.0` (found live: a dev-mode
  container listening on its own loopback is unreachable through the
  compose port-forward regardless of `ports:`).
- `bin/e2e` now also health-checks authentication-service.
- Fixed an adjacent, previously-latent bug found in the same pass:
  `playwright.config.ts`'s `webServer` command set `PUBLIC_API_BASE_URL`
  for the client build but never the BFF's own runtime `PERSON_API_URL`
  (`src/lib/server/config.ts`), so a UI-submitted mutation could
  silently target this crate's `.env` default (`:5150`) instead of the
  live instance the suite actually started (`:8080`) — both now point
  at the same base.
- Verified live end-to-end: signup → log line → `/verify` → both
  cookies present in `storageState`; then a guarded page
  (`/persons/new`) rendered its form instead of redirecting to
  `/signin`. See spec §16 OQ-5 and §13 T-27 for the full record.

### Added — duplicate review-queue screen, completed (repo FE-4)

- The `/review` board already existed as a SVAR Kanban with
  drag-to-decide, but had never been specified — no functional
  requirement, no §13 task, no tests, and its two existing strings were
  missing from the i18n parity assertion. That gap is closed (FR-14…
  FR-19 plus spec §5 sub-states and T-25) alongside the four real
  functional gaps below.
- **Filters.** A status filter and a page-size control, wired to
  `?status=` and `?limit=`. `listReviewQueue()` takes a
  `ReviewQueueOptions` argument; "all" is the **absence** of `status`,
  because the endpoint has no such token and answers `422
  INVALID_STATUS` for one it does not recognise. There is no `offset`
  server-side, so page size is the whole of the pagination story and no
  page control is offered that would not work.
- **A keyboard path.** Drag-to-decide is mouse-only, so it can no longer
  be the only way to act. A native queue table now sits beside the board
  with a `Compare` button on each row, and the comparison panel carries
  real `Confirm` / `Reject` buttons. Dragging still works — it is simply
  no longer load-bearing for accessibility.
- **Side-by-side comparison.** Selecting a pair opens an inline panel
  that loads both records with two parallel `GET /api/persons/{id}`
  calls — there is no combined pair endpoint — and shows name, birth
  date, gender, primary address and primary contact against each other,
  plus the match score, quality, detection method and provenance. The
  fetch uses `allSettled`, not `all`: one side may have been merged away,
  and half a comparison beats none. The matcher's `score_breakdown`
  renders as a component / weight / score table showing only the
  components actually present; a `null` breakdown renders an explicit
  note, and a missing component is omitted rather than shown as `0.00`,
  which would read as "compared and did not match" when the truth is
  "not compared".
- **Provenance** (`operator` / `import`) is now visible on the board
  cards and in the queue table, not only in the panel — it is triage
  information, so it belongs where the triage happens. An unrecognised
  token falls through as itself rather than being swallowed.
- **Confirming does not merge.** The decision endpoint is a pure status
  change and the service records no link between a confirmed item and a
  merge, so the panel deep-links `/persons/merge?main=…&duplicate=…` —
  in **either** survivor order, because a review item names an unordered
  pair and the service does not designate a survivor. `/persons/merge`
  gained a one-line query-string seed for those two ids, both still
  editable.
- Decision buttons are **disabled** for anything not `pending`, mirroring
  the service's `WHERE status = 'pending'` guard rather than offering a
  request guaranteed to answer `422 INVALID_REVIEW_TRANSITION`.
- `ReviewQueueItem` gained the **`provenance`** field the type had been
  missing since the service added the column.
- New pure `src/lib/review.ts`: the status vocabulary, `canDecide`, the
  seven weighted `MATCH_COMPONENTS`, `breakdownRows`, and `mergeHref`.
- Tests: `tests/unit/review.test.ts` (15), four repository tests, an
  i18n-parity extension for 57 new keys across all 13 locales plus the
  three review keys that had never been listed, and two route-stubbed
  Playwright smoke assertions.

### Added — bulk import/export screen (repo FE-3)

- New `/persons/bulk` route: upload a JSONL or CSV file with a **dry-run**
  toggle, submit a filtered **export** with a masking profile, and watch
  each `202`-accepted job poll to a terminal state with its row-count
  breakdown (created / upserted / to-review / errored). A **recent bulk
  jobs** table lists what the service returns, filtered client-side by
  kind and status — the `bulk-jobs` endpoint accepts only `limit`, so
  there is nothing to filter server-side.
- `ApiClient` now passes a **`FormData` body through untouched** —
  no `JSON.stringify`, and the default `content-type` is stripped so
  `fetch` can set the multipart boundary itself (a leftover
  `application/json` makes the service answer `400 BAD_MULTIPART`). JSON
  bodies are unchanged, pinned by a regression test.
- Repository gains `importPersons` / `exportPersons` / `getImportJob` /
  `getExportJob` / `listBulkJobs`; types gain `BulkJobView` /
  `BulkJobAccepted` / `BulkExportRequest`. New `src/lib/bulk.ts` holds
  the pure rules mirroring `src/bulk/handlers.rs`: the terminal-state set
  (an unknown status is **not** terminal, so a newer service cannot
  freeze the poll), the dry-run token encoding, progress clamping, and
  the import-format set that excludes export-only Parquet.
- Each submit carries a **fresh `Idempotency-Key`** (SEC-B9), so a
  retried submit dedupes while two distinct uploads do not collide.
- Three behaviours are dictated by the service, not chosen here.
  `download_url` / `errors_url` are shown as **plain text, not links**:
  they are opaque artifact-store references (`file://…` / `s3://…`) and
  the service exposes no endpoint that serves their bytes (spec §16
  OQ-7). `include_soft_deleted` is **not offered** — the endpoint
  accepts it but the worker rejects it, so the job would be accepted and
  then fail. A `404` while polling stops the loop and reports
  expired-or-gone, since the service returns `404` both for a job past
  its retention TTL and for another actor's job.
- Parquet is offered for export with a caption noting it is behind a
  default-off Cargo feature the front-end cannot detect, so a `failed`
  job reads as a build choice rather than a UI bug. A `full` masking
  profile may draw `401`/`403`; it surfaces as an inline banner.
- Tests: `tests/unit/bulk.test.ts` (14), an i18n-parity extension for 69
  new keys across all 13 locales, and a route-stubbed Playwright smoke
  assertion.

### Added — cross-service links panel (repo FE-2)

- `/persons/[id]` gains a **Cross-service links** panel: it lists this
  person's active outbound edges, asserts a new one, and withdraws one
  behind a `confirm()`. The three kinds person may originate are the
  only options offered, each labelled with the target type it requires
  (`same_identity` → worker, `works_at` / `member_of` → organization),
  and the `to_ref` placeholder follows the selected kind.
- New `src/lib/links.ts` holds the kind ↔ target-type rules and an
  `EntityRef` parser mirroring the service's `validate_edge` /
  `EntityRef::from_str`, so a valid-looking ref pointing at the wrong
  kind of record is caught in the form instead of coming back as a
  `422`. The server stays authoritative — its `422` reason is shown
  inline, as are `404` / `401` / `403`.
- Repository gains `listLinks()` / `createLink()` / `deleteLink()`;
  types gain `EntityLink` + `CreateLinkRequest`. **Deliberately
  distinct** from the existing `PersonLink`, which is the within-entity
  person→person merge relationship and a matcher signal; cross-service
  edges are never a matcher signal
  ([`cross-service-linking.md`](../../agents/share/cross-service-linking.md)
  §7), so the two never share a type or a section.
- Note on the wire: `DELETE …/links/{id}` answers `200` with an empty
  envelope rather than `204`, so the repository method tolerates a body
  it does not read.
- i18n: 26 new keys across all 13 locales (the parity test enforces the
  full set, and `pnpm check` fails outright if one locale is missing a
  key — verified by deliberately deleting one).
- Tests: `tests/unit/links-validation.test.ts` (12 tests pinning the
  accept/reject matrix against the Rust side's), three repository
  URL/verb tests plus a 422-reason-surfacing test, and a Playwright
  smoke assertion that stubs the two API calls at the network layer so
  the smoke project keeps its "no live service required" contract.

### Added — drag-to-decide review board (2026-07-19)

- 2026-07-19 — `/review` now loads the **stored** review queue on mount
  (`GET /api/persons/review-queue`, a safe read; the scan button still
  runs the destructive-classed batch scan explicitly) and dragging a
  pending card into Confirmed / Rejected records the decision through
  `POST /api/persons/review-queue/{id}/decision`. Illegal drags are
  refused client-side and the reload restores the stored truth.
- Repository gains `listReviewQueue()` / `decideReview()`; types gain
  `ReviewDecision` + `ReviewQueueListResponse`.
- e2e: the dashboard smoke spec now opens the hamburger dropdown before
  asserting nav links (the nav is toggle-only at every viewport width;
  the old spec predated that layout).

### Fixed

- 2026-07-19 — dedup-report drift: `ReviewStatus` now matches the wire: the services serialize the
  status lowercase (`pending` / `confirmed` / `rejected` /
  `automerged`), so the type and the review board's column ids use
  the lowercase tokens (labels stay human-cased). The previous
  capitalized type would have parked every live card outside the
  columns.

### Added

- 2026-07-19 — SVAR moderate fit: new **/review** route (nav-linked): the batch-deduplication
  scan's review queue as SVAR Kanban columns (Pending / Confirmed /
  Rejected / AutoMerged). The scan runs only on the button (POST
  /deduplicate is destructive-classed, never a page-load side
  effect). Read-only: the service exposes no review-decision
  endpoint yet — that endpoint is the seam that would make the
  columns drag targets. +nav.review/review.run x 13 locales.
- 2026-07-19 — SVAR moderate fit: new **/expiry** route (nav-linked):
  identity-document expiry dates as all-day events in the SVAR
  Calendar (read-only; capped search window); selecting an entry
  opens the person. +nav.expiry x 13.

- 2026-07-19 — SVAR component seams: **@svar-ui/svelte-calendar**,
  **@svar-ui/svelte-kanban**, **@svar-ui/svelte-gantt**, and
  **@svar-ui/svelte-filemanager** are installed (no routes yet —
  candidate features are catalogued per project; see the roadmap).

- 2026-07-19 — SVAR DataGrid + Filter: the persons index grid migrates from `wx-svelte-grid` to
  **@svar-ui/svelte-grid**, and a **@svar-ui/svelte-filter**
  FilterBar now sits above it (client-side contains-filtering over
  the flattened rows, every column except the opaque id). The
  legacy `wx-svelte-*` deps are removed.

### Fixed

- Prettier formatting drift across `src/` (left behind by recent
  BFF/auth-era edits) broke the `pnpm lint` (`prettier --check src`)
  gate. Reformatted with `pnpm format`; no behavioural change —
  `svelte-check` and the vitest suite are unchanged and green.

### Added

- **Theme + locale switchers in the layout shell** documented as in-scope. `src/routes/+layout.svelte` renders Lily `ThemeSelect` and `LocaleSelect` (persisted to `localStorage`). Spec §2.1 now lists both as in-scope; §2.2 narrows the out-of-scope item to full i18n message catalogues only. New functional requirements FR-11 (theme switcher) and FR-12 (locale picker) added to spec §6. README Stack/Lily sections and `index.md` updated to list `lily-design-system-svelte-theme-select` + `lily-design-system-svelte-locale-select`.
- **Smoke test** asserting the persons list exposes the `fuzzy` and `phonetic` toggles (spec §6 FR-2), in `tests/e2e/persons.spec.ts`.
- **Unit tests** for the create-form validation rules (spec §6 FR-4: family + given required, birth date not in future), in `tests/unit/person-form-validation.test.ts`.

### Changed

- **Documentation harmonization pass.** Reconciled docs with the shipped code and the resolved OQ-5: corrected the integration-suite test count from 8 to 9 (8 FR-mapped + 1 audit-presence, which maps to no FR); updated the Validation status table and §14 implementation status to reflect that the live stack now builds/runs (6/9 integration tests pass); clarified in spec §11 and `agents/testing.md` that the suite is 9 tests = 8 FR + 1 non-FR audit-presence test.

### Added (prior)

- **Live-service integration test suite** at `tests/integration/golden-paths.spec.ts`. 9 Playwright tests driving the live SvelteKit preview against a running `person-service-with-loco` over real HTTP. Covers spec §6 FR-1 (search-finds-record), FR-3 (create lands on detail page) + the 409-duplicate inline path, FR-5 (detail renders nested fields), FR-6 (edit PUTs the full Person), FR-7 (soft-delete hides record), FR-8 (match check renders score), FR-9 (merge soft-deletes the duplicate), and per-record audit log presence.
- `playwright.config.ts` refactored to two projects: `smoke` (existing, no service needed; `tests/e2e/`) and `integration` (new, live service required; `tests/integration/`). The webServer command now bakes `PUBLIC_API_BASE_URL` into the preview build so the front-end talks to the configured service.
- `bin/e2e` wrapper script that health-checks the service at `PUBLIC_API_BASE_URL/api/health` before running Playwright. Forwards extra args (`--headed`, `--ui`, etc.) to playwright.
- `package.json` adds `test:integration` script. `test:e2e` now scoped to the `smoke` project.

### Documentation

- `spec.md §11 Testing Strategy` extended with a 3-row layer table (unit / smoke / integration) plus run commands and a section on test idempotency.
- `spec.md §13 Tasks` marks T-12a integration suite as complete with harness-level validation (svelte-check clean, playwright `--list` discovers all 9 tests, smoke project still 6/6, `bin/e2e` exits 1 with a clear message when the service is down).
- `spec.md §16 OQ-5` (**resolved 2026-06-04**): the original live-stack blocker — the `person-service-with-loco` had no `src/main.rs`/`[[bin]]` target and a Dockerfile that failed on `debian:bookworm-slim` — has been resolved. The service crate gained `src/main.rs` and the Dockerfile was rebased on `debian:13-slim`; the container builds and runs end-to-end. 6 / 9 Playwright integration tests now pass against the live stack; the remaining 3 are test-data interactions with the now-correctly-aggressive duplicate detector.

### Validation status

| Stage | Result |
| --- | --- |
| `svelte-check` | ✅ 0 errors, 0 warnings (352 files) |
| `pnpm test` (vitest unit) | ✅ 8/8 |
| `pnpm test:e2e` (smoke; refactored config) | ✅ 6/6 |
| `pnpm exec playwright test --project=integration --list` | ✅ 9 tests discovered |
| `bin/e2e` with no service running | ✅ exits 1, prints the bring-up instructions |
| `bin/e2e` against a live service | ⚠️ 6/9 pass (OQ-5 resolved; remaining 3 are duplicate-detector test-data interactions) |

## [0.1.0] — 2026-06-02

Initial scaffold for the Person Service front-end. SvelteKit 2 + Svelte 5 runes + SVAR Svelte DataGrid + Lily Design System Svelte Headless.

### Added

- **Routes (MVP).** Dashboard with service-health + recent-audit feed; persons list with full-text / fuzzy / phonetic search and SVAR DataGrid; create with real-time 409 duplicate detection inline; detail view (identity, identifiers, addresses, telecom, emergency contacts); edit; soft-delete with confirm; per-record audit log; match check; merge with two-ID preview.
- **API layer.** `ApiClient` (envelope + error normalisation) + `ApiError` with `isConflict` / `isNotFound` / `isValidation` shortcuts; `PersonRepository` binding the [Person Service REST surface](../person-service-with-loco/agents/restful.md) (`GET /api/health`, CRUD on `/api/persons`, `/search`, `/match`, `/check-duplicates`, `/merge`, `/deduplicate`, `/{id}/audit`, `/{id}/masked`, `/{id}/export`, `/api/audit/recent`).
- **TypeScript types.** Snake-case domain types mirroring [`person-service-with-loco/agents/models.md`](../person-service-with-loco/agents/models.md): `Person`, `HumanName`, `Address`, `ContactPoint`, `Identifier` (MRN/SSN/DL/NPI/PPN/TAX), `IdentityDocument` (Passport/National-ID/etc.), `EmergencyContact`, `PersonLink`, `MatchResult` + `MatchQuality` + `MatchBreakdown`, `MergeRequest` / `MergeRecord` / `MergeResponse`, `BatchDeduplicationRequest`/`Response`, `ReviewQueueItem`, `AuditEntry`.
- **Form primitives.** `LabeledField`, `FieldError`, `FieldRow`, `createForm` Svelte 5 rune-based store (`value` / `errors` / `submitting` / `submitError` / `submit()` / `reset()`).
- **Components.** `SearchBox`, `PersonGrid` (SVAR `Grid` with `select` mode and `init` callback subscribing to `select-row`), `HumanNameInput`, `PersonForm`, `MatchResultsList` with per-component breakdown disclosure.
- **Tests.** 5 Vitest unit tests for `ApiClient` envelope + error handling, 3 unit tests for `PersonRepository` wiring, 6 Playwright smoke tests covering every MVP route shell.
- **SDD doc set.** `spec.md` (§1–§18; live work queue in §13; open questions in §16), `README.md`, `AGENTS.md`, `CLAUDE.md`.

### Configuration

- `PUBLIC_API_BASE_URL` env var (default `http://localhost:8080`). Read via `import.meta.env` so vitest can load the module without the SvelteKit Vite plugin.
- SPA-only (`src/routes/+layout.ts` exports `ssr = false; prerender = false;`). The SVAR Grid is browser-only and would break SSR; this is a backend admin UI so SSR adds no value.

### Cross-references

- Service spec: [`../person-service-with-loco/spec.md`](../person-service-with-loco/spec/index.md).
- Service REST contract: [`../person-service-with-loco/agents/restful.md`](../person-service-with-loco/agents/restful.md).
- Service model types: [`../person-service-with-loco/agents/models.md`](../person-service-with-loco/agents/models.md).
- Service matching reference: [`../person-service-with-loco/agents/matching.md`](../person-service-with-loco/agents/matching.md).
