# organization-front-end-with-svelte

Operator UI for the [Organization Service](../organization-service-with-loco):
organization **CRUD + matching**.

SvelteKit 2 · Svelte 5 (runes) · TypeScript strict · SPA.

## Routes

| Route | Purpose |
|---|---|
| `/` | List organizations |
| `/new` | Create |
| `/[pid]` | Detail + delete + check-duplicates |
| `/[pid]/edit` | Edit |

## Prerequisites

- Node 20+ and pnpm
- A running [Organization Service](../organization-service-with-loco)

## Quick start

```bash
cp .env.example .env     # PUBLIC_API_BASE_URL=http://localhost:5150
pnpm install
pnpm dev                 # http://localhost:5173
```

## Configuration

| Var | Default | Purpose |
|---|---|---|
| `PUBLIC_API_BASE_URL` | `http://localhost:5150` | Organization service REST base URL. |
| `VITE_AUTH_FRONTEND_URL` | `http://localhost:5173` | Central authentication front-end base URL for the SSO sign-in handoff. |

## How it works

The organization record body **is** the `organization_matcher::Organization`
shape, serialized snake_case (`name`, `legal_name`, `alternate_names`,
`identifiers` (LEI/DUNS/…), `url`, `same_as`, `address`,
`jurisdiction`, `founding_date`, `telephone`, `email`, `keywords` —
entity spec OQ-1, resolved). `telephone`/`email` are contact fields
(personal data; see spec §12). The form edits these; `check-duplicates`
posts the current record and lists stored matches with their scores,
excluding the record itself.

## Session / authentication

**Target model (BFF).** The browser holds **no token** and never calls
the organization service directly. Sign-in via the central
[authentication-service](../../authentication/authentication-service-with-loco)
passwordless magic-link establishes a server-side **cookie session**
(opaque id in a `__Host-mxi_session` httpOnly cookie); the browser talks
only to this front-end's **own SvelteKit server (BFF)**, which exchanges
the session for a short-lived **PASETO v4.public** token and calls the
organization service server-side with it. Mutating requests are
CSRF-protected. There is no browser-held bearer, no `localStorage`, and
no `mxi_access_token`. The organization service only *requires* a
credential when started with `ORGANIZATION_REQUIRE_AUTH` enabled (off by
default).

Source of truth:
[`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
(RS256 JWT + JWKS and the cross-origin `#access_token` fragment handoff
are decommissioned). **Pivot in progress** — the current runtime still
uses the older client-held-token flow; the BFF + cookie + CSRF code
follow-up is tracked in spec §13.

## Testing

```bash
pnpm run check     # svelte-check (strict, 0 errors / 0 warnings)
pnpm run build
pnpm test          # vitest unit suite (tests/unit/, 49)
pnpm test:e2e      # Playwright smoke (tests/e2e/, 4; runs the production build)
```

## License

Dual-licensed under MIT OR Apache-2.0.
