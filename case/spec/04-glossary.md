## 4. Glossary

Entity-level terms. Per-subproject vocabularies: service
[spec §4](../case-service-with-loco/spec/index.md), matcher
[spec §3](../case-matcher-rust-crate/spec/index.md),
front-end
[spec §4](../case-front-end-with-svelte/spec/index.md).

| Term | Meaning |
|---|---|
| **Entity** | One domain concept (here: Case) delivered as a trio of subprojects in one directory |
| **Trio** | The three subprojects: service crate, matcher crate, front-end project |
| **Entity-level spec** | This document set — source of truth for the cross-subproject contract |
| **Crate spec** | A subproject's own `spec/` — source of truth for that subproject's internals |
| **Case** | A tracked unit of government work about a subject — a benefits claim, court matter, complaint, investigation, licence application, etc. |
| **Subject** | An involved party a case is about — an opaque reference (e.g. a person `pid` or organisation `pid`); never personal detail in-line |
| **Agency** | The government body handling the case (`agency_id` / `agency_name`); scopes the local `case_number` |
| **Case number** | `case_number` — a case's local identifier within its handling agency; unique only within that `agency_id`; never matched across agencies |
| **Case type** | `Benefit`, `Legal`, `SocialServices`, `Healthcare`, `Housing`, `Immigration`, `Licensing`, `Complaint`, `Appeal`, `Investigation`, `Tax`, `Employment`, `Custom` |
| **Status** | `Open`, `InProgress`, `Pending`, `OnHold`, `Closed`, `Resolved`, `Rejected`, `Withdrawn`, `Custom` |
| **Priority** | `Low`, `Normal`, `High`, `Urgent` — data only; never participates in matching |
| **Deterministic identifier** | Globally unique identifier (`Docket`, `ExternalCaseId`, `Uri`, `Uuid`); a shared value pins the match score to 1.0 |
| **Agency-scoped identifier** | `AgencyCaseNumber` / `LocalId` — unique only within an agency; never short-circuits or cross-matches |
| **pid** | The public UUID of a stored case record (route param; distinct from the row's internal `id`) |
| **`data`** | The `cases.data` JSONB column holding the full `Case` payload verbatim |
| **DTO contract** | The API body **is** `case_matcher::Case` — no separate service model, no adapter |
| **Match** | A comparison between two cases yielding a 0.00–1.00 score, `Confidence` band, `is_match`, and per-component breakdown |
| **Check-duplicates** | `POST …/check-duplicates` — match a query against stored cases, return ranked hits above threshold |
| **Merge** | Fold a confirmed-duplicate case into a survivor: union list fields, former-title alias, soft-delete the duplicate, history + snapshot, `Merged` event |
| **Soft delete** | Retention with `deleted_at` set; never `DELETE FROM` |
| **Audit log** | `audit_logs` row per CRUD / merge: action + JSON snapshot + `actor` + timestamp |
| **Event stream** | In-memory ring buffer of `CaseEvent`s (`created`/`updated`/`deleted`/`merged`); durable broker is roadmap |
| **Personal data** | Case records concern identified people / organisations — GDPR / UK DPA personal data (§12) |
| **SSO** | Single sign-on via the [authentication entity](../../authentication/): magic-link + cookie session, PASETO v4 public tokens ([`authentication-sessions.md`](../../agents/share/authentication-sessions.md), supersedes RS256 JWT + JWKS) |
| **Drift policy** | Front-ends keep per-project copies of types/client/forms; no shared package (repo decision 2026-06-02) |
