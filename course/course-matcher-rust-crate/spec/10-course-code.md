## 10. Course code

- When both records have `provider_id` AND both `provider_id` match
  AND both have `course_code`:
  - `course_code(a.course_code) == course_code(b.course_code)` →
    1.0.
  - Else 0.0.
- Otherwise the component is `None` (skipped from the weighted
  average).

Rationale: `CS101` exists at most universities. Without sharing the
provider it's noise; with shared provider it's identity-grade.

