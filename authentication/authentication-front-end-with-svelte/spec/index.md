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

> `src/lib/api/{client,auth}.ts` (`ApiClient` / `AuthRepository`) are
> **not** part of this layering — no route imports them. They are the
> pre-BFF client-held-token model's HTTP layer, kept alive only by their
> own unit tests. See §13.

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

> The e2e suite below is currently **failing 5 of 9 cases** — a real,
> live-verified gap, not a doc-only staleness issue. See the bullet below
> and §13.

- **Unit (vitest, `tests/unit/`, 36 tests across 5 files, all passing):**
  `client.test.ts` (9) and `auth.test.ts` (10) pin `ApiClient` request
  shaping and `AuthRepository` path/verb/body construction — but these
  exercise the **dead** `src/lib/api/` layer (§8, §13), not the live BFF
  path; `session.test.ts` (3) pins the BFF cookie-parsing helpers
  (`src/lib/server/session.ts`); `i18n.test.ts` (13) covers `translate`/
  `t` lookups, the fallback chain, full 13-locale key-parity, RTL
  detection, and region-subtag reduction (`cy-GB` → `cy`); `layout.test.ts`
  (1) is a smoke import check.
- **E2E (playwright, `tests/e2e/smoke.spec.ts`, 9 cases, 4 passing / 5
  failing):** stubs `**/api/auth/**` via `page.route()`, which only
  intercepts requests the **browser** issues. Since the BFF migration
  moved every auth-service call server-side (Node `fetch` against
  `AUTH_API_URL`), the stub no longer engages for any of them — the five
  failing cases are exactly the ones whose assertions depend on a stubbed
  auth-service response (sign-in submit, verify success, both `return_to`
  cases — which also assert removed behaviour — and the signed-in
  dashboard via a `localStorage`-seeded session, itself a removed
  mechanism). The four passing cases are the ones that render without any
  auth-service round trip (sign-up/sign-in form render, verify's
  missing-token error, signed-out home). Fixing this needs either a
  Node-level fetch intercept (e.g. pointing `AUTH_API_URL` at a stub HTTP
  server started per-test) or a real auth-service instance; see §13.

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
- [ ] **E2E suite is red (5/9), discovered 2026-08-04 (this audit).**
      `tests/e2e/smoke.spec.ts`'s `page.route()` stubs cannot intercept
      the BFF's server-side `fetch` calls (§11) — this has been true
      since `f66ff50f` moved those calls server-side, so the suite has
      likely been silently broken since 2026-06-17 (not introduced by
      this audit; `git log` shows no touching commit since). Needs either
      a Node-level fetch intercept (stub `AUTH_API_URL` with a real HTTP
      server per test) or running against a live auth-service; also drop
      the two cases asserting the removed `return_to` handoff and the
      `localStorage`-seeded-session case (§13 above; sessions are cookies
      now).
- [ ] **`src/lib/api/{client,auth}.ts` + `src/lib/config.ts`'s
      `PUBLIC_API_BASE_URL`/`VITE_RETURN_TO_ALLOWLIST` are dead,
      discovered 2026-08-04 (this audit).** No route imports them; their
      only callers are their own unit tests (`client.test.ts`,
      `auth.test.ts`, 19 tests). Decide: delete them (and retire/rewrite
      those 19 tests against the real `src/lib/server/` layer instead),
      or repurpose them as the Node-level e2e fetch-intercept fixture the
      item above needs. Flagged rather than silently deleted — this is a
      code decision, not a doc fix.
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

**Known gaps, both discovered by this audit (2026-08-04):** the
`tests/e2e/smoke.spec.ts` suite is red (4/9 passing) because its
browser-side `page.route()` stubs cannot see the BFF's server-side
`fetch` calls (§11); `src/lib/api/{client,auth}.ts` (+ `config.ts`'s
`PUBLIC_API_BASE_URL`/`VITE_RETURN_TO_ALLOWLIST`) are dead code kept
alive only by their own 19 unit tests. Neither blocks `pnpm check` /
`pnpm test` / `pnpm build`, all of which are green.

## 15. Roadmap

v0.1 (shipped, since removed): functional magic-link UI (bearer-token
SPA). v0.2 (shipped): **BFF + httpOnly-cookie session migration**
(`authentication-sessions.md`) — CSRF landed 2026-07-19; the
cross-origin `return_to` handoff and the pre-BFF `src/lib/api/` client
were meant to be restated, not silently orphaned — see §13. v0.3: repair
the e2e suite (§11/§13) and resolve the `src/lib/api/` dead-code
question; shared-nav integration with sibling front-ends is otherwise
complete (all now session-cookie + BFF based, no shared client token).

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
