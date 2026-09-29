# ThemePicker (Svelte helper)

A reusable, headless Svelte 5 theme picker — an **icon button that
opens a WAI-ARIA APG listbox** — that **loads themes dynamically at
runtime** from a developer-specified directory.

The single source of truth is [spec/index.md](./spec/index.md). This file is the
human-readable guide. For topic deep-dives see
[docs/](./docs/) and for working code see [examples/](./examples/).

## Table of contents

- [Why this exists](#why-this-exists)
- [Install](#install)
- [Quick start](#quick-start)
- [How it works](#how-it-works)
- [Rendered markup](#rendered-markup)
- [Keyboard](#keyboard)
- [Default theme](#default-theme)
- [Props](#props)
- [Custom button rendering](#custom-button-rendering)
- [Persistence](#persistence)
- [Accessibility](#accessibility)
- [SSR and hydration](#ssr-and-hydration)
- [Preloading for zero-flicker switching](#preloading-for-zero-flicker-switching)
- [Multiple pickers in one app](#multiple-pickers-in-one-app)
- [Recipes](#recipes)
- [Troubleshooting](#troubleshooting)
- [Testing](#testing)

## Why this exists

Most theme selects couple selection, persistence, and styling into one
opinionated widget. This one splits the contract cleanly:

- **Authors** drop theme CSS files (e.g. `light.css`, `dark.css`) into
  a directory served by the app.
- **This component** owns selection, dynamic loading, persistence, and
  accessibility.
- **Consumers** own the visual style of the select via the
  `theme-picker` class hook.

The result is a small reusable widget that works in any Svelte 5 host
(SvelteKit, plain Vite, Astro, Storybook) and against any theme
catalog — Lily™'s 41 DaisyUI-inspired themes, NHS-aligned themes, or
your own bespoke set.

## Install

The directory is published as a folder-style import. Consumers either
copy it into their project or wire it as a workspace dependency. The
only runtime dependency is `svelte` ≥ 5.

```ts
import ThemePicker from "./lily-design-system-svelte-theme-picker/ThemePicker.svelte";
// or via the barrel:
import { ThemePicker } from "./lily-design-system-svelte-theme-picker";
import type {
  Props,
  ChildArgs,
} from "./lily-design-system-svelte-theme-picker";
```

## Quick start

1. Drop theme CSS files into a directory served by your app, e.g.
   `static/assets/themes/light.css`, `static/assets/themes/dark.css`.
   Each theme scopes its tokens to `:root[data-theme="<slug>"]`
   (the convention every Lily theme uses).
2. Render the select, pointing it at the directory and listing the
   available slugs.

```svelte
<script lang="ts">
  import ThemePicker, { themeName } from "./lily-design-system-svelte-theme-picker/ThemePicker.svelte";

  let theme = $state("");
</script>

<ThemePicker
  label="Theme"
  themesUrl="/assets/themes/"
  themes={["light", "dark", "abyss"]}
  bind:value={theme}
  storageKey="lily-theme"
/>

<p class="theme-picker-status" aria-live="polite">
  Active theme: {themeName(theme)}
</p>
```

3. **Style the listbox.** The package ships zero CSS, and the popup
   needs positioning or it will render in normal flow and shove the
   page down when it opens. See
   [docs/styling.md § Positioning the listbox](./docs/styling.md#positioning-the-listbox).

`themeName` is the same function the options use, so the status line
and the control cannot drift apart.

The status line is recommended, though no longer strictly compensatory.
The listbox marks the active option with `aria-selected="true"`, so a
user who opens the control does hear which theme is current. But the
_closed_ control is one glyph, so nothing on the page states the active
theme unless you state it. `aria-live="polite"` announces mutations
only, so it stays silent on first paint and speaks once per change.
Full reasoning and when to omit it:
[docs/accessibility.md](./docs/accessibility.md#the-status-region).

When the user picks `dark`, the component:

- swaps a managed `<link rel="stylesheet">` in `<head>` to
  `/assets/themes/dark.css`,
- sets `data-theme="dark"` on `<html>`,
- writes `"dark"` to `localStorage["lily-theme"]`,
- calls the optional `onChange("dark")` callback.

## How it works

On every theme change the select performs four steps, in order:

1. **Locate or create** a managed
   `<link rel="stylesheet" data-lily-theme-picker="{name}">` in
   `document.head`.
2. **Swap the href** to `${themesUrl}${slug}${extension}` so the new
   theme's CSS is fetched and applied. The previous theme's CSS is
   unloaded when the href changes.
3. **Set `data-theme="{slug}"`** on the resolved target element
   (defaults to `document.documentElement`). Theme CSS files match
   this attribute via their `:root[data-theme="…"]` selector.
4. **Persist + notify**: if `storageKey` is set, write to
   `localStorage` (silently swallowing private-mode errors); then call
   `onChange?.(slug)`.

All four steps are SSR-safe — the component only mutates the DOM
inside a `$effect`, which never runs on the server.

## Rendered markup

```html
<div class="theme-picker">
  <input type="hidden" name="theme" value="dark" />
  <button
    type="button"
    class="theme-picker-button"
    aria-label="Theme"
    aria-haspopup="listbox"
    aria-expanded="false"
    aria-controls="theme-picker-1-list"
  >
    <svg class="theme-picker-icon" viewBox="0 0 16 16" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><circle cx="8" cy="8" r="6"/><path d="M8 2a6 6 0 0 1 0 12z" fill="currentColor" stroke="none"/></svg>
  </button>
  <ul
    class="theme-picker-list"
    id="theme-picker-1-list"
    role="listbox"
    aria-label="Theme"
    tabindex="-1"
    hidden
  >
    <li
      class="theme-picker-option"
      id="theme-picker-1-option-0"
      role="option"
      aria-selected="false"
    >
      Light
    </li>
    <li
      class="theme-picker-option"
      id="theme-picker-1-option-1"
      role="option"
      aria-selected="true"
    >
      Dark
    </li>
    <li
      class="theme-picker-option"
      id="theme-picker-1-option-2"
      role="option"
      aria-selected="false"
    >
      Abyss
    </li>
  </ul>
</div>
```

### Why an icon button

The closed control costs one glyph of page width whether the catalog
holds three themes or forty-five with names like
`united-kingdom-national-health-service-england-for-patients`. A native
`<select>` is as wide as its longest option, or truncates it.

The icon is a bundled outline SVG (contrast/half-circle) — not a
Unicode character, so it renders identically on every platform. It is
`aria-hidden`; the accessible name comes from `label`.

This shape has three real costs — an icon-only control's name rests
entirely on `aria-label`, a hand-rolled listbox has weaker
assistive-technology support than a native `<select>`, and the glyph is
a font-dependent character that may substitute or fail to render. They
are set out in full, with mitigations, in
[docs/accessibility.md](./docs/accessibility.md). **For some audiences
a native `<select>` is the better choice** — read that page before
adopting this helper in an accessibility-critical context.

The hidden input keeps the control working inside a `<form>`. Its
`name` also discriminates the managed `<link>`, so two selects on one
page need two distinct `name`s.

## Keyboard

Follows the WAI-ARIA APG
[Listbox pattern](https://www.w3.org/WAI/ARIA/apg/patterns/listbox/).
Every key is implemented by the component — none of it comes from the
platform.

On the **button**:

| Key                              | Action                                |
| -------------------------------- | ------------------------------------- |
| `Enter` / `Space` / `Arrow Down` | Open with the selected option active. |
| `Arrow Up`                       | Open with the **last** option active. |

On the **listbox** (focus moves there on open):

| Key                       | Action                                             |
| ------------------------- | -------------------------------------------------- |
| `Arrow Down` / `Arrow Up` | Move the active option; **clamps**, does not wrap. |
| `Home` / `End`            | Jump to the first / last option.                   |
| `Enter` / `Space`         | Select, apply, close, refocus the button.          |
| `Escape`                  | Close and refocus **without** changing the theme.  |
| `PageUp` / `PageDown`     | Move the active option by ten; clamps.             |
| `Tab`                     | Close; focus lands on the button so the default Tab proceeds from the picker's position. |
| Printable character       | Typeahead over the labels; 500 ms buffer. A single character advances to the next match and repeats cycle; differing characters refine. |

Clicking an option selects it; clicking outside or moving focus out of
the root closes the listbox.

## Default theme

The default theme is `"light"` whenever `"light"` appears in your
`themes` list. The full resolution order on first mount is:

1. `value` prop (if non-empty)
2. `localStorage[storageKey]` (if `storageKey` is set and readable)
3. `matchSystemTheme(themes)` (only if `detectFromSystem` is set)
4. `defaultValue` prop
5. `"light"` (if present in `themes`)
6. `themes[0]`
7. `""` — nothing is applied; the select waits for user interaction

The select never displays the word `"default"`. Option labels come from
`themeName(slug)`, which title-cases each hyphen-separated word
(`"high-contrast"` → `"High Contrast"`); override with `themeLabels`.

### Following the OS colour scheme

Pass `detectFromSystem` to resolve `prefers-color-scheme` to the
`"dark"` or `"light"` slug on first visit — but only when that slug is
in `themes`, and only when neither `value` nor storage supplied one, so
an explicit user choice always wins:

```svelte
<ThemePicker
  label="Theme"
  themesUrl="/assets/themes/"
  themes={["light", "dark"]}
  detectFromSystem
  storageKey="lily-theme"
/>
```

It resolves **once**, on mount. To keep following the OS setting for
the whole session, add a `matchMedia` listener — see
[docs/recipes.md](./docs/recipes.md). This mirrors `locale-picker`'s
`detectFromNavigator`.

## Props

The complete table is in [spec/index.md §4.1](./spec/index.md#41-props). Highlights:

| Prop               | Type                     | Required | Notes                                                                                 |
| ------------------ | ------------------------ | -------- | ------------------------------------------------------------------------------------- |
| `label`            | `string`                 | yes      | `aria-label` on the button **and** the listbox. The control's entire accessible name. |
| `themesUrl`        | `string`                 | yes      | Trailing `/` is auto-added.                                                           |
| `themes`           | `string[]`               | yes      | Available slugs.                                                                      |
| `value`            | `string` (bindable)      | no       | Two-way bind for the current slug.                                                    |
| `defaultValue`     | `string`                 | no       | Initial when nothing else applies.                                                    |
| `storageKey`       | `string`                 | no       | `localStorage` persistence.                                                           |
| `detectFromSystem` | `boolean`                | no       | Resolve `prefers-color-scheme` on first visit.                                        |
| `name`             | `string`                 | no       | Hidden input `name`, and the managed `<link>` discriminator; defaults to `"theme"`.   |
| `extension`        | `string`                 | no       | Defaults to `".css"`.                                                                 |
| `target`           | `HTMLElement \| null`    | no       | `data-theme` target; defaults to `<html>`.                                            |
| `themeLabels`      | `Record<string, string>` | no       | Per-slug display label override.                                                      |
| `onChange`         | `(slug) => void`         | no       | Callback fired after apply.                                                           |
| `children`         | `Snippet<[ChildArgs]>`   | no       | Replaces the button's glyph.                                                          |

**There is no `placeholder` prop.** It was removed along with the
native `<select>` it belonged to.

See [docs/props-reference.md](./docs/props-reference.md) for a
field-by-field reference.

## Custom button rendering

Pass a `children` snippet to replace the glyph inside the trigger
button. It receives `{ value, open, labelFor }` — and it does **not**
render the options; the listbox is component-owned.

```svelte
<ThemePicker
  label="Theme"
  themesUrl="/assets/themes/"
  themes={["light", "dark", "abyss"]}
  bind:value={theme}
>
  {#snippet children({ value, open, labelFor })}
    <svg viewBox="0 0 16 16" aria-hidden="true">…</svg>
    <span class="theme-picker-text">{labelFor(value)}</span>
    <span aria-hidden="true">{open ? "▴" : "▾"}</span>
  {/snippet}
</ThemePicker>
```

Adding a visible word like this is the recommended mitigation for the
icon-only naming tradeoff, at the cost of the narrow control. An inline
SVG here also removes the font dependency on the Unicode glyph.

Working example: [`examples/custom-rendering.svelte`](./examples/custom-rendering.svelte).
Topic guide: [`docs/custom-rendering.md`](./docs/custom-rendering.md).

## Persistence

Pass a `storageKey` to persist the active slug to `localStorage`. On a
fresh mount the select reads back the stored slug as part of the
initial-value resolution (§ Default theme).

Errors writing to or reading from `localStorage` (private mode, quota,
disabled storage) are silently swallowed — the select continues to
work in-memory.

If you need cookie-based persistence (so SSR can read the theme before
first paint), see [`docs/ssr.md`](./docs/ssr.md) and the
[`examples/sveltekit-cookie/`](./examples/sveltekit-cookie/) recipe.

## Accessibility

- Built to the WAI-ARIA APG **Listbox** pattern: a `<button
aria-haspopup="listbox">` controlling a `<ul role="listbox">` whose
  active option is tracked with `aria-activedescendant`.
- `aria-label={label}` names both the button and the listbox.
- The full keyboard contract is implemented by the component — see
  [Keyboard](#keyboard).
- The active state is exposed four ways: `aria-selected` on the option,
  `data-theme` on the target, the hidden input's value, and the `value`
  binding. No colour-only meaning is required.
- WCAG 2.2 AAA is the target; visible focus styling — and the
  active-option indicator (`[data-active]`, `[aria-selected]`) — is the
  consumer's CSS responsibility.

**Three tradeoffs, stated plainly:**

1. The button is icon-only, so its accessible name rests **entirely**
   on `aria-label`. An empty or untranslated `label` leaves the control
   announced as a bare "button", with no fallback.
2. A hand-rolled listbox has **weaker assistive-technology support**
   than a native `<select>` — particularly on mobile, where a native
   select opens the OS picker. For some audiences a native `<select>`
   is genuinely the better choice; the headless `ThemePicker` container
   in `@lilydesignsystem/svelte-headless` is one.
3. The glyph is a **font-dependent character**. It may substitute to a
   mismatched font, or render as a "tofu" box, or not render at all,
   depending on what is installed on the device.

Each has mitigations. Read
[`docs/accessibility.md`](./docs/accessibility.md) before adopting this
helper in an accessibility-critical context.

## SSR and hydration

The select compiles cleanly under `@sveltejs/vite-plugin-svelte` SSR.
On the server no effects run and no DOM is touched. The markup renders
with whatever `value` was supplied, which decides `aria-selected` and
the hidden input's value. Option ids come from an incrementing module
counter, so server and client agree and hydration matches.

For zero-flicker SSR, resolve the theme on the server (e.g. from a
cookie) and pass it as `value`. See
[`docs/ssr.md`](./docs/ssr.md) and
[`examples/sveltekit-cookie/`](./examples/sveltekit-cookie/).

## Preloading for zero-flicker switching

By default the select swaps one `<link>` href, so the active theme is
fetched on demand. To switch instantly between themes, preload them
all yourself:

```html
<link rel="stylesheet" href="/assets/themes/light.css" />
<link rel="stylesheet" href="/assets/themes/dark.css" />
<link rel="stylesheet" href="/assets/themes/abyss.css" />
```

The select still mutates `data-theme`, and since every theme's CSS is
scoped to `:root[data-theme="…"]`, the active rules switch
instantly with the attribute change — no network round-trip.

Topic guide: [`docs/preloading.md`](./docs/preloading.md). Working
example: [`examples/preloaded.svelte`](./examples/preloaded.svelte).

## Multiple pickers in one app

Pass a distinct `name` prop to each select. The `name` is used as
both the hidden input's `name` and the discriminator on the managed
`<link>` element (`data-lily-theme-picker="{name}"`).

Example: [`examples/multiple-pickers.svelte`](./examples/multiple-pickers.svelte).

## Recipes

Quick cookbook in [`docs/recipes.md`](./docs/recipes.md):

- Following the OS colour scheme via `prefers-color-scheme`.
- Reading a theme cookie in SvelteKit before render.
- Migrating from a `localStorage`-only select to a cookie-backed one.
- Building a flyout / dropdown UI around the select.
- Loading themes from a CDN.

## Troubleshooting

See [`docs/troubleshooting.md`](./docs/troubleshooting.md). Common pitfalls:

- **CSS does not switch.** Check that each theme file scopes its rules
  to `:root[data-theme="<slug>"]` (not `:root` alone). Otherwise the
  first-loaded theme leaks across.
- **404 on theme href.** Check the file is served from `themesUrl` and
  uses the configured `extension` (defaults to `.css`).
- **SSR mismatch warning.** Pass a server-resolved `value` (cookie)
  so the SSR markup matches what the effect will set on the client.
- **Theme does not persist.** Confirm `storageKey` is set and that
  `localStorage` is available (not blocked by private mode).

## Testing

`pnpm test` under a vitest + jsdom + `@testing-library/svelte` setup
exercises every numbered acceptance criterion in
[spec/index.md §7](./spec/index.md#7-testing-acceptance-criteria).

## License

MIT or Apache-2.0 or GPL-2.0 or GPL-3.0 or BSD-3-Clause. Contact
joel@joelparkerhenderson.com for other terms.

---

Lily™ and Lily Design System™ are trademarks.
