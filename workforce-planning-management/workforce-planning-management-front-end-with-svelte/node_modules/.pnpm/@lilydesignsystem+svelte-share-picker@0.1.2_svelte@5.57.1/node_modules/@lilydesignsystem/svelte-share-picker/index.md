# SharePicker (Svelte helper)

A headless Svelte 5 share control: a single-icon button (a bundled
outline-arrow SVG) that opens the **native share sheet** where the
browser has one, and otherwise shows a list of destinations you
supply, plus **copy the page URL**.

The single source of truth is [spec/index.md](./spec/index.md). This file
is the human-readable guide.

## Install

```ts
import SharePicker from "./lily-design-system-svelte-share-picker/SharePicker.svelte";
// or via the barrel:
import {
  SharePicker,
  canShareNatively,
  type ShareTarget,
} from "./lily-design-system-svelte-share-picker";
```

## Quick start

```svelte
<script lang="ts">
  import SharePicker, { type ShareTarget }
    from "./lily-design-system-svelte-share-picker/SharePicker.svelte";

  const targets: ShareTarget[] = [
    {
      id: "mastodon",
      label: "Mastodon",
      href: (url, title) =>
        `https://mastodon.social/share?text=${encodeURIComponent(title)}%20${encodeURIComponent(url)}`,
    },
    {
      id: "email",
      label: "Email",
      href: (url, title) =>
        `mailto:?subject=${encodeURIComponent(title)}&body=${encodeURIComponent(url)}`,
      newTab: false,
    },
  ];
</script>

<SharePicker
  label="Share this page"
  title="An article worth reading"
  {targets}
  copyLabel="Copy link"
  copiedLabel="Link copied"
  copyFailedLabel="Could not copy — copy it from the address bar"
/>
```

`url` defaults to the current page, so the common case needs no wiring.

## You supply the destinations

This package ships **no** social-network URLs. That is deliberate: which
networks belong in your product is an editorial and privacy decision, the
share endpoints change, and networks die. You pass `targets`, so the
labels localise with the rest of your copy and no third-party endpoint is
baked into a design system.

`href` is a function, so you own the whole URL and its encoding:

```ts
{ id: "linkedin", label: "LinkedIn",
  href: (url) => `https://www.linkedin.com/sharing/share-offsite/?url=${encodeURIComponent(url)}` }
```

## Native share sheet

With `strategy="auto"` (the default), pressing the button on a device
with `navigator.share` opens the OS sheet — the user gets their real
installed apps, and nothing is disclosed to a third party by the act of
opening it. Where there is no sheet, the list opens instead.

This means **behaviour differs by platform**, which is worth knowing when
you write help text or test scripts. Force one path with
`strategy="list"` or `strategy="native"`.

A dismissed sheet ends the interaction — the list does not then pop open,
which would resurrect UI the user just dismissed.

## Copy to clipboard

Supply `copyLabel` and a copy item appears. There is no default label,
because a default would be a hardcoded English string. `copiedLabel` and
`copyFailedLabel` are announced in a polite live region — copying is
otherwise silent, so without them the user gets no confirmation.

Failure is handled, not assumed away: a denied permission, an insecure
context, or a browser with no async clipboard all announce
`copyFailedLabel` rather than throwing.

## Why links, not a menu

Destinations render as real `<a>` elements, not `role="menuitem"`. A
menuitem role strips middle-click, open-in-new-tab, and copy-link-address
— affordances users genuinely reach for on a share list. The WAI-ARIA APG
suggests a disclosure when the items are links. Copy is a real action, so
it is a `<button>`.

## Props

Full table in [spec/index.md §4.1](./spec/index.md#41-props). Required:
`label`. Everything else is optional.

## Accessibility

- The glyph is `aria-hidden`; the name comes from `aria-label`.
- `Escape` closes and returns focus to the button; arrows move between
  items and clamp; `Home` / `End` jump; `Tab` closes and moves on.
- The status region is polite and empty on load.
- **Tradeoff:** an icon-only control's name rests entirely on
  `aria-label` — there is no visible text fallback. See
  [docs/accessibility.md](./docs/accessibility.md).

## Styling

Class hooks: `.share-picker` (root), `.share-picker-button`,
`.share-picker-icon`, `.share-picker-list`, `.share-picker-list-item`,
`.share-picker-target`, `.share-picker-copy`, `.share-picker-status`.

The package ships no CSS beyond the icon markup itself. The root
`themes/` stylesheets style the button and popup; the icon is an
explicitly-sized SVG (`width`/`height`, not font-size-based scaling),
so it renders the same visual size as the other helpers' icons without
needing a per-glyph optical correction.

## Tests

`npx vitest run @lilydesignsystem/svelte-share-picker` from the catalog
root — 27 cases, one or more per §7 clause.

---

Lily™ and Lily Design System™ are trademarks.
