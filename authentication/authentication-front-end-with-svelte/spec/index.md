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
/signup      request magic link for a new account  (optional ?return_to=)
/signin      request magic link for an existing account  (optional ?return_to=)
/verify      consume ?token= -> store session -> redirect (return_to handoff or /)
```

## 6. Functional requirements

1. **Sign up** posts `{email, name?}`; on success shows a "check your
   email/console" confirmation. Always treated as success (the service
   never reveals account existence).
2. **Sign in** posts `{email}`; same confirmation behaviour.
3. **Verify** reads `?token=`, calls the service, stores
   `{token, pid, name, email}`, then redirects:
   - If an allowlisted `return_to` was parked on `/signin`/`/signup`
     (see FR 6), redirect the browser to
     `${return_to}#access_token=<jwt>` (the cross-origin SSO handoff;
     token in the URL fragment, full navigation via
     `window.location.assign`).
   - Otherwise redirect to `/`. On failure it offers to request a new
     link.
6. **Cross-origin SSO handoff (issuer side).** `/signin` and `/signup`
   read `?return_to=`; if `origin(return_to)` is allowlisted
   (`VITE_RETURN_TO_ALLOWLIST`, comma-separated exact origins, plus our
   own origin) it is parked in `sessionStorage["mxi_return_to"]` across
   the magic-link email round-trip. A present-but-not-allowlisted value
   is ignored (never parked, never handed the token). The token is
   delivered in the URL **fragment** (browsers do not send fragments to
   servers). The allowlist is the control that prevents token
   exfiltration via a crafted `return_to`. Protocol:
   [`jwt-enforcement.md`](../../../AGENTS/share/jwt-enforcement.md)
   ("Token acquisition handoff").
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

Browser `localStorage`: `mxi.auth.token`, `mxi.auth.user`, and the shared
federation key `mxi_access_token` (the issued token is mirrored here so a
same-origin sibling SPA reads the bearer credential with no handoff;
exported as `FEDERATION_TOKEN_KEY`). Browser `sessionStorage`:
`mxi_return_to` (a parked, allowlisted cross-origin handoff target, set on
`/signin`/`/signup` and consumed by `/verify`). No server-side persistence
in the front-end.

## 11. Testing strategy

- **Unit (vitest, `tests/unit/`):** `ApiClient` request shaping (URL
  join, JSON body + content-type, per-request bearer token for `/me` +
  `/signout`, empty-body → `undefined`) and error mapping (`ApiError`
  message extraction, `isUnauthorized` / `isBadRequest`, non-JSON
  fallback) — `client.test.ts`; `AuthRepository` path + verb + body
  construction for signup / magic-link request / verify (URL-encoded
  token) / `me` / signout — `auth.test.ts`. 16 tests.
- **Unit (vitest) — cross-origin handoff:** `return-to.test.ts` (24)
  exhaustively covers the PURE `isAllowedReturnTo` (allowed origin, self
  origin, different host/port/scheme rejected, `javascript:`/`data:`/
  relative/garbage rejected, empty allowlist ⇒ self only),
  `parseAllowlist` (trim + drop blanks), `nextDestination` (external with
  fragment vs. home; token never appended to a non-allowlisted URL), and
  the `sessionStorage` persist/read/clear helpers. `session.test.ts` (3)
  asserts `start()` writes BOTH the legacy token key and the federation
  key `mxi_access_token`, and `clear()` removes the federation key.
- **E2E (playwright, `tests/e2e/smoke.spec.ts`):** the auth API is
  stubbed via `page.route`, so a broken endpoint contract surfaces as a
  failing assertion without a running service. Smoke-loads sign-up,
  sign-in (incl. submit → "link sent"), verify (token → redirect to the
  signed-in dashboard; missing token → error), the cross-origin handoff
  (parked allowlisted `return_to` → redirect to
  `return_to#access_token=…`; non-allowlisted `return_to` → home, no
  token), and home in both signed-in (session seeded into `localStorage`)
  and signed-out states. 9 tests. `playwright.config.ts` runs against
  `vite preview` (build + preview on port 4173) to avoid the `vite dev`
  cold-start module race.

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
- [x] Cross-origin SSO token handoff (issuer side): shared federation key
      `mxi_access_token`; `src/lib/auth/return-to.ts` (`isAllowedReturnTo`
      / `parseAllowlist` / `nextDestination` + sessionStorage helpers);
      `return_to` parked on `/signin`/`/signup` and consumed by `/verify`
      (fragment delivery via `window.location.assign`); `VITE_RETURN_TO_ALLOWLIST`
      config. Tests: `return-to.test.ts` (24) + `session.test.ts` (3) +
      2 playwright handoff cases. *(2026-06-13; protocol
      [`jwt-enforcement.md`](../../../AGENTS/share/jwt-enforcement.md).)*
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
- ~~Where does post-login redirect target live (deep-link return URL)?~~
  **Resolved (2026-06-13):** it does not live in the magic-link (built
  server-side, no `return_to`). The operator app passes
  `?return_to=<absolute URL>` to `/signin`/`/signup`; an allowlisted
  value is parked in `sessionStorage["mxi_return_to"]` and consumed by
  `/verify`, which hands the token off via `return_to#access_token=…`.
  See FR 6 + §11 and the handoff protocol
  [`jwt-enforcement.md`](../../../AGENTS/share/jwt-enforcement.md).

## 17. References

- Sibling service spec (link above); SvelteKit + Svelte 5 runes docs.

## 18. Change control

Update this spec in the same PR as any behavioural change; bump
`CHANGELOG.md` under `[Unreleased]`.
