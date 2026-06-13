## 18. Change Control

Material changes to this spec — the DTO contract (§5.3), shared
invariants (§5.4), the FR / NFR tables, the REST or route summary
(§9), compliance scope (§12) — MUST land in the same commit as the
corresponding code and test changes (three-part PRs).

Scope rules:

- **Crate internals change** (a field, a weight, an endpoint): edit
  the owning crate's spec; touch this spec only if the integration
  contract or an entity-wide summary here mentions it.
- **Integration contract changes** (adapter mapping, REST surface the
  front-end consumes, confidence vocabulary): this spec is
  authoritative — edit it first, update the crate specs and the
  bridge tests in the same PR.
- **Disagreement found**: open a §13 task naming the loser; do not
  silently rewrite either document.

Bullet what changed, not how: every spec edit should be a diff a
reviewer can read in isolation.
