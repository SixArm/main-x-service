# case-front-end-with-svelte

Operator UI for the [Case Service](../case-service-with-loco):
case **CRUD + matching** (governmental case management).

SvelteKit 2 · Svelte 5 (runes) · TypeScript strict · SPA.

## Routes

| Route | Purpose |
|---|---|
| `/` | List cases |
| `/cases` | SVAR DataGrid index with FilterBar (client-side filtering) |
| `/board` | Status Kanban board (SVAR) — drag a card to another column to change status |
| `/new` | Create |
| `/[pid]` | Detail + delete + check-duplicates + "subject of this case" cross-service links panel |
| `/[pid]/edit` | Edit |
| `/merge` | Merge a confirmed duplicate into a survivor + recent merge history |
| `/signin` | Magic-link sign-in (this app's own BFF flow) |
| `/verify` | Magic-link verification landing page |

## Prerequisites

- Node 20+ and pnpm
- A running [Case Service](../case-service-with-loco)

## Quick start

```bash
cp .env.example .env     # CASE_API_URL / AUTH_API_URL (server-side)
pnpm install
pnpm dev                 # http://localhost:5173
```

## Configuration

| Var | Default | Purpose |
|---|---|---|
| `CASE_API_URL` | `http://localhost:5150` | Case service base URL (server-side only; read in `src/lib/server/config.ts`). |
| `AUTH_API_URL` | `http://localhost:5150` | Authentication service base URL (server-side only; session→PASETO exchange + magic-link flow). |

Neither variable reaches the client bundle. The browser talks only to
the app's own origin: entity-API calls go through the same-origin
`/api/proxy` BFF route, which forwards them to the case service with a
server-injected PASETO.

## Sign in (BFF magic-link)

The app has its **own** `/signin` + `/verify` magic-link flow — there is
no cross-origin redirect to the auth front-end. The operator requests a
magic link at `/signin`; verifying it at `/verify` establishes a
**server-side session** and sets an **httpOnly session cookie**
(`__Host-mxi_session`); the browser holds **no token** — there is no
`localStorage`, no URL fragment, and no `mxi_access_token`. This app's
own **SvelteKit server acts as a Backend-For-Frontend (BFF)**: it holds
the session cookie, exchanges it for a short-lived **PASETO v4.public**
token, and calls the case service server-side with that bearer.
State-changing requests carry a **CSRF token**. See
[`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
(source of truth; RS256/JWKS decommissioned).

## How it works

The case record body **is** the `case_matcher::Case` shape (title,
case number, agency, case type, status, priority, opened date, subjects,
keywords, identifiers, sameAs, languages). The form edits these;
`check-duplicates` posts the current record and lists stored matches with
their scores.

## Testing

```bash
pnpm run check     # svelte-check (strict, 0 errors / 0 warnings)
pnpm test          # vitest unit tests
pnpm test:e2e      # Playwright smoke tests (build + preview)
pnpm run build
```

## License

Dual-licensed under MIT OR Apache-2.0.
