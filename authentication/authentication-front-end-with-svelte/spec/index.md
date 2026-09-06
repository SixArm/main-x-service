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
> [`authentication-sessions.md`](../../../agents/share/authentication-sessions.md)
> (single source of truth for the session/cookie/CSRF/BFF rules). The
> browser no longer holds any credential: the prior
> `mxi_access_token` / `localStorage` access-token model and the
> cross-origin `#access_token=` fragment handoff are **removed**. See §13.

## 1. Purpose and vision

A small, dependency-light SvelteKit app that lets an operator sign up,
sign in, and sign out using passwordless email magic links. Login
establishes a **server-side session**; the browser holds only the
httpOnly `__Host-mxi_session` cookie (per
[`authentication-sessions.md`](../../../agents/share/authentication-sessions.md)
§3), never a token in JS.

## 2. Scope

In scope: the routes (`/`, `/signup`, `/signin`, `/verify`,
`/admin/attributes`), the
SvelteKit server acting as a **BFF** (session cookie handling, server-side
calls to the auth service, CSRF protection), and a dependency-free
13-locale UI (see §7 and §4). Out of scope: passwords,
social login, account management beyond sign-in, any data-grid UI, and
(since the BFF migration) any cross-origin redirect back to another
front-end — see §13.

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
- **Locale** — the selected UI language, one of the 13 codes in
  `src/lib/i18n.svelte.ts`'s `LOCALES` (`en` `cy` `es` `fr` `de` `ar` `ru`
  `hi` `zh` `bn` `pt` `id` `ur` — the same 13-locale set the sibling
  front-ends carry). Persisted to `localStorage["mxi.auth.locale"]` and
  sent as the `locale` hint on signup / magic-link requests so the email
  language matches the UI. Welsh is a deliberate UK public-sector
  Welsh-language-duty choice; `en`/`cy` mirror the service's
  `src/i18n.rs`, the other eleven match the family convention. `ar`/`ur`
  render right-to-left.

## 5. Information architecture

```
/            account dashboard (me + sign out)  — layout server load resolves the session
/signup      request magic link for a new account
/signin      request magic link for an existing account
/verify      consume ?token= server-side -> session + CSRF cookies set -> redirect to /
/admin/attributes  ABAC attribute admin (?pid=…) — view/replace a user's attributes (access=admin)
```

There is no `?return_to=` on any of these routes — the pre-BFF
cross-origin handoff (an operator app links here, signs in, is bounced
back with a token) was removed with the BFF migration, not merely
restated credential-free. See §13.

Requests that establish or use the session are handled by the **SvelteKit
server (BFF)**, not by client JS:

- `/verify` is consumed by a **server route** (`+page.server.ts`) that
  exchanges the magic-link token with the auth service; the auth service
  responds with `Set-Cookie: __Host-mxi_session=…` (and
  `__Host-mxi_csrf=…`) (`authentication-sessions.md` §3, §7), which the
  BFF re-hosts on its own origin and always redirects to `/`.
- The signed-in user is resolved by the **root layout's server load**
  (`+layout.server.ts`, so every route gets it, not just `/`): it reads
  the session cookie, exchanges it for a short-lived bearer
  (`POST /token`), and calls `GET /me` server-side with that bearer. `/`
  (`+page.svelte`) renders this layout data as the dashboard; it has no
  load of its own.
- Sign-out (`+page.server.ts` action on `/`) does the same token exchange,
  then calls `POST /signout`, which revokes the session at the auth
  service, then clears both cookies.

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
   plus `__Host-mxi_csrf=…` (`authentication-sessions.md` §3, §4, §7). The
   BFF re-hosts both cookies on its own origin (httpOnly); the browser is
   now logged in via the session. No token is returned to or stored by
   the browser. The route then **always redirects to `/`** — there is no
   `return_to` target (see §13; removed with the BFF migration, not
   merely restated credential-free). On failure it offers to request a
   new link. The verify outcome may carry `is_verified`; the front-end
   neither stores it nor any token (the cookie is the sole credential).
4. ~~Cross-origin `return_to` (issuer side).~~ **Removed** (§13). Every
   sibling front-end is now its own independent BFF with its own
   `/signin` and its own session against the auth service directly
   (`authentication-sessions.md` §6), so there is no "sign in here, bounce
   back there" flow left to support. `VITE_RETURN_TO_ALLOWLIST` and
   `src/lib/auth/return-to.ts` are gone; do not reintroduce this FR number
   for something unrelated — it stays retired so historical references
   keep meaning.
