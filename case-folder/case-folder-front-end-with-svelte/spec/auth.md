# Authentication (Svelte edition)

> Part of the [Svelte edition specification](index.md). Cross-cutting
> flow + token claims: [root auth](../../spec/auth.md). The API side:
> [loco auth](../../case-folder-service-with-rust/spec/auth.md).

The client drives the magic-link UI and relies on the API's HttpOnly
session cookie. It stores **no token in JavaScript**.

## Routes

| Route             | Purpose                                                                 |
| ----------------- | ----------------------------------------------------------------------- |
| `/login`          | Email form → `api.auth.requestLink(email)`. In dev shows the returned magic link to click. |
| `/auth/callback`  | Reads `?token=…` → `api.auth.verify(token)` → caches user → redirects to `/`. |

Both are **exempt** from the layout auth guard.

## Wiring

- **`src/lib/api/client.ts`** — every request sends `credentials: 'include'`
  so the session cookie flows. New `api.auth` namespace:
  `requestLink`, `verify`, `me`, `logout`. Base URL defaults to **relative**
  (`/api`) so requests are same-origin through the dev proxy.
- **`vite.config.ts`** — `server.proxy` forwards `/api` (and `/healthz`)
  to the Loco API (`LOCO_API_PROXY`, default `http://localhost:5150`), so
  the browser sees one origin and the HttpOnly cookie is first-party.
- **`src/routes/+layout.ts`** — calls `api.auth.me()`; on `401` (and when
  not already on `/login` or `/auth/callback`) `throw redirect(307, '/login')`;
  otherwise caches the user.
- **`src/lib/store/cache.svelte.ts`** — adds `user` state + `setUser`,
  read reactively by the layout to show who is signed in + a Sign-out button.
- **`src/routes/+layout.svelte`** — utility row shows the signed-in
  email + **Sign out** (`api.auth.logout()` → `goto('/login')`); nav is
  hidden when there is no user.

## Sessions

The session is an HttpOnly cookie set by the API — unreadable by JS
(NFR-9). The app never persists identity to `localStorage`; reload
re-runs the layout `me()` check. Same-origin is required for the cookie,
which is why dev uses the Vite proxy (and production is same-origin).

## Tests

`tests/e2e/global-setup.ts` performs a login through the proxy
(`request`→`verify`) and saves Playwright `storageState` to
`tests/e2e/.auth/state.json`; `playwright.config.ts` loads it via
`use.storageState`, so the existing suites run authenticated with no
per-test changes. `tests/e2e/auth.spec.ts` clears cookies and exercises
the real `/login` → `/auth/callback` flow, the signed-out redirect, and
sign-out.
