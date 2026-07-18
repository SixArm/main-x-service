## 14. Implementation Status

**Spec-only; no code yet.** This entity exists, as of 2026-06-18, only
as this entity-level specification. None of the three subprojects has
been scaffolded; there is no matcher crate, no service crate, and no
front-end. Every capability described in §6–§10 is a **target**, not a
delivered feature, and the §13 task list is the build-out backlog
(every box unchecked).

### 14.1 Delivered

| Subproject | Capability | Notes |
|---|---|---|
| (entity) | Canonical specification | This §1–§18 entity spec: domain model (§5, the canonical home) — the `WorkItem` type, the four matchable kinds, the kind gate, and the matchable/operational partition — the cross-subproject DTO contract, and the family-integration adoptions (cross-service links, bulk import/export) |

Nothing else is delivered. The matcher / service / front-end rows are
intentionally empty.

### 14.2 Open gaps

The entire build is open. The gap list is, in effect, §13 in full;
the headline gaps:

| Gap | Task |
|---|---|
| No matcher crate (no `WorkItem` type, no kind gate, no matching) | T-2 |
| No service crate (no per-collection CRUD, no matching endpoints, no sub-resources, no derived views, no roll-up) | T-3, T-4 |
| No audit log / event stream / PASETO token verification | T-5 |
| No front-end (no routes, no sub-resource workspaces) | T-6 |
| No cross-service link write-side (`entity_links`, `linked`/`unlinked`) | T-7 |
| No bulk import / export | T-8 |
| No OpenAPI / Swagger; no richer validation | T-9 |
| No durable event bus (MVP will ship in-memory, like the siblings) | §15 |
| No posts / comments / members sub-resources (deferred from the plan lineage) | §15 |
| No cross-service link **aggregator** (`link-graph-service`) — out of this trio's scope | §15 / [cross-service-linking.md](../../agents/share/cross-service-linking.md) |

When the trio is scaffolded (T-1), this section becomes an honest
delivered/gap snapshot like the sibling entities; until then it
records the spec-only status truthfully.
