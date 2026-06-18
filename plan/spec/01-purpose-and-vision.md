## 1. Purpose and Vision

### 1.1 Purpose

The **plan entity** is the project / programme / initiative registry
of the Main X Index — a federated identity index serving a worldwide
public governmental system with millions of users. A *plan* is a
matchable identity for a **project, product, programme, initiative,
portfolio, or epic**: the unit of work an organisation funds, staffs,
and reports on. It is delivered as a trio of subprojects that compose
into one capability:

| Subproject | Role |
|---|---|
| [plan-service-with-loco](../plan-service-with-loco/) | Registry service **and** project-management tool — loco.rs CRUD + matching over REST; PostgreSQL persistence; operational sub-resources (goals, tasks, issues, posts, comments, members) and derived views (timeline / burndown) |
| [plan-matcher-rust-crate](../plan-matcher-rust-crate/) | Canonical pairwise matching library — deterministic + probabilistic, embedded by the service |
| [plan-front-end-with-svelte](../plan-front-end-with-svelte/) | Operator UI — SvelteKit SPA over the service's REST API |

The entity has **two faces that share one record**:

- **A matchable identity registry.** It gives an organisation one
  canonical record per initiative — portfolio-level deduplication:
  "is this migration project the same initiative the other department
  already chartered?" The **thin** `Plan` record (the matcher type) is
  deduplicated and matchable on the attributes that identify an
  initiative (name, goal titles, owner-scoped plan code, sponsoring
  organisation, plan type, timeframe, keywords, identifiers).
- **A project-management tool.** A `Plan` also *owns* operational
  sub-resources — goals, tasks, issues, posts, comments, members — and
  derived views (timeline / Gantt, burndown). This high-volume
  operational data lives in **separate service tables** and is
  **deliberately excluded** from the matcher payload (§5.6); only the
  thin identity record is matched.

### 1.2 Vision

One canonical record per real-world initiative, usable both for
portfolio dedup and as the live project workspace:

- **Registry of plan identities.** The entity records *which*
  initiatives exist and how they are identified — not (for matching
  purposes) the day-to-day task churn. Identifiers (external PM-tool
  ids such as Jira project keys, Asana GIDs, Trello board ids, plus
  URIs / UUIDs) make it the linkage hub between PM tools, sponsoring
  organisations, and the people who lead and staff the work.
- **Explainable matching.** Every match decision returns a
  per-component score breakdown (name, goals, plan code, owner org,
  plan type, timeframe, keywords, relationships, tags) that an
  auditor can inspect — no black boxes.
- **Federated by reference.** A plan's lead, assignees, authors, and
  members are **`EntityRef`s** into the person / worker / authentication
  entities; its sponsoring organisation is an `EntityRef` into the
  organization entity; and any goal / task / issue can carry a
  cross-service link to **any** index entity
  ([`agents/share/cross-service-linking.md`](../../agents/share/cross-service-linking.md)).
- **Multinational by design.** `in_language` on every record;
  operator surfaces localize to the locales in
  [`agents/share/locales.md`](../../agents/share/locales.md)
  (roadmap, §15).
- **Audit-grade.** Soft delete, full audit logging, and event
  streaming from the family baseline, suitable for
  government-portfolio information governance.

### 1.3 Non-goals

- **Not** a replacement for full-feature PM suites (Jira, Asana, MS
  Project, Linear). The operational sub-resources cover charter-level
  planning and tracking; deep PM features (custom workflows,
  automation rules, time tracking, sprint ceremonies) stay in the
  source tools and link in via external-id identifiers (§5.5).
- **Not** a finance / budgeting system — no cost ledgers, no
  invoicing. Budget figures, where modelled at all, are descriptive,
  not transactional.
- **Not** a document / file store — posts and comments are Markdown
  text; binary attachments are out of scope for MVP.
- **Not** an authentication / authorisation provider. Sign-on for the
  whole index is the [authentication entity](../../authentication/)
  (passwordless magic-link, cookie session + PASETO v4 public token); this
  entity is a token *verifier* and references user identities by `EntityRef`.
  Auth source of truth (supersedes the RS256-JWT model):
  [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md).
- **Cross-service links are never a match signal** — a plan that
  *links to* a person or org is not thereby the *same* as another plan
  ([`agents/share/cross-service-linking.md`](../../agents/share/cross-service-linking.md) §7).
