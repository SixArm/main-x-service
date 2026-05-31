# Release — agent guide

See [`../spec.md`](../spec.md) §9 for the formal versioning policy.

## Versioning

- Pre-1.0 (current): minor bumps MAY contain breaking API changes, per Cargo convention. Document them clearly under "Breaking" in the CHANGELOG entry.
- Post-1.0: strict SemVer.
- Default-weight or default-threshold changes count as behaviour changes. Bump minor and call out under "Behaviour Change" in `CHANGELOG.md`.
- The `0.4.0` release is the first under the geographic place-matcher domain. Subsequent minor bumps should preserve the 0.4.x public surface unless a deliberate breaking change is documented.

## `#[non_exhaustive]` implications

The following items carry `#[non_exhaustive]`:

- `Place` and `Address` — gaining fields is non-breaking. Downstream code MUST construct via `Place::builder()` and `Address::new()` rather than struct-literal syntax.
- `PlaceCategory` and `PlaceIdScheme` — gaining variants is non-breaking. Downstream `match` statements MUST include a `_ => …` arm.
- `MatchingError` — gaining variants is non-breaking.

Removing fields or variants from a `#[non_exhaustive]` item is breaking. Renaming a serde key is also breaking.

## Release checklist

Before publishing a new version:

- [ ] All scheduled work for this release is complete.
- [ ] `spec.md` reflects what the released code does (no drift). Spot-check that no `AGENTS/*.md` file contradicts `spec.md`.
- [ ] `CHANGELOG.md` has a new dated section for this version, with the changes grouped under `### Added`, `### Changed`, `### Breaking`, `### Removed`, `### Dependencies` as needed.
- [ ] `Cargo.toml` `version` matches the CHANGELOG.
- [ ] `cargo fmt --check` passes.
- [ ] `cargo clippy --all-targets -- -D warnings` passes.
- [ ] `cargo test` passes (unit + integration + doctest + property).
- [ ] `cargo doc --no-deps` builds with no warnings.
- [ ] `cargo publish --dry-run` succeeds.
- [ ] `cargo run` and `cargo run --example basic_usage` smoke-tested.

## Publishing

1. Commit the version bump and CHANGELOG entry as a single commit titled `Release v<X.Y.Z>`.
2. `cargo publish`.
3. `git tag v<X.Y.Z> -m "Release v<X.Y.Z>"` and `git push --tags`.
4. Create a GitHub release with the CHANGELOG section as the body.

## Smoke tests post-publish

After `cargo publish` succeeds:

1. In a scratch directory, `cargo new smoke && cd smoke && cargo add place-matcher@<X.Y.Z>`.
2. Replace `src/main.rs` with the quick-start example from `README.md`.
3. `cargo run` and confirm the printed output matches expectations.
4. `cargo doc --open` and confirm the rendered docs match the current spec.

## What goes in CHANGELOG vs spec

- **`CHANGELOG.md`**: what changed in this version. Past tense. Bullet points.
- **`spec.md`**: what the library *is*, right now. Present tense. Authoritative.

If a CHANGELOG entry doesn't have a corresponding spec edit, one of them is wrong.

## Yanking

- Yank a release (`cargo yank --version X.Y.Z`) only for correctness or licence / legal issues.
- Document the reason in the CHANGELOG entry for that version, marked `[YANKED]`.

## Dependency updates

- Patch-bump dependencies regularly; document in CHANGELOG under "Dependencies".
- Minor / major dependency bumps that change public behaviour require a behaviour-change note even if our crate's surface is unchanged.
- Run `cargo audit` before every release. Zero findings is the bar. Pin or yank if a transitive dependency is flagged.
- Current direct runtime dependencies (`Cargo.toml`): `serde`, `serde_json`, `unicode-normalization`, `strsim`, `thiserror`, `soundex`. No `tokio`, `async-std`, or other runtimes.
