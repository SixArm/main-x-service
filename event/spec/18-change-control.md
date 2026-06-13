## 18. Change Control

Material changes to this spec — the service ↔ matcher DTO contract
(§5.3), shared invariants (§5.5), the FR/NFR tables, the `/api/v1`
versioning rule (§8.3), compliance scope (§12) — MUST land in the
same commit as the corresponding code change, alongside a seam-test
change where the integration contract moved (§11.4).

Authority recap (full statement in [`index.md`](index.md)):

- **Crate internals** — the crate's own spec wins; fix this document
  if it drifted.
- **Integration contract** — this document wins; open a task in the
  losing crate's §13 to bring it in line.
- Never silently rewrite either side; the disagreement itself goes
  through a task.

A change that touches two subprojects (e.g. a field added to the
wire format: service model + front-end types) is still **one PR**
with three parts per subproject affected: spec edit + code edit +
test edit.

Bullet what changed, not how: every spec edit should be a diff a
reviewer can read in isolation.
