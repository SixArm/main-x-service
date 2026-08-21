# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md) — single source of truth;
> [README.md](./README.md) — user-facing intro; [AGENTS.md](./AGENTS.md) — agent guide.

## [Unreleased]

### Added

- 2026-07-19 — SVAR component seams: **@svar-ui/svelte-calendar**,
  **@svar-ui/svelte-kanban**, **@svar-ui/svelte-gantt**, and
  **@svar-ui/svelte-filemanager** are installed (no routes yet —
  candidate features are catalogued per project; see the roadmap).

- 2026-07-19 — SVAR DataGrid + Filter: **@svar-ui/svelte-grid** and **@svar-ui/svelte-filter** are
  installed per the family convention; the auth service exposes no
  listable resource, so no grid route is mounted (deps ready for
  when one lands).

- 2026-07-19 — Lily Design System: the hand-rolled locale `<select>` is replaced by the Lily
  **LocaleSelect** (wired to the i18n store: `value={i18n.locale}`,
  `onChange → i18n.set`, `applyDir` off — the app keeps its own
  `lang`/`dir` effect), and the **Lily headless** component library
  is now a dependency alongside the existing ThemeSelect.

- **Operator UI for ABAC attribute assignment** (`/admin/attributes`).
  Pick a user by pid, view their ABAC subject attributes, edit the JSON
  map, and Save (PUT) — all through the BFF (`src/lib/server/admin.ts`
  exchanges the session for a PASETO and calls the auth service's admin
  API). The admin API requires the signed-in operator to carry
  `access=admin`; a `403` is surfaced in the UI. Sending `{}` clears all
  attributes. DB-free unit test for the new cookie parsing.
- **CSRF synchroniser-token plumbing through the BFF.** The auth
  service's `POST /token` now requires the session's CSRF token in the
  `X-CSRF-Token` header (`authentication-sessions.md` §4). The BFF now
  captures the `__Host-mxi_csrf` token at verify, re-hosts it as an
  httpOnly cookie on its own origin (`hooks.server.ts` → `locals.csrfToken`),
  and echoes it on every `/token` exchange (`server/auth.ts`). Without
  this the whole BFF (`/me`, sign-out, admin) would have broken on `403`.
  Browser↔BFF CSRF stays SvelteKit's native form-action origin check.

### Changed

