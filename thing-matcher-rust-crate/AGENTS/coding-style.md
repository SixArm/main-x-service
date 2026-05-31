# Coding style — agent guide

Read alongside [`../spec.md`](../spec.md) §8 (determinism and safety) and §9 (public API contract).

## Formatting and linting

- `cargo fmt` is the source of truth for whitespace and line breaks.
- `cargo clippy --all-targets -- -D warnings` MUST be clean. Treat clippy lints as errors.
- If a lint is genuinely wrong, suppress it locally with `#[allow(clippy::lint_name)]` and add a one-line comment explaining why. Do not blanket-allow at module level.
- `#![forbid(unsafe_code)]` and `#![deny(missing_docs)]` live at the top of `src/lib.rs`. Do not remove them.

## Naming

- Types: `PascalCase` (`MatchingEngine`, `MatchBreakdown`).
- Functions / methods / locals / fields: `snake_case`.
- Constants: `SCREAMING_SNAKE_CASE`.
- Enum variants: `PascalCase` (`Combined`, `JaroWinkler`).
- Use en-GB spelling in documentation prose (`normalisation`, `behaviour`, `optimised`); use en-US spelling in code identifiers (`normalize_name`, `optimize`), matching the wider Rust crate ecosystem.

## Rust idioms

- Prefer `Option` / `Result` over panics. **No `unwrap`** in library code, ever. (Tests, examples, and `main.rs` may use `unwrap` on values that the test pins to a known shape — but prefer `expect("…")` with a message.)
- Prefer `match` over chains of `if let` when there are 3+ arms.
- Prefer iterator combinators (`.filter`, `.map`, `.collect`) over manual loops for transformation pipelines.
- Use `impl Into<String>` on builder setters (we do this already on `ThingBuilder`).
- Use `&str` for read-only string parameters; only take `String` when you need ownership.

## Error handling

- Library code returns `crate::Result<T>` (alias for `Result<T, MatchingError>`).
- New error variants go in `src/error.rs`. Update [`../spec.md`](../spec.md) §3.9 (error model) in the same change.
- Parsers and constructors that can reject input return `Option<T>` (e.g. `Identifier::new`); the matcher itself is infallible — empty fields or malformed URLs degrade to `None` in the breakdown rather than an error (`spec.md` §5.5).

## Doc comments

- Every `pub` item MUST have a `///` doc comment (enforced by `#![deny(missing_docs)]`).
- Module-level docs use `//!` and explain *purpose*, not *implementation*.
- Doctests on public items MUST compile. Use `# use thing_matcher::…;` hidden imports if needed.
- Prefer one-sentence summaries followed by a paragraph of detail. Avoid bullet-only doc blocks.
- Worked examples in doc comments SHOULD reuse the standard fixtures (Eiffel Tower / Tour Eiffel via Wikidata `Q243`, Pride and Prejudice via ISBN `9780141439518`, Big Ben) for cognitive economy.

## Inline comments

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

- Avoid generics unless they earn their keep. The crate is not a framework; small concrete types are easier to reason about and easier to audit.
- Where generics are warranted (e.g. `ThingBuilder::name<S: Into<String>>(…)`), keep the bound minimal and document the intent.

## Dependencies

- New runtime dependencies require a justification in the PR description and a note in [`../spec.md`](../spec.md) if they expand the trust boundary.
- Current direct runtime dependencies: `serde`, `serde_json`, `unicode-normalization`, `strsim`, `thiserror`, `soundex`. No `tokio`, `async-std`, or other runtimes.
- Dev-only dependencies (e.g. `proptest`, `criterion`) are lower-risk but should still be deliberate.
