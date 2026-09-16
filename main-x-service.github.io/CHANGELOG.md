# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
missed here), `package.json`'s `homepage` (was a stale project-page
URL from before the domain decision; now `https://main-x-service.github.io/`),
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
