# Changelog

Repo-level change summary. **Each subproject's `CHANGELOG.md` is the
authoritative, detailed history for that subproject** (Keep a
Changelog format); this file records monorepo-wide events only.
Milestone narrative lives in [NEWS.md](NEWS.md).

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- `.github/FUNDING.yml`: GitHub Sponsors enabled (`joelparkerhenderson`).
  Open Collective intentionally omitted — no collective exists yet.
  CONTRIBUTING.md and NEWS.md updated to match (`spec/free-open-source-funding`).
- Documented the Trusted Publishing intent (`spec/trusted-publishing`):
  a "Publishing" section in README.md and SECURITY.md stating the
  current manual `cargo publish` reality and the plan to adopt
  OIDC-based publishing once it's production-ready across every forge
  and target we use. No publishing behaviour changed.
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
- Tightened the Rust MSRV policy from N-3 to **N-2**: the floor moves
  from 1.95 to **1.96**, `ci/msrv.txt` and all 46 non-`fuzz`
  `Cargo.toml` manifests updated, `spec/rust-msrv-n-minus-3/` replaced
  by `spec/rust-msrv-n-minus-2/` (PRO-H13).
- `spec/index.md` now lists the geo-naming, decimal-coordinate,
  serde-float, and special-files specs.

### Removed

- `task.org` (committed editor scratch) from the repo root (PRO-R4).
