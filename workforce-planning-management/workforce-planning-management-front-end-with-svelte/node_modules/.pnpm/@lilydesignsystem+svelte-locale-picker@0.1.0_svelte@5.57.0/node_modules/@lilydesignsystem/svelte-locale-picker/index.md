# LocalePicker (Svelte helper)

A reusable, headless Svelte 5 locale picker — an **icon button that
opens a WAI-ARIA APG listbox** — that applies the chosen locale to the
document root via `lang` and `dir`, with optional `localStorage`
persistence and `navigator.languages` detection.

For the full contract see [spec/index.md](./spec/index.md) — it is the single source
of truth for the API, behaviour, and tests.

## Install

This directory is published as a folder-style import; consumers either
copy it into their project or wire it as a workspace dependency. The
only runtime dependency is `svelte` ≥ 5.

```ts
import LocalePicker from "./lily-design-system-svelte-locale-picker/LocalePicker.svelte";
```

Or via the barrel (recommended; gives you the typed helpers too):

```ts
import LocalePicker, {
  bcp47LocaleTag,
  isRtlLocale,
  localeName,
  type Props,
  type ChildArgs,
} from "./lily-design-system-svelte-locale-picker";
```

## Quick start

Render the select with a `label` and the list of locales your app
supports. The select writes `lang` and `dir` onto `<html>` so your
i18n library, your CSS (`html[dir="rtl"]`), and assistive technology
all see the change.

```svelte
<script lang="ts">
    import LocalePicker, {
        bcp47LocaleTag,
        localeName,
    } from "./lily-design-system-svelte-locale-picker/LocalePicker.svelte";

    let locale = $state("");
</script>

<LocalePicker
    label="Language"
    locales={["en", "en_US", "fr", "fr_CA", "ar", "he"]}
    bind:value={locale}
    storageKey="lily-locale"
    detectFromNavigator
/>

<p class="locale-picker-status" aria-live="polite">
    Active language:
    <span lang={bcp47LocaleTag(locale)}>{localeName(locale)}</span>
</p>
```

