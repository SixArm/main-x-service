## 5. Domain Model

The model surface mirrors `schema.org/Course` field-for-field where
sensible. The complete property table — schema.org → Rust mapping
— is in [`AGENTS/models.md`](../AGENTS/models.md). High-level shape:

- `Course` (the template) — Thing + CreativeWork + LearningResource
  + Course-specific properties (course_code, number_of_credits,
  course_prerequisites, available_language, financial_aid_eligible,
  educational_credential_awarded, total_historical_enrollment,
  syllabus_sections, instances).
- `CourseInstance` — schedule, mode (online / onsite / blended /
  self-paced), instructors, location, capacity, enrollment window.
- `Provider` — the issuing organisation.
- `CourseIdentifier` — `{ property_id, value, name?, url? }`
  matching `schema.org/PropertyValue`. `property_id` enumerates the
  scheme; `is_deterministic()` exposes which schemes short-circuit
  matching.
- `Syllabus` — hierarchical table-of-contents node with `teaches`,
  `time_required`, and `sub_sections`.
- `EducationalCredential` — schema.org/EducationalOccupationalCredential.
- `MergeRequest` / `MergeResponse` / `ReviewQueueItem` — same shape
  as the sibling services.

