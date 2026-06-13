# authentication-front-end-with-svelte

Operator UI for the [Authentication Service](../authentication-service-rust-crate):
passwordless email magic-link **sign up / sign in / sign out**.

SvelteKit 2 · Svelte 5 (runes) · TypeScript strict · SPA.

## Routes

| Route | Purpose |
|---|---|
| `/` | Account dashboard (current user, sign out) |
| `/signup` | Create an account → emailed a magic link |
| `/signin` | Request a magic link for an existing account |
| `/verify?token=…` | Consume the magic link → store the access token → redirect home |

## Prerequisites

- Node 20+ and pnpm
- A running [Authentication Service](../authentication-service-rust-crate)

## Quick start

```bash
cp .env.example .env     # PUBLIC_API_BASE_URL=http://localhost:5150
pnpm install
pnpm dev                 # http://localhost:5173
```

Sign up, then look at the **auth service console** — in development the
magic link is logged there (no SMTP). Open it to land on `/verify`, which
stores your token and signs you in.

## Configuration

| Var | Default | Purpose |
|---|---|---|
| `PUBLIC_API_BASE_URL` | `http://localhost:5150` | Auth service REST base URL (no trailing slash). |

## How it works

The access token returned by `/verify` is an RS256 JWT — the federation's
bearer credential. It is kept in `localStorage` (`src/lib/auth/session.svelte.ts`)
and sent as `Authorization: Bearer <jwt>` to protected endpoints. Other
Main X services accept the same token by verifying it offline against the
auth service's JWKS.

## Testing

```bash
pnpm run check     # svelte-check (strict, 0 errors expected)
pnpm run build
pnpm run test      # vitest (unit)
pnpm run test:e2e  # playwright
```

## Project layout

See [AGENTS.md](./AGENTS.md) for the `src/` tree and conventions, and
[spec/index.md](./spec/index.md) for the specification.
