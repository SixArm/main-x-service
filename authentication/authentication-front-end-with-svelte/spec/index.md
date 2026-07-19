# Authentication Front-End — Specification

> **Single source of truth.** Code conforms to this spec. A behavioural
> change is a three-part PR: spec + code + test. Live work queue is §13;
> open questions are §16.
>
> Sibling service:
> [authentication-service-with-loco](../../authentication-service-with-loco/spec/index.md).

> **Supersedes the prior bearer-token SPA model.** This spec adopts the
> httpOnly-cookie + Backend-For-Frontend (BFF) session model defined in
> the canonical design doc
> [`authentication-sessions.md`](../../../AGENTS/share/authentication-sessions.md)
> (single source of truth for the session/cookie/CSRF/BFF rules). The
> browser no longer holds any credential: the prior
> `mxi_access_token` / `localStorage` access-token model and the
> cross-origin `#access_token=` fragment handoff are **removed**. See §13.

## 1. Purpose and vision

A small, dependency-light SvelteKit app that lets an operator sign up,
sign in, and sign out using passwordless email magic links. Login
establishes a **server-side session**; the browser holds only the
httpOnly `__Host-mxi_session` cookie (per
[`authentication-sessions.md`](../../../AGENTS/share/authentication-sessions.md)
§3), never a token in JS.

## 2. Scope

In scope: the routes (`/`, `/signup`, `/signin`, `/verify`,
`/admin/attributes`), the
SvelteKit server acting as a **BFF** (session cookie handling, server-side
calls to the auth service, CSRF protection), and a dependency-free
bilingual UI (English + Welsh; see §7 and §4). Out of scope: passwords,
social login, account management beyond sign-in, and any data-grid UI.

## 3. Stakeholders and users

Operators of the Main X Index family who need to authenticate before
using any sibling front-end.

## 4. Glossary

- **Magic link** — one-time URL (`/verify?token=…`) that signs a user in.
- **Session** — a **server-side** session row in the authentication-service
  (`authentication-sessions.md` §3), referenced by an opaque id carried in
  the httpOnly `__Host-mxi_session` cookie. The browser holds no session
  data and no token.
- **BFF (Backend-For-Frontend)** — this app's own SvelteKit server
  (`hooks.server.ts` / `+page.server.ts` / `+server.ts`), which holds the
  session cookie and is the only party that talks to the auth service
  (`authentication-sessions.md` §6).
- **CSRF token** — a per-session synchroniser/double-submit token required
  on browser↔BFF mutating requests (`authentication-sessions.md` §4).
- **Locale** — the selected UI language, one of `en` (English) or `cy`
  (Cymraeg / Welsh). Persisted to `localStorage["mxi.auth.locale"]` and
  sent as the `locale` hint on signup / magic-link requests so the email
  language matches the UI. Welsh is a deliberate UK public-sector
  Welsh-language-duty choice; the catalog mirrors the service's `src/i18n.rs`.

## 5. Information architecture

```
/            account dashboard (me + sign out)  — server load reads the session cookie
/signup      request magic link for a new account  (optional ?return_to=)
/signin      request magic link for an existing account  (optional ?return_to=)
/verify      consume ?token= server-side -> session cookie set -> redirect (return_to or /)
/admin/attributes  ABAC attribute admin (?pid=…) — view/replace a user's attributes (access=admin)
```

Requests that establish or use the session are handled by the **SvelteKit
server (BFF)**, not by client JS:

- `/verify` is consumed by a **server route** (`+page.server.ts` /
  `+server.ts`) that exchanges the magic-link token with the auth service;
  the auth service responds with `Set-Cookie: __Host-mxi_session=…`
  (`authentication-sessions.md` §3, §7), which the BFF passes through to
  the browser.
- `/` (dashboard) is rendered by a **server load** that reads the session
  cookie, calls `GET /me` server-side, and passes the profile to the page.
- Sign-out posts to a **server route** that revokes the session at the auth
  service and clears the cookie.

The browser carries only the httpOnly cookie and talks only to this app's
own origin (`authentication-sessions.md` §6); it never reads a credential
and never calls the auth service directly.

