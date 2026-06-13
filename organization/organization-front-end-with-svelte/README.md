# organization-front-end-with-svelte

Operator UI for the [Organization Service](../organization-service-rust-crate):
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
- A running [Organization Service](../organization-service-rust-crate)

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
`jurisdiction`, `founding_date`, `keywords` — entity spec OQ-1,
resolved). The form edits these; `check-duplicates` posts the current
record and lists stored matches with their scores.

## Session / authentication

The sidebar has a small **Session** panel. When signed out it shows a
**Sign in** button that sends you to the central authentication
front-end (`${VITE_AUTH_FRONTEND_URL}/signin?return_to=…`). After the
passwordless magic-link, the auth front-end redirects back with the
access token in the URL fragment (`…#access_token=<jwt>`); the SPA
captures it on load, stores it, and strips the fragment from the address
bar. The client then attaches `Authorization: Bearer <token>` to every
API request; "Sign out" clears it. A "Paste a token" disclosure remains
for dev. The token lives under the family-shared
`localStorage["mxi_access_token"]` key; the auth provider is the central
[authentication-service](../../authentication/authentication-service-rust-crate)
(passwordless magic-link → access token). The organization service only
*requires* a token when started with `ORGANIZATION_REQUIRE_AUTH` enabled
(off by default), so the SPA works without a token until that flag is
set. See the family contract in
[`agents/share/jwt-enforcement.md`](../../agents/share/jwt-enforcement.md).

## Testing

```bash
pnpm run check     # svelte-check (strict, 0 errors / 0 warnings)
pnpm run build
```

## License

Dual-licensed under MIT OR Apache-2.0.
