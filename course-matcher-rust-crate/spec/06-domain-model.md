## 6. Domain model

`src/course.rs`:

- `Course { name, alternate_names, course_code, provider_id,
  provider_name, educational_level, learning_resource_type,
  keywords, teaches, identifiers, same_as, in_language }`.
- `CourseIdentifier { scheme, value }`.
- `IdentifierScheme` — 12 variants (see §15).
- `EducationalLevel` — 12 variants + `Custom(String)` (see §12).
- `LearningResourceType` — 11 variants + `Custom(String)`.

