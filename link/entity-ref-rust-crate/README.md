# entity-ref

The one shared **contract** for cross-service entity linking in the Main X
Index family — `agents/share/cross-service-linking.md` §3 (the `EntityRef`
value type) and §9 (the v1 edge-kind registry). Rollout **step 1**: land
the contracts, no behaviour yet.

A record in another service is named by an opaque URN string
`"<entity_type>:<uuid>"`, e.g. `person:0c4f1e2a-…`. This crate owns the
parsing/validation and the static metadata; it is pure, panic-free, and
dependency-light (serde + uuid + thiserror), so it can be copied per
project until a second non-aggregator consumer justifies a shared dep.

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

## Where this is going

Consumed next by the `link-graph-service-with-loco` aggregator (the read
model) and by the entity services that originate edges (`entity_links` +
`linked`/`unlinked` events). Cross-service links are **never** a matcher
signal — see `cross-service-linking.md` §7.
