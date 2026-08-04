# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec.md](./spec/index.md) — single source of truth (numbered §1–§18; live work queue in §13); [README.md](./README.md) — user-facing intro; [AGENTS.md](./AGENTS.md) — agent guide.

## [Unreleased]

### Added — BFF: cookie sessions + PASETO proxy (2026-06-18)

- Retroactive entry (DOC-4, 2026-08-04): this landed in `f66ff50f`
  alongside a family-wide rename pass but was never logged here. The
  front-end became a **Backend-For-Frontend** per
  [`authentication-sessions.md`](../../agents/share/authentication-sessions.md)
  §6: `/signin` + `/verify` per-app magic-link pages
  (`src/routes/signin/`, `src/routes/verify/+page.server.ts`), an
  httpOnly `__Host-mxi_session` cookie read by `src/hooks.server.ts`
  into `event.locals.sessionId`, and a same-origin reverse proxy
  (`src/routes/api/proxy/[...path]/+server.ts`) that exchanges the
  session for a short-lived PASETO (`src/lib/server/auth.ts`) and
  forwards to the Worker Service. The browser never holds a token —
  `ApiClient`'s base URL points at the proxy (`src/lib/config.ts`),
  unchanged for page code. Server-only config moved to
  `src/lib/server/config.ts` (`WORKER_API_URL`, `AUTH_API_URL`),
  replacing the client-exposed `PUBLIC_API_BASE_URL`. CSRF protection
  on mutating browser→BFF calls (§4 of the same doc) was not part of
  this change — see `spec/13-tasks.md` T-22.

### Added — cross-service links panel (2026-08-03, FE-2)

- The worker detail route gains a **Cross-service links** panel
  (`src/lib/components/LinksPanel.svelte`): it lists the worker's active
  outbound `entity_links` edges, lets an operator assert a new one, and
  withdraws one behind a `confirm()`. These are edges to records in
  *other* services — not the within-service `Worker.links`, which is
  untouched.
- Only the two kinds the service permits a worker to originate are
  offered: `same_identity` (→ a `person` record, the federation
  backbone) and `employed_by` (→ an `organization`, where `role` is the
  job title). Optional `confidence`, `provenance`, `valid_from` and
  `valid_to` are exposed; blank `provenance` defaults to `operator`
  server-side.
- `src/lib/api/links.ts` mirrors the service's `validate_edge` as pure
  functions (`checkToRef`, `checkConfidence`), so a malformed URN or a
  wrong target type is explained inline instead of coming back as a 422.
  The server stays the authority — an unanticipated 422's reason string
  is surfaced verbatim.
- Repository gains `listLinks()` / `createLink()` / `deleteLink()`;
  types gain `EntityLink`, `CreateLinkRequest`, `WorkerEdgeKind`, and
  `EntityRefUrn`.
- i18n: 32 new `links.*` keys across all 13 locales. The `provenance`
  placeholder is deliberately left untranslated — `operator` is the
  literal value the service stores, not UI prose.
- Tests: `tests/unit/links-validation.test.ts` pins the accept/reject
  matrix against the Rust `validate_edge` cases; `tests/unit/workers.test.ts`
  pins the three endpoint URLs, methods, and the 422 reason path; the e2e
  smoke spec stubs the two API calls at the network layer so the panel is
  asserted without a running service.

### Added — drag-to-decide review board (2026-07-19)

- 2026-07-19 — `/review` now loads the **stored** review queue on mount
  (`GET /api/workers/review-queue`, a safe read; the scan button still
  runs the destructive-classed batch scan explicitly) and dragging a
  pending card into Confirmed / Rejected records the decision through
  `POST /api/workers/review-queue/{id}/decision`. Illegal drags are
  refused client-side and the reload restores the stored truth.
- Repository gains `listReviewQueue()` / `decideReview()`; types gain
  `ReviewDecision` + `ReviewQueueListResponse`.
- e2e: the dashboard smoke spec now opens the hamburger dropdown before
  asserting nav links (the nav is toggle-only at every viewport width;
  the old spec predated that layout).

### Fixed

- 2026-07-19 — dedup-report drift: `ReviewStatus` lowered to the wire tokens (see the person entry),
  and the i18n catalogue gains `detail.loading` (x 13 locales) so
  the expiry page uses the family's key rather than the local
  `common.loading` variant.

### Added

