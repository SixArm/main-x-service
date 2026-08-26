# Changelog

Repo-level change summary. **Each subproject's `CHANGELOG.md` is the
authoritative, detailed history for that subproject** (Keep a
Changelog format); this file records monorepo-wide events only.
Milestone narrative lives in [NEWS.md](NEWS.md).

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- Root special files per `spec/special-files-for-public-repos`:
  LICENSE.md, CITATION.cff, CONTRIBUTING.md, SECURITY.md,
  MAINTAINERS.md, GOVERNANCE.md, CODEOWNERS, AI_STATEMENT.md, NEWS.md,
  INSTALL.md, COMPARISONS.md, BENCHMARKS.md, RFC.md, and this file
  (2026-08-26, tasks.md PRO-R3).
- `tasks.md` Phase 8: the professionalization audit queue (2026-08-26)
  — full-tree verification snapshot plus 35 evidence-backed tasks.

### Changed

- Finished the `spec/` reorg: `agents-directory-name-is-lowercase` and
  `rust-msrv-n-minus-3` are directories with `index.md`; all
  repo-wide references swept; the `docs` CI stage exclusion updated
  (PRO-R1/PRO-R2).
- `spec/index.md` now lists the geo-naming, decimal-coordinate,
  serde-float, and special-files specs.

### Removed

- `task.org` (committed editor scratch) from the repo root (PRO-R4).
