## 1. Purpose and Vision

### 1.1 Purpose

The **course entity** is the course-identity registry of the Main X
Index — a federated identity index serving a worldwide public
governmental system with millions of users. It models the abstract
course (`schema.org/Course` — "CS101 Introduction to Computer
Science") separately from its specific offerings
(`schema.org/CourseInstance` — "CS101, Fall 2026, Prof. Smith,
in-person, Tue/Thu 09:00"). One course → many instances. It is
delivered as a trio of subprojects that compose into one capability:

| Subproject | Role |
|---|---|
| [course-service-with-loco](../course-service-with-loco/) | Registry service — CRUD (course + instance sub-resource), search, matching, merge, audit, privacy over REST. Boots through loco.rs; the **reference implementation** for the family's idiomatic-loco controller conversion. |
| [course-matcher-rust-crate](../course-matcher-rust-crate/) | Canonical pairwise matching library — deterministic + probabilistic, embedded by the service |
| [course-front-end-with-svelte](../course-front-end-with-svelte/) | Operator UI — SvelteKit SPA over the service's REST API |

The entity gives education ministries, accreditation bodies, and
public training programmes one canonical record per course regardless
of how many LMS, OER repository, and catalogue systems hold a shard of
that identity — with the provider-scoped course codes, educational
levels, and external identifiers a national course catalogue needs.

### 1.2 Vision

One canonical course record per real-world course, at national-
catalogue scale:

- **Population-scale catalogues.** Millions of course templates and
  tens of millions of course instances, sustained ingestion from
  national curriculum feeds, university catalogues, and public
  training-programme registries.
- **Template / instance split by design.** The `Course` is the
  durable identity; each term's offering is a `CourseInstance`
  sub-resource — so a course keeps one identity across decades of
  offerings.
- **Multilingual by design.** Courses carry `in_language` (BCP-47)
  and `available_language`; the operator surface localizes to the
  locales in [`agents/share/locales.md`](../../agents/share/locales.md)
  (roadmap, §15).
- **Explainable matching.** Every match decision returns a
  per-component score breakdown (name, course code, provider,
  educational level, keywords, teaches) that an operator, auditor,
  or court can inspect — no black boxes.
- **Audit-grade.** Every CRUD / merge operation writes an audit-log
  row and publishes an event, suitable for FERPA-grade trails where
  education-relevant and GDPR / UK DPA accountability everywhere.
- **Operator-complete.** The front-end covers the full duplicate
  lifecycle — surface candidates on create, score hypotheticals,
  review, merge, and audit — so catalogue stewardship needs no SQL.

### 1.3 Non-goals

- **Not** a learning-management system. No enrollments, grades,
  submissions, or content — the entity points at LMSs via
  `CourseInstance` references and the external-identifier collection.
- **Not** a student-record or enrollment registry — student records
  are person-linked domain systems that reference courses by
  `course_id`; the [person entity](../../person/) holds the people.
- **Not** a credential issuer. The entity models the **shape** of an
  `EducationalCredential` awarded by a course; it does not issue Open
  Badges or Verifiable Credentials.
- **Not** a marketplace or discovery-ranking engine. No payment,
  enrollment workflow, or recommendation algorithms.
- **Not** an authentication / authorisation provider. Sign-on for the
  whole index is the [authentication entity](../../authentication/)
  (passwordless magic-link, RS256 JWT + JWKS); the course entity is a
  JWT *verifier* (roadmap, §15).
