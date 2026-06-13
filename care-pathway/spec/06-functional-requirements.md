## 6. Functional Requirements

Each requirement names its owning subproject. Deferred requirements
are marked and tracked in §13 / §15.

### 6.1 Registry CRUD — service

- **FR-1** Create a care pathway: `POST /api/care-pathways` with a
  `CarePathway` body; reject a blank `name` with `422`; return
  `{pid, name}`.
- **FR-1a** Validate clinical codes (service): reject with `422` any
  `condition_codes` entry whose `code` is malformed for its declared
  `system` — ICD-10, ICD-11, and SNOMED CT (SCTID Verhoeff check digit)
  are format-checked; `Custom` codes need only be non-blank. All
  problems (including a blank `name`) are reported in one response. Code
  rules live in [`crate::validation`](../care-pathway-service-rust-crate/src/validation.rs).
  Existence-in-a-release checks (terminology server) remain deferred (§13 T-9).
- **FR-2** List active pathways: `GET /api/care-pathways` returns
  `{pid, name}` refs, most-recent first, capped at 100.
- **FR-3** Read: `GET /api/care-pathways/{pid}` returns the stored
  `CarePathway`; `404` for unknown or soft-deleted `pid`.
- **FR-4** Update: `PUT /api/care-pathways/{pid}` replaces the whole
  payload (and the denormalised `name`); reject a blank `name` or a
  malformed `condition_codes` entry (FR-1a) with `422`.
- **FR-5** Soft delete: `DELETE /api/care-pathways/{pid}` sets
  `deleted_at`; the record disappears from list/read/match.

### 6.2 Matching — matcher (algorithm) + service (endpoints)

Algorithm reference:
[`AGENTS/matching.md`](../AGENTS/matching.md) and the matcher
[spec §5–§18](../care-pathway-matcher-rust-crate/spec/index.md).

- **FR-6** Deterministic short-circuits (matcher): score pins to
  1.0 on —
  - R-0: any shared value on a deterministic identifier scheme
    (`Doi`, `Wikidata`, `GuidelineId`, `Uri`, `Uuid`);
  - R-1: same non-empty `provider_id` + equal normalised
    `pathway_code`;
  - R-2: any case-folded `same_as` URL overlap.
- **FR-7** Probabilistic components (matcher), renormalised weighted
  average over the components both records carry:

  | Component | Weight | Algorithm |
  |---|---:|---|
  | Name | 0.30 | Best Jaro-Winkler over `name` + `alternate_names`; Soundex +0.05 bonus capped at 0.95 |
  | Condition codes | 0.25 | Jaccard over `"system:code"` tokens |
  | Pathway code | 0.15 | Same provider: 1.0/0.0; across providers: skipped |
  | Care setting | 0.10 | Exact enum 1.0/0.0; skipped when either unset |
  | Interventions | 0.10 | Jaccard over folded sets |
  | Keywords | 0.10 | Jaccard over folded sets |

- **FR-8** Explainability (matcher): every result carries `score`,
  `Confidence` (`High` ≥ 0.95, `Medium` ≥ 0.70, else `Low`),
  `is_match` (threshold 0.85 default; `strict` 0.95 / `lenient`
  0.70), and a per-component `MatchBreakdown`.
- **FR-9** Ad-hoc ranking (service): `POST /api/care-pathways/match`
  scores a `{query, candidates}` set without persistence, returning
  ranked `(index, MatchResult)` pairs.
- **FR-10** Duplicate check (service):
  `POST /api/care-pathways/check-duplicates` matches a query against
  stored pathways and returns hits above threshold as
  `{pid, name, score, confidence, is_match}`, sorted by score
  descending.

### 6.3 Operator UI — front-end

- **FR-11** List active pathways at `/`.
- **FR-12** Create at `/new`; on success redirect to the detail page.
- **FR-13** Detail at `/[pid]`: render the stored record; offer
  edit, delete, and check-duplicates.
- **FR-14** Edit at `/[pid]/edit`; `PUT` then redirect to detail.
- **FR-15** Check-duplicates posts the current record and lists
  matches (name, score, confidence), excluding the record itself.
- **FR-16** The form edits the full DTO: comma-list inputs for
  names/interventions/keywords/sameAs, row editors for condition
  codes and identifiers.

### 6.4 Deferred — family parity (see §15 roadmap)

The following Main X Index common features are **deferred** for this
entity (MVP decision, §2.3):

| Deferred capability | Owner when it lands |
|---|---|
| Full-text search (Tantivy) + search UI | service + front-end |
| Event streaming on CRUD | service |
| Audit logging + audit query API | service |
| Privacy controls (masking, GDPR export, consent) | service |
| Record merge with link tracking + snapshots | service + front-end |
| Real-time duplicate detection on create (`409`) | service |
| OpenAPI / Swagger, gRPC | service |
| Richer validation (ICD / SNOMED code formats) | service (+ inline UI validation) |
