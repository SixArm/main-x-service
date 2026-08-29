# Changelog

Repo-level change summary. **Each subproject's `CHANGELOG.md` is the
authoritative, detailed history for that subproject** (Keep a
Changelog format); this file records monorepo-wide events only.
Milestone narrative lives in [NEWS.md](NEWS.md).

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Fixed

- `.github/workflows/ci.yml`'s `supply-chain` job now runs the same
  `scripts/ci-check.sh evidence` stage `.woodpecker.yml` does, closing
  a CI-platform divergence that made root AGENTS.md's "byte-identical
  commands" claim false (`tasks.md` PRO-R5).
- Root `AGENTS.md` + `agents/share/overview.md` doc corrections
  (PRO-R6): the "matchers run to §25" claim was false for six of nine
  matcher crates (place/thing/event stop at §13; organization/
  care-pathway/case/portfolio are each still one `spec/index.md`) —
  replaced with the real spread. The Library-crates tables claimed
  entity-ref was "not yet published to crates.io"; checking crates.io
  directly found it **was** published (`entity-ref` 0.2.0, 2026-08-05)
  — fixed that row and added the missing note that integrity-mac, by
  contrast, genuinely isn't published.

### Added

- Dependabot enabled per `spec/dependabot`: GitHub Dependabot security
  updates (`automated-security-fixes`) turned on at the repo level
  (vulnerability alerts were already on), plus a generated
  `.github/dependabot.yml` covering every one of the ~64 Cargo
  workspaces (main crate + `migration/` + `fuzz/` sub-crates, since
  this repo has no root `Cargo.toml` for Dependabot to walk on its
  own) and all 16 SvelteKit front-ends' `npm` packages, plus
  `github-actions`, on a weekly schedule with routine minor/patch
  updates grouped per directory. `scripts/dependabot-generate.sh`
  regenerates the file from the live tree (same discovery pattern as
  `scripts/ci-crates.sh`) so it stays in sync as crates/front-ends are
  added, removed, or moved.
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