### Layout shell & navigation

Cross-cutting UI rule for every `*-front-end-with-svelte` app:

- Global navigation MUST be a **top navigation bar** (header) spanning the full viewport width. There MUST NOT be a left-hand navigation sidebar / rail.
- On narrow viewports the top-bar navigation MUST collapse behind a **hamburger menu** toggle.
- The main content area MUST be **full-width** — never inset by a persistent side-navigation column.

## 6. Functional requirements

1. **Sign up** posts `{email, name?, locale?}` to the auth service via the
   BFF; on success shows a "check your email/console" confirmation. Always
   treated as success (the service never reveals account existence).
   `locale` is the current UI locale (§4) sent so the magic-link email
   language matches the UI; when omitted it drops out of the JSON body and
   the service defaults to English.
2. **Sign in** posts `{email, locale?}` via the BFF; same confirmation and
   `locale` behaviour as Sign up.
3. **Verify** is handled **server-side**: the `/verify` server route reads
   `?token=`, calls the auth service, and the auth service establishes a
   **server-side session** and returns `Set-Cookie: __Host-mxi_session=…`
   (`authentication-sessions.md` §3, §7). The BFF passes the cookie through
   to the browser; the browser is now logged in via the session. No token
   is returned to or stored by the browser. The route then redirects:
   - If an allowlisted `return_to` was parked on `/signin`/`/signup`
     (see FR 4), redirect to `return_to` (a plain navigation; **no**
     `#access_token=` fragment — there is no token to hand off). The
     destination's own origin is signed in via its own session cookie.
   - Otherwise redirect to `/`. On failure it offers to request a new link.
   The verify outcome may carry `is_verified`; the front-end neither
   stores it nor any token (the cookie is the sole credential).
4. **Cross-origin `return_to` (issuer side).** `/signin` and `/signup`
   read `?return_to=`; an allowlisted value (`VITE_RETURN_TO_ALLOWLIST`,
   comma-separated exact origins, plus our own origin) is preserved across
   the magic-link email round-trip and used as the post-verify redirect
   target. A present-but-not-allowlisted value is ignored. **No credential
   travels in the redirect** — the prior `#access_token=` URL-fragment
   handoff is removed; the target relies on its own session cookie. The
   allowlist remains an open-redirect control. Session protocol:
   [`authentication-sessions.md`](../../../AGENTS/share/authentication-sessions.md)
   §6.
5. **Dashboard** is server-rendered: the `/` server load reads the session
   cookie and calls `GET /me` server-side. On `401`/no session it renders
   the signed-out view.
6. **Sign out** posts to a **server route** that calls `POST /signout`
   (the auth service revokes the session, `authentication-sessions.md` §3)
   and clears the `__Host-mxi_session` cookie.
7. **CSRF protection.** Every browser→BFF mutating request
   (`POST`/`PUT`/`PATCH`/`DELETE`: sign up, sign in, verify-submit where
   applicable, sign out) MUST carry a per-session CSRF token, validated
   server-side, with an `Origin`/`Referer` allow-list backstop. Safe
   methods (`GET`/`HEAD`) are exempt. The session cookie is `SameSite=Lax`.
   (`authentication-sessions.md` §4.)
8. **Layout shell** presents global navigation as a full-width **top
   bar** (header) with a **hamburger** toggle on narrow viewports — NOT a
   left sidebar — and the main content area is **full-width**.

## 7. Non-functional requirements

- Svelte 5 runes only; TypeScript strict (`noUncheckedIndexedAccess`).
- **BFF, not pure SPA.** The auth-bearing paths (verify, dashboard load,
  sign-out) run on the SvelteKit **server** so the httpOnly session cookie
  is held server-side (`authentication-sessions.md` §6). The session is a
  cookie, never `localStorage`. Read-only UI may still render client-side
  but any auth-bearing fetch goes through the BFF. Only non-credential UI
  state (the selected `locale`) is kept in `localStorage`.