5. **Dashboard** is server-rendered by the **root layout's** server load
   (`+layout.server.ts`, so it also drives every other route's signed-in
   chrome): it reads the session cookie, exchanges it for a bearer
   (`POST /token`), and calls `GET /me` server-side with that bearer. On
   no session, an exchange failure, or a `GET /me` `401`, it renders the
   signed-out view. `/`'s own `+page.server.ts` carries only the sign-out
   **action**, no load.
6. **Sign out** posts to a **server route** that calls `POST /signout`
   (the auth service revokes the session, `authentication-sessions.md` §3)
   and clears both the `__Host-mxi_session` and `__Host-mxi_csrf` cookies.
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
- No data-grid dependency (accepted drift); Lily `ThemePicker` /
  `LocalePicker` (headless) ARE used for the top-bar theme/locale chrome.
  The six `@svar-ui/*` packages `package.json` had accumulated
  (calendar, filemanager, filter, gantt, grid, kanban — "installed per
  family convention" per `CHANGELOG.md`, no route ever imported any of
  them) were **removed 2026-08-29** (PRO-P24) so the dependency tree
  actually matches this claim instead of contradicting it.
- **Localization (i18n).** The full family-standard **13-locale** UI
  (`en` `cy` `es` `fr` `de` `ar` `ru` `hi` `zh` `bn` `pt` `id` `ur`)
  implemented with **no i18n library** — `src/lib/i18n.svelte.ts` holds a
  per-locale strings catalog, a reactive `$state` current-locale store
  (persisted to `localStorage`), and a `t(key)` accessor. The
  dependency-free approach keeps the front-end lean (matching the
  no-data-grid posture above) despite the full locale count. Welsh
  support is a deliberate UK public-sector Welsh-language-duty choice and
  mirrors the service's catalog
  (`authentication-service-with-loco/src/i18n.rs`); the other eleven
  match the sibling front-ends' coverage. The fallback chain is: target
  locale → English → the key string itself; an unknown or
  region-subtagged input (`cy-GB`) reduces to its primary language (`cy`)
  or falls back to English. `ar`/`ur` render `<html dir="rtl">`.
  `pnpm test` pins full 13-locale key-parity coverage
  (`tests/unit/i18n.test.ts`).

## 8. Architecture

**BFF layering.** The SvelteKit **server** is the auth boundary:
`hooks.server.ts` reads the `__Host-mxi_session` / `__Host-mxi_csrf`
cookies and populates `event.locals` (`sessionId`, `csrfToken`);
`+page.server.ts` / `+layout.server.ts` routes hold the session and call
the auth service server-side via plain `fetch` against `AUTH_API_URL` in
`src/lib/server/auth.ts` / `src/lib/server/admin.ts` (raw-JSON; attaches
the session cookie or a minted bearer on the **server**, never in the
browser). The browser talks only to this app's own origin. There is
**no client-side token store**: client auth state is hydrated from the
root layout's server load (signed-in vs signed-out), and CSRF tokens are
issued by the BFF (§6 FR 7).

> `src/lib/api/{client,auth}.ts` (`ApiClient` / `AuthRepository`) and
> `src/lib/config.ts` were the pre-BFF client-held-token model's HTTP
> layer — no route imported either one once the BFF landed. **Deleted
> 2026-08-29** (PRO-P24), along with the 19 unit tests that existed only
> to exercise them; `src/lib/api/types.ts` (the wire-shape types) stays —
> it is genuinely shared, imported by `src/lib/server/auth.ts`,
> `src/lib/server/admin.ts`, and `src/routes/admin/attributes/`. See §13.

## 9. API consumption

All auth-service calls are made **server-side by the BFF**. The browser
never sends an `Authorization: Bearer` header and never sees a token; it
calls only this app's own server routes (carrying the httpOnly session
cookie + a CSRF token on mutations, §6 FR 7).

| Route / action | Auth-service endpoint (server-side) | Request body |
|---|---|---|
| `/signup` | `POST /api/auth/signup` | `{email, name?, locale?}` |
| `/signin` | `POST /api/auth/magic-link` | `{email, locale?}` |
| `/verify` | `GET /api/auth/magic-link/{token}` → `Set-Cookie` ×2 | _(token in path)_ |
| every page load | `POST /api/auth/token` (cookie+CSRF→bearer) then `GET /api/auth/me` (bearer) | — |
| `/` sign out | `POST /api/auth/token` (as above) then `POST /api/auth/signout` (bearer) | — |
| `/admin/attributes` | `POST /api/auth/token` then `GET`/`PUT /api/auth/admin/users/{pid}/attributes` (bearer) | `{attributes}` on `PUT` |

