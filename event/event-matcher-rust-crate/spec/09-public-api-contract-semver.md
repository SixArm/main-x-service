## 9. Public API contract (SemVer)

The crate follows Semantic Versioning. Pre-1.0, minor bumps MAY contain breaking API changes (Cargo convention) and MUST be documented under "Breaking" in the CHANGELOG entry for that version.

- **0.4.0** is the first release of `Event-matcher` under the geographic Event-matcher domain. The 0.3.x line targeted a different domain and is not upgrade-compatible.
- The set of items re-exported from `lib.rs` is the public surface. Adding to it is non-breaking; removing or renaming is breaking.
- Default-weight or default-threshold changes count as observable behaviour changes and MUST be documented under "Behaviour Change" in the CHANGELOG. They MAY require a minor bump.
- Every behavioural change SHOULD ship with a new CHANGELOG entry (under "Unreleased" until the next release).

### 9.1 `#[non_exhaustive]` items

The following items carry `#[non_exhaustive]`:

- `Place` and `Address` — adding fields is non-breaking. Downstream code MUST construct via the builder / `new` rather than struct-literal syntax.
- `PlaceCategory` and `PlaceIdScheme` — adding variants is non-breaking. Downstream `match` statements MUST include a `_ => ...` arm.
- `MatchingError` — adding variants is non-breaking.

Removing fields or variants is breaking.

### 9.2 JSON shape stability

Every public data type derives `serde::{Serialize, Deserialize}`. `MatchConfig` carries `#[serde(default)]` at struct level so partial documents inherit defaults from `MatchConfig::default()`. `MatchBreakdown::email_score` and `MatchResult::confidence` carry `#[serde(default)]` to allow legacy payloads predating those fields to round-trip.

Renaming a JSON key or changing its type is breaking.

### 9.3 Code wins on divergence

When `spec.md` and the code disagree, the **code** is what is shipped to users — the spec is updated to match the code, and the divergence is flagged in the CHANGELOG so a human can decide whether the spec's original intent should be restored in a follow-up.

---


### Adapter-Contract Tests (CI guardrail)

`tests/adapter_contract.rs` (14 tests) pins the public-API surface
that downstream service adapters depend on. Every public builder method,
every `MatchingEngine` entry point, every `MatchBreakdown` field, every
`MatchConfig` preset, and every enum variant the downstream calls is
touched. Renaming or removing any of these symbols fails the matcher's
own CI before publish — making cross-crate breakage deliberate. See
[`AGENTS/testing.md`](../AGENTS/testing.md) for the per-section breakdown.

