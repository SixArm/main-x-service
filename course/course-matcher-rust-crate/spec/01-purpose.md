## 1. Purpose

A small, library-friendly, dependency-light crate for pairwise
matching of course records. Score in `[0.0, 1.0]`, per-component
breakdown, deterministic short-circuits for high-precision schemes.

Modelled loosely on [schema.org/Course](https://schema.org/Course),
re-using only the properties that carry signal for identity
matching. The full Course model (Syllabus, EducationalCredential,
CourseInstance, …) lives in the consuming
[`course-service`](../../course-service-with-loco/) crate.