- **Credential.** The session cookie travels server↔server; the verify
  call returns `Set-Cookie: __Host-mxi_session=…` and
  `Set-Cookie: __Host-mxi_csrf=…` (`authentication-sessions.md` §3, §4,
  §7), which the BFF re-hosts on its own origin. There is no client-held
  bearer. `/me`, `/signout`, and the admin endpoints are **not** called
  with the session cookie directly — each spends the session on a
  `POST /token` exchange first (§6 FR 5) and calls with
  `Authorization: Bearer <short-lived PASETO>`.
- `locale` is the optional email-language hint (the current UI locale,
  §4; any of the 13 supported codes); it drops out of the JSON body when
  unset (service defaults to English). Responses are raw JSON (loco).

## 10. Persistence

- **Session** — the httpOnly `__Host-mxi_session` cookie (Secure,
  `SameSite=Lax`, `Path=/`), set by the auth service and held by the
  browser opaquely; JS cannot read it (`authentication-sessions.md` §3).
  Session state of record lives in the auth service's `sessions` table,
  not in this front-end.
- **CSRF token** — a per-session synchroniser token issued by the auth
  service at verify (`__Host-mxi_csrf`), re-hosted httpOnly by the BFF on
  its own origin, and echoed in `X-CSRF-Token` on the BFF's own `POST
  /token` calls (`authentication-sessions.md` §4). **Resolved** — this is
  implemented (`src/lib/server/session.ts`), not an open code-time choice.
- **Browser `localStorage`** — only the non-credential UI preference
  `mxi.auth.locale` (the selected UI locale, §4 / §7). The
  `mxi.auth.token` / `mxi.auth.user` / shared `mxi_access_token`
  (`FEDERATION_TOKEN_KEY`) keys are **removed** (§13).
- **Browser `sessionStorage`** — unused. The `mxi_return_to` parking key
  described in earlier revisions of this section is **removed** along
  with the rest of the cross-origin handoff (§13).

## 11. Testing

> **Repaired 2026-08-29 (PRO-P24).** The e2e suite is green (6/6); see
> the bullet below and §13 for what changed.

- **Unit (vitest, `tests/unit/`, 17 tests across 3 files, all passing):**
  `session.test.ts` (3) pins the BFF cookie-parsing helpers
  (`src/lib/server/session.ts`); `i18n.test.ts` (13) covers `translate`/
  `t` lookups, the fallback chain, full 13-locale key-parity, RTL
  detection, and region-subtag reduction (`cy-GB` → `cy`); `layout.test.ts`
  (1) is a smoke import check. `client.test.ts` (9) and `auth.test.ts`
  (10) — which pinned the **dead** `src/lib/api/` layer (§8), not the
  live BFF path — were deleted alongside that code (36 → 17).
- **E2E (playwright, `tests/e2e/smoke.spec.ts`, 6 cases, all passing):**
  every auth-service call this app makes happens **server-side** (the
  BFF's own Node `fetch` against `AUTH_API_URL`, `src/lib/server/auth.ts`
  / `admin.ts`), so `page.route()` — which only intercepts requests the
  **browser** issues — can never see any of them; this had been true,
  and the suite silently red, since the BFF migration (`f66ff50f`,
  2026-06-17). The fix points `AUTH_API_URL` at a real (if tiny) Node
  HTTP server, `tests/e2e/mock-auth-server.mjs`, implementing the handful
  of endpoints the BFF calls (signup, magic-link request/verify, token
  exchange, `/me`, signout) with one fixed user and one fixed magic-link
  token — started as a second Playwright `webServer` entry alongside the
  app (`playwright.config.ts`), rather than stubbed in the browser. The
  session cookie the BFF sets on itself (`__Host-mxi_session`, `Secure`)
  is accepted by Chromium over plain HTTP because `localhost` is a
  potentially-trustworthy origin, so the full cookie → `/token` exchange
  → bearer → `/me` round trip exercises for real. The two `return_to`
  cases (cross-origin token handoff) and the `localStorage`-seeded
  signed-in-dashboard case were **deleted, not fixed** — they asserted
  mechanisms removed by the BFF migration (§13; there is no `return_to`
  handoff and no client-held session left to test), and the remaining
  "verify route consumes the token and lands on the signed-in dashboard"
  case already covers signed-in dashboard rendering via the real (mock)
  cookie flow. 9 cases → 6.

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
> the [`authentication-sessions.md`](../../../agents/share/authentication-sessions.md)
> session model and **supersedes the prior bearer-token SPA model**. The
> completed cross-origin-handoff and `localStorage`-token items below
> remain in the log as history — including the "allowlist survives"
> framing on the BFF-migration line, which was accurate the day it was
> written but is **not the end state** (see the removal entry dated
> 2026-08-04 below): the cross-origin handoff was fully deleted, not
> merely made credential-free.

- [x] **BFF migration (core landed).** The SvelteKit **server** auth
      boundary is in place: `hooks.server.ts`
      (read `__Host-mxi_session` / `__Host-mxi_csrf`, populate
      `event.locals`), `+layout.server.ts` (dashboard load via a
      `POST /token` exchange + `GET /me`, server-side, driving every
      route) and `+page.server.ts` for `/verify`
      (token exchange → relay `Set-Cookie`) and for `/` (sign-out: revoke
      + clear cookies). The
      `mxi_access_token` / `FEDERATION_TOKEN_KEY` federation key, the
      `localStorage` token (`mxi.auth.token`/`mxi.auth.user`) model
      (`src/lib/auth/session.svelte.ts`), and the `#access_token=` URL
      **fragment** handoff are **removed**
      (`authentication-sessions.md` §3, §7).