- **Re-spec to httpOnly-cookie + BFF session model (2026-06-17).** Adopted
  the canonical design doc
  [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
  (single source of truth). **Supersedes the prior bearer-token SPA
  model.** Spec/docs only in this entry; the code follow-up is tracked in
  spec §13.
  - **No token in the browser.** Login now establishes a **server-side
    session**; the browser holds only the httpOnly Secure `SameSite=Lax`
    `__Host-mxi_session` cookie (JS never reads a credential). The
    `localStorage` access token (`mxi.auth.token` / `mxi.auth.user`), the
    shared federation key `mxi_access_token` / `FEDERATION_TOKEN_KEY`, and
    the cross-origin `#access_token=` URL-fragment handoff are **removed**.
  - **BFF pattern.** The SvelteKit **server** (`hooks.server.ts` /
    `+page.server.ts` / `+server.ts`) holds the session and is the only
    party that calls the auth service; magic-link verify is server-side
    and the auth service sets the session cookie (relayed to the browser).
    Mutating browser→BFF calls add **CSRF** protection
    (`authentication-sessions.md` §4).
  - **Magic-link UX unchanged** as the flow; only the outcome changed (a
    session cookie, not a stored token). Sign-out now revokes the session
    server-side and clears the cookie.
  - `return_to` redirect keeps the allowlist as an **open-redirect**
    control but carries **no credential** (plain navigation; the target
    relies on its own session cookie).
  - Spec sections touched: §1–§2 (intro/scope), §4 (glossary), §5–§6
    (information architecture + functional requirements: session via
    cookie, server-side auth, CSRF), §7–§12 (NFR/architecture/API/
    persistence/testing/compliance), §13 (BFF code-follow-up task + the
    supersede note), §14–§17.

### Added

- **Bilingual UI (English + Welsh / Cymraeg).** A dependency-free i18n
  store (`src/lib/i18n.svelte.ts`): a per-locale strings catalog, a
  reactive `$state` current-locale persisted to
  `localStorage["mxi.auth.locale"]`, a `t(key)` accessor, a fallback
  chain (target → English → the key), and region-subtag reduction
  (`cy-GB` → `cy`). A `<select>` language switcher in `+layout.svelte`
  re-renders every string live. The chosen locale is sent as the optional
  `locale` field on signup / magic-link requests so the magic-link **email
  language matches the UI**; it drops out of the body when unset (service
  defaults to English). Welsh is a deliberate UK public-sector
  Welsh-language-duty choice, mirroring the service catalog (`src/i18n.rs`).
  Tests: `tests/unit/i18n.test.ts` (9) plus `locale`-body assertions in
  `auth.test.ts`.
- **Doc harmonization (2026-06-15).** Brought `spec/index.md`, `README.md`,
  `index.md`, and this changelog into agreement with the shipped i18n
  feature and the cross-origin handoff: §2 scope, §4 glossary, §6
  FR (renumbered sequentially 1–6 and `locale`/`is_verified` documented),
  §7 NFR, §9 request bodies, §10 the `mxi.auth.locale` key, and §11 test
  itemization (now reconciled to 56 vitest + 9 playwright). Added a
  `setUser()` persistence test (`session.test.ts`) and a worked
  cross-origin-handoff example to `index.md`.

- **Cross-origin SSO token handoff (issuer side).** This front-end is now
  the issuer in the first-party, OAuth-implicit-shaped token handoff (see
  `agents/share/jwt-enforcement.md`).
  - **Federation key.** `session.start()` mirrors the issued access token
    to the shared `localStorage["mxi_access_token"]` key (in addition to
    the back-compat `mxi.auth.token` / `mxi.auth.user`); `clear()` removes
    it. Key name exported as `FEDERATION_TOKEN_KEY`.
  - **`return_to` allowlist (`src/lib/auth/return-to.ts`).** Pure helpers
    `isAllowedReturnTo(returnTo, allowlist, selfOrigin)` (absolute http(s)
    + origin exactly in allowlist or equal to self — rejects
    `javascript:` / `data:` / relative / cross-origin / garbage),
    `parseAllowlist(env)`, `nextDestination(returnTo, token, …)` (the pure
    redirect decision: `{kind:"external",url}` with the token in the URL
    fragment, or `{kind:"home"}`), plus `sessionStorage` persist / read /
    clear. The allowlist is the control that stops token exfiltration via
    a crafted `return_to`.
  - **Round-trip.** `/signin` + `/signup` park an allowlisted
    `?return_to=` in `sessionStorage["mxi_return_to"]` (the emailed link
    carries no `return_to`); `/verify` consumes it after a successful
    sign-in and redirects to `return_to#access_token=<jwt>` via
    `window.location.assign` (cross-origin, not SvelteKit `goto`).
  - Tests: `tests/unit/return-to.test.ts` (24) + `tests/unit/session.test.ts`
    (3); 2 new playwright handoff cases (allowlisted → fragment redirect;
    non-allowlisted → home, no token). Playwright 7 → 9.
    `pnpm run check` clean. (Vitest reached 52 here; the i18n suite and
    the `locale`/`setUser` assertions above bring the current total to 56.)

- **Inaugural scaffold (v0.1.0).** SvelteKit 2 / Svelte 5 (runes) SPA
  for the Authentication Service.
  - Routes: `/` (account + sign out), `/signup`, `/signin`, and
    `/verify` (consumes `?token=` → stores the access token → redirects).
  - Lean API client (`src/lib/api/client.ts`) matched to the loco
    service's **raw JSON** responses (no `{success,data,error}`
    envelope), with bearer-token support and an `ApiError` type.
  - `AuthRepository` wrapping signup / magic-link / verify / me /
    signout; `LoginResponse` / `CurrentUser` types mirroring the service
    views.
  - Runes-based client session (`src/lib/auth/session.svelte.ts`):
    access token + cached profile persisted to `localStorage`.
  - SPA mode via `+layout.ts` (`ssr = false`, `prerender = false`);
    `adapter-auto`.
  - Deliberately dependency-light: no SVAR DataGrid and no Lily Design
    System (this UI has no tables) — accepted drift from the sibling
    front-ends. `pnpm run check` clean; production build succeeds.
