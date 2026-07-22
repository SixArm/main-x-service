## 14. Implementation Status

**All three subprojects implemented** (2026-06-19; originally
spec-only as of 2026-06-18). The matcher crate, the service crate, and
the SvelteKit front-end are built, tested, and clippy-clean:
**kind-agnostic** matching (no kind gate) over one recursive `Plan`;
one `/api/plans` REST collection over one `plans` table (nullable
`kind`, nullable `parent_pid`) with CRUD + matching + merge + audit +
durable-outbox events + PASETO/ABAC auth (default-off) + the PPM
Governance / Visibility / Strategy phases; and the operator SPA (SVAR
grid / Kanban / Gantt, Lily chrome, 13-locale i18n). The four former
work-item kinds were unified into the recursive `Plan` on 2026-07-20
(§13 T-10). Consult each subproject's own spec §13/§14 for what remains
open (service: operational sub-resources + derived views, deduplicate +
review queue, links, bulk, Tantivy, privacy; see the tables below for
per-subproject detail).

### 14.1 Delivered

| Subproject | Capability | Notes |
|---|---|---|
| (entity) | Canonical specification | This §1–§18 entity spec: domain model (§5, the canonical home) — the recursive `Plan` type, its optional `kind` label, kind-agnostic matching, and the matchable/operational partition — the cross-subproject DTO contract, and the family-integration adoptions (cross-service links, bulk import/export) |

Nothing else is delivered. The matcher / service / front-end rows are
intentionally empty.

### 14.2 Open gaps

The entire build is open. The gap list is, in effect, §13 in full;
the headline gaps:

| Gap | Task |
|---|---|
| No matcher crate (no `Plan` type, no kind-agnostic matching) | T-2 |
| No service crate (no plans-collection CRUD, no matching endpoints, no sub-resources, no derived views, no roll-up) | T-3, T-4 |
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
