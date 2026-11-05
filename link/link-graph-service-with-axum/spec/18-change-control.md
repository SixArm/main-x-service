## 18. Change Control

### 18.1 Spec is the source of truth

When code and this spec disagree, the spec wins. Bring the code in line
by opening a task in [§13](13-tasks.md); do not silently rewrite the
spec to match the code.

### 18.2 Design docs are upstream of this spec

This service realises two **shared** design docs. They sit *above* this
spec:

- [`cross-service-linking.md`](../../../agents/share/cross-service-linking.md)
  — the contracts (`EntityRef`, edge-kind registry, topology, integrity
  lifecycle, governance).
- [`event-bus.md`](../../../agents/share/event-bus.md) — the envelope and
  delivery semantics.

A change to either of those docs **propagates down** to this spec: if a
contract changes upstream, open a §13 task to follow it here. Do not
diverge from the shared contracts in this spec alone — the
`EntityRef` format and the edge-kind registry are frozen centrally
(design §3, §9), copied per project, and any drift is a bug.

### 18.3 Three-part PRs

A behavioural change is **one PR with three parts**:

1. **Spec edit** — the relevant numbered section here (and §13 task
   status).
2. **Code edit** — `src/` (once the crate exists).
3. **Test edit** — the matching tier in [§11](11-testing-strategy.md).

A change that also touches a shared contract is **four parts**: the
shared design doc edit comes first, then the three parts above follow
it.

### 18.4 The partition rule is load-bearing

Any edit that would make a cross-service link a matcher signal, or store
one in a within-entity `relationships` field, is **rejected** — it
violates the partition rule
([design §7](../../../agents/share/cross-service-linking.md#7-relationship-to-within-entity-matching-the-partition-rule)).
Cross-service links live only in this service, the per-service
`entity_links` write-side, and the `linked` / `unlinked` events.

### 18.5 The edge-kind registry is closed

Adding an edge kind is a deliberate change: a new row in the design §9
registry first, then a copied registry-row + endpoint-type pair +
inverse here (§5.4), plus tests. It is never an ad-hoc per-deployment
extension.

### 18.6 Versioning

[Semantic Versioning](https://semver.org/spec/v2.0.0.html); the
[`CHANGELOG.md`](../CHANGELOG.md) follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). The roadmap
in [§15](15-roadmap.md) maps versions to rollout stages.
