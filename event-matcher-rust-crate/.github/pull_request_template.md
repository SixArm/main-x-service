<!--
Thanks for contributing to Event-matcher.  See AGENTS.md, CONTRIBUTING.md,
and spec.md for the project conventions.
-->

## Summary

<!-- One or two sentences on what changed and why. -->

## Spec impact

<!--
This crate practises spec-driven development.  Any behaviour change in
`src/matcher.rs` (or any other library file under `src/`) MUST be
reflected in `spec.md` in the same PR; the `spec-drift` CI check
(`scripts/spec-drift-check.sh`) enforces this.

If your change is purely internal (refactor, test-only, tooling) and
genuinely does not warrant a spec update, add a path pattern to
`.spec-allow` and explain why here.
-->

- [ ] `spec.md` updated to reflect any behaviour change
- [ ] If `.spec-allow` was modified, the justification is documented above
- [ ] `CHANGELOG.md` entry added under `[Unreleased]`

## Test plan

<!--
List the tests / commands you ran:
- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`
- Any benches or examples relevant to the change
-->
