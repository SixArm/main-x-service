## 16. Open Questions

- **OQ-1 — Coordinate precision masking.** Today we round to 2 dp
  (~1 km). Is that the right default for a private-residence
  privacy view, or should we offer multiple buckets (`coarse` /
  `medium` / `precise`)?
- ~~**OQ-2 — Hierarchy cycle detection.**~~ RESOLVED (T-16, 2026-09-05):
  the "validation rejects on insert" claim was aspirational until now —
  `validate_place` rejects a direct self-reference
  (`contained_in_place == Some(place.id)`) and
  `SeaOrmPlaceRepository::create`/`update` walk the ancestor chain
  (`ancestor_chain_contains`) to reject a multi-hop cycle (A contains
  B, then B is made to contain A), returning `409 Conflict`. Since a
  place has at most **one** `contained_in_place`, the hierarchy is a
  tree (one upward path per place), so "two paths from A to B" does
  not arise in this data model — the walk needs no branching, just a
  bounded single-chain traversal.

