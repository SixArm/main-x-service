## 19. Build, Tooling, and Release

### 19.1 Toolchain

Rust edition **2024**. Standard `cargo build` / `cargo build --release` / `cargo test` (unit + integration + doctests) / `cargo clippy --all-targets -- -D warnings` / `cargo fmt`. Demo: `cargo run` (`src/main.rs`). Examples: `cargo run --example basic_usage`, `cargo run --example custom_config`.

### 19.2 Release Procedure

(1) Update `Cargo.toml` per SemVer; (2) `CHANGELOG.md` dated section; (3) update this spec if behaviour / API changed; (4) `cargo test` / `cargo clippy` / `cargo fmt --check`; (5) `cargo publish --dry-run` then `cargo publish`; (6) tag `v<version>` and push.

### 19.3 Versioning

Pre-1.0: minor bumps MAY contain breaking changes (per Cargo convention) — document prominently. Post-1.0: strict SemVer.

---

