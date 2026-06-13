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

## Theming

Colour tokens live in `static/themes/<slug>.css`, each scoped to
`:root[data-theme="<slug>"]`. Theme-invariant tokens (typography,
spacing, layout) stay in `src/lib/css/nhs.css` under `:root`.

`ThemePicker` (from `@lily/theme-picker`) manages exactly one
`<link rel="stylesheet" data-lily-theme-picker="theme">` in
`document.head` and toggles the active theme by mutating its `href` and
the `data-theme` attribute on `<html>`. Available themes:

| Slug                 | Purpose                                          |
| -------------------- | ------------------------------------------------ |
| `nhs`                | NHS UK design tokens (default).                  |
| `nhs-high-contrast`  | Pure black/white + yellow focus for low-vision.  |

To add a theme: drop a `static/themes/<slug>.css` file that defines the
same `--nhs-*` colour tokens under `:root[data-theme="<slug>"]`, then
extend the `themes` array in `+layout.svelte`.

**Lily helpers come from the sibling repo.** `@lily/locale-picker` and
`@lily/theme-picker` are not npm dependencies; they resolve via
SvelteKit `kit.alias` to source files in
`~/git/lilydesignsystem/lily-design-system/lily-design-system-svelte-helpers/`.
The sibling repo must exist for dev/build/check. Don't vendor them in;
don't add a fallback path — fail loudly if the sibling is missing.

## Locale

`LocalePicker` (from `@lily/locale-picker`) sets `lang` and `dir` on
`<html>`. Available locales: `en` (default), `cy` (Cymraeg), `gd`
(Gàidhlig). All three are LTR; `dir` will become meaningful if an RTL
locale (e.g. `ar`, `ur`) is added later — the helper detects RTL
automatically.

The picker does **not** translate UI strings — there is no i18n
catalogue. The current value is published via `lang` so screen readers
select the right voice; that's the only behavioural effect today.
