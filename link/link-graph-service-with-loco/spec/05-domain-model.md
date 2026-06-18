## 5. Domain Model

The model is small and graph-shaped. There is **no entity record** of
the family's usual kind (no CRUD aggregate, no matcher DTO) — this
service stores a **derived projection** of other services' events.

### 5.1 `EntityRef` (the one shared contract)

```rust
pub enum EntityType {
    Person, Worker, Organization, Case, Place, Thing,
    Event, Course, CourseInstance, CarePathway,
}

pub struct EntityRef { pub entity_type: EntityType, pub id: Uuid }
// Display => "person:0c4f…"; FromStr parses & validates the type.
// A static entity_type -> service map resolves the owning service.
```

- Pure data, no behaviour beyond `parse` / `Display` and the
  `entity_type → service` lookup. **Copied per project** rather than
  packaged — drift is cheap; the format is frozen in
  [design §3](../../../agents/share/cross-service-linking.md#3-the-entityref--the-one-shared-contract).
- `entity_type` is the discriminator (not the service) so multi-entity
  services (course hosts `course` + `courseinstance`) resolve cleanly.

### 5.2 `Edge` — a node in the derived graph

```rust
pub struct Edge {
    pub edge_id: Uuid,            // = source `linked` event's edge_id
    pub from_ref: EntityRef,      // canonical "from" (smaller ref if symmetric)
    pub to_ref: EntityRef,
    pub kind: EdgeKind,           // closed registry (§5.4)
    pub directed: bool,           // false for symmetric kinds
    pub role: Option<String>,     // e.g. job title for employed_by
    pub confidence: Option<f64>,  // 1.0 operator-asserted; <1 suggested
    pub provenance: Provenance,   // operator | import | matcher_suggested
    pub valid_from: Option<NaiveDate>,
    pub valid_to: Option<NaiveDate>,
    pub status: EdgeStatus,       // unverified | verified | dangling
    pub observed_at: DateTime<Utc>,   // when the linked event was consumed
    pub source_event_id: Uuid,        // dedup provenance
}
```

### 5.3 Supporting value types

- `EdgeStatus` — `Unverified | Verified | Dangling`; the integrity
  lifecycle (§8 architecture), derived from `entity_presence`.
- `Provenance` — `Operator | Import | MatcherSuggested`. `matcher_suggested`
  edges enter at `confidence < 1.0` and are destined for a future review
  queue ([design §5.2](../../../agents/share/cross-service-linking.md#52-provenance--the-suggestion-queue)).
- `EntityPresence` — `{ ref: EntityRef, alive: bool, last_seq: i64 }`;
  the existence oracle.
- `FreshnessWatermark` — per-entity-topic `{ entity, last_occurred_at,
  last_seq, lag }`; the `as_of` source and the `/health/freshness` body.

### 5.4 `EdgeKind` — the closed v1 registry

Mirrors [design §9](../../../agents/share/cross-service-linking.md#9-v1-edge-kind-registry)
exactly:

| Kind | From → To | Direction | Card. | Temporal | Inverse | Sensitivity |
|---|---|---|---|---|---|---|
| `same_identity` | person ↔ worker | symmetric | 1:1 | no | (self) | medium — identity assertion |
| `works_at` / `member_of` | person → organization | directed | M:N | yes | `has_member` | medium |
| `employed_by` | worker → organization | directed | M:N | yes (+`role`) | `employs` | medium |
| `subject_of` / `about` | case → person | directed | M:N | sometimes | `is_subject_of` | **high** — see §12 |

- `same_identity` is the **federation backbone**: it resolves one
  human across the person and worker registries and powers
  `single-view`. With `same_identity` + `employed_by`, a person's
  employer is *derivable* (`person → worker → org`).
- The registry is **closed** in v1. Adding a kind later (e.g. course
  `taught_by` worker) is a new registry row + endpoint-type pair +
  inverse; the topology is unchanged.

### 5.5 What this model is NOT

- Not an entity aggregate, not a matcher DTO, not a system of record.
- Not within-entity `relationships` — those stay on each domain model
  and remain matcher signals. The two never mix (the partition rule,
  [design §7](../../../agents/share/cross-service-linking.md#7-relationship-to-within-entity-matching-the-partition-rule)).
