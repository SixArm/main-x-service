# event-front-end-with-svelte — index

Navigation aid + worked flow examples. The behavioural source of truth
is [`spec.md`](spec/index.md); deep references live in [`AGENTS/`](AGENTS/).

## Top-level documents

| Document | Purpose |
|---|---|
| [spec.md](spec/index.md) | Single source of truth (§1–§18; live work queue in §13) |
| [README.md](README.md) | User-facing intro, routes, env vars |
| [CLAUDE.md](CLAUDE.md) | `@AGENTS.md` re-export for Claude Code session bootstrap |
| [AGENTS.md](AGENTS.md) | Agent guide (ground rules, drift policy, tech-stack rules) |
| [CHANGELOG.md](CHANGELOG.md) | Keep-a-Changelog history |

## AGENTS/ (per-area detail)

| Document | Purpose |
|---|---|
| [AGENTS/index.md](AGENTS/index.md) | This directory's index |
| [AGENTS/spec-driven-development.md](AGENTS/spec-driven-development.md) | SDD discipline (three-part PRs, front-end specifics) |
| [AGENTS/testing.md](AGENTS/testing.md) | Vitest unit + Playwright smoke |

## Sibling service

- [`../event-service-with-loco/`](../event-service-with-loco/) — the system of record this UI calls. Its [`spec.md`](../event-service-with-loco/spec/index.md) and [`AGENTS/restful.md`](../event-service-with-loco/AGENTS/restful.md) are the API contract.

## Route map

The SPA mounts at `/`. All operator workflows live under `/events`.

| Path | Purpose |
|---|---|
| `/` | Dashboard — service health + recent audit feed |
| `/events` | List + search (SVAR DataGrid; `q` + fuzzy toggle + date / status / type filters) |
| `/events/new` | Create form with real-time 409 duplicate detection inline |
| `/events/[id]` | Detail view (identity, time window, Location union, Party / Offer) |
| `/events/[id]/edit` | Edit form |
| `/events/[id]/audit` | Per-record audit log |
| `/events/match` | Score-bearing match check against a candidate Event |
| `/events/merge` | Two-ID merge preview + confirm |
| `/calendar` | SVAR Calendar over the event time-window; drag-to-reschedule |
| `/signin` | Magic-link sign-in (BFF; plain English only — no i18n yet) |
| `/verify` | Magic-link verification landing (BFF; plain English only) |

## Worked flows

### Search → create with duplicate inline

1. Operator hits `/events`, searches `"Annual Conference"` in the SearchBox,
   optionally enabling the **Fuzzy** toggle and narrowing by date / status / type.
2. Grid renders SVAR DataGrid with matching rows.
3. Operator clicks **New event** → `/events/new`.
4. Form submits to `POST /api/events`. Service returns 409 with a
   `MatchResult[]` under `error.details` when the matcher flags
   probable duplicates.
5. Inline `MatchResultsList` renders each candidate with name,
   identifier, time window, per-component breakdown, deterministic
   flag.

### Match-check workflow

1. `/events/match` posts a partial Event body to
   `POST /api/events/match`.
2. Service returns blocked candidates sorted by descending score
   (name similarity + independent Gaussian-decay scoring of
   `start_date`/`end_date` endpoint distance + identifier
   short-circuits). Window-overlap scoring is **not** implemented —
   see `event-matcher`'s spec OQ-C, still open.
3. The page renders quality + per-component breakdown.

### Merge workflow

1. Operator picks main + duplicate IDs in `/events/merge`.
2. Page calls `POST /api/events/merge` with the two IDs and a
   merge reason.
3. On success: detail page redirect to main; duplicate is
   soft-deleted server-side.

## Environment

Both are **server-side only** (`src/lib/server/config.ts`), never bundled into the browser. The browser talks only to this app's own origin — entity-API calls go through the same-origin `/api/proxy` BFF route, which injects a server-exchanged PASETO.

| Variable | Default | Purpose |
|---|---|---|
| `EVENT_API_URL` | `http://localhost:5150` | Event Service base URL |
| `AUTH_API_URL` | `http://localhost:5150` | Authentication Service base URL (session→PASETO exchange, magic-link) |

## Tech stack reminder

- SvelteKit 2 + Svelte 5 **runes only** (`$state`, `$derived`, `$effect`, `$props`, `$bindable`).
- SPA-only (`+layout.ts` exports `ssr = false; prerender = false;`).
- TypeScript strict + `noUncheckedIndexedAccess`.
- SVAR DataGrid for tabular data; Lily Headless for accessibility primitives; native HTML elsewhere.
- No global stores for HTTP state — construct an `EventRepository` per page/component.
- Auth is BFF-style: httpOnly `__Host-mxi_session` cookie only in the browser; the SvelteKit server exchanges it for a short-lived PASETO. CSRF is not yet implemented (spec §13 T-23b).
- i18n: 13 locales via a dependency-free rune store (`src/lib/i18n.svelte.ts`); `/signin` and `/verify` are not yet translated (spec §13 T-24).

See [`AGENTS.md`](AGENTS.md) for the full ground-rules.