- [x] **CSRF (was a residual, now landed).** The auth service's
      `POST /token` requires the session's CSRF token in
      `X-CSRF-Token`; the BFF captures `__Host-mxi_csrf` at verify,
      re-hosts it httpOnly on its own origin (`locals.csrfToken`), and
      echoes it on every `/token` exchange (`src/lib/server/session.ts`,
      `src/lib/server/auth.ts`). Browser↔BFF CSRF is SvelteKit's native
      form-action origin check. *(landed alongside the admin-attributes
      feature, 2026-07-19 per `CHANGELOG.md`.)*
- [x] **Cross-origin `return_to` handoff — fully removed, 2026-08-04
      (this audit, DOC-4).** Not merely restated credential-free as the
      2026-06-17 entry above framed it: `src/lib/auth/return-to.ts` and
      `tests/unit/return-to.test.ts` were deleted in `f66ff50f` (the same
      commit that landed the BFF core above), and no replacement was ever
      wired in — `/verify` unconditionally redirects to `/`,
      `VITE_RETURN_TO_ALLOWLIST`/`?return_to=` are read nowhere in
      `src/routes/`. **This spec, `README.md`, `index.md`, and
      `AGENTS.md` all still described the feature as live** (FR 4, the
      §5 route table, §10's sessionStorage bullet, README's "Cross-origin
      `return_to`" section, index.md's worked example) for the ~7 weeks
      between the deleting commit and this audit — corrected in place.
      Confirmed via `organization-front-end-with-svelte`'s own
      independent `/signin` + session cookie: every sibling front-end is
      its own BFF now, so there is no "sign in here, bounce back there"
      flow left to support (`authentication-sessions.md` §6).
- [x] **E2E suite is red (5/9), discovered 2026-08-04 (this audit);
      repaired 2026-08-29 (PRO-P24).**
      `tests/e2e/smoke.spec.ts`'s `page.route()` stubs cannot intercept
      the BFF's server-side `fetch` calls (§11) — this had been true
      since `f66ff50f` moved those calls server-side, so the suite was
      silently broken from 2026-06-17 to 2026-08-29. Fixed by pointing
      `AUTH_API_URL` at a real Node HTTP server
      (`tests/e2e/mock-auth-server.mjs`), started as a second Playwright
      `webServer` entry (§11); also dropped the two cases asserting the
      removed `return_to` handoff and the `localStorage`-seeded-session
      case (§13 above; sessions are cookies now). 9 cases → 6, all
      passing.
- [x] **`src/lib/api/{client,auth}.ts` + `src/lib/config.ts` were dead,
      discovered 2026-08-04 (this audit); deleted 2026-08-29 (PRO-P24).**
      No route imported them; their only callers were their own unit
      tests (`client.test.ts`, `auth.test.ts`, 19 tests, deleted with
      them). Decided **delete** over repurposing as the e2e fetch-
      intercept fixture: the mock server the e2e fix needed (above) has
      to run as a standalone process under plain `node` (a second
      Playwright `webServer`), not as an in-process `fetch` wrapper, so
      `ApiClient`/`AuthRepository`'s shape didn't fit that job either.
      `src/lib/api/types.ts` (the wire-shape types) is kept — it is
      genuinely shared with `src/lib/server/`.
- [x] **SVAR/"no data-grid dependency" reconciled, discovered 2026-08-04
      (this audit); resolved 2026-08-29 (PRO-P24).** `package.json`
      carried six unused `@svar-ui/*` packages (calendar, filemanager,
      filter, gantt, grid, kanban — added 2026-07-19 "per family
      convention", no route ever imported any of them; confirmed by
      `grep -ri svar src/` returning nothing). Removed rather than
      wiring in a grid this entity has no listable resource to justify,
      so the dependency tree matches the §7/§2 "no data-grid dependency"
      claim instead of contradicting it. `pnpm-lock.yaml` regenerated.
- [x] Add vitest unit tests for `ApiClient` and `AuthRepository`
      (`tests/unit/client.test.ts` + `tests/unit/auth.test.ts`, now 19
      tests — grew from the original 16). *(2026-06-13; entity spec
      T-11.)*
- [x] Add a playwright smoke test for the routes
      (`tests/e2e/smoke.spec.ts`, now 9 tests — grew from the original 7;
      `playwright.config.ts` via `vite preview`). Also fixed the scaffold
      artifact in `src/app.html` (meta description named the Course
      Service). *(2026-06-13; entity spec T-11. See the "E2E suite is
      red" entry above — test count grew but pass rate did not keep up.)*
- [x] ~~Cross-origin SSO token handoff (issuer side)~~ **Removed
      2026-08-04** — see above. Original entry (2026-06-13, protocol
      [`jwt-enforcement.md`](../../../agents/share/jwt-enforcement.md))
      is kept below purely as history of what was built and later
      deleted: shared federation key `mxi_access_token`;
      `src/lib/auth/return-to.ts` (`isAllowedReturnTo` / `parseAllowlist`
      / `nextDestination` + sessionStorage helpers); `return_to` parked
      on `/signin`/`/signup` and consumed by `/verify` (fragment delivery
      via `window.location.assign`); `VITE_RETURN_TO_ALLOWLIST` config.
      Tests: `return-to.test.ts` (24) + `session.test.ts` (3, the
      pre-BFF one in `src/lib/auth/`, distinct from today's
      `tests/unit/session.test.ts`) + 2 playwright handoff cases.
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
- [x] **Expanded en/cy → the full 13-locale family catalog** (`es` `fr`
      `de` `ar` `ru` `hi` `zh` `bn` `pt` `id` `ur`, matching the sibling
      front-ends), with RTL support for `ar`/`ur` and a Lily `LocalePicker`
      replacing the plain `<select>` in the top bar. Landed by
      `f66ff50f` (2026-06-18) and `459f8daa`; **this spec, README, index,
      and AGENTS.md still described "bilingual (English + Welsh)" only**
      until this audit (2026-08-04, DOC-4) — corrected in place. Tests:
      `tests/unit/i18n.test.ts` grew to 13, including a full-13-locale
      key-parity assertion and an RTL spot-check.
- [ ] **No unit coverage of the real `src/lib/server/auth.ts` /
      `admin.ts` functions at all** (noticed alongside the dead-code
      finding above, 2026-08-04) — `client.test.ts`/`auth.test.ts` cover
      only the dead `src/lib/api/` layer; `session.test.ts` covers just
      the cookie-parsing helpers. `verifyMagicLink` / `requestMagicLink` /
      `signup` / `exchangeToken` / `currentUser` / `signout` (the
      functions every real route actually calls) have zero direct unit
      tests — only indirect coverage via the e2e suite, which is
      currently red (see above). Add unit tests for these against a
      spied `fetch`, mirroring what `client.test.ts` does for the dead
      layer.
- [ ] Add a component/playwright test for the UI→API locale handoff: that
      `/signin` + `/signup` pass `i18n.locale` through to the server
      action body (currently exercised only indirectly, and only against
      the dead `AuthRepository`, not the live `src/lib/server/auth.ts`
      path — see above). The pure i18n store is unit-covered; the page
      wiring and the real server-side call are not.
- [x] ~~Consider an in-memory token option (vs. `localStorage`) for
      stricter XSS posture.~~ **Resolved (2026-06-17):** obsoleted by the
      BFF/cookie migration — no token lives in the browser at all
      (`authentication-sessions.md` §3, §6); see the BFF task above.

- [x] **AFE-1 (S) Extend the PRO-H10 page-visit guard to
      `/admin/attributes`.** Repo `tasks.md` PRO-H10 (2026-08-29,
      DONE) decided the family's page-visit posture — guard only pages
      whose entire purpose is a mutation, redirect to `/signin` via a
      `requireSignedIn(locals)` helper — and rolled it to
      person/worker/thing/event/course. *(verified:
      `grep -rn "requireSignedIn" src/` in this crate returns nothing;
      `src/routes/admin/attributes/+page.server.ts`'s `load` instead
      returns `{ pid, target: null, error: "Sign in as an admin to
      manage attributes." }` in-page, and its `save` action separately
      does an ad-hoc `if (!locals.sessionId) return fail(401, …)` —
      neither redirects)*. `/admin/attributes` is exactly the
      mutation-page shape PRO-H10 targets (its whole purpose is
      submitting a `PUT`), so it should follow the same pattern as the
      five reference crates rather than its own bespoke in-page error.
      Spec + code + test: add `requireSignedIn` to
      `src/lib/server/session.ts` (or import the pattern), guard the
      `load`, keep the `save` action's existing `401` (form actions
      don't redirect on POST failure the same way), add a unit test.
      **Acceptance:** an anonymous visit to `/admin/attributes` (any
      `?pid=`) redirects to `/signin`; `pnpm test` green.
  - **Resolved (2026-09-06).** `requireSignedIn(locals)` added to
    `src/lib/server/session.ts`, called at the top of
    `/admin/attributes`'s `load` (before the `?pid=` branch, so a bare
    `/admin/attributes` visit redirects too, not just one with a target
    chosen) — removing the in-page "Sign in as an admin..." message
    entirely; the `save` action's existing ad-hoc `401` is untouched, per
    the acceptance note (form actions don't redirect on `POST` failure
    the same way `load` does). One deliberate deviation from the
    reference crates' signature: `requireSignedIn` is declared as a
    TypeScript **assertion function**
    (`asserts locals is App.Locals & { sessionId: string }`) rather than
    plain `void`, because — unlike the reference crates' guarded pages,
    which call it and return immediately — this route's `load` still
    needs `locals.sessionId` narrowed to `string` afterward to pass it to
    `getUserAttributes`; the narrowing is genuine type safety here, not
    decoration copied for its own sake. New `requireSignedIn` unit tests
    in `tests/unit/session.test.ts` (redirects when signed out; passes
    through silently when signed in, mirroring the reference crates'
    own test shape) plus two `tests/e2e/admin-attributes.spec.ts` cases
    (anonymous + `?pid=` redirects to `/signin`; anonymous with no
    `?pid=` at all also redirects) replacing the old in-page-message
    assertion the guard now makes unreachable. `pnpm test` 27/27 (was
    25); `pnpm run check` 0 errors/warnings; `pnpm run test:e2e` 14/14
    (`admin-attributes.spec.ts` up from 4 to 5 tests: one old case
    replaced by two new ones); `pnpm run build` clean.

- [ ] **AFE-2 (M) No UI for the GDPR account-export / erasure rights
      the backend already implements.** The auth service exposes
      `GET /api/auth/account/export` (subject data export),
      `GET /api/auth/account/audit` (subject's own audit trail), and
      `DELETE /api/auth/account` (erasure) — see
      `authentication-service-with-loco/AGENTS.md`'s API-surface table.
      *(verified: this crate's `src/lib/server/auth.ts` exports only
      `verifyMagicLink`/`requestMagicLink`/`signup`/`exchangeToken`/
      `currentUser`/`signout` — no export/audit/erasure call; the
      dashboard `src/routes/+page.svelte` shows name/email/id and a
      sign-out button only, no "download my data" or "delete my
      account" control)*. This is the operator-facing UI for the one
      service in the family whose entire job is a data subject's own
      account, and it offers no way to exercise either right without
      calling the API directly. Add `exportAccount`/`accountAudit`/
      `eraseAccount` to `src/lib/server/auth.ts` (mirroring the
      existing bearer-exchange pattern `admin.ts` uses), a dashboard
      section or `/account/export`+`/account/delete` routes, and a
      confirm-before-erase step (erasure is destructive and
      irreversible per the service's own semantics). Spec (§5 route
      table, §6, §9) + code + tests (unit for the new server calls;
      e2e for the export/erase happy path against the mock server,
      extending `tests/e2e/mock-auth-server.mjs`).
      **Acceptance:** a signed-in user can view/download their export
      and (with confirmation) erase their account from the UI;
      `pnpm test` and `pnpm run test:e2e` green.

- [x] **AFE-3 (S) Zero e2e coverage of `/admin/attributes`.** *(resolved
      2026-09-05.)* This route (its own `load` + `save` server action,
      per §13 AFE-1 above) had no end-to-end coverage at all, unlike
      every other route in `src/routes/`.
  - **Resolved.** `tests/e2e/mock-auth-server.mjs` gained: a second,
    `access=admin` login identity (a new `magic-admin-456` token,
    distinct pid/email from the ordinary `magic-123` login, so the 403
    path is a real ABAC-shaped denial rather than an untested branch);
    session-aware `GET /api/auth/me` (previously hard-coded to the one
    fixture user regardless of which identity signed in); and a new
    `GET`/`PUT /api/auth/admin/users/{pid}/attributes` handler — 401
    with no session, 403 for a signed-in non-admin, 200 otherwise,
    backed by an in-memory `pid -> attributes` map so a `PUT` genuinely
    changes what a subsequent `GET` returns. New
    `tests/e2e/admin-attributes.spec.ts`, four cases: an admin viewing
    an existing (seeded) user's attributes, an admin saving a valid
    change (asserted by the saved value round-tripping back into the
    editor, not just the "saved" banner), a signed-in non-admin caller
    seeing the exact `403` + `description` the mock service returns
    (and never seeing the target's data), and an unauthenticated
    visitor seeing the load function's own sign-in prompt rather than
    any target data. **Acceptance (met):** `pnpm run test:e2e` — 10/10
    passing (6 pre-existing + 4 new), run twice to rule out flakiness;
    `pnpm run check` (svelte-check, 0 errors) and `pnpm test` (vitest,
    17/17) unaffected.

- [x] **AFE-4 (S) Surface the `429` rate-limit response distinctly from
      a generic failure.** *(resolved 2026-09-06.)* The auth service
      rate-limits `signup`/`magic-link` issuance (5 requests / 5 min
      per email, `authentication-service-with-loco/AGENTS.md`) and
      returns `429` over the cap. *(verified: `src/lib/server/auth.ts`'s
      `requestMagicLink`/`signup` returned only `res.ok` — a plain
      boolean — discarding the status code entirely, so
      `src/routes/signin/+page.server.ts`'s action collapsed every
      non-2xx response, `429` included, into the single generic
      `error: "failed"` outcome; no `i18n` key or UI copy distinguished
      "try again in a few minutes" from any other failure.)* Since the
      always-`200` anti-enumeration shape means `429` is the one
      documented non-2xx outcome these two endpoints intentionally
      produce, it was worth a distinct, honest message rather than a
      generic "something went wrong".
  - **Resolved.** `requestMagicLink`/`signup` now return a
    `MagicLinkOutcome` (`"sent" | "rateLimited" | "failed"`) instead of
    a boolean, classifying `429` distinctly from any other non-2xx
    status. Both `signin`/`signup` `+page.server.ts` actions map
    `"rateLimited"` to a new `error: "rate-limited"` outcome, and both
    pages render the new `account.rateLimited` i18n key ("Too many
    requests. Please wait a few minutes and try again.") instead of
    the generic `signin.failed`/`signup.failed` copy when it fires.
    Added across all 13 locales — `tests/unit/i18n.test.ts`'s existing
    full-coverage assertion covers it, no new test needed there.
  - New `tests/unit/auth.test.ts` (8 cases) pins `requestMagicLink`/
    `signup`'s three-way classification and request-body shape
    directly against a mocked `fetch`. `tests/e2e/mock-auth-server.mjs`
    gained a `RATE_LIMITED_EMAIL` fixture returning `429` from both
    endpoints; two new `tests/e2e/smoke.spec.ts` cases (one per page)
    submit that email and assert the distinct message renders (and the
    generic one does not) — verified to fail with the BFF/page changes
    reverted and pass with them restored.
  - **Acceptance met:** `pnpm test` 25/25 (was 17); `pnpm run check`
    clean; `pnpm exec playwright test` 13/13 (was 11); `pnpm run lint`
    clean save for two pre-existing, untouched files
    (`src/lib/server/admin.ts`,
    `src/routes/admin/attributes/+page.server.ts`).
      **Acceptance:** a stubbed `429` response in a unit test produces
      the distinct rate-limited UI state, not the generic failure one;
      `pnpm test` (incl. the 13-locale key-parity check) green.

- [x] **T-12: `/verify` crashed with a raw 500 when the authentication service was unreachable.** *(resolved 2026-09-06.)* `src/routes/verify/+page.server.ts` called `await verifyMagicLink(fetch, token)` with no `try`/`catch`. A network-level failure (the authentication service unreachable, timed out, connection reset) makes `fetch` throw rather than resolve — uncaught, that propagated out of `load` and SvelteKit rendered its generic 500 error page instead of this route's own friendly UI. The same bug class was found and fixed first in `place-front-end-with-svelte` (T-26) and `thing-front-end-with-svelte` (T-23); ported here.
  - **Resolved.** A `try`/`catch` around the call, a new `"serviceUnavailable"` error variant, and its message (all 13 locales) in `+page.svelte`.
  - **Acceptance:** unlike the other front-ends, this crate already had a real stub-auth-server e2e harness (`tests/e2e/mock-auth-server.mjs`), so the fix is pinned there: a new `magic-network-error` token makes the stub reset the connection, and a new Playwright test in `tests/e2e/smoke.spec.ts` asserts the friendly message renders with a `200` response rather than a raw 500 — verified to fail with the `try`/`catch` reverted and pass with it restored. Three-part change: spec (here) + code + test.

## 14. Implementation status

Shipped (v0.1, prior bearer-token SPA model, since fully removed): all
four routes, lean client, repository, runes session, SPA config,
bilingual (en/cy) i18n, and the cross-origin token handoff.

**Re-spec'd 2026-06-17** to the httpOnly-cookie + BFF model
(`authentication-sessions.md`); **the BFF is fully landed and is the
only auth path in the live app**: `hooks.server.ts` + `+layout.server.ts`
+ `+page.server.ts` handle session/CSRF cookies, the `/token` exchange,
and every mutating action; `mxi_access_token` / the `localStorage` token
/ the `#access_token=` fragment handoff / the cross-origin `return_to`
redirect are all removed, the last of these only corrected in the docs
by this audit (2026-08-04, DOC-4) though it was code-removed back in
`f66ff50f`. i18n has grown from bilingual to the full 13-locale family
catalog (§13). ABAC attribute admin (`/admin/attributes`) landed
2026-07-19.

**Gaps discovered by the 2026-08-04 audit were closed 2026-08-29
(PRO-P24):** the `tests/e2e/smoke.spec.ts` suite is now green (6/6) —
`AUTH_API_URL` points at a real Node HTTP mock server instead of relying
on browser-side `page.route()` stubs the BFF's server-side `fetch` calls
were never visible to (§11); `src/lib/api/{client,auth}.ts` and
`src/lib/config.ts` (dead code kept alive only by their own 19 unit
tests) are deleted; the six unused `@svar-ui/*` packages are removed
from `package.json`, reconciling the dependency tree with the
"no data-grid dependency" claim (§7). `pnpm check` / `pnpm test` /
`pnpm build` / `pnpm test:e2e` are all green.

## 15. Roadmap

v0.1 (shipped, since removed): functional magic-link UI (bearer-token
SPA). v0.2 (shipped): **BFF + httpOnly-cookie session migration**
(`authentication-sessions.md`) — CSRF landed 2026-07-19; the
cross-origin `return_to` handoff and the pre-BFF `src/lib/api/` client
were meant to be restated, not silently orphaned — see §13. v0.3
(shipped 2026-08-29, PRO-P24): repaired the e2e suite (§11/§13), deleted
the dead `src/lib/api/{client,auth}.ts` + `config.ts`, and removed the
six unused `@svar-ui/*` packages; shared-nav integration with sibling
front-ends is otherwise complete (all now session-cookie + BFF based, no
shared client token).

## 16. Open questions

- ~~`localStorage` vs in-memory token storage (XSS vs UX/refresh
  tradeoff)?~~ **Resolved (2026-06-17):** neither — the credential is the
  httpOnly session cookie; no token lives in the browser
  (`authentication-sessions.md` §3, §6).
- ~~**CSRF token transport** — double-submit cookie vs. synchroniser
  token in the BFF payload?~~ **Resolved (2026-07-19):** a synchroniser
  token, issued by the auth service at verify and re-hosted httpOnly by
  the BFF (§10, §13; `src/lib/server/session.ts`).
- Should the dashboard auto-refresh `/me` periodically, or only on the
  layout server load (currently: only on load/navigation)?
- ~~Where does post-login redirect target live (deep-link return URL)?~~
  **Resolved (2026-06-13; updated 2026-06-17); superseded 2026-08-04:**
  the cross-origin `?return_to=` handoff this answer described was
  **removed entirely** in `f66ff50f` (2026-06-18), not merely made
  credential-free as this entry (written the same window) implied.
  `/verify` unconditionally redirects to `/`; there is no deep-link
  return URL of any kind today. See §13 for the removal and
  [`authentication-sessions.md`](../../../agents/share/authentication-sessions.md)
  §6 for why each sibling front-end no longer needs one (its own BFF, its
  own session, its own `/signin`).

## 17. References

- [`authentication-sessions.md`](../../../agents/share/authentication-sessions.md)
  — canonical session / cookie / CSRF / BFF design (single source of
  truth; supersedes the RS256-JWT + JWKS bearer model).
- Sibling service spec (link above); SvelteKit + Svelte 5 runes docs.

## 18. Change control

Update this spec in the same PR as any behavioural change; bump
`CHANGELOG.md` under `[Unreleased]`.
