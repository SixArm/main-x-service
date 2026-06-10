# Release — Agent Guide

See [`../spec.md`](../spec/index.md) §19 for the formal procedure.

## Versioning

- Pre-1.0 (current): minor bumps MAY contain breaking API changes, per Cargo convention. Document them clearly.
- Post-1.0: strict SemVer.
- Default-weight or default-threshold changes count as behaviour changes. Bump minor and call out under "Behaviour Change" in `CHANGELOG.md`.

## Release Checklist

Before publishing a new version:

- [ ] All open §23 tasks scheduled for this release are checked off.
- [ ] `spec.md` is updated to reflect what the released code does (no drift).
- [ ] `CHANGELOG.md` has a new dated section for this version.
- [ ] `Cargo.toml` `version` matches the CHANGELOG.
- [ ] `cargo fmt --check` passes.
- [ ] `cargo clippy --all-targets -- -D warnings` passes.
- [ ] `cargo test` passes.
- [ ] `cargo doc --no-deps` builds with no warnings.
- [ ] `cargo publish --dry-run` succeeds.
- [ ] `README.md` examples still compile (they're doctests in spirit; run `cargo test --doc`).

## Publishing

1. Commit the version bump and CHANGELOG entry as a single commit titled `Release v<X.Y.Z>`.
2. `cargo publish`.
3. `git tag v<X.Y.Z> -m "Release v<X.Y.Z>"` and `git push --tags`.
4. Create a GitHub release with the CHANGELOG section as the body.

## What Goes In CHANGELOG vs Spec

- **CHANGELOG.md**: what changed in this version. Past tense. Bullet points.
- **spec.md**: what the library is, right now. Present tense. Authoritative.

If a CHANGELOG entry doesn't have a corresponding spec edit, one of them is wrong.

## Yanking

- Yank a release (`cargo yank --version X.Y.Z`) only for data-safety bugs or licence/legal issues.
- Document the reason in the CHANGELOG entry for that version, marked `[YANKED]`.

## Dependency Updates

- Patch-bump dependencies regularly; document in CHANGELOG under "Dependencies".
- Minor/major dependency bumps that change public behaviour require a behaviour-change note even if our crate's surface is unchanged.
- Run `cargo audit` before every release. Zero findings is the bar. Pin or yank if a transitive dep is flagged.
- Current direct dependencies (0.6.0): `jiff ^0.2`, `serde ^1.0`, `serde_json ^1.0`, `unicode-normalization ^0.1`, `strsim ^0.11`, `thiserror ^2.0`, `soundex ^0.2`, `united-kingdom-national-health-service-number ^1.0` (aliased upstream `nhs-number ^1.0`). No `tokio`, `async-std`, or other runtimes.
