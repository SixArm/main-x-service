# case-front-end-with-svelte

Operator UI for the [Case Service](../case-service-with-loco):
case **CRUD + matching** (governmental case management).

SvelteKit 2 · Svelte 5 (runes) · TypeScript strict · SPA.

## Routes

| Route | Purpose |
|---|---|
| `/` | List cases |
| `/new` | Create |
| `/[pid]` | Detail + delete + check-duplicates |
| `/[pid]/edit` | Edit |

## Prerequisites

- Node 20+ and pnpm
- A running [Case Service](../case-service-with-loco)

## Quick start

```bash
cp .env.example .env     # PUBLIC_API_BASE_URL=http://localhost:5150
pnpm install
pnpm dev                 # http://localhost:5173
```

## Configuration

| Var | Default | Purpose |
|---|---|---|
| `PUBLIC_API_BASE_URL` | `http://localhost:5150` | Case service REST base URL. |
| `VITE_AUTH_FRONTEND_URL` | `http://localhost:5173` | Central authentication front-end (SSO sign-in) base URL. "Sign in" redirects to `${VITE_AUTH_FRONTEND_URL}/signin?return_to=…`; the magic-link establishes a server-side session and sets an httpOnly cookie. |

## Sign in (SSO)

The operator clicks **Sign in** in the sidebar and is sent to the central
authentication front-end (`VITE_AUTH_FRONTEND_URL`). After the
passwordless magic-link, the auth service establishes a **server-side
session** and sets an **httpOnly session cookie** (`__Host-mxi_session`);
the browser holds **no token** — there is no `localStorage`, no URL
fragment, and no `mxi_access_token`. This app's own **SvelteKit server
acts as a Backend-For-Frontend (BFF)**: it holds the session cookie,
exchanges it for a short-lived **PASETO v4.public** token, and calls the
case service server-side with that bearer. State-changing requests carry
a **CSRF token**. See
[`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
(source of truth; RS256/JWKS decommissioned).

> Auth pivot in progress: the family moved from client-held bearer
> tokens to the BFF + cookie-session + PASETO model above. The runtime
> here may still reflect the old client-held flow; the BFF follow-up is
> tracked in the spec.

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
