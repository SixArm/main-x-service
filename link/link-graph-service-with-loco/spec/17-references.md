## 17. References

### 17.1 Canonical design (source of truth for this service)

- [`cross-service-linking.md`](../../../agents/share/cross-service-linking.md)
  — the cross-service entity-linking design; this service is its §4.3
  **read-model aggregator** (the read side of the hybrid topology).
  Fixes `EntityRef` (§3), topology (§4), integrity lifecycle (§5),
  freshness (§6), the partition rule (§7), reconciliation (§8), the
  closed edge-kind registry (§9), `case ↔ person` governance (§10), and
  rollout (§11).
- [`event-bus.md`](../../../agents/share/event-bus.md) — the durable
  event bus this service consumes; transactional outbox (§3), canonical
  envelope (§4), publisher seam (§5), delivery semantics (§6), topics /
  partitioning / config (§7), and the §9 consumer model this service
  realises.

### 17.2 Family shared docs

- [overview.md](../../../agents/share/overview.md) — family overview.
- [architecture.md](../../../agents/share/architecture.md) — family
  architecture conventions.
- [dataflow.md](../../../agents/share/dataflow.md) — create / match /
  merge / search flows (this service is downstream of all of them).
- [auditability.md](../../../agents/share/auditability.md) — audit log +
  event streaming conventions.
- [privacy.md](../../../agents/share/privacy.md) — masking, GDPR,
  consent.
- [match.md](../../../agents/share/match.md) — matching (referenced only
  to assert the **partition rule**: cross-service links are never a
  match signal, design §7).
- [jwt-enforcement.md](../../../agents/share/jwt-enforcement.md) —
  blanket `/api/*` JWT enforcement (coordinated family rollout).
- [compliance-for-healthcare.md](../../../agents/share/compliance-for-healthcare.md)
  / [compliance-for-technology.md](../../../agents/share/compliance-for-technology.md).
- [rust-loco-stack.md](../../../agents/share/rust-loco-stack.md) —
  Rust + Loco stack; [loco.md](../../../agents/share/loco.md) — Loco
  conventions; [postgresql.md](../../../agents/share/postgresql.md) —
  PostgreSQL.
- [observability.md](../../../agents/share/observability.md) /
  [rust-tracing-opentelemetry-stack.md](../../../agents/share/rust-tracing-opentelemetry-stack.md).

### 17.3 Participating services (producers this service consumes)

- [person-service-with-loco](../../../person/person-service-with-loco/spec/index.md)
  + [worker-service-with-loco](../../../worker/worker-service-with-loco/spec/index.md)
  — the `same_identity` backbone.
- [organization-service-with-loco](../../../organization/organization-service-with-loco/spec/index.md)
  — affiliation targets.
- [case-service-with-loco](../../../case/case-service-with-loco/spec/index.md)
  — the high-governance `subject_of` / `about` producer (§12).
- [authentication-verifier-rust-crate](../../../authentication/authentication-verifier-rust-crate/spec/index.md)
  — offline PASETO v4.public verification.

### 17.4 External

- [Fluvio](https://www.fluvio.io/) — the bus transport.
- [schema.org](https://schema.org/) — entity vocabularies the producing
  services model.
