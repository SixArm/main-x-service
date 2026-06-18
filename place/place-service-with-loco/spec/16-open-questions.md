## 16. Open Questions

- **OQ-1 — Coordinate precision masking.** Today we round to 2 dp
  (~1 km). Is that the right default for a private-residence
  privacy view, or should we offer multiple buckets (`coarse` /
  `medium` / `precise`)?
- **OQ-2 — Hierarchy cycle detection.** Validation rejects on insert,
  but we have no online "no two paths from A to B" check. Acceptable?

