# Lily Design System™ — Themes

`@lilydesignsystem/themes` packages the 45 reference theme stylesheets
that live at the canonical monorepo root
([`themes/`](https://github.com/LilyDesignSystem/lily-design-system/tree/main/themes))
as an installable npm package, so a consumer who wants Lily's default
look does not have to clone the monorepo or hand-copy CSS files.

Each stylesheet targets the Lily headless components' kebab-case class
hooks (see [css-style-sheet-template.css](https://github.com/LilyDesignSystem/lily-design-system/blob/main/css-style-sheet-template.css))
directly — no build step, no CSS framework, no JavaScript runtime.
This package is CSS only: 45 files, one per theme, unmodified from the
canonical source.

## Install

```sh
npm install @lilydesignsystem/themes
```

## Usage

**Static `<link>` (what the `theme-picker` helper does):**

```html
<link rel="stylesheet" href="/node_modules/@lilydesignsystem/themes/light.css" data-lily-theme-picker>
```

`theme-picker` swaps this `href` at runtime and sets `data-theme` on
the document root — see
[AGENTS/helpers.md](https://github.com/LilyDesignSystem/lily-design-system/blob/main/AGENTS/helpers.md)
and the [`theme-picker` package](https://www.npmjs.com/package/@lilydesignsystem/svelte-theme-picker)
for each framework.

**Bundler CSS import (Vite, webpack, etc.):**

```js
import "@lilydesignsystem/themes/light.css";
```

**Copy into your own static assets** (the pattern every Lily example
app uses, via `bin/sync`):

```sh
cp node_modules/@lilydesignsystem/themes/*.css public/themes/
```

## The 45 themes

Alphabetical, with the UK/US government themes grouped at the end
(matching `picker-bar`'s default ordering):

abyss, acid, adobe-spectrum, aqua, autumn, black, bumblebee, business,
caramellatte, cmyk, coffee, corporate, cupcake, cyberpunk, dark, dim,
dracula, emerald, fantasy, forest, garden, halloween, lemonade, light,
lofi, luxury, mozilla-protocol, night, nord, pastel, retro, silk,
sunset, synthwave, valentine, winter, wireframe,
united-kingdom-government-digital-service,
united-kingdom-national-health-service-england-for-patients,
united-kingdom-national-health-service-england-for-practitioners,
united-kingdom-national-health-service-scotland-for-patients,
united-kingdom-national-health-service-scotland-for-practitioners,
united-kingdom-national-health-service-wales-for-patients,
united-kingdom-national-health-service-wales-for-practitioners,
united-states-web-design-system.

Each file name is the slug `theme-picker` sets on
`:root[data-theme="{slug}"]`.

## Conventions

- Every selector is `:where(...)` or lives under `@layer lily { ... }`
  (zero specificity), so consumer CSS always wins the cascade — see
  [AGENTS/theme.md](https://github.com/LilyDesignSystem/lily-design-system/blob/main/AGENTS/theme.md).
  Verified by `bin/check-theme`.
- Every class targeted is a real catalog component slug or a
  documented helper hook (`theme-picker`, `locale-picker`,
  `text-size-picker`, `motion-picker`, `share-picker`,
  `date-time-picker`, `picker-bar`) — no invented hooks.
- Nothing here ships JavaScript, fonts, icons, or imagery.

## License

Free open source, under your choice of MIT, Apache-2.0, GPL-2.0-only,
GPL-3.0-only, or BSD-3-Clause. See [LICENSE.md](LICENSE.md).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Work happens in the canonical
[monorepo](https://github.com/LilyDesignSystem/lily-design-system);
`themes/` there is the single source of truth this package packages.

---

Lily™ and Lily Design System™ are trademarks.
