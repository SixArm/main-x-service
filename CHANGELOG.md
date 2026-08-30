# Changelog

Repo-level change summary. **Each subproject's `CHANGELOG.md` is the
authoritative, detailed history for that subproject** (Keep a
Changelog format); this file records monorepo-wide events only.
Milestone narrative lives in [NEWS.md](NEWS.md).

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- course-service, place-service, and thing-service: real OpenTelemetry
  OTLP export (`tasks.md` PRO-H12 slices 1–3 of 7) — `src/observability.rs`,
  ported from person-service; the fourth, fifth, and sixth entity
  registries with a working exporter alongside person/worker/event
  (PRO-H9). place and thing both needed the `otlp-test-tonic`
  package-rename PRO-H9's three crates needed (despite showing `–` on
  `overview.md`'s gRPC-stub capability row — that row tracks a stub
  module, not a declared `tonic` dependency, and both already declare
  one for their own not-yet-built gRPC task) and both had stale,
  zero-consumer 0.27/0.28 OpenTelemetry dependency pins bumped to the
  family's settled 0.32/0.33 in the same change. See each crate's own
  `CHANGELOG.md` for the full record; `agents/share/{overview.md,
  observability.md,rust-tracing-opentelemetry-stack.md}` updated (also
  correcting a stale claim found while landing course — those docs
  still described worker/event as carrying PRO-H9's dead stub after
  PRO-H9 had already replaced both, 2026-08-28).

### Fixed

- `tasks.md` bookkeeping: **seven** Phase 8 items (PRO-P1, PRO-P8,
  PRO-P10, PRO-P15, PRO-P24, PRO-P28, PRO-P30) had already landed on
  `main` in earlier commits but were never checked off — found while
  working the queue in order, verified each against the actual
  commit/diff rather than trusting the title match, and closed with
  result notes. Also documented (`agents/share/rust-loco-stack.md`) that the family's
  "PostgreSQL NOT SQLite" rule holds at the runtime-driver level, not
  the compiled-code level: `loco-rs`'s own `with-db` feature hardcodes
  `sqlx-sqlite` for every loco-based crate regardless of what that
  crate's own manifest requests (found while verifying PRO-P28).

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

- `examples/compose/authentication-dev.yml`: a new family-reusable
  compose stack running authentication-service in
  `LOCO_ENV=development` — the only mode that logs a real magic-link
  URL to the console (SEC-A3) instead of silently emailing it, for
  anything that needs to complete a real passwordless sign-in without
  SMTP. person-front-end-with-svelte's live-integration suite is the
  first consumer (`tasks.md` PRO-P32; see that crate's own CHANGELOG
  for the full record).
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
