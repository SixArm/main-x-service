## 6. Functional Requirements

Each requirement names its owning subproject. Deferred requirements
are marked and tracked in §13 / §15.

### 6.1 Registry CRUD — service

- **FR-1** Create a case: `POST /api/cases` with a `Case` body;
  reject a blank `title` with `422`; return `{pid, title}`.
- **FR-1a** Validate the payload (service): all problems are reported
  in one `422` — blank `title`; `opened_date` present but not a valid
  ISO 8601 date; any `identifiers` entry whose `value` is blank; any
  blank entry in `subjects` or `keywords`. Validation rules live in
  [`crate::validation`](../case-service-with-loco/src/validation.rs).
  Docket / case-number format checks and status-transition rules
  remain deferred (§13 T-9).
- **FR-2** List active cases: `GET /api/cases` returns `{pid, title}`
  refs, most-recent first, capped at 100.
- **FR-3** Read: `GET /api/cases/{pid}` returns the stored `Case`;
  `404` for unknown or soft-deleted `pid`.
- **FR-4** Update: `PUT /api/cases/{pid}` replaces the whole payload
  (and the denormalised `title`); reject a blank `title` or a
  validation failure (FR-1a) with `422`.
- **FR-5** Soft delete: `DELETE /api/cases/{pid}` sets `deleted_at`;
  the record disappears from list/read/match.
- **FR-5a** Title search: `GET /api/cases/search?q=` — case-insensitive
  Postgres `ILIKE` substring match on the denormalised `title` (cap
  50, wildcards escaped); blank `q` → `400`.

### 6.2 Matching — matcher (algorithm) + service (endpoints)

Algorithm reference:
[`agents/matching.md`](../agents/matching.md) and the matcher
[spec §5–§18](../case-matcher-rust-crate/spec/index.md).

- **FR-6** Deterministic short-circuits (matcher): score pins to
  1.0 on —
  - R-0: any shared value on a deterministic identifier scheme
    (`Docket`, `ExternalCaseId`, `Uri`, `Uuid`);
  - R-1: same non-empty `agency_id` + equal normalised `case_number`;
  - R-2: any case-folded `same_as` URL overlap.

  `AgencyCaseNumber` / `LocalId` / `Custom` schemes never short-circuit
  and never cross-match.
- **FR-7** Probabilistic components (matcher), renormalised weighted
  average over the components both records carry:

  | Component | Weight | Algorithm |
  |---|---:|---|
  | Title | 0.30 | Best Jaro-Winkler over `title` + `alternate_titles`; Soundex +0.05 bonus capped at 0.95 |
  | Subjects | 0.25 | Jaccard over the folded subject-id set |
  | Case number | 0.15 | Same agency: 1.0/0.0; across agencies: skipped |
  | Case type | 0.10 | Exact enum 1.0/0.0; skipped when either unset |
  | Status | 0.05 | Exact enum 1.0/0.0; skipped when either unset |
  | Keywords | 0.15 | Jaccard over folded sets |

  (`priority`, `opened_date`, `agency_name`, `in_language` do not
  contribute to the score.)
- **FR-8** Explainability (matcher): every result carries `score`,
  `Confidence` (`High` ≥ 0.95, `Medium` ≥ 0.70, else `Low`),
  `is_match` (threshold 0.85 default; `strict` 0.95 / `lenient`
  0.70), and a per-component `MatchBreakdown`.
- **FR-9** Ad-hoc ranking (service): `POST /api/cases/match` scores a
  `{query, candidates}` set without persistence, returning ranked
  `(index, MatchResult)` pairs.
- **FR-10** Duplicate check (service): `POST /api/cases/check-duplicates`
  matches a query against stored cases and returns hits above
  threshold as `{pid, title, score, confidence, is_match}`, sorted by
  score descending. The in-memory scan is capped at a named constant
  (`CHECK_DUPLICATES_SCAN_CAP`) with a `tracing::warn!` at the cap.
- **FR-10a** Record merge (service): `POST /api/cases/merge` folds a
  confirmed-duplicate case into a surviving one — union the list
  fields, keep the duplicate's title as an `alternate_titles` entry,
  soft-delete the duplicate, write a `merge_records` history row (with
  a snapshot of the transferred payload), and publish a `Merged`
  event. Equal `main_pid`/`duplicate_pid` → `422`; unknown pid →
  `404`. `GET /api/cases/merges/recent` lists the history. Merge logic
  lives in [`crate::merge`](../case-service-with-loco/src/merge.rs).

### 6.3 Audit, events, auth, docs — service

- **FR-17** Audit log: every create / update / delete / merge writes a
  best-effort `audit_logs` row (action + JSON snapshot + `actor` +
  timestamp; logs on failure, never fails the request). Read at
  `GET /api/cases/audit/recent` and `GET /api/cases/{pid}/audit`.
- **FR-18** Event streaming: each CRUD / merge publishes a `CaseEvent`
  (`created`/`updated`/`deleted`/`merged`) to an in-memory ring buffer
  (cap 1 000). Read at `GET /api/cases/events/recent`. Durable broker
  is roadmap (§15).
- **FR-19** Token verification: PASETO v4 public tokens (Ed25519)
  verified offline against the auth-service's published key via the
  embedded [`authentication-verifier`](../../authentication/) crate (env
  `CASE_PASETO_KEYS` / `CASE_TOKEN_ISSUER` / `CASE_TOKEN_AUDIENCE`).
  `AuthUser` (required) and `MaybeAuthUser` (optional) extractors; `GET
  /api/cases/whoami` is protected; the audit / merge `actor` is stamped
  from the token when present. Blanket `/api/*` enforcement +
  paseto-keys-over-HTTP fetch are follow-ups (§13 T-7). Auth model source
  of truth:
  [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
  (supersedes the RS256-JWT + JWKS model).
- **FR-20** API docs: OpenAPI 3 document at `GET /api-docs/openapi.json`
  and Swagger UI at `GET /swagger-ui`.

### 6.4 Operator UI — front-end

- **FR-11** List active cases at `/`.
- **FR-12** Create at `/new`; on success redirect to the detail page.
- **FR-13** Detail at `/[pid]`: render the stored record; offer edit,
  delete, and check-duplicates.
- **FR-14** Edit at `/[pid]/edit`; `PUT` then redirect to detail.
- **FR-15** Check-duplicates posts the current record and lists
  matches (title, score, confidence), excluding the record itself.
- **FR-16** The form edits the full DTO: comma-list inputs for
  titles/subjects/keywords/sameAs/languages, selects for
  type/status/priority, a date input for `opened_date`, and a row
  editor for identifiers.

### 6.5 Deferred — family parity (see §15 roadmap)

| Deferred capability | Owner when it lands |
|---|---|
| **Per-field privacy masking + GDPR data-subject export** (priority — case data is personal data, §12) | service + front-end |
| Durable event bus (replacing in-process ring buffer) | service |
| Full-text / fuzzy search (Tantivy) + search UI | service + front-end |
| Front-end search box + audit / event views | front-end |
| Blanket `/api/*` token enforcement + paseto-keys-over-HTTP fetch | service |
| Real-time duplicate detection on create (`409`) | service |
| Deeper validation (docket / case-number formats, status transitions) | service (+ inline UI validation) |
| gRPC | service |
