## 18. Change Control

Material changes to this spec — the trio composition, the service ↔
matcher DTO contract (§5.3), shared invariants (§5.5), integration
requirements (FR-19–FR-21), compliance scope — MUST land in the same
commit as the corresponding code change. Three-part PRs: spec edit +
code edit + test edit.

Routing rules:

- **Crate-internal change** (a field's type, a weight, an endpoint's
  body shape) → edit the **crate spec**; touch this spec only if the
  integration contract moves.
- **Contract change** (adapter routing rule, envelope shape, a field
  crossing the service↔front-end wire, an invariant in §5.5) → edit
  **this spec** *and* the affected crate spec(s) in the same PR; the
  bridge tests or wire-type tests change with it.
- **Disagreement found** → open a task in [§13](13-tasks.md) (entity)
  or the crate's §13; do not silently rewrite either spec. Crate spec
  wins on internals; this spec wins on the contract.

Bullet what changed, not how: every spec edit should be a diff a
reviewer can read in isolation.
