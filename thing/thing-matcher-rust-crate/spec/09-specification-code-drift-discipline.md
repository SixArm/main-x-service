## 9. Specification / code drift discipline

When the spec disagrees with the code, the spec wins **only after** the disagreement is resolved in writing: an Open Question in §10 OR a PR that updates one of {spec, code, tests} in the same commit set as the other two. (§13 in this crate's spec is [References](./13-references.md), not a task queue — this crate carries no `plan.md`/`tasks.md`; per `agents/spec-driven-development.md`'s document hierarchy, outstanding work is consolidated into §10.)

Three-part-PR rule: a behavioural change is one PR that contains a spec edit, a code edit, and a test edit. PRs touching `src/matcher.rs` or `src/scorer.rs` without a corresponding spec edit MUST be flagged in review.

---


### Adapter-Contract Tests (CI guardrail)

`tests/adapter_contract.rs` (11 tests) pins the public-API surface
that downstream service adapters depend on. Every public builder method,
every `MatchingEngine` entry point, every `MatchBreakdown` field, every
`MatchConfig` preset, and every enum variant the downstream calls is
touched. Renaming or removing any of these symbols fails the matcher's
own CI before publish — making cross-crate breakage deliberate. See
[`agents/testing.md`](../agents/testing.md) for the per-section breakdown.

