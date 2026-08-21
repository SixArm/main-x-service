# Security and Privacy — Agent Guide

This is an identity-adjacent library. Treat every line of code as if a worker's care might depend on it (because it might).

## Hard Rules

1. **No IO from library code.** No `std::fs`, no `std::net`, no `tokio`, no `reqwest`, no `tracing`, no `log`. `src/main.rs` is the only exception (it prints demo output).
2. **No global state.** No `static mut`, no `OnceCell` holding worker data, no thread-local caches.
3. **No `unsafe`.** Full stop (enforced by `#![forbid(unsafe_code)]` in `lib.rs`).
4. **No PII in fixtures, examples, doctests, or comments.** Use the synthetic names in `examples/` and `tests/`. National identifiers must round-trip through their parsers as illustrative-only values (see `agents/testing.md` Fixtures section).
5. **No logging.** Not even at debug level. If a downstream service wants to log, that is their decision and their threat model.
6. **No panics on user data.** Use `Option`/`Result`. If you must panic, panic on programmer error (an invariant violation), not data error.
7. **No unwrapping of national identifiers.** A malformed UK NHS Number, France NIR, España TSI, Éire IHI, UK NI H&C Number, or US SSN is normal user input, not a bug. Parsers return `Option<String>`; honour that contract.
8. **Identifiers are scheme-local.** An NHS Number and an H&C Number that happen to share the same 10 digits refer to different workers in different registries. Do not cross-match them — the matcher's `deterministic_match` and per-scheme `MatchBreakdown` fields encode this; do not regress it.

## Threat Model (Brief)

- **In scope:** correctness, side-effect freedom, deterministic output.
- **Out of scope:** confidentiality of in-memory data (the caller owns the records), input validation of valid date ranges, sanitising data that goes back to a database.

## GDPR and Equivalent Regimes

The library handles personal data passed to it by the caller but does not store, log, or transmit it. Responsibility for lawful processing under GDPR (or any equivalent data-protection regime that applies to the consumer) sits with the calling application. Do not introduce code that changes this stance.

## Data Safety

- Probabilistic matches are *suggestions*, never decisions.
- Every `MatchResult` carries a `MatchBreakdown`. Downstream apps SHOULD surface it to operators, not just the boolean `is_match`.
- Default threshold (0.85) is conservative. If you propose lowering it, justify with data and discuss in a `spec.md` PR.

## Dependencies

- Each new dependency is a supply-chain attack surface. Justify it in the PR description.
- Prefer crates with permissive licences (MIT/Apache-2.0/BSD) that are compatible with the project's own dual licence.
- Avoid procedural macros that fetch at compile time, panic in macros, or pull in `build.rs` scripts of unknown provenance.

## Vulnerability Reporting

- For library bugs that affect correctness or safety, open a GitHub issue.
- For security-sensitive issues, email Joel Henderson at `joel@joelparkerhenderson.com` per `CONTRIBUTING.md`.

## Things That Look Innocuous But Aren't

- ❌ Adding `tracing` "just for debugging." Logging frameworks plus PII = data spill.
- ❌ Adding `serde_json::to_writer(File::create(...)?, &worker)` "for diagnostics." That writes PII to disk.
- ❌ Caching normalised values in a `HashMap` for performance. Even in-process, this widens the lifetime of PII.
- ❌ Adding telemetry via `metrics` or `opentelemetry`. Out of scope.

## When You Are Unsure

Default to *not* adding the feature. If you genuinely believe it is needed, propose it in `spec.md` and let a human sign off.
