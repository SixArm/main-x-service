## 16. Open Questions

- **OQ-1**: Should `CourseInstance` reference the
  [event-service](../../../event/event-service-with-loco/) `Event` resource
  rather than carry its own `schedule` field? Decision deferred until
  we see a real cross-service integration requirement; for MVP we
  keep `Schedule` inline so the front-end works without joining
  services.
- **OQ-2**: Should `Provider` move into a separate
  `organization-service` shared by `course`, `event`, `worker`? The
  Person Service has an inline `Organization` model with the same
  pattern. Defer until two consumers ask for it.
- **OQ-3** *(resolved in T-6)*: Should the matcher's deterministic-
  identifier set include `CourseCode`? **No** — `CS101` exists at
  many providers, so a globally-unique deterministic short-circuit
  would mis-merge. **Yes when the same provider matches both
  records** — the matcher implements this via rule R-1
  (`provider_id + normalised(course_code)` → score 1.0) without
  promoting `CourseCode` to the `is_deterministic()` set.
- **OQ-4**: Internationalisation of `EducationalLevel`. The schema.org
  vocabulary doesn't fully cover non-English systems (e.g. UK A-levels,
  German Abitur, French Baccalauréat). The `Custom(String)` escape
  hatch handles them; a controlled vocabulary is a future task.