- No data-grid / design-system dependency (accepted drift).
- **Localization (i18n).** Bilingual UI (English `en` + Welsh `cy`)
  implemented with **no i18n library** — `src/lib/i18n.svelte.ts` holds a
  per-locale strings catalog, a reactive `$state` current-locale store
  (persisted to `localStorage`), and a `t(key)` accessor. The surface is
  tiny, so the dependency-free approach keeps the front-end lean (matching
  the no-data-grid posture above). Welsh support is a deliberate UK
  public-sector Welsh-language-duty choice and mirrors the service's
  catalog (`authentication-service-with-loco/src/i18n.rs`). The fallback
  chain is: target locale → English → the key string itself; an unknown
  or region-subtagged input (`cy-GB`) reduces to its primary language
  (`cy`) or falls back to English.

## 8. Architecture

**BFF layering.** The SvelteKit **server** is the auth boundary:
`hooks.server.ts` reads/validates the `__Host-mxi_session` cookie and
populates `event.locals`; `+page.server.ts` / `+server.ts` routes hold the
session and call the auth service server-side via a lean `ApiClient`
(raw-JSON; attaches the session cookie / minted bearer on the **server**,
never in the browser) → `AuthRepository` (endpoint methods). The browser
talks only to this app's own origin. There is **no client-side
`session.svelte.ts` holding a token**: client auth state, where needed, is
hydrated from the server load (signed-in vs signed-out), and CSRF tokens
are issued by the BFF (§6 FR 7).

## 9. API consumption

All auth-service calls are made **server-side by the BFF**. The browser
never sends an `Authorization: Bearer` header and never sees a token; it
calls only this app's own server routes (carrying the httpOnly session
cookie + a CSRF token on mutations, §6 FR 7).

| Route / action | Auth-service endpoint (server-side) | Request body |
|---|---|---|
| `/signup` | `POST /api/auth/signup` | `{email, name?, locale?}` |
| `/signin` | `POST /api/auth/magic-link` | `{email, locale?}` |
| `/verify` | `GET /api/auth/magic-link/{token}` → `Set-Cookie` | _(token in path)_ |
| `/` load | `GET /api/auth/me` (session cookie) | — |
| `/` sign out | `POST /api/auth/signout` (session cookie) | — |

- **Credential.** The session cookie travels server↔server; the verify
  call returns `Set-Cookie: __Host-mxi_session=…`
  (`authentication-sessions.md` §3, §7), which the BFF relays to the
  browser. There is no client-held bearer.
- `locale` is the optional email-language hint (the current UI locale,
  §4); it drops out of the JSON body when unset (service defaults to
  English). Responses are raw JSON (loco). Errors surface as `ApiError`
  with a `status` and message.

## 10. Persistence

- **Session** — the httpOnly `__Host-mxi_session` cookie (Secure,
  `SameSite=Lax`, `Path=/`), set by the auth service and held by the
  browser opaquely; JS cannot read it (`authentication-sessions.md` §3).
  Session state of record lives in the auth service's `sessions` table,
  not in this front-end.
- **CSRF token** — a per-session token issued by the BFF for mutating
  requests (`authentication-sessions.md` §4); transport (double-submit
  cookie vs. synchroniser token in the BFF payload) is a code-time choice
  (§13 / `authentication-sessions.md` §10).
- **Browser `localStorage`** — only the non-credential UI preference
  `mxi.auth.locale` (the selected UI locale, §4 / §7). The
  `mxi.auth.token` / `mxi.auth.user` / shared `mxi_access_token`
  (`FEDERATION_TOKEN_KEY`) keys are **removed** (§13).
- **Browser `sessionStorage`** — the parked allowlisted `return_to` may be
  carried here across the email round-trip (no credential), or via the
  emailed link / server; it carries **no** token.

> The existing suites below were written against the bearer-token SPA
> model. They are restated to the BFF/cookie model and re-landed in the
> §13 follow-up (the three-part rule pairs that code change with its test
> change). Counts will be reconciled at that time.

