# organization-front-end-with-svelte

Operator UI for the [Organization Service](../organization-service-with-loco):
organization **CRUD + matching**.

SvelteKit 2 · Svelte 5 (runes) · TypeScript strict · SPA.

## Routes

| Route | Purpose |
|---|---|
| `/` | List organizations |
| `/organizations` | SVAR grid + filter index |
| `/new` | Create |
| `/[pid]` | Detail + delete + check-duplicates |
| `/[pid]/edit` | Edit |
| `/review` | Drag-to-decide duplicate review board |
| `/merge` | Merge a duplicate into a survivor + recent merge history |
| `/signin` | Magic-link sign-in (BFF) |
| `/verify` | Magic-link verification (BFF) |

## Prerequisites

- Node 20+ and pnpm
- A running [Organization Service](../organization-service-with-loco)

## Quick start

```bash
pnpm install
pnpm dev                 # http://localhost:5173
```

## Configuration

| Var | Default | Purpose |
|---|---|---|
| `ORGANIZATION_API_URL` | `http://localhost:5150` | Organization service REST base URL (read server-side by the BFF proxy; see `src/lib/server/config.ts`). |
| `AUTH_API_URL` | `http://localhost:5150` | Authentication service base URL (BFF-side magic-link + session→PASETO exchange). |

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

**BFF model (current).** The browser holds **no token** and never calls
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
are decommissioned). The runtime is the BFF: sign-in via the app's own
`/signin` + `/verify` routes, API calls via the same-origin `/api/proxy`
route, which injects the server-exchanged PASETO.

## Testing

```bash
pnpm run check     # svelte-check (strict, 0 errors / 0 warnings)
pnpm run build
pnpm test          # vitest unit suite (tests/unit/, 54)
pnpm test:e2e      # Playwright smoke (tests/e2e/, 7; runs the production build)
```

## License

Dual-licensed under MIT OR Apache-2.0.
