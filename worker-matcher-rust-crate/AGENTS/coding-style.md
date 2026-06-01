# Coding Style — Agent Guide

## Formatting and Linting

- `cargo fmt` is the source of truth for whitespace and line breaks.
- `cargo clippy --all-targets -- -D warnings` MUST be clean. Treat clippy lints as errors.
- If a lint is genuinely wrong, suppress it locally with `#[allow(clippy::lint_name)]` and add a one-line comment explaining why. Do not blanket-allow at module level.

## Naming

- Types: `PascalCase` (`MatchingEngine`, `MatchBreakdown`).
- Functions / methods / locals / fields: `snake_case`.
- Constants: `SCREAMING_SNAKE_CASE`.
- Enum variants: `PascalCase` (`Male`, `Combined`).
- Use en-GB spelling in documentation prose (`normalisation`, `behaviour`, `optimised`); use en-US spelling in code identifiers (`normalize_name`, `optimize`), matching the wider Rust crate ecosystem.
  - Rationale: the prose voice is chosen for editorial consistency; the code identifiers follow ecosystem convention. Mixing the two within a single layer forces the next reader to spend energy resolving the inconsistency.

## Rust Idioms

- Prefer `Option`/`Result` over panics. **No `unwrap`** in library code, ever. (Tests, examples, and `main.rs` may use `unwrap` on values that the test pins to a known shape — but prefer `expect("...")` with a message.)
- Prefer `match` over chains of `if let` when there are 3+ arms.
- Prefer iterator combinators (`.filter`, `.map`, `.collect`) over manual loops for transformation pipelines.
- Use `impl Into<String>` on builder setters (we do this already on `WorkerBuilder`).
- Use `&str` for read-only string parameters; only take `String` when you need ownership.

## Error Handling

- Library code returns `crate::Result<T>` (alias for `Result<T, MatchingError>`).
- New error variants go in `src/error.rs`. Update [`../spec.md`](../spec.md) §15 in the same change.
- Do not `unwrap()` `NHSNumber::from_str`. If parsing fails, return `None` from a scorer — never a 0.0 score with a discarded error.

## Doc Comments

- Every `pub` item MUST have a `///` doc comment.
- Module-level docs use `//!` and explain *purpose*, not *implementation*.
- Examples in doctests should compile. Use `# use worker_matcher::...;` hidden imports if needed.
- Prefer one-sentence summaries followed by a paragraph of detail. Avoid bullet-only doc blocks.

## Inline Comments

- Default to **no comments**. Code should be self-explanatory through naming.
- Add a comment only when *why* is non-obvious: a deliberate choice that surprises the next reader.
- Do not narrate *what* the code does (`// increment counter`) — the code says that.
- Do not reference tasks or PRs in comments (those rot). Use git history.

## Functions

- Keep public functions small. If a method on `MatchingEngine` exceeds ~30 lines, extract a helper.
- Helper methods inside `matcher.rs` may freely take `&self` for access to config; do not pass config through as an argument.

## Visibility

- Default to `pub(crate)` for items that span modules but are not part of the public API.
- Reserve `pub` for items that should be re-exported from `lib.rs`. If it's `pub` and not re-exported, fix one of the two.

## Tests

See [`testing.md`](./testing.md).

## Generics

- Avoid generics unless they earn their keep. The crate is not a framework; small concrete types are easier to reason about for identity-adjacent code.
- Where generics are warranted (`WorkerBuilder::uk_nhs_number<S: Into<String>>(...)`, `NicknameTable::with_class<I, S>(...)`), keep the bound minimal and document the intent.

## Dependencies

- New runtime dependencies require a justification in the PR description and a note in [`../spec.md`](../spec.md) §22 risks if they expand the trust boundary.
- Dev-only dependencies (e.g. `proptest`, `criterion`) are lower-risk but should still be deliberate.
