# entity-ref

The one shared **contract** for cross-service entity linking in the Main X
Index family — `agents/share/cross-service-linking.md` §3 (the `EntityRef`
value type) and §9 (the v1 edge-kind registry). Rollout **step 1**: land
the contracts, no behaviour yet.

A record in another service is named by an opaque URN string
`"<entity_type>:<uuid>"`, e.g. `person:0c4f1e2a-…`. This crate owns the
parsing/validation and the static metadata; it is pure, panic-free, and
dependency-light (serde + uuid + thiserror). The rollout note below once
framed this as copyable per project until a second non-aggregator
consumer justified a shared dependency — that threshold has since been
crossed several times over; see "Who actually consumes this" below.

```rust
use entity_ref::{EdgeKind, EntityRef, EntityType, Sensitivity};

// Parse / display / (de)serialise as the single URN string.
let r: EntityRef = "person:0c4f1e2a-0000-4000-8000-000000000000".parse()?;
assert_eq!(r.entity_type, EntityType::Person);
assert_eq!(r.service(), "person-service");           // entity_type → owning service
assert_eq!(r.to_string(), "person:0c4f1e2a-0000-4000-8000-000000000000");

// The closed v1 edge-kind registry validates endpoint types.
assert!(EdgeKind::EmployedBy.permits(EntityType::Worker, EntityType::Organization));
assert_eq!(EdgeKind::EmployedBy.inverse(), Some("employs"));
assert_eq!(EdgeKind::SubjectOf.sensitivity(), Sensitivity::High); // case → person (§10)
```

## Types

| Item | Purpose |
|---|---|
| `EntityType` | The globally-unique entity discriminator (`person`, `worker`, …, `courseinstance`, `care_pathway`); `as_str`, `from_token`, and the `service()` map (course + courseinstance → `course-service`). |
| `EntityRef` | `{entity_type, id: Uuid}`; `FromStr`/`Display`/serde as the `"type:uuid"` URN (one indexable `TEXT` column). |
| `EdgeKind` | The closed v1 registry (`same_identity`, `works_at`, `member_of`, `employed_by`, `subject_of`) with `is_symmetric` / `is_temporal` / `inverse` / `sensitivity` / `permits(from, to)`. |
| `Sensitivity` | `Medium` (affiliation / identity) vs `High` (`case → person`, §10). |

## Who actually consumes this

As of 2026-08-04, eight crates depend on this one as a real Cargo `path`
dependency (`entity-ref = { path = "../../link/entity-ref-rust-crate" }`)
— not a copy-per-project, despite the framing above:

- **`link-graph-service-with-loco`** — the aggregator (the read model);
  by far the heaviest user, importing across `auth.rs`, `consumer.rs`,
  `controllers/graph.rs`, `events.rs`, `graph.rs`, `models/edges.rs`,
  `models/entity_presence.rs`, `probe.rs`, `reconcile.rs`, and
  `suggest/{job,mod}.rs`.
- **`person-service-with-loco`**, **`worker-service-with-loco`**,
  **`case-service-with-loco`** — the three entity services that
  originate edges (`entity_links` write-side + `linked`/`unlinked`
  events), as the design doc anticipated.
- **`contact-relationship-management-service-with-rust`**,
  **`content-management-system-service-with-rust`**,
  **`patient-flow-service-with-rust`**,
  **`workforce-planning-management-service-with-rust`** — the four
  consumer apps, which were *not* anticipated by the original rollout
  note. Each uses `EntityRef`/`EntityType` in its own `src/validation.rs`
  and `src/clients.rs` to validate and dereference cross-service refs
  (e.g. `person_ref`, `worker_ref`) rather than to originate edges.

Cross-service links are still **never** a matcher signal — see
`cross-service-linking.md` §7.
