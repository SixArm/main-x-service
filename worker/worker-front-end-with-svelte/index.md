# worker-front-end-with-svelte — index

Navigation aid + worked flow examples. The behavioural source of truth
is [`spec.md`](spec/index.md); deep references live in [`agents/`](agents/).

## Top-level documents

| Document | Purpose |
|---|---|
| [spec.md](spec/index.md) | Single source of truth (§1–§18; live work queue in §13) |
| [README.md](README.md) | User-facing intro, routes, env vars |
| [CLAUDE.md](CLAUDE.md) | `@AGENTS.md` re-export for Claude Code session bootstrap |
| [AGENTS.md](AGENTS.md) | Agent guide (ground rules, drift policy, tech-stack rules) |
| [CHANGELOG.md](CHANGELOG.md) | Keep-a-Changelog history |

## agents/ (per-area detail)

| Document | Purpose |
|---|---|
| [agents/index.md](agents/index.md) | This directory's index |
| [agents/spec-driven-development.md](agents/spec-driven-development.md) | SDD discipline (three-part PRs, front-end specifics) |
| [agents/testing.md](agents/testing.md) | Vitest unit + Playwright smoke |

## Sibling service

- [`../worker-service-with-loco/`](../worker-service-with-loco/) — the system of record this UI calls. Its [`spec.md`](../worker-service-with-loco/spec/index.md) and [`agents/restful.md`](../worker-service-with-loco/agents/restful.md) are the API contract.

## Route map

The SPA mounts at `/`. Most operator workflows live under `/workers`; sign-in
and the reverse proxy are the BFF (see [Architecture](#architecture) below).

| Path | Purpose |
|---|---|
| `/` | Dashboard — service health + recent audit feed |
| `/workers` | List + search (SVAR DataGrid; `q` + fuzzy + phonetic toggles) |
| `/workers/new` | Create form with real-time 409 duplicate detection inline |
| `/workers/[id]` | Detail view (identity, identifiers, addresses, telecom, **cross-service links panel**) |
| `/workers/[id]/edit` | Edit form |
| `/workers/[id]/audit` | Per-record audit log |
| `/workers/match` | Score-bearing match check against a candidate Worker |
| `/workers/merge` | Two-ID merge preview + confirm |
| `/review` | Stored duplicate-review board — drag pending cards to Confirmed/Rejected |
| `/expiry` | Credential-expiry calendar |
| `/signin` | Per-app magic-link sign-in (BFF) |
| `/verify` | Magic-link verification (BFF) |

## Architecture

This is a **Backend-For-Frontend**: the browser holds only the httpOnly
`__Host-mxi_session` cookie and calls the same-origin `/api/proxy`, never
the Worker Service directly. See [`spec/08-architecture.md`](spec/08-architecture.md)
for the full diagram and [`AGENTS.md`](AGENTS.md#bff-pattern-auth--api-proxy)
for the file-by-file breakdown.

## Worked flows

### Search → create with duplicate inline

1. Operator hits `/workers`, searches `"Jane Smith"` in the SearchBox.
2. Grid renders SVAR DataGrid with matching rows.
3. Operator clicks **New worker** → `/workers/new`.
4. Form submits to `POST /api/workers`. Service returns 409 with a
   `MatchResult[]` under `error.details` when the matcher flags
   probable duplicates.
5. Inline `MatchResultsList` renders each candidate with name,
   identifier, per-component breakdown, deterministic flag.

### Match-check workflow

1. `/workers/match` posts a partial Worker body to
   `POST /api/workers/match`.
2. Service returns blocked candidates sorted by descending score.
3. The page renders quality + per-component breakdown.

### Merge workflow

1. Operator picks main + duplicate IDs in `/workers/merge`.
2. Page calls `POST /api/workers/merge` with the two IDs and a
   merge reason.
3. On success: detail page redirect to main; duplicate is
   soft-deleted server-side.

### Cross-service links workflow

1. Operator opens `/workers/[id]`; the **Cross-service links** panel
   (`LinksPanel.svelte`) loads `GET /api/workers/{id}/links`.
2. Operator picks `same_identity` (→ a `person`) or `employed_by`
   (→ an `organization`, with `role` as job title) and a target
   `EntityRef` URN; `src/lib/api/links.ts` mirrors the service's
   `validate_edge` so a malformed URN or wrong target type is caught
   before the request.
3. `POST /api/workers/{id}/links` asserts the edge (idempotent upsert
   server-side); `DELETE .../links/{link_id}` withdraws one behind a
   `confirm()`.
4. These are edges to records in *other* services — distinct from the
   within-service `Worker.links`, which this panel never touches (the
   partition rule, [`cross-service-linking.md`](../../agents/share/cross-service-linking.md) §7).

### Sign-in workflow (BFF)

1. `/signin` posts an email to the SvelteKit server action, which asks
   the authentication-service for a magic link whose `return_url` is
   this app's own origin.
2. The emailed link opens `/verify?token=…`; the server exchanges the
   token for an opaque session id and sets it as the httpOnly
   `__Host-mxi_session` cookie, then redirects to `/`.
3. Every subsequent `WorkerRepository` call goes through `/api/proxy`,
   which exchanges the session for a short-lived PASETO server-side
   and forwards `Authorization: Bearer <paseto>` to the Worker Service.
   The browser never holds a token.

## Environment

The browser has no public API env var — it only ever calls the
same-origin `/api/proxy`. The BFF (`src/lib/server/config.ts`) reads:

| Variable | Default | Purpose |
|---|---|---|
| `WORKER_API_URL` | `http://localhost:5150` | Worker Service base URL — the proxy forwards here |
| `AUTH_API_URL` | `http://localhost:5150` | Authentication Service base URL — magic-link + session→PASETO exchange |

See [`.env.example`](.env.example) and [`README.md`](README.md#configuration).

## Tech stack reminder

- SvelteKit 2 + Svelte 5 **runes only** (`$state`, `$derived`, `$effect`, `$props`, `$bindable`).
- CSR/SPA only (`+layout.ts` exports `ssr = false; prerender = false;`); `+page.server.ts`/`+layout.server.ts` still run for the BFF (session reads, form actions) — see Architecture above.
- TypeScript strict + `noUncheckedIndexedAccess`.
- SVAR DataGrid/Filter/Kanban/Calendar for tabular/board/calendar data; Lily Headless for accessibility primitives; native HTML elsewhere.
- No global stores for HTTP state — construct a `WorkerRepository` per page/component.
- i18n: 13 locales with full key parity, pinned by `tests/unit/i18n.test.ts`.

See [`AGENTS.md`](AGENTS.md) for the full ground-rules.
