# case-front-end-with-svelte

Operator UI for the [Case Service](../case-service-rust-crate):
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
- A running [Case Service](../case-service-rust-crate)

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
| `VITE_AUTH_FRONTEND_URL` | `http://localhost:5173` | Central authentication front-end (SSO sign-in) base URL. "Sign in" redirects to `${VITE_AUTH_FRONTEND_URL}/signin?return_to=…`; the auth front-end hands the access token back via the URL fragment. |

## Sign in (SSO)

The operator clicks **Sign in** in the sidebar and is sent to the central
authentication front-end (`VITE_AUTH_FRONTEND_URL`). After the
passwordless magic-link, the auth front-end redirects back to this app
with the access token in the URL fragment
(`…#access_token=<jwt>`); the app captures it on load, stores it under the
family-shared `localStorage["mxi_access_token"]`, and strips the fragment
from the address bar. The `ApiClient` then attaches it as
`Authorization: Bearer <token>` on every request. A manual token-paste
field is kept for development. See
[`agents/share/jwt-enforcement.md`](../../agents/share/jwt-enforcement.md).

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
