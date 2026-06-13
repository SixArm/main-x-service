## 9. Specification / code drift discipline

When the spec disagrees with the code, the spec wins **only after** the disagreement is resolved in writing: a task in §13 OR a PR that updates one of {spec, code, tests} in the same commit set as the other two.

Three-part-PR rule: a behavioural change is one PR that contains a spec edit, a code edit, and a test edit. PRs touching `src/matcher.rs` or `src/scorer.rs` without a corresponding spec edit MUST be flagged in review.

---


### Adapter-Contract Tests (CI guardrail)

`tests/adapter_contract.rs` (10 tests) pins the public-API surface
that downstream service adapters depend on. Every public builder method,
every `MatchingEngine` entry point, every `MatchBreakdown` field, every
`MatchConfig` preset, and every enum variant the downstream calls is
touched. Renaming or removing any of these symbols fails the matcher's
own CI before publish — making cross-crate breakage deliberate. See
[`AGENTS/testing.md`](../AGENTS/testing.md) for the per-section breakdown.

