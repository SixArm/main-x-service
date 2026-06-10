## 19. Build, Tooling, and Release

### 19.1 Toolchain

Rust edition **2024**. Commands: `cargo build` / `cargo build --release` / `cargo test` (unit + integration + doctests) / `cargo clippy --all-targets -- -D warnings` / `cargo fmt` / `cargo run` (demo) / `cargo run --example basic_usage`. Full release discipline in [`AGENTS/release.md`](../AGENTS/release.md).

### 19.2 Release Procedure

Bump `Cargo.toml` per SemVer → update `CHANGELOG.md` → update this spec if behaviour or API changed → `cargo test` + `cargo clippy` + `cargo fmt --check` → `cargo publish --dry-run` then `cargo publish` → tag `v<version>` and push.

### 19.3 Versioning

- Pre-1.0: minor bumps MAY contain breaking changes (per Cargo convention) — document them prominently.
- Post-1.0: strict SemVer.

---

