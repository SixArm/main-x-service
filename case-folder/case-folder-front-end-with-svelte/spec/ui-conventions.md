# UI conventions

> Part of the [Svelte edition specification](index.md).

- **NHS Number anywhere on screen:** `.nhs-number` (monospace, bold).
- **Status badges:**
  - `in-cabinet` → `Badge type="success"`
  - `in-transit` → `Badge type="warning"`
- **Cabinet utilisation badges:**
  - `> 85%` → `error`
  - `> 60%` → `warning`
  - otherwise → `success`
  - unknown capacity → `default` (`—`)
- **All forms** use `Lily.Form` (auto-prevents default submit) +
  `Lily.Field` for label / hint / error wiring.
- **Success after a write** is announced with `Alert type="success"`.
- **Skip link** is the first focusable element on every page.
- **Chrome utility row** above the site `<Header>` carries the Lily
  `LocalePicker` and `ThemePicker`. Both persist to `localStorage` under
  `case-folder:locale` and `case-folder:theme`.

## Navigation & layout

Cross-cutting UI rule for every `*-front-end-with-svelte` app:

- Global navigation MUST be a **top navigation bar** (the site `<Header>`)
  spanning the full viewport width. There MUST NOT be a left-hand
  navigation sidebar / rail.
- On narrow viewports the top-bar navigation MUST collapse behind a
  **hamburger menu** toggle.
- The main content area MUST be **full-width** — never inset by a
  persistent side-navigation column.

## Theming

The app uses the **full shared Lily/DaisyUI theme catalogue** (~41 themes,
including the NHS England/Scotland/Wales patient & practitioner themes), for
parity with the rest of the `*-front-end-with-svelte` family. The theme
stylesheets are served from `static/assets/themes/`, a **symlink** to the
shared design-system themes (`~/git/lilydesignsystem/lily-design-system/themes`).
The default is `united-kingdom-national-health-service-england-for-practitioners`.

`ThemePicker` (from `lily-design-system-svelte-theme-picker`) manages exactly one
`<link rel="stylesheet" data-lily-theme-picker="theme">` in
`document.head` and toggles the active theme by mutating its `href`
(`/assets/themes/<slug>.css`) and the `data-theme` attribute on `<html>`.

Each Lily theme stylesheet defines the DaisyUI `--color-*` tokens. So that
they actually restyle this NHS app, `src/lib/css/nhs.css` **bridges** the base
`--nhs-*` colour tokens onto the active theme's `--color-*` (with the NHS
values as fallbacks for first paint). Theme-invariant tokens (typography,
spacing, layout) stay in `src/lib/css/nhs.css` under `:root`.

To add a theme: add it upstream in the shared design-system themes repo and
extend the `themes` array in `+layout.svelte`. (The previous app-local
`static/themes/nhs*.css` files were dropped in favour of the shared catalogue.)

**Lily helpers come from the sibling repo.** `lily-design-system-svelte-locale-picker` and
`lily-design-system-svelte-theme-picker` are declared as `file:` dependencies in
`package.json` pointing at the sibling helper repo at
`~/git/lilydesignsystem/lily-design-system/lily-design-system-svelte-helpers/`;
`npm install` symlinks them into `node_modules` and they are imported by
package name. The sibling repo must exist for install/dev/build/check. Don't vendor them in;
don't add a fallback path — fail loudly if the sibling is missing.

## Locale

`LocalePicker` (from `lily-design-system-svelte-locale-picker`) sets `lang` and `dir` on
`<html>`. Available locales: `en` (default), `cy` (Cymraeg), `gd`
(Gàidhlig). All three are LTR; `dir` will become meaningful if an RTL
locale (e.g. `ar`, `ur`) is added later — the helper detects RTL
automatically.

The picker does **not** translate UI strings — there is no i18n
catalogue. The current value is published via `lang` so screen readers
select the right voice; that's the only behavioural effect today.
