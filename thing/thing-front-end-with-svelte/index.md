# thing-front-end-with-svelte — index

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

- [`../thing-service-with-loco/`](../thing-service-with-loco/) — the system of record this UI calls. Its [`spec.md`](../thing-service-with-loco/spec/index.md) and [`AGENTS/restful.md`](../thing-service-with-loco/AGENTS/restful.md) are the API contract.

## Route map

The SPA mounts at `/`. All operator workflows live under `/things`.

| Path | Purpose |
|---|---|
| `/` | Dashboard — service health + recent audit feed |
| `/things` | List + search (SVAR DataGrid; `q` + fuzzy + phonetic toggles) |
| `/things/new` | Create form with real-time 409 duplicate detection inline |
| `/things/[id]` | Detail view (identity, PropertyValue identifiers — DOI / ISBN / GTIN / …) |
| `/things/[id]/edit` | Edit form |
| `/things/[id]/audit` | Per-record audit log |
| `/things/match` | Score-bearing match check against a candidate Thing |
| `/things/merge` | Two-ID merge preview + confirm |
| `/review` | Stored duplicate-review board (SVAR Kanban: Pending / Confirmed / Rejected / AutoMerged) |
| `/signin` | Per-app magic-link sign-in (BFF) |
| `/verify` | Magic-link verification (BFF) |

## Worked flows

### Search → create with duplicate inline

1. Operator hits `/things`, searches `"Widget 9000"` in the SearchBox.
2. Grid renders SVAR DataGrid with matching rows.
3. Operator clicks **New thing** → `/things/new`.
4. Form submits to `POST /api/things`. Service returns 409 with a
   `MatchResult[]` under `error.details` when the matcher flags
   probable duplicates.
5. Inline `MatchResultsList` renders each candidate with name,
   identifier (DOI / ISBN / GTIN / …), per-component breakdown,
   deterministic flag.

### Match-check workflow

1. `/things/match` posts a partial Thing body to
   `POST /api/things/match`.
2. Service returns blocked candidates sorted by descending score.
3. The page renders quality + per-component breakdown.

### Merge workflow

1. Operator picks main + duplicate IDs in `/things/merge`.
2. Page calls `POST /api/things/merge` with the two IDs and a
   merge reason.
3. On success: detail page redirect to main; duplicate is
   soft-deleted server-side.

## Environment

The browser calls only the same-origin BFF proxy (`/api/proxy`); there is
no public/client-held API base URL. These are read server-side only, in
`src/lib/server/config.ts` (see `.env.example`):

| Variable | Default | Purpose |
|---|---|---|
| `THING_API_URL` | `http://localhost:5150` | Thing Service base URL — the proxy injects a server-exchanged PASETO and forwards |
| `AUTH_API_URL` | `http://localhost:5150` | Authentication Service base URL — magic-link login + session→PASETO exchange |

## Tech stack reminder

- SvelteKit 2 + Svelte 5 **runes only** (`$state`, `$derived`, `$effect`, `$props`, `$bindable`).
- SPA-only (`+layout.ts` exports `ssr = false; prerender = false;`).
- TypeScript strict + `noUncheckedIndexedAccess`.
- SVAR DataGrid for tabular data; Lily Headless for accessibility primitives; native HTML elsewhere.
- No global stores for HTTP state — construct a `ThingRepository` per page/component.

See [`AGENTS.md`](AGENTS.md) for the full ground-rules.
