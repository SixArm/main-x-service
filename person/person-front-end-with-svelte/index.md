# person-front-end-with-svelte — index

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
| [agents/testing.md](agents/testing.md) | Vitest unit + Playwright smoke + integration |

## Sibling service

- [`../person-service-with-loco/`](../person-service-with-loco/) — the system of record this UI calls. Its [`spec.md`](../person-service-with-loco/spec/index.md) and [`agents/restful.md`](../person-service-with-loco/agents/restful.md) are the API contract.

## Route map

The SPA mounts at `/`. All operator workflows live under `/persons`. The
persistent **top navigation bar** (every route — a hamburger-collapsed
header, NOT a left sidebar, per FR-13) also carries a Lily **theme
switcher** (`ThemePicker`, FR-11) and **locale switcher** (`LocalePicker`,
FR-12); both persist their selection to `localStorage`.

| Path | Purpose |
|---|---|
| `/` | Dashboard — service health + recent audit feed |
| `/persons` | List + search (SVAR DataGrid; `q` + fuzzy + phonetic toggles) |
| `/persons/new` | Create form with real-time 409 duplicate detection inline |
| `/persons/[id]` | Detail view (identity, identifiers, addresses, telecom, emergency contacts, cross-service links panel) |
| `/persons/[id]/edit` | Edit form |
| `/persons/[id]/audit` | Per-record audit log |
| `/persons/match` | Score-bearing match check against a candidate Person |
| `/persons/merge` | Two-ID merge preview + confirm (accepts `?main=`/`?duplicate=` seed) |
| `/persons/bulk` | Bulk import/export — upload + dry-run, filtered export, job polling |
| `/review` | Duplicate review-queue board (SVAR Kanban) + queue table + comparison panel |
| `/expiry` | Identity-document expiry calendar (SVAR Calendar) |
| `/signin`, `/verify` | BFF magic-link sign-in / verification |

## Worked flows

### Search → create with duplicate inline

1. Operator hits `/persons`, searches `"Jane Doe"` in the SearchBox.
2. Grid renders SVAR DataGrid with matching rows.
3. Operator clicks **New person** → `/persons/new`.
4. Form submits to `POST /api/persons`. Service returns 409 with a
   `MatchResult[]` under `error.details` when the matcher flags
   probable duplicates.
5. Inline `MatchResultsList` renders each candidate with name,
   identifier, per-component breakdown, deterministic flag.

### Match-check workflow

1. `/persons/match` posts a partial Person body to
   `POST /api/persons/match`.
2. Service returns blocked candidates sorted by descending score.
3. The page renders quality + per-component breakdown.

### Merge workflow

1. Operator picks main + duplicate IDs in `/persons/merge` (or arrives
   pre-seeded from `?main=`/`?duplicate=`, e.g. from the review queue).
2. Page calls `POST /api/persons/merge` with the two IDs and a
   merge reason.
3. On success: detail page redirect to main; duplicate is
   soft-deleted server-side.

### Review-queue workflow

1. Operator opens `/review`; the board (Kanban) and queue table both
   render `GET /api/persons/review-queue` results, filterable by
   `?status=` / `?limit=`.
2. Selecting a pair (drag on the board, or a queue row's **Compare**
   button) opens an inline panel loading both persons in parallel and
   the matcher's `score_breakdown`.
3. **Confirm** / **Reject** call the decision endpoint (a pure status
   change — it does not merge); a `confirmed` item deep-links to
   `/persons/merge?main=…&duplicate=…`.

### Cross-service links workflow

1. On `/persons/[id]`, the **Cross-service links** panel lists this
   person's active outbound `entity_links` edges.
2. Asserting a new edge is restricted client-side to the three valid
   kind→target-type pairs (`same_identity`→worker, `works_at`/
   `member_of`→organization — `src/lib/links.ts`) before
   `POST /api/persons/{id}/links` is sent.
3. Withdrawing calls `DELETE .../links/{linkId}` behind a `confirm()`.

### Sign-in workflow (BFF)

1. `/signin` posts an email to the authentication service, which emails
   a magic link back to this app's own `/verify`.
2. `/verify` consumes the link server-side and sets the httpOnly
   `__Host-mxi_session` cookie — no token ever reaches browser JS.
3. Every subsequent entity-API call goes through
   `/api/proxy/[...path]`, which exchanges the session for a short-lived
   PASETO and forwards it as `Authorization: Bearer …`.

## Environment

Server-side only (never exposed to the browser bundle — the browser calls
the same-origin `/api/proxy`; see `src/lib/server/config.ts`):

| Variable | Default | Purpose |
|---|---|---|
| `PERSON_API_URL` | `http://localhost:5150` | Person Service base URL — the BFF proxy injects a PASETO and forwards |
| `AUTH_API_URL` | `http://localhost:5150` | Authentication Service base URL — magic-link login + session→PASETO exchange |

`PUBLIC_API_BASE_URL` (default `http://localhost:8080`) is a separate
variable read only by the Playwright **integration** suite (`bin/e2e`,
`playwright.config.ts`), not by the app itself — see README.md
"Configuration".

## Tech stack reminder

- SvelteKit 2 + Svelte 5 **runes only** (`$state`, `$derived`, `$effect`, `$props`, `$bindable`).
- SPA-only (`+layout.ts` exports `ssr = false; prerender = false;`).
- TypeScript strict + `noUncheckedIndexedAccess`.
- SVAR DataGrid for tabular data; Lily Headless for accessibility primitives; native HTML elsewhere.
- No global stores for HTTP state — construct a `PersonRepository` per page/component.

See [`AGENTS.md`](AGENTS.md) for the full ground-rules.