- **Unit (vitest, `tests/unit/`):** `ApiClient` request shaping (URL
  join, JSON body + content-type, empty-body → `undefined`) and error
  mapping (`ApiError` message extraction, `isUnauthorized` /
  `isBadRequest`, non-JSON fallback) — `client.test.ts`; `AuthRepository`
  path + verb + body construction for signup / magic-link request / verify
  (URL-encoded token) / `me` / signout — `auth.test.ts`, including that the
  optional `locale` argument rides along in the JSON body on signup /
  magic-link (and is dropped when omitted). These run against the
  **server-side** client (cookie/bearer attached server-side, not a
  browser-held token).
- **Unit (vitest) — i18n:** `i18n.test.ts` covers `translate`/`t` en/cy
  lookups, the fallback chain (unknown locale → English, unknown key →
  the key itself), that every locale covers the core keys, the reactive
  locale switch, region-subtag reduction (`cy-GB` → `cy`), and the
  unsupported-locale default. *(Unchanged by the cookie migration.)*
- **Unit (vitest) — `return_to`:** the allowlist helpers
  (`isAllowedReturnTo` / `parseAllowlist`) still gate the post-verify
  redirect target (open-redirect control). The pure redirect decision is
  restated to a **plain navigation** to `return_to` (no `#access_token=`
  fragment). The federation-key / token-mirroring `session.test.ts` cases
  are **removed** with the `localStorage` token model.
- **Unit (vitest) — CSRF / session:** new cases for the BFF CSRF token
  (issue per session, accept matching, reject missing/mismatched, exempt
  safe methods) and the server-side session-cookie read/clear path.
- **E2E (playwright, `tests/e2e/smoke.spec.ts`):** the auth API is stubbed
  via `page.route`, so a broken endpoint contract surfaces as a failing
  assertion without a running service. Smoke-loads sign-up, sign-in (incl.
  submit → "link sent"), verify (token → BFF sets the session cookie →
  redirect to the signed-in dashboard; missing token → error), the
  `return_to` redirect (allowlisted → redirect to `return_to`, **no token
  in the URL**; non-allowlisted → home), and home in both signed-in
  (session cookie set) and signed-out states. `playwright.config.ts` runs
  against `vite preview` (build + preview on port 4173) to avoid the
  `vite dev` cold-start module race.

## 12. Compliance

**No client-held credential.** The browser holds only the opaque httpOnly
`__Host-mxi_session` cookie, which JS cannot read — this removes the
`localStorage` token-exfiltration class entirely
(`authentication-sessions.md` §3). The session id and any minted bearer
exist only server-side; never log them; the auth service revokes the
session on sign-out and the BFF clears the cookie. CSRF protection (§6 FR
7) guards the cookie-authenticated mutating requests.

## 13. Tasks (live work queue)

> **2026-06-17 — re-spec to httpOnly-cookie + BFF.** This spec now adopts
> the [`authentication-sessions.md`](../../../AGENTS/share/authentication-sessions.md)
> session model and **supersedes the prior bearer-token SPA model**. The
> completed cross-origin-handoff and `localStorage`-token items below
> remain in the log as history; the code follow-up below reverses their
> credential-handling parts.

- [x] **BFF migration (core landed).** The SvelteKit **server** auth
      boundary is in place: `hooks.server.ts`
      (read `__Host-mxi_session`, populate `event.locals`),
      `+page.server.ts` for `/` (dashboard load via `GET /me`,
      server-side) and for `/verify`
      (token exchange → relay `Set-Cookie`), and sign-out (revoke +
      clear cookie). Cookie handling relays
      `Set-Cookie: __Host-mxi_session=…` from
      the auth service and clears on sign-out
      (`authentication-sessions.md` §3, §7). The
      `mxi_access_token` / `FEDERATION_TOKEN_KEY` federation key, the
      `localStorage` token (`mxi.auth.token`/`mxi.auth.user`) model
      (`src/lib/auth/session.svelte.ts`), and the `#access_token=` URL
      **fragment** handoff are **removed**
      (the allowlist survives as an open-redirect control; the redirect
      is a plain navigation to `return_to`).
