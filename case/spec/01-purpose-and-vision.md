## 1. Purpose and Vision

### 1.1 Purpose

The **case entity** is the governmental case-management registry of
the Main X Index — a federated identity index serving a worldwide
public governmental system with millions of users. A *case* is a
tracked unit of government work about a subject (a citizen, resident,
or organisation): a benefits claim, a legal or court case, a
social-services case, a licensing application, a complaint, an appeal,
an investigation, a tax matter, an employment matter, and so on. It is
delivered as a trio of subprojects that compose into one capability:

| Subproject | Role |
|---|---|
| [case-service-rust-crate](../case-service-rust-crate/) | Registry service — loco.rs CRUD + matching over REST; PostgreSQL persistence |
| [case-matcher-rust-crate](../case-matcher-rust-crate/) | Canonical pairwise matching library — deterministic + probabilistic, embedded by the service |
| [case-front-end-with-svelte](../case-front-end-with-svelte/) | Operator UI — SvelteKit SPA over the service's REST API |

The entity gives a government one canonical record per tracked case —
a benefits claim, a court matter, a complaint, an investigation —
regardless of how many agencies, departments, and source systems hold
a variant of that case, deduplicated and matchable on the attributes
that identify a case (title, involved subjects, agency-scoped case
number, case type, status, document identifiers).

### 1.2 Vision

One canonical record per tracked case, across agencies and source
systems:

- **Registry of case identities.** The entity records *which* cases
  exist and how they are identified and routed — not the full case
  file. Identifiers (docket numbers, external case ids, URIs, UUIDs)
  make it the linkage hub between agencies, courts, and downstream
  case-handling systems.
- **Explainable matching.** Every match decision returns a
  per-component score breakdown (title, subjects, case number, case
  type, status, keywords) that a caseworker or auditor can inspect —
  no black boxes.
- **Subject-centric.** A case is *about* people or organisations; the
  involved `subjects` carry the second-highest matching weight, so two
  records about the same matter for the same person corroborate
  strongly.
- **Multinational by design.** `in_language` on every record;
  operator surfaces localize to the locales in
  [`agents/share/locales.md`](../../agents/share/locales.md)
  (roadmap, §15).
- **Audit-grade and privacy-aware.** Soft delete, full audit logging,
  and event streaming from day one — and, because case data is
  *personal data* about identified people (§12), per-field masking and
  GDPR data-subject export are first-class roadmap items (§15), not
  optional extras.

### 1.3 Non-goals

- **Not** a case *execution* / workflow engine — no BPMN / CMMN / DMN
  runtime, no task-routing or SLA orchestration. Execution belongs to
  the agencies' case-handling systems; this registry interoperates
  with them via identifiers (§17).
- **Not** the full case file — it holds case *identity and routing
  metadata* (title, type, status, involved-party references), never
  the substantive record (documents, decisions, correspondence). Those
  stay in the originating agency's system of record.
- **Not** a person / organisation registry. `subjects` are opaque
  references (e.g. person `pid`s); resolving them is the
  [person entity](../../person/) and the
  [organization entity](../../organization/).
- **Not** an authentication / authorisation provider. Sign-on for the
  whole index is the [authentication entity](../../authentication/)
  (passwordless magic-link, RS256 JWT + JWKS); this entity is a JWT
  *verifier*.
