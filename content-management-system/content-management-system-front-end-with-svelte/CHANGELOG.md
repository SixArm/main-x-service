# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed — `/verify` crashed with a raw 500 when the authentication service was unreachable (CMS-T27)

`src/routes/verify/+page.server.ts` called `await verifyMagicLink(fetch,
token)` with no `try`/`catch`. A network-level failure (the
authentication service unreachable, timed out, connection reset) makes
`fetch` throw rather than resolve — uncaught, that propagated out of
`load` and SvelteKit rendered its generic 500 error page instead of
this route's own friendly UI. The same bug class was found and fixed
first in `place-front-end-with-svelte` (T-26) and
`thing-front-end-with-svelte` (T-23); ported here: a `try`/`catch`
around the call, a new `"serviceUnavailable"` error variant, and its
message in `+page.svelte`. New `tests/unit/verify.test.ts` unit-tests
the `load` function directly (missing token / service unavailable /
invalid token), verified to fail with the `try`/`catch` reverted and
pass with it restored. See spec §13 CMS-T27.

### Added — root sign-in gate (CMS-T31)

No `+layout.server.ts` existed anywhere under `src/routes`, so a
visitor with no session reached every authoring/asset/workflow view
and only discovered they were signed out once an API call silently
failed through the BFF proxy. Ported the identical WPM-T38/CRM-T26 fix
(same underlying architecture): new root `src/routes/+layout.server.ts`
redirects to `/signin` (303) when `locals.sessionId` is `null`,
excluding `/signin`/`/verify`. `/preview/[pid]/[locale]` needed no
exclusion — it's a `+server.ts` endpoint, not a page, so this layout's
`load` never runs for it. `dashboard.spec.ts` and `entries.spec.ts`
each gained a small `signIn()` + `test.beforeEach`; a new
`tests/e2e/sign-in-gate.spec.ts` (no session, the opposite of the other
two) proves the redirect. See `../spec/tasks.md` CMS-T31.

### Added

- 2026-07-31 — CMS-T26 views: the authoring UI. Entry list with
  content-type and key filters; **entry detail** with the locale
  matrix (status, what is live, staleness as "N source revisions
  behind"), the structured **block editor**, revision history with
  compare and restore, workflow transitions with reasons, the publish
  gate showing each blocker's rule *and* remedy, and a server-side
  preview panel; asset library with the alt-text gate and orphan
  reporting; workflow backlog and schedule queue; translation
  dashboard with locale coverage; insights rendering every health rule
  and honest ratios; and read-only site settings. New pure modules
  `$lib/blocks` (block model operations) and `$lib/format`
  (percent / duration / bytes / staleness), plus `BlockEditor`,
  `StateBadge`, and `SitePicker` components. i18n grew to **82 keys ×
  13 locales**. 28 vitest tests + 7 Playwright specs.
  - **A `409` renders a comparison, never a retry button** — a retry
    would silently discard whoever saved first.
  - **No `{@html}`, no HTML serialization, and no `toHtml` helper.**
    The editor manipulates blocks; a serializer would get used, and
    the round trip would stop being lossless.
  - **Staleness keeps three outcomes** — up to date, N behind, and
    *unknown* — because collapsing "unknown" into "up to date" tells a
    translator their page is fine when nobody knows.

- 2026-07-31 — CMS-T25 scaffold: the app is real. SvelteKit 2 +
  Svelte 5 runes SPA, TypeScript strict, a same-origin **BFF proxy**
  that exchanges the httpOnly session cookie for a short-lived PASETO
  server-side, the magic-link sign-in/verify/sign-out flow, a typed
  CMS API client whose paths were checked against the running
  service's OpenAPI document, **13-locale i18n** (with the parity
  test), the Lily locale/theme pickers, and a dashboard that renders
  content health and the backlog straight from the API.
  - **The proxy refuses two paths on purpose.**
    `POST …/variants/{locale}/preview` returns a raw preview token
    and `/api/preview-tokens/…` manages them; forwarding either would
    put a credential that renders unpublished content into browser
    JavaScript. The app's own `/preview/{pid}/{locale}` route mints
    the token, spends it, revokes it, and returns only the render.
  - Lily's helper packages were **renamed upstream**
    (`*-select` → `*-picker`); this front-end uses the new names,
    which is why copy-adapting a sibling's `package.json` today fails
    to install.
  - `localStorage` access is guarded: `browser` being true does not
    mean storage works (Safari private mode throws), and this runs in
    a module-level constructor, so an unguarded read took the whole
    app down rather than just the locale switcher.
  - 12 vitest tests + 2 Playwright specs; `svelte-check` clean;
    verified live end to end against the seeded service — including
    the proxy's `403` and a preview round trip that left no token in
    the response and a revoked token in the database.

- 2026-07-30 — CMS-T0 specification round: the cross-cutting spec
  (`../spec/`) and this edition's doc scaffold. No code yet; this
  edition is CMS-T25/T26 in the queue.

### Fixed

- 2026-07-31 — the API client did a `GET` on
  `/api/entries/{pid}/variants`, which the service serves for `POST`
  only; the T25 path check had compared paths without their methods,
  so it looked verified. There is no variants listing — the entry read
  returns them — and the check now compares verbs too.
- 2026-07-31 — `listRevisions` and `publishCheck` declared the wrong
  response types (the history endpoint returns summaries without
  bodies).
- 2026-07-31 — the insights view keyed an `{#each}` on rule + subject,
  and two `broken_reference` findings on one page share both. The
  duplicate key crashed the whole view. Only a live render against
  real data showed it; the stubbed spec had one finding per rule.