You also need to **style the listbox** — the package ships zero CSS,
and an unpositioned popup renders in normal flow and shoves the page
down when it opens. Use logical properties, since this control flips
the page to RTL. See
[docs/styling.md § Positioning the listbox](./docs/styling.md#positioning-the-listbox).

The status line is recommended, though no longer strictly compensatory.
The listbox marks the active option with `aria-selected="true"`, so a
screen-reader user who opens the control does hear which locale is
current. But the _closed_ control shows only a glyph — and unlike a
theme select, the active locale is not something a user can infer by
looking, unless they can already read the page, which is the one thing
this control cannot assume. `aria-live="polite"` announces mutations
only, so it stays silent on first paint and speaks once per change, and
the `lang` on the `<span>` keeps the locale name pronounced in its own
language. Full reasoning and when to omit it:
[docs/accessibility.md](./docs/accessibility.md#the-status-region).

> The example above wraps the name in `lang` while showing
> `localeName`, which returns the **English** name — so drop the `lang`
> unless you are supplying endonyms via `localeLabels`. See the note in
> [docs/accessibility.md](./docs/accessibility.md#the-status-region).

When the user picks `ar`, the component:

- sets `lang="ar"` on `<html>`,
- sets `dir="rtl"` on `<html>` (auto-detected from the locale),
- writes `"ar"` to `localStorage["lily-locale"]`,
- fires `onChange("ar")` if provided.

The select does NOT translate strings — that is the consumer's
i18n library (e.g. `svelte-i18n`, Paraglide, Inlang, Tolgee, raw
`Intl.*`). Wire the bindable `value` or `onChange` to your library so
it loads the right messages.

## BCP 47 normalisation

Language tags follow **BCP 47** (RFC 5646). The `lang` attribute on
HTML elements must use hyphens, while many applications carry locale
identifiers with underscores (`en_US`, `zh_Hant_TW`). The select
accepts whichever form you prefer in the `locales` array and converts
to the hyphen form when writing to the DOM. The bindable `value`
preserves your original form, so round-trips are lossless.

```ts
bcp47LocaleTag("en_US"); // "en-US"
bcp47LocaleTag("zh_Hant_TW"); // "zh-Hant-TW"
bcp47LocaleTag("en"); // "en"
```

References:

- W3C — [Language tags in HTML and XML](https://www.w3.org/International/articles/language-tags/)
- IETF — [RFC 5646 (BCP 47), Tags for Identifying Languages](https://www.rfc-editor.org/rfc/rfc5646)
- IANA — [Language Subtag Registry (registry file)](https://www.iana.org/assignments/language-subtag-registry/language-subtag-registry)

## RTL auto-detection

`isRtlLocale(locale)` returns `true` for any locale whose base
language is one of `ar`, `arc`, `ckb`, `dv`, `fa`, `he`, `iw`, `ji`,
`ks`, `ku`, `mzn`, `ps`, `sd`, `ug`, `ur`, `yi`, OR whose script
subtag is one of `Arab`, `Hebr`, `Thaa`, `Syrc`, `Nkoo`, `Mong`,
`Adlm`.

```ts
isRtlLocale("ar"); // true
isRtlLocale("he_IL"); // true
isRtlLocale("uz_Arab_AF"); // true (script subtag)
isRtlLocale("en"); // false
```

Pass `applyDir={false}` if you want full control of `dir` yourself.

## Examples

### Rendered markup

```svelte
<script lang="ts">
    import LocalePicker from "./lily-design-system-svelte-locale-picker/LocalePicker.svelte";
    let locale = $state("en");
</script>

<LocalePicker label="Language" locales={["en", "cy"]} bind:value={locale} />
```

Renders:

```html
<div class="locale-picker">
  <input type="hidden" name="locale" value="en" />
  <button
    type="button"
    class="locale-picker-button"
    aria-label="Language"
    aria-haspopup="listbox"
    aria-expanded="false"
    aria-controls="locale-picker-1-list"
  >
    <svg class="locale-picker-icon" viewBox="0 0 16 16" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><circle cx="8" cy="8" r="6"/><path d="M2 8h12"/><path d="M8 2c2.2 0 4 2.7 4 6s-1.8 6-4 6-4-2.7-4-6 1.8-6 4-6z"/></svg>
  </button>
  <ul
    class="locale-picker-list"
    id="locale-picker-1-list"
    role="listbox"
    aria-label="Language"
    tabindex="-1"
    hidden
  >
    <li
      class="locale-picker-option"
      id="locale-picker-1-option-0"
      role="option"
      aria-selected="true"
      lang="en"
    >
      English
    </li>
    <li
      class="locale-picker-option"
      id="locale-picker-1-option-1"
      role="option"
      aria-selected="false"
      lang="cy"
    >
      Welsh
    </li>
  </ul>
</div>
```

Each locale option carries its own `lang` attribute so a screen reader
pronounces "Cymraeg" with a Welsh voice (WCAG 3.1.2, Language of
Parts). The button and the list carry none — they are in whatever
language you wrote `label` in.

The glyph is U+1F310 GLOBE WITH MERIDIANS followed by **U+FE0E
VARIATION SELECTOR-15**, which forces monochrome text presentation so
the control matches `theme-picker`'s `◑` instead of rendering as a blue
colour emoji. It is `aria-hidden`; the accessible name comes from
`label`.

The hidden input keeps the control working inside a `<form>`, carrying
the consumer-form code.

### Why an icon button

The closed control costs one glyph of page width whether you offer
three locales or all 436 in `locales.tsv`. A native `<select>` is as
wide as its longest option, or truncates it.

This shape has three real costs — an icon-only control's name rests
entirely on `aria-label` (which is itself written in _one_ language,
for the one control a user reaches when they cannot read the page); a
hand-rolled listbox has weaker assistive-technology support than a
native `<select>`; and the glyph is a font-dependent character that may
substitute, render in colour, or fail to render. They are set out in
full, with mitigations, in
[docs/accessibility.md](./docs/accessibility.md). **For some audiences
a native `<select>` is the better choice** — read that page before
adopting this helper in an accessibility-critical or public-service
context.

### Keyboard

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
| `Escape`                  | Close and refocus **without** changing the locale. |
| `PageUp` / `PageDown`     | Move the active option by ten; clamps.             |
| `Tab`                     | Close; focus lands on the button so the default Tab proceeds from the picker's position. |
| Printable character       | Typeahead over the labels; 500 ms buffer. A single character advances to the next match and repeats cycle; differing characters refine. |

Clicking an option selects it; clicking outside or moving focus out of
the root closes the listbox.

Typeahead matches the **label**, and default labels are endonyms, so
a user types "Fra" for "Français". With `localeLabels` overrides the
consumer's spelling wins. Choose deliberately for long lists.

### Pretty labels for the option text

By default each option shows the language's **endonym** — its own name
for itself, "Cymraeg" not "Welsh" — via `Intl.DisplayNames` asked in
that language (exported as `localeEndonym`). The user who needs a
language menu is the one who cannot read the page's language, and the
exonym means nothing to them. The English names from `locales.tsv` and
the raw code remain as fallbacks for runtimes without the data.
Override per-code with `localeLabels`:

```svelte
<LocalePicker
    label="Langue"
    locales={["en", "fr", "ar"]}
    localeLabels={{ en: "English", fr: "Français", ar: "العربية" }}
    bind:value={locale}
/>
```

Each option carries a `lang="…"` attribute so each one is
announced in its own language. Prefer endonyms — the person who needs
this control is the person who cannot read your default language.

### Customising the button

The `children` snippet **replaces the glyph inside the trigger
button**. It receives `{ value, open, labelFor }` and does **not**
render the options — the listbox is component-owned.

Pairing the glyph with the active locale's endonym is the strongest
mitigation for the icon-only naming tradeoff:

```svelte
<script lang="ts">
    import LocalePicker, {
        bcp47LocaleTag,
        isRtlLocale,
    } from "./lily-design-system-svelte-locale-picker/LocalePicker.svelte";

    let locale = $state("en");
</script>

<LocalePicker
    label="Language"
    locales={["en", "fr", "ar"]}
    localeLabels={{ en: "English", fr: "Français", ar: "العربية" }}
    bind:value={locale}
>
    {#snippet children({ value, open, labelFor })}
        <svg viewBox="0 0 16 16" aria-hidden="true">…</svg>
        <span
            class="locale-picker-text"
            lang={bcp47LocaleTag(value)}
            dir={isRtlLocale(value) ? "rtl" : "ltr"}
        >
            {labelFor(value)}
        </span>
        <span aria-hidden="true">{open ? "▴" : "▾"}</span>
    {/snippet}
</LocalePicker>
```

The `lang` on the span is only correct because the labels are endonyms —
which the built-in labels now are. The component applies the same rule
to its own options: `lang` is set only when the label is the derived
endonym, never on consumer labels of unknown language.

The snippet's output lives inside a `<button>`, so it must not contain
interactive elements. The pre-listbox patterns built on the old
`ChildArgs` — a custom `<select>`, a radio group, a button group, a
`<datalist>` combobox — are no longer possible; read `value` and drive
your own controls instead. See
[docs/custom-rendering.md](./docs/custom-rendering.md).

### Wiring an i18n library

```svelte
<script lang="ts">
    import LocalePicker from "./lily-design-system-svelte-locale-picker/LocalePicker.svelte";
    import { locale as i18nLocale } from "svelte-i18n"; // or Paraglide, Inlang, …

    let current = $state("");
</script>

<LocalePicker
    label="Language"
    locales={["en", "fr", "ar"]}
    bind:value={current}
    detectFromNavigator
    storageKey="app-locale"
    onChange={(code) => i18nLocale.set(code)}
/>
```

### Server-resolved initial value (SSR)

For flicker-free first paint, resolve the locale on the server (from a
cookie or `Accept-Language`) and pass it as `value`:

```svelte
<script lang="ts">
    let { initialLocale }: { initialLocale: string } = $props();
    let locale = $state(initialLocale);
</script>

<LocalePicker
    label="Language"
    locales={["en", "fr", "ar"]}
    value={locale}
    bind:value={locale}
/>
```

During SSR the component renders the button and the (hidden) listbox,
with `aria-selected` and the hidden input reflecting the supplied
`value`, and the document already arrives with the correct `lang`
attribute on `<html>` — which is what prevents the flicker. Option ids
come from an incrementing module counter, so server and client agree
and hydration matches.

### Render into a scoped target instead of `<html>`

Set `target` to a specific element when you want the locale scoped to a
region (e.g. a multilingual side panel):

```svelte
<script lang="ts">
    import LocalePicker from "./lily-design-system-svelte-locale-picker/LocalePicker.svelte";
    let region: HTMLElement | null = $state(null);
    let panelLocale = $state("fr");
</script>

<section bind:this={region}>
    <p>This panel switches language independently of the page.</p>
    <LocalePicker
        label="Panel language"
        locales={["en", "fr", "ar"]}
        target={region}
        bind:value={panelLocale}
    />
</section>
```

`<html>` stays in the page's default locale; the section gets the
chosen one.

## Built-in locale data

`locales.ts` ships the 436 codes from `locales.tsv` mapped to their
English names. Since the endonym change these are a **fallback** — the
default label is `localeEndonym(code)`, and the table is consulted only
when the runtime lacks `Intl.DisplayNames` data for a code. You can also import
the data directly:

```ts
import {
  defaultLocaleLabels,
  RTL_LANGUAGE_TAGS,
  RTL_SCRIPT_SUBTAGS,
} from "./lily-design-system-svelte-locale-picker";

console.log(defaultLocaleLabels["en_US"]); // "English (United States)"
console.log(RTL_LANGUAGE_TAGS.has("ar")); // true
```

## Props

See [spec/index.md §4](./spec/index.md#4-public-api) for the full table.

Required props: `label`, `locales`.

Common optional props: `value` (bindable), `defaultValue`,
`storageKey`, `detectFromNavigator`, `localeLabels`, `applyDir`,
`target`, `onChange`, `class`, `name`, `children`.

**There is no `placeholder` prop.** It was removed along with the
native `<select>` it belonged to. Field-by-field reference:
[docs/props-reference.md](./docs/props-reference.md).

## Class hooks

`.locale-picker` (root `<div>`), `.locale-picker-button` (the trigger),
`.locale-picker-icon` (the glyph span), `.locale-picker-list` (the
`<ul role="listbox">`), `.locale-picker-option` (each
`<li role="option">`). Plus `[data-active]` for the keyboard cursor and
`[aria-selected]` for the applied locale — style both.

The `.locale-picker-placeholder` hook is gone with the placeholder
option.

The package ships zero CSS, so **you must position the listbox**, using
logical properties so it survives the RTL switch this control performs.
See [docs/styling.md](./docs/styling.md).

## Accessibility

- Built to the WAI-ARIA APG **Listbox** pattern: a `<button
aria-haspopup="listbox">` controlling a `<ul role="listbox">` whose
  active option is tracked with `aria-activedescendant`.
- `aria-label={label}` names both the button and the listbox.
- The full keyboard contract is implemented by the component — see
  [Keyboard](#keyboard).
- Each `<li role="option">` carries `lang="…"` so its name is
  pronounced in the right language (WCAG 3.1.2, Language of Parts).
- The document root carries `lang` and (by default) `dir` so the page
  satisfies WCAG 3.1.1 (Language of Page) and bidi text/layout
  inverts correctly for RTL locales.
- The active state is exposed four ways: `aria-selected` on the option,
  `lang` on the target, the hidden input's value, and the `value`
  binding. No colour-only meaning.
- Choosing a locale returns focus to the trigger button and does not
  navigate — WCAG 3.2.2 (On Input).

**Three tradeoffs, stated plainly:**

1. The button is icon-only, so its accessible name rests **entirely**
   on `aria-label` — and `aria-label` is written in _one_ language.
   This is the control a user reaches for precisely when they cannot
   read the page, so the circularity is real. Pairing the glyph with
   the active locale's endonym via `children` is the strongest
   mitigation.
2. A hand-rolled listbox has **weaker assistive-technology support**
   than a native `<select>` — particularly on mobile, where a native
   select opens the OS picker. For some audiences, and public-service
   audiences especially, a plain `<select aria-label>` with one
   `<option lang>` per locale is genuinely the better choice; it is
   about fifteen lines, and this package's exported pure helpers still
   do the logic.
3. The glyph is a **font-dependent character**. VS15 requests
   monochrome presentation but cannot guarantee it, and on a device
   with no covering font the button renders empty or as a "tofu" box.

Each has mitigations. Read
[docs/accessibility.md](./docs/accessibility.md) before adopting this
helper in an accessibility-critical context.

## Tests

`pnpm test` under a vitest + jsdom + `@testing-library/svelte` setup
exercises every numbered acceptance clause in
[spec/index.md §7](./spec/index.md#7-testing-acceptance-criteria) — 27
clauses covering the markup contract, the pure helpers, locale
application, initial-value resolution, spread + custom children, and
the APG keyboard contract, plus four untagged extras for
case-insensitive RTL detection and the navigator matcher.

## Files in this directory

| File                    | Purpose                                                           |
| ----------------------- | ----------------------------------------------------------------- |
| `spec/index.md`         | Single source of truth — API, behaviour, tests.                   |
| `LocalePicker.svelte`  | The component implementation.                                     |
| `LocalePicker.test.ts` | vitest suite covering every spec §7 item.                         |
| `locales.ts`            | Built-in code → English-name map and RTL sets.                    |
| `locales.tsv`           | Canonical 436-row source for `locales.ts`.                        |
| `index.ts`              | Re-export barrel.                                                 |
| `index.md`              | This file — quick start + worked examples.                        |
| `docs/`                 | Deep-dive guides — see [Documentation](#documentation).           |
| `examples/`             | Runnable Svelte 5 example components — see [Examples](#examples). |

## Documentation

Shared with `theme-picker` (same topics, written for this helper):

| Guide                                                  | Covers                                                              |
| ------------------------------------------------------ | ------------------------------------------------------------------- |
| [docs/props-reference.md](./docs/props-reference.md)   | Field-by-field reference for every prop, with rationale.            |
| [docs/styling.md](./docs/styling.md)                   | Class and attribute hooks, positioning the listbox, RTL-safe CSS.   |
| [docs/custom-rendering.md](./docs/custom-rendering.md) | The `children` snippet — replacing the button's glyph.              |
| [docs/recipes.md](./docs/recipes.md)                   | Short solutions to adjacent problems.                               |
| [docs/troubleshooting.md](./docs/troubleshooting.md)   | Symptoms, root causes, fixes.                                       |
| [docs/accessibility.md](./docs/accessibility.md)       | APG listbox contract, the three tradeoffs, screen-reader matrix.    |
| [docs/ssr.md](./docs/ssr.md)                           | Cookie, URL-prefix, Accept-Language, streaming SSR, FOUC avoidance. |

Specific to locale-picker (no `theme-picker` counterpart):

| Guide                                                  | Covers                                                                         |
| ------------------------------------------------------ | ------------------------------------------------------------------------------ |
| [docs/concepts.md](./docs/concepts.md)                 | Mental model, lifecycle diagram, why the defaults are what they are.           |
| [docs/bcp47.md](./docs/bcp47.md)                       | Language-tag syntax (RFC 5646), IANA registry, subtag composition.             |
| [docs/rtl.md](./docs/rtl.md)                           | What's auto-detected, what `dir="rtl"` actually changes, CSS tips.             |
| [docs/i18n-integration.md](./docs/i18n-integration.md) | Wiring svelte-i18n, Paraglide, Tolgee, raw `Intl.*`, SvelteKit URL strategies. |

`theme-picker`'s `preloading.md` has no counterpart here — it is about
stylesheet preloading, which this helper does not do.

## Examples

Each file in `examples/` is a complete, runnable Svelte 5 component
you can copy into your project.

| Example                                                       | Demonstrates                                                          |
| ------------------------------------------------------------- | --------------------------------------------------------------------- |
| [basic.svelte](./examples/basic.svelte)                       | The default rendering, plus the `.locale-picker-status` live region. |
| [custom-rendering.svelte](./examples/custom-rendering.svelte) | `children` snippet — globe + the active locale's endonym + caret.     |
| [many-locales.svelte](./examples/many-locales.svelte)         | A 23-locale list in a one-glyph control; typeahead and scrolling.     |
| [persistence.svelte](./examples/persistence.svelte)           | `storageKey` plus `detectFromNavigator` on first visit.               |
| [rtl-demo.svelte](./examples/rtl-demo.svelte)                 | Live RTL preview — Arabic, Hebrew, Persian, Urdu, Pashto.             |
| [nhs-style.svelte](./examples/nhs-style.svelte)               | NHS UK-style utility banner with endonyms and a `class` hook.         |
| [with-svelte-i18n.svelte](./examples/with-svelte-i18n.svelte) | Binding to svelte-i18n's `locale` store.                              |
| [with-paraglide.svelte](./examples/with-paraglide.svelte)     | Driving Paraglide JS's `setLocale()` from `onChange`.                 |
| [ssr-cookie.svelte](./examples/ssr-cookie.svelte)             | SvelteKit cookie-based SSR — no flash of default locale.              |
| [scoped-target.svelte](./examples/scoped-target.svelte)       | Multiple per-region selects, each scoped to its own panel.            |

These were previously numbered `01-radios`, `02-select`, `03-buttons`,
… — names left over from a radio-group rendering the package has not
had for some time. The mapping is recorded in
[examples/README.md](./examples/README.md#renamed-from-the-radio-group-era).

---

Lily™ and Lily Design System™ are trademarks.
