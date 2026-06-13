## 18. Change Control

Material changes to this spec — the adapter contract (§5.3), the wire
contract (§5.4), shared invariants (§5.5), composition requirements
(§6.1), compliance scope (§12) — MUST land in the same commit as the
corresponding code change, alongside the owning subproject's own spec
edit. That is the three-part-PR rule applied at two levels: a
seam-touching PR edits **both** this spec and the crate spec.

Routing rule for any edit:

| The change is about… | Edit… |
|---|---|
| A subproject's internals (fields, weights, modules, routes) | That subproject's spec only |
| How the trio composes, the DTO/wire contract, shared invariants | This spec **and** the affected subproject spec(s) |
| Entity-wide goals, compliance posture, roadmap priority | This spec |

Conflict resolution (restating the authority model): crate spec wins
on crate internals; this spec wins on the integration contract. A
discovered disagreement is filed as a §13 task — never silently
reconciled.

Bullet what changed, not how: every spec edit should be a diff a
reviewer can read in isolation. Avoid re-flowing surrounding
paragraphs in the same PR as a content change — keep stylistic churn
out of behavioural diffs.
