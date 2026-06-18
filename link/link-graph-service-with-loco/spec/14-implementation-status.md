## 14. Implementation Status

**Spec-only; no code yet.**

This service exists as a specification, scaffolded from the canonical
design doc
[`cross-service-linking.md`](../../../agents/share/cross-service-linking.md)
(it is the §4.3 read-model aggregator) and the
[`event-bus.md`](../../../agents/share/event-bus.md) §9 consumer model.
No Rust crate, no `Cargo.toml`, no migrations, no controllers, no tests
have been written. Every task in §13 is unchecked.

### 14.1 Upstream prerequisites (not in this crate)

This service consumes contracts that are themselves at design / rollout
stage:

- The **durable event bus** ([event-bus.md](../../../agents/share/event-bus.md))
  is a design doc; today's transport is in-memory and volatile, which is
  why the integrity story has the interim **lazy verify-on-read** path
  (§6 FR-11). Until topics go durable, replay-based rebuild (§7 NFR-4)
  and event-driven verification are partial.
- The **`linked` / `unlinked` events** and per-service **`entity_links`**
  write-side (design §4.1/§4.2) must land in **person** + **worker**
  first (the `same_identity` backbone, design §11 step 2) before this
  aggregator has anything to consume.
- The **`EntityRef`** value type and **edge-kind registry** are shared
  *contracts* copied per project (design §3, §9); this crate copies them
  (T-2, T-3) rather than depending on a shared package.

### 14.2 Build order

Per design §11: contracts (T-1..T-4) → consume/project the
`same_identity` backbone (T-5..T-9) → interim verify (T-10) → reads
(T-11..T-15) → affiliations + `case ↔ person` governance (T-16..T-18) →
hardening / durable-bus flip (T-19..T-23). The in-memory default and
lazy verify-on-read mean the aggregator can stand up before every
topic is durable.

### 14.3 Family registration

When the crate is scaffolded, register it in the repo-root
[`AGENTS.md`](../../../AGENTS.md) service-crate table and
[`agents/share/overview.md`](../../../agents/share/overview.md) so the
umbrella docs reflect the new cross-cutting service. (Tracked as a
follow-up to T-1.)