- [ ] **BFF migration residuals.**
      - **CSRF**: issue a per-session token at the BFF and validate it on
        all browser→BFF mutations, with an `Origin`/`Referer` backstop
        (`authentication-sessions.md` §4); pick the token transport (open
        question, `authentication-sessions.md` §10).
      - Restate the tests per §11 (drop federation-key/token-mirror cases;
        add CSRF + server-side session cases; restate the `return_to` e2e
        to assert no token in the URL — the e2e suite still asserts the
        old fragment handoff).
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
- [x] Bilingual (en/cy) dependency-free i18n: `src/lib/i18n.svelte.ts`
      (per-locale catalog, reactive `$state` locale persisted to
      `localStorage["mxi.auth.locale"]`, `t(key)` accessor, fallback
      chain, region-subtag reduction); a `<select>` locale switcher in
      `+layout.svelte`; the `locale` hint sent on signup / magic-link so
      the email language matches the UI. Welsh = UK public-sector
      Welsh-language duty. Tests: `tests/unit/i18n.test.ts` (9) plus
      `locale` body assertions in `auth.test.ts`. *(spec catch-up
      2026-06-15: code + tests shipped 2026-06-14; spec/README/index/
      CHANGELOG harmonized to match.)*
- [ ] Add a component/playwright test for the UI→API locale handoff: that
      `/signin` + `/signup` pass `i18n.locale` into the repository call
      (currently exercised only indirectly). The pure i18n store and the
      repository `locale` body are unit-covered; the page wiring is not.
- [x] ~~Consider an in-memory token option (vs. `localStorage`) for
      stricter XSS posture.~~ **Resolved (2026-06-17):** obsoleted by the
      BFF/cookie migration — no token lives in the browser at all
      (`authentication-sessions.md` §3, §6); see the BFF task above.

## 14. Implementation status

Shipped (v0.1, prior bearer-token SPA model): all four routes, lean
client, repository, runes session, SPA config, bilingual (en/cy) i18n with
a reactive locale switcher, and the cross-origin token handoff.

**Re-spec'd 2026-06-17** to the httpOnly-cookie + BFF model
(`authentication-sessions.md`), and the **BFF core has landed in code**:
server hooks + server loads, cookie relay/clear, and removal of
`mxi_access_token` / the `localStorage` token + the
`#access_token=` fragment handoff. Remaining (§13): the per-session CSRF
token and restating the test suites to the BFF model.

## 15. Roadmap

v0.1 (shipped): functional magic-link UI (bearer-token SPA). v0.2: **BFF +
httpOnly-cookie session migration** (`authentication-sessions.md`) — core
landed; CSRF + test restatement remain. v0.3: shared-nav integration with
sibling front-ends, all
now session-cookie + BFF based (no shared client token).

## 16. Open questions

- ~~`localStorage` vs in-memory token storage (XSS vs UX/refresh
  tradeoff)?~~ **Resolved (2026-06-17):** neither — the credential is the
  httpOnly session cookie; no token lives in the browser
  (`authentication-sessions.md` §3, §6).
- **CSRF token transport** — double-submit cookie vs. synchroniser token
  in the BFF payload? (Family-level open question,
  `authentication-sessions.md` §10; lean: synchroniser token via the BFF.)
- Should the dashboard auto-refresh `/me` periodically, or only on the
  server load?
- ~~Where does post-login redirect target live (deep-link return URL)?~~
  **Resolved (2026-06-13; updated 2026-06-17):** it does not live in the
  magic-link. The operator app passes `?return_to=<absolute URL>` to
  `/signin`/`/signup`; an allowlisted value survives the email round-trip
  and `/verify` redirects there. **No credential is handed off** — the
  prior `return_to#access_token=…` fragment is removed; the target relies
  on its own session cookie. See FR 4 + §11 and
  [`authentication-sessions.md`](../../../AGENTS/share/authentication-sessions.md)
  §6.

## 17. References

- [`authentication-sessions.md`](../../../AGENTS/share/authentication-sessions.md)
  — canonical session / cookie / CSRF / BFF design (single source of
  truth; supersedes the RS256-JWT + JWKS bearer model).
- Sibling service spec (link above); SvelteKit + Svelte 5 runes docs.

## 18. Change control

Update this spec in the same PR as any behavioural change; bump
`CHANGELOG.md` under `[Unreleased]`.
