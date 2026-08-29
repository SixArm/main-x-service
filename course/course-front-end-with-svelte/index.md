# course-front-end-with-svelte — index

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
| [agents/testing.md](agents/testing.md) | Vitest unit + Playwright smoke (live integration is manual) |

## Sibling service

- [`../course-service-with-loco/`](../course-service-with-loco/) — the system of record this UI calls. Its [`spec.md`](../course-service-with-loco/spec/index.md) and [`agents/restful.md`](../course-service-with-loco/agents/restful.md) are the API contract.

## Route map

The SPA mounts at `/`. All operator workflows live under `/courses`.

| Path | Purpose |
|---|---|
| `/` | Dashboard — service health + recent audit feed |
| `/courses` | List + search (SVAR DataGrid; name / identifier / additional_type filters; full-text + `fuzzy` toggle — no phonetic toggle, see FR-2) |
| `/courses/new` | Create form with real-time 409 duplicate detection inline |
| `/courses/[id]` | Detail view (identity, identifiers, teaches, keywords, alternate names, same-as links, instances read-only) |
| `/courses/[id]/edit` | Edit form |
| `/courses/[id]/audit` | Per-record audit log |
| `/courses/match` | Score-bearing match check against a candidate Course |
| `/courses/merge` | Two-ID merge preview + confirm |
| `/board` | Course lifecycle Kanban board (SVAR Kanban; drag writes status) |
| `/calendar` | CourseInstance schedule calendar (SVAR Calendar, read-only) |
| `/signin` | Per-app magic-link sign-in (BFF auth page) |
| `/verify` | Magic-link verification (BFF auth page) |

## Worked flows

### Search → create with duplicate inline

1. Operator hits `/courses`, searches `"Intro to CS"` in the SearchBox.
   A single **Fuzzy** checkbox (default on) toggles edit-distance
   tolerance; there is no phonetic checkbox (the service param is a
   no-op — see FR-2). An empty query lists all (`q.trim()` → the
   service's `list` fallback).
2. Grid renders SVAR DataGrid with matching rows.
3. Operator clicks **New course** → `/courses/new`.
4. Form submits to `POST /api/courses`. Service returns 409 with a
   `MatchResult[]` under `error.details` when the matcher flags
   probable duplicates.
5. Inline `MatchResultsList` renders each candidate with name,
   `course_code`, per-component breakdown, deterministic flag.

### Match-check workflow

1. `/courses/match` posts a partial Course body to
   `POST /api/courses/match`.
2. Service returns blocked candidates sorted by descending score.
3. The page applies a client-side "Display threshold" numeric input
   (`<input type="number" step="0.05" min="0" max="1">`) — the
   server-side threshold is fixed, so this cutoff is display-only
   (see CHANGELOG v0.2.0 for why).

### Merge workflow

1. Operator picks main + duplicate IDs in `/courses/merge`.
2. Page calls `POST /api/courses/merge` with the two IDs and a
   merge reason.
3. On success: detail page redirect to main; duplicate is
   soft-deleted server-side.

## Environment

The browser calls the same-origin BFF proxy at `/api/proxy`; there is no
public API base URL. Both variables below are server-only (read in
`src/lib/server/config.ts`, never exposed to the client bundle):

| Variable | Default | Purpose |
|---|---|---|
| `COURSE_API_URL` | `http://localhost:8084` (the code's own fallback matches this, T-28) | Course Service base URL — the proxy injects a server-exchanged PASETO and forwards |
| `AUTH_API_URL` | `http://localhost:5150` | Authentication Service base URL — magic-link login + session→PASETO exchange |

### Sign-in workflow (BFF)

1. Operator visits `/signin`, submits an email. The server action calls
   the authentication service's magic-link endpoint with a `return_url`
   back to this app's own `/verify` (no token in the browser).
2. The emailed link lands on `/verify?token=…`; the SvelteKit server
   exchanges the token, re-hosts the resulting opaque session id as the
   httpOnly `__Host-mxi_session` cookie, and redirects home.
3. `src/routes/api/proxy/[...path]/+server.ts` exchanges that session for
   a short-lived PASETO server-side on every proxied API call
   (`src/lib/server/auth.ts`). See
   [`../../agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md).
   CSRF protection for the browser→BFF mutating calls is not yet
   implemented (spec §13 tracks it as a follow-up).

## Tech stack reminder

- SvelteKit 2 + Svelte 5 **runes only** (`$state`, `$derived`, `$effect`, `$props`, `$bindable`).
- SPA-only (`+layout.ts` exports `ssr = false; prerender = false;`).
- TypeScript strict + `noUncheckedIndexedAccess`.
- SVAR DataGrid, Kanban, Calendar for structured data; Lily Headless for accessibility primitives; native HTML elsewhere.
- Lily `ThemePicker` + `LocalePicker` (13 locales) wired live in the layout shell.
- No global stores for HTTP state — construct a `CourseRepository` per page/component.

See [`AGENTS.md`](AGENTS.md) for the full ground-rules.
