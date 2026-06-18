## 1. Purpose and Vision

### 1.1 Purpose

The Course Service is a registry of **course identities**. It models
the abstract course (`schema.org/Course` — "CS101 Introduction to
Computer Science") separately from its specific offerings
(`schema.org/CourseInstance` — "CS101, Fall 2026, Prof. Smith,
in-person, Tue/Thu 09:00"). One course → many instances.

It sits in the Main X Index family between the more-abstract
[Thing Service](../../../thing/thing-service-with-loco/) (anything with an
identity) and the time-bounded
[Event Service](../../../event/event-service-with-loco/) (occurrences with
locations and parties). A `Course` is a template; a `CourseInstance`
is closer to an `Event` and may eventually reference one.

### 1.2 Vision

A single course-identity surface that:

- Carries every property from `schema.org/Course` /
  `schema.org/CourseInstance` / `LearningResource` / `CreativeWork` /
  `Thing` that is relevant to interoperability with LMS, OER, and
  catalog systems.
- Matches probabilistically (name + course-code + provider +
  educational-level + topic / teaches) and deterministically (DOI,
  Wikidata, LMS course-id, OER id, URI, UUID).
- Detects duplicates in real time on create *and* in batch on
  demand, routing them through a review queue with auto-merge for
  high-confidence matches.
- Emits audit logs and event-streaming records suitable for HIPAA-
  / FERPA-grade trails.

### 1.3 Non-goals

- **Not** a learning-management system. We do not store enrollments,
  grades, submissions, or content. We point at LMSs via the
  `CourseInstance.location_id` / `instructor_ids` references and the
  external-identifier collection.
- **Not** a credential issuer. We model the **shape** of an
  `EducationalCredential` awarded by a course; we do not issue Open
  Badges or Verifiable Credentials.
- **Not** a marketplace. No payment, enrollment, or course-discovery
  ranking algorithms.
- **Not** an authentication / authorisation provider. JWT auth is
  planned (§15) but identity proofing is out of scope.

