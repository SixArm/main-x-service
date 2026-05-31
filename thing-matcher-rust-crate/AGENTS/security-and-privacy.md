# Security and privacy — agent guide

See [`../spec.md`](../spec.md) §8 for the formal determinism and safety guarantees.

## Scope

`thing-matcher` is a pure scoring library. A "thing" record (book, artwork, software, device, …) is rarely personal data in itself, but a `Thing` record can carry associated personal data via `owner` (a person's name) or `local_id` (a CRM key, customer-system reference, …). The crate handles this data only in-process and only as input to scoring — it does not persist, log, transmit, or otherwise observe it. Responsibility for lawful processing under any applicable data-protection regime sits with the caller.

## Hard rules

1. **No IO from library code.** No `std::fs`, no `std::net`, no `tokio`, no `reqwest`, no `tracing`, no `log`. `src/main.rs` is the only exception (it prints demo output).
2. **No global state.** No `static mut`, no `OnceCell` holding place data, no thread-local caches.
3. **No `unsafe`.** Enforced by `#![forbid(unsafe_code)]` in `lib.rs`.
4. **No real personal data in fixtures, examples, doctests, or comments.** Use synthetic names, RFC 2606 reserved `example.org` / `example.com` / `example.net` for URLs, and well-known cultural fixtures (Eiffel Tower, Pride and Prejudice, Big Ben) for canonical names.
5. **No logging.** Not even at debug level. If a downstream service wants to log, that is their decision and their threat model.
6. **No panics on user data.** Use `Option` / `Result`. If you must panic, panic on programmer error (an invariant violation), not on data error.
7. **No telemetry, no analytics.** No metrics, no opentelemetry, no anonymous usage reporting.

## Threat model (brief)

- **In scope:** correctness, side-effect freedom, deterministic output, absence of data-exfiltration paths.
- **Out of scope:** confidentiality of in-memory data (the caller owns the records), input validation of arbitrary semantic correctness, sanitising data that goes back to a database.

## Determinism as a safety property

The library is pure and deterministic: same inputs always produce the same outputs (`spec.md` §8). This makes it safe to use in privacy-sensitive contexts (the same input can be re-played for audit) and makes scoring decisions defensible. Do not introduce code paths that depend on clocks, RNGs, environment variables, or external services.

## Dependencies

- Each new dependency is a supply-chain attack surface. Justify it in the PR description.
- Prefer crates with permissive licences (MIT / Apache-2.0 / BSD) that are compatible with the project's own multi-licence offering.
- Avoid procedural macros that fetch at compile time, panic in macros, or pull in `build.rs` scripts of unknown provenance.
- Current direct runtime dependencies (`Cargo.toml`): `serde`, `serde_json`, `unicode-normalization`, `strsim`, `thiserror`, `soundex`. No `tokio`, `async-std`, or other runtimes.
- Run `cargo audit` before every release; zero findings is the bar. See [release.md](./release.md).

## Vulnerability reporting

- For library bugs that affect correctness, open a GitHub issue.
- For security-sensitive issues, email Joel Henderson at `joel@joelparkerhenderson.com` per `CONTRIBUTING.md`.

## Things that look innocuous but aren't

- Adding `tracing` "just for debugging." A logging framework plus `owner` strings or `local_id` keys on `Thing` records is a data spill waiting to happen.
- Adding `serde_json::to_writer(File::create(...)?, &thing)` "for diagnostics." That writes potentially-PII to disk.
- Caching normalised values in a `HashMap` for performance. Even in-process, this widens the lifetime of data the caller expected to be ephemeral.
- Adding telemetry via `metrics` or `opentelemetry`. Out of scope.
- Adding any deserialiser that accepts data from untrusted sources without an explicit byte-limit guard (the `serde_json::from_str` callers in the crate accept arbitrary nesting depth — the input side is the caller's risk perimeter).

## When you are unsure

Default to *not* adding the feature. If you genuinely believe it is needed, propose it in `spec.md` and let a human sign off.