- **Test suites (T-11).** vitest unit tests
  (`tests/unit/client.test.ts` + `tests/unit/auth.test.ts`, 16) covering
  the `ApiClient` (URL join, JSON body, bearer-token attachment for
  `/me` + `/signout`, empty-body handling, `ApiError` classification,
  non-JSON fallback) and the `AuthRepository` (exact path/verb/body for
  signup, magic-link request, verify with URL-encoded token, `me`,
  signout); playwright smoke tests (`tests/e2e/smoke.spec.ts`, 7) that
  stub the auth API and load every route (sign-up / sign-in / verify /
  signed-in + signed-out home). `playwright.config.ts` runs against
  `vite preview`.

### Fixed

- Prettier formatting drift across `src/` (left behind by recent
  BFF/auth-era edits) broke the `pnpm lint` (`prettier --check src`)
  gate. Reformatted with `pnpm format`; no behavioural change —
  `svelte-check` and the vitest suite are unchanged and green.
- `src/app.html` meta description named the *Course Service* — corrected
  to the Authentication Service (scaffold copy-paste artifact).

### Configuration

- `AUTH_API_URL` (default `http://localhost:5150`): auth service REST
  base URL, read **server-side only** by the BFF (`src/lib/server/auth.ts`,
  `src/lib/server/admin.ts`). This is the var that configures the running
  app — corrected 2026-08-04 (DOC-4); previously documented as
  `PUBLIC_API_BASE_URL`, which is not read by any live route (see below).
- `PUBLIC_API_BASE_URL` / `VITE_RETURN_TO_ALLOWLIST` — **dead**, feed only
  the disconnected `src/lib/api/{client,auth}.ts` pre-BFF layer, which no
  route imports (only its own unit tests do). The `return_to` handoff
  `VITE_RETURN_TO_ALLOWLIST` fed is itself removed (`f66ff50f`,
  2026-06-18) — see the docs-audit entry below.

### Documentation (2026-08-04, DOC-4 audit)

- Corrected `.env.example`, `README.md`, `AGENTS.md`, and `spec/index.md`
  to document `AUTH_API_URL` (the real, server-only var the live BFF
  reads) instead of `PUBLIC_API_BASE_URL` (unused by any route).
- Removed/corrected extensive stale documentation of the cross-origin
  `return_to` handoff (spec §5/§6/§10/§11/§13/§16, `README.md`,
  `index.md`): the feature — and its implementing file
  `src/lib/auth/return-to.ts` — was fully deleted in `f66ff50f`
  (2026-06-18), but the docs kept describing it as live for the ~7 weeks
  since. `/verify` now documented accurately: always redirects to `/`.
- Documented that the UI ships the full family-standard **13-locale**
  catalog (`spec/index.md` §4/§7, `README.md`, `AGENTS.md`, `index.md`),
  not just English + Welsh as previously written — the expansion landed
  in code back in `f66ff50f`/`459f8daa` but was never reflected in prose.
- Documented, rather than silently fixed, two real gaps found live: (1)
  `tests/e2e/smoke.spec.ts` fails 5 of 9 cases because its browser-side
  `page.route()` stubs cannot intercept the BFF's server-side `fetch`
  calls — a gap dating to the same BFF migration, not introduced here;
  (2) `src/lib/api/{client,auth}.ts` is dead code (no route imports it,
  only its own 19 unit tests do). Both recorded in `spec/index.md`
  §11/§13/§14 as open work, not fixed in this pass.
- Fixed `agents/share/…` link casing to `agents/share/…` across `spec/`,
  `README.md`, `AGENTS.md`, `index.md`, `CHANGELOG.md` — this crate was
  the only one of the 11 front-ends using the uppercase form (431 other
  references repo-wide use lowercase; git tracks the directory itself as
  `AGENTS`, but case-insensitive local checkouts masked the mismatch).
- Added the missing `## 11. Testing` heading to `spec/index.md` — the
  testing content existed but had no section header, leaving §10
  (Persistence) run straight into it and the numbering jump from §10 to
  §12 unexplained.
