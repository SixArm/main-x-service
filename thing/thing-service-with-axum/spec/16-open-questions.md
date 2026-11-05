## 16. Open Questions

- **OQ-1 — Image storage.** Today `image[]` is URLs only. Should the
  service offer a blob store endpoint, or hand off to a separate
  asset service?
- **OQ-2 — Matcher trait abstraction.** Promote `compute_match` to a
  `ThingMatcher` trait now (paving for ML), or defer until T-5 actually
  needs it?
- **OQ-3 — `additional_type` validation.** Should we reject values
  outside a curated allowlist (schema.org sub-types only), or accept
  any URL and warn?

