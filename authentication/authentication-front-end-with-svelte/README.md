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
| `VITE_RETURN_TO_ALLOWLIST` | _(empty)_ | Comma-separated operator-app origins (exact `scheme://host[:port]`) the access token may be handed to via the cross-origin SSO handoff. Unset/empty ⇒ same-origin only. |

## How it works

The access token returned by `/verify` is an RS256 JWT — the federation's
bearer credential. It is kept in `localStorage` (`src/lib/auth/session.svelte.ts`)
under both the back-compat `mxi.auth.token` key and the shared federation
key `mxi_access_token` (so a same-origin sibling SPA can read it without a
handoff). It is sent as `Authorization: Bearer <jwt>` to protected
endpoints. Other Main X services accept the same token by verifying it
offline against the auth service's JWKS.

### Cross-origin SSO handoff

An operator SPA on a different origin cannot read our `localStorage`, so
this front-end is the **issuer** in a first-party, OAuth-implicit-shaped
token handoff (see `AGENTS/share/jwt-enforcement.md`):

1. The operator SPA links here as
   `/signin?return_to=<absolute operator-app URL>` (or `/signup?...`).
2. If `origin(return_to)` is allowlisted (`VITE_RETURN_TO_ALLOWLIST`, or
   our own origin), it is parked in `sessionStorage["mxi_return_to"]`
   across the magic-link email round-trip. A non-allowlisted value is
   ignored — never parked, never handed the token.
3. After `/verify` signs the user in, the browser is redirected to
   `return_to#access_token=<jwt>` (token in the URL **fragment**, which
   browsers never send to servers). The operator SPA reads it from
   `location.hash` and `history.replaceState`s it away.

The allowlist (`src/lib/auth/return-to.ts`) is the security control that
stops token exfiltration via a crafted `return_to`.

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
