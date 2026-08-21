## 19. Build, Tooling, and Release

### 19.1 Toolchain

Rust edition **2024**. Commands: `cargo build` / `cargo build --release` / `cargo test` (unit + integration + doctests) / `cargo clippy --all-targets -- -D warnings` / `cargo fmt` / `cargo run` (demo) / `cargo run --example basic_usage`. Full release discipline in [`agents/release.md`](../agents/release.md).

### 19.2 Release Procedure

Bump `Cargo.toml` per SemVer → update `CHANGELOG.md` → update this spec if behaviour or API changed → `cargo test` + `cargo clippy` + `cargo fmt --check` → `cargo publish --dry-run` then `cargo publish` → tag `v<version>` and push.

### 19.3 Versioning

- Pre-1.0: minor bumps MAY contain breaking changes (per Cargo convention) — document them prominently.
- Post-1.0: strict SemVer.

### 19.4 Spec-Drift CI Check (T-7)

`.github/workflows/spec-drift.yml` runs on every pull request to `main`
and invokes `scripts/spec-drift-check.sh`: any `src/matcher.rs` change
in the diff MUST be accompanied by a `spec/` update in the same PR, or
the PR fails. Path-pattern exceptions live in `.spec-allow` (ships
empty). Runnable locally pre-push: `bash scripts/spec-drift-check.sh
main HEAD`. This is the mechanical backstop for the spec-first
discipline in [`agents/spec-driven-development.md`](../agents/spec-driven-development.md)
— it catches a code change with no matching spec edit; it does not
catch the opposite failure mode (a spec section describing behaviour
the code doesn't have), which is a manual audit's job.

---

