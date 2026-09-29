# Changelog — @lilydesignsystem/svelte-headless

The format is loosely based on [Keep a Changelog](https://keepachangelog.com/)
and the package follows [Semantic Versioning](https://semver.org/).

## 0.2.0 — 2026-09-21

**`Listbox` and `IconButton` extended, additively, to support the
`*-helpers` catalog's icon-button-triggered pickers depending on them
instead of hand-rolling equivalent markup/keyboard logic.** Both
components had zero consumers elsewhere in this 491-component catalog
(confirmed by search before changing either), so these are pure
additions with no risk to an existing consumer — full suite (4917
tests) still green.

- `IconButton` gains `baseClass` (default `"icon-button"`, unchanged)
  — a consumer whose own contract requires an exact class (no extra
  `icon-button` token) sets `baseClass` instead of layering `class` on
  top of it — and a bindable `ref` exposing the rendered `<button>` so
  a consumer can call `.focus()` on it.
- `Listbox` gains `baseClass` (default `"listbox"`, unchanged, same
  reasoning as `IconButton`'s) and `as` (default `"div"`, unchanged —
  a consumer needing e.g. a `<ul>` root sets `as="ul"`), plus an opt-in
  `navigation="active-descendant"` mode (default remains
  `"roving-focus"`, byte-for-byte unchanged behaviour): the root holds
  real focus and tracks a virtual cursor (`aria-activedescendant`,
  bindable `activeIndex`) rather than moving DOM focus between
  options, plus `clamp` (vs. wrap), `typeahead`, `pageSize` paging,
  and `onActivate`/`onEscape`/`onTabOut` callbacks — the full
  WAI-ARIA APG listbox keyboard contract the picker helpers already
  implement by hand. A bindable `ref` prop exposes the rendered root
  so a consumer can call `.focus()` on it.

## 0.1.0 — 2026-09-16

**Package renamed: `lily-design-system-svelte-headless` → `@lilydesignsystem/svelte-headless`.** npm scoped packages
are registry-distinct from their unscoped counterparts, so this is a
new package with no publish history of its own — version reset to
`0.1.0` per this project's established rename precedent (the July
2026 `*-select` → `*-picker` rename). No code or behaviour change
relative to `lily-design-system-svelte-headless`'s last published version (`0.1.0`);
its full changelog continues below, now read as history prior to the
rescope. The old unscoped name is deprecated on the registry (never
unpublished), pointing consumers here.

---

## 0.3.1 — 2026-08-26

Metadata-only patch; no code change. Ships the corrected package
metadata to the registry:

- `license` is the project SPDX menu (`MIT OR Apache-2.0 OR
  GPL-2.0-only OR GPL-3.0-only OR BSD-3-Clause`), replacing the
  single-license field that contradicted the repository's LICENSE.md.
- `repository`, `homepage`, and `bugs` point at the LilyDesignSystem
  organisation and the documentation site.
- `description` says "Targets WCAG 2.2 AAA." — the previous
  "WCAG 2.2 AAA compliant" claimed a conformance no audit supports.
- `author` names the maintainer rather than a bare email address.

Also as of this release, 0.2.0 is marked deprecated on npm (it
declared a `main` that was never built; see 0.3.0's notes).

## 0.3.0 — 2026-08-23

### Fixed

- **The package had no entry point.** `package.json` declared
  `"main": "index.js"` and no such file was ever built or shipped, so
  every `import … from "@lilydesignsystem/svelte-headless"` failed at
  resolution. 0.2.0 is broken on npm for this reason. The package now
  builds a real `dist/` with svelte-package and points `main`/`types`/
  `exports` at it.

### Added

- A generated barrel (`index.ts`, written by `build.mjs`) exporting all
  491 components in the catalog. It is generated rather than
  hand-maintained because the catalog grows, and a hand-written list
  silently omits new components.
- `build.mjs` — generates the barrel, builds `dist/`, and fails loudly if
  the bundle comes out empty.
- A `files` allowlist.

### Changed

- **Tarball contents.** Previously the package shipped its entire working
  tree — sources, tests, Storybook stories, docs and config. It now ships
  only `dist/` plus the docs.
- `peerDependencies` now declares the framework it needs.
- The description said "236 components"; the catalog has 491.

### Also fixed

- `ProgressCircle`'s tests queried `getByRole("Progress")`, which is not a
  valid ARIA role and never matched; the component correctly renders
  `role="progressbar"`. Those four tests had been failing, in both the
  `components/` and `src/lib/components/` mirrors.

All 4906 tests pass; 8 were failing before.