- 2026-07-19 — SVAR moderate fit: new **/review** route (nav-linked): the batch-deduplication
  scan's review queue as SVAR Kanban columns (Pending / Confirmed /
  Rejected / AutoMerged). The scan runs only on the button (POST
  /deduplicate is destructive-classed, never a page-load side
  effect). Read-only: the service exposes no review-decision
  endpoint yet — that endpoint is the seam that would make the
  columns drag targets. +nav.review/review.run x 13 locales.
- 2026-07-19 — SVAR moderate fit: new **/expiry** route (nav-linked):
  credential/registration document expiry dates as all-day calendar
  events (read-only); selecting an entry opens the worker.
  +nav.expiry x 13.

- 2026-07-19 — SVAR component seams: **@svar-ui/svelte-calendar**,
  **@svar-ui/svelte-kanban**, **@svar-ui/svelte-gantt**, and
  **@svar-ui/svelte-filemanager** are installed (no routes yet —
  candidate features are catalogued per project; see the roadmap).

- 2026-07-19 — SVAR DataGrid + Filter: the workers index grid migrates from `wx-svelte-grid` to
  **@svar-ui/svelte-grid**, with a **@svar-ui/svelte-filter**
  FilterBar above it (client-side filtering). Legacy `wx-svelte-*`
  deps removed.

### Fixed

- Prettier formatting drift across `src/` (left behind by recent
  BFF/auth-era edits) broke the `pnpm lint` (`prettier --check src`)
  gate. Reformatted with `pnpm format`; no behavioural change —
  `svelte-check` and the vitest suite are unchanged and green.

Nothing yet.

## [0.1.0] — 2026-06-02

Initial scaffold for the Worker Service front-end. Copy-adapted from `person-front-end-with-svelte`. SvelteKit 2 + Svelte 5 runes + SVAR Svelte DataGrid + Lily Design System Svelte Headless.

### Added

- **Routes (MVP).** Dashboard with service-health + recent-audit feed; workers list with full-text / fuzzy / phonetic search and SVAR DataGrid; create with real-time 409 duplicate detection inline; detail view (identity, identifiers, addresses, telecom, emergency contacts); edit; soft-delete with confirm; per-record audit log; match check; merge with two-ID preview.
- **API layer.** `ApiClient` (envelope + error normalisation) + `ApiError`; `WorkerRepository` binding the [Worker Service REST surface](../worker-service-with-loco/AGENTS/restful.md) (`GET /api/health`, CRUD on `/api/workers`, `/search`, `/match`, `/check-duplicates`, `/merge`, `/deduplicate`, `/{id}/audit`, `/{id}/masked`, `/{id}/export`, `/api/audit/recent`).
- **TypeScript types.** Snake-case domain types mirroring [`worker-service-with-loco/AGENTS/models.md`](../worker-service-with-loco/AGENTS/models.md): `Worker`, `HumanName`, `Address`, `ContactPoint`, `Identifier` (MRN/SSN/DL/NPI/PPN/TAX), `IdentityDocument`, `EmergencyContact`, `WorkerLink`, `MatchResult`, `MergeRequest`/`Record`/`Response`, `BatchDeduplicationRequest`/`Response`, `ReviewQueueItem`, `AuditEntry`.
- **Form primitives.** `LabeledField`, `FieldError`, `FieldRow`, `createForm` Svelte 5 rune-based store.
- **Components.** `SearchBox`, `WorkerGrid` (SVAR `Grid` with `select` mode + `init`/`select-row`), `HumanNameInput`, `WorkerForm`, `MatchResultsList`.
- **Tests.** 5 Vitest unit tests for `ApiClient`, 3 unit tests for `WorkerRepository`, 6 Playwright smoke tests covering every MVP route shell.
- **SDD doc set.** `spec.md` (§1–§18; live work queue in §13; open questions in §16), `README.md`, `AGENTS.md`, `CLAUDE.md`.

### Configuration

- `PUBLIC_API_BASE_URL` env var (default `http://localhost:8080`).
- SPA-only (`src/routes/+layout.ts` exports `ssr = false; prerender = false;`).

### Cross-references

- Service spec: [`../worker-service-with-loco/spec.md`](../worker-service-with-loco/spec/index.md).
- Service REST contract: [`../worker-service-with-loco/AGENTS/restful.md`](../worker-service-with-loco/AGENTS/restful.md).
- Service model types: [`../worker-service-with-loco/AGENTS/models.md`](../worker-service-with-loco/AGENTS/models.md).
- Service matching reference: [`../worker-service-with-loco/AGENTS/matching.md`](../worker-service-with-loco/AGENTS/matching.md).
