# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed — full Lily `PickerBar` in the header, not `ThemePicker` alone

Per `spec/lily-design-system-svelte-with-picker-bar/index.md`: replaced
the lone `ThemePicker` with `@lilydesignsystem/svelte-picker-bar`
(theme + locale + text-size + share, same as every operator
front-end), with `themes`/`sizes` left unset so it uses Lily's own
defaults (45 themes, alphabetical with the UK/US government themes
grouped at the end; the 7-step `largest`…`smallest` text-size scale)
rather than a hand-maintained list — the site's previous `THEMES`
array (33 slugs) was already a subset of Lily's own 45. Added the
matching `[data-text-size]` CSS rules to `static/assets/style.css`
(this site never had a text-size picker before). No i18n catalogue
exists here, so the locale picker runs self-contained — a local
`LOCALES`/`LOCALE_LABELS` array, `lang`/`dir` applied by the picker
itself rather than an app store (same special case as
`patient-flow-front-end-with-svelte`). `SHARE_TARGETS` matches every
other front-end: Email / LinkedIn / Reddit / Bluesky / Mastodon (via
mastodonshare.com), plus Copy Link.

Verified live: all four pickers render, 45 theme options, every share
target present, no console errors.

### Changed — fixed the site for actual publish (PRO-H14)

`lily-design-system-svelte-theme-picker` switched from a `file:` path
onto the sibling `lilydesignsystem` checkout to a registry version
(`^0.1.1`), and `static/assets/themes` from a symlink onto that same
checkout to a vendored copy (45 files). Both previously resolved
correctly only while this directory was nested inside the monorepo —
one directory level shallower than needed once `git subtree split`
promotes it to a standalone repo root, so both silently broke in the
exported sibling repo. Same class of bug `agents/share/
svelte-front-end-stack.md` found and fixed in every operator
front-end; here it went unnoticed until the first actual publish
attempt, since nothing had exercised the exported form before.

Also fixed `pnpm-workspace.yaml`'s unedited `esbuild: set this to true
or false` template line (the same defect WEB-3 fixed family-wide,
missed here), `package.json`'s `homepage` (now the confirmed live URL,
`https://sixarm.github.io/main-x-service.github.io/` — a bare
`main-x-service.github.io` isn't obtainable under the `SixArm` org;
GitHub Pages only serves that top-level form for a repo named
`<owner>.github.io` where `<owner>` genuinely is the account name),
and pinned `deploy.yml`'s `pnpm/action-setup` to `11.0.8` exact rather
than a floating `9`.

### Added

- `static/llms.txt` / `static/llms.json` — the website-appropriate
  versions the root `spec/llms-json-and-llms-txt/index.md` calls for:
  the same curated entries as the root files, with every `url`
  rewritten to its GitHub source (this site renders only overview
  pages, not a mirror of the whole tree), plus a new "This site"
  section for the four real routes.

## [0.1.0] - 2026-08-31

### Added

- Initial public documentation site: home, architecture, subprojects
  index, and about pages, per `spec/monorepo-github-pages/index.md`.
- `.github/workflows/deploy.yml` for the eventual `git subtree`-exported
  sibling repo.
