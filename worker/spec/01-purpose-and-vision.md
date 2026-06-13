## 1. Purpose and Vision

### 1.1 Purpose

The Worker entity is the **workforce / professional identity
registry** of the Main X Index — the federated identity index that
serves a worldwide public governmental system with millions of users.
One entity, three subprojects:

- a **service** (system of record — CRUD, search, dedup, merge,
  audit),
- a **matcher** (canonical pairwise-comparison library the service
  embeds),
- a **front-end** (operator UI over the service REST API).

A worker is anyone whose role and credentials matter to the caller:
licensed professionals, public-sector staff, contractors, operators,
drivers, field engineers. The worker record carries professional
identifiers (NPI, DEA, board licence, employee number, ODS code) and
credential documents alongside the demographic fields the person
index uses.

### 1.2 Vision

**One canonical worker record per professional, at national and
international scale.** Licensing boards, credentialing authorities,
public-sector HR systems, scheduling systems, and payroll systems
each hold shards of a professional's identity; the Worker entity
reconciles those shards into a single trustworthy record that:

- matches probabilistically and deterministically against arbitrary
  input (typed name, partial NPI, credential number, national
  identifier from any of 42 supported schemes) and returns ranked
  candidates with per-component score breakdowns;
- detects duplicates in real time on create and in batch on demand,
  with a human review queue between the two;
- supports credential verification by employers and government
  verifiers against the registry record;
- emits HIPAA-grade audit logs and event-streaming records for every
  CRUD / merge / link operation, suitable for regulator inspection;
- serves multiple locales (see
  [`agents/share/locales.md`](../../agents/share/locales.md)) with
  diacritic-correct name matching for non-English names.

### 1.3 Non-goals

- **Not a general person registry** — that is the
  [person entity](../../person/). The worker entity is for identities
  whose professional role / credentials are the point.
- **Not an authentication provider** — single sign-on is the
  [authentication entity](../../authentication/)
  (passwordless magic-link, RS256 JWT + JWKS). The worker entity
  *consumes* that SSO (roadmap, §15); it never proofs identity itself.
- **Not a credentialing / licensing source** — the registry records
  credentials and links to the issuing authority; it does not issue
  or adjudicate them.
- **Not a payroll, scheduling, or HR system.**
