# Authentication Front-End — Specification

> **Single source of truth.** Code conforms to this spec. A behavioural
> change is a three-part PR: spec + code + test. Live work queue is §13;
> open questions are §16.
>
> Sibling service:
> [authentication-service-rust-crate](../../authentication-service-rust-crate/spec/index.md).

## 1. Purpose and vision

A small, dependency-light SvelteKit SPA that lets an operator sign up,
sign in, and sign out using passwordless email magic links, and that
holds the resulting RS256 JWT as the federation's bearer credential.

## 2. Scope

In scope: the four routes (`/`, `/signup`, `/signin`, `/verify`), the API
client, and client-side session storage. Out of scope: passwords, social
login, account management beyond sign-in, and any data-grid UI.

## 3. Stakeholders and users

Operators of the Main X Index family who need to authenticate before
using any sibling front-end.

## 4. Glossary

- **Magic link** — one-time URL (`/verify?token=…`) that signs a user in.
- **Session** — the client-side `{token, user}` held in `localStorage`.
- **Bearer token** — the RS256 JWT sent as `Authorization: Bearer`.

## 5. Information architecture

```
/            account dashboard (me + sign out)
/signup      request magic link for a new account
/signin      request magic link for an existing account
/verify      consume ?token= -> store session -> redirect to /
```

## 6. Functional requirements

1. **Sign up** posts `{email, name?}`; on success shows a "check your
   email/console" confirmation. Always treated as success (the service
   never reveals account existence).
2. **Sign in** posts `{email}`; same confirmation behaviour.
3. **Verify** reads `?token=`, calls the service, stores
   `{token, pid, name, email}`, and redirects to `/`. On failure it
   offers to request a new link.
4. **Dashboard** loads `GET /me` with the stored token; on `401` it
   clears the session and shows the signed-out view.
5. **Sign out** calls `POST /signout` (best-effort) and clears the
   session.

## 7. Non-functional requirements

- Svelte 5 runes only; TypeScript strict (`noUncheckedIndexedAccess`).
- SPA (no SSR/prerender); session in `localStorage`.
- No data-grid / design-system dependency (accepted drift).

## 8. Architecture

`ApiClient` (lean, raw-JSON, bearer-aware) → `AuthRepository` (endpoint
methods) → routes. `session.svelte.ts` is the single source of
client-side auth state, exposed as runes so the layout and pages react to
sign-in/out.

## 9. API consumption

| Route / action | Endpoint |
|---|---|
| `/signup` | `POST /api/auth/signup` |
| `/signin` | `POST /api/auth/magic-link` |
| `/verify` | `GET /api/auth/magic-link/{token}` |
| `/` load | `GET /api/auth/me` (bearer) |
| `/` sign out | `POST /api/auth/signout` (bearer) |

Responses are raw JSON (loco). Errors surface as `ApiError` with a
`status` and message.

## 10. Persistence

Browser `localStorage`: `mxi.auth.token`, `mxi.auth.user`. No server-side
persistence in the front-end.

## 11. Testing strategy

- **Unit (vitest, `tests/unit/`):** `ApiClient` request shaping (URL
  join, JSON body + content-type, per-request bearer token for `/me` +
  `/signout`, empty-body → `undefined`) and error mapping (`ApiError`
  message extraction, `isUnauthorized` / `isBadRequest`, non-JSON
  fallback) — `client.test.ts`; `AuthRepository` path + verb + body
  construction for signup / magic-link request / verify (URL-encoded
  token) / `me` / signout — `auth.test.ts`. 16 tests.
- **E2E (playwright, `tests/e2e/smoke.spec.ts`):** the auth API is
  stubbed via `page.route`, so a broken endpoint contract surfaces as a
  failing assertion without a running service. Smoke-loads sign-up,
  sign-in (incl. submit → "link sent"), verify (token → redirect to the
  signed-in dashboard; missing token → error), and home in both
  signed-in (session seeded into `localStorage`) and signed-out states.
  7 tests. `playwright.config.ts` runs against `vite preview` (build +
  preview on port 4173) to avoid the `vite dev` cold-start module race.

## 12. Compliance

The token is sensitive: it is the bearer credential for the whole
family. Keep it in `localStorage` only; never log it; clear it on sign
out and on `401`.

## 13. Tasks (live work queue)

- [x] Add vitest unit tests for `ApiClient` and `AuthRepository`
      (`tests/unit/client.test.ts` + `tests/unit/auth.test.ts`, 16
      tests). *(2026-06-13; entity spec T-11.)*
- [x] Add a playwright smoke test for the routes
      (`tests/e2e/smoke.spec.ts`, 7 tests; `playwright.config.ts` via
      `vite preview`). Also fixed the scaffold artifact in
      `src/app.html` (meta description named the Course Service).
      *(2026-06-13; entity spec T-11.)*
- [ ] Consider an in-memory token option (vs. `localStorage`) for stricter
      XSS posture.

## 14. Implementation status

Done: all four routes, lean client, repository, runes session, SPA
config. `pnpm run check` clean (0 errors/0 warnings); production build
succeeds. Test suites added: vitest unit (16) + playwright smoke (7),
both green.

## 15. Roadmap

v0.1 (here): functional magic-link UI. v0.2: tests (unit + e2e),
loading/empty states polish. v0.3: shared-nav integration with sibling
front-ends once they consume the auth token.

## 16. Open questions

- `localStorage` vs in-memory token storage (XSS vs UX/refresh tradeoff)?
- Should the front-end auto-refresh `/me` periodically, or only on load?
- Where does post-login redirect target live (deep-link return URL)?

## 17. References

- Sibling service spec (link above); SvelteKit + Svelte 5 runes docs.

## 18. Change control

Update this spec in the same PR as any behavioural change; bump
`CHANGELOG.md` under `[Unreleased]`.
