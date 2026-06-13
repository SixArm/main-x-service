# case-matcher — Specification

> **Single source of truth.** Code conforms to this spec. A behavioural
> change is a three-part PR: spec edit + code edit + test edit. Live
> work queue is §23; open questions are §16.

## 1. Purpose

`case-matcher` is a reusable, dependency-light Rust library for
**pairwise governmental case-management record matching**. A *case*
(case management / case tracking) is an open or historical matter
handled by a public agency on behalf of one or more subjects — a
benefit claim, legal action, social-services referral, licensing
application, complaint, appeal, investigation, and so on. Given two
`Case` records the matcher returns a `MatchResult`: score in
`[0.0, 1.0]`, `Confidence`, `is_match`, and a per-component
`MatchBreakdown`. It is the canonical algorithm embedded in
`case-service`'s matching layer.

## 2. Scope

In scope: the attributes that distinguish one case from another —
title, involved-party subjects, agency-scoped case number, case type,
status, keywords, and document identifiers. Out of scope: the full case
content (timeline, notes, attachments, outcomes), party-level personal
data, and anything requiring IO, a runtime, or network access.

## 3. Glossary

- **Case** — an open or historical matter tracked by an agency for one
  or more subjects.
- **Deterministic identifier** — globally unique (`Docket`,
  `ExternalCaseId`, URI, UUID). A match pins the score to `1.0`.
- **Agency-scoped code** — `case_number`/`AgencyCaseNumber`/`LocalId`;
  only unique within the handling organisation.
- **Subject** — an opaque involved-party id (e.g. a person pid); shared
  subjects strongly corroborate identity.

## 4. Research basis

Governmental cases are largely identified by their **handling agency +
local case number**, by a court/tribunal **docket**, by a
**cross-system external id**, or by the **subjects** involved. Matching
therefore combines deterministic linkage on those identifiers with
fuzzy comparison of the title and overlap of the involved-party
subjects.

## 5. Algorithm overview

```
Input: Case A, Case B, MatchConfig
  ├─ R-0 deterministic identifier match?        ─yes─> 1.0
  ├─ R-1 same agency + case_number?             ─yes─> 1.0
  ├─ R-2 same_as URL overlap?                   ─yes─> 1.0
  │
  ├─ title_score        (always)      Jaro-Winkler + Soundex bonus
  ├─ subjects_score     (≥1 set)      Jaccard over folded subject ids
  ├─ case_number_score  (same agency) 1.0/0.0
  ├─ case_type_score    (both set)    exact enum (1.0/0.0)
  ├─ status_score       (both set)    exact enum (1.0/0.0)
  ├─ keywords_score     (≥1 set)      Jaccard
  └─ renormalised weighted average over present components
```

`priority` and `opened_date` are carried on `Case` but never scored.

## 6. Domain model

`Case`: `title` (required), `alternate_titles`, `case_number`,
`agency_id`, `agency_name`, `case_type`, `status`, `priority` (data
only), `opened_date` (data only), `subjects`, `keywords`, `identifiers`
(`CaseIdentifier { scheme, value }`), `same_as`, `in_language`.

`CaseType`: `Benefit`, `Legal`, `SocialServices`, `Healthcare`,
`Housing`, `Immigration`, `Licensing`, `Complaint`, `Appeal`,
`Investigation`, `Tax`, `Employment`, `Custom(String)`.
`CaseStatus`: `Open`, `InProgress`, `Pending`, `OnHold`, `Closed`,
`Resolved`, `Rejected`, `Withdrawn`, `Custom(String)`.
`Priority` (data only): `Low`, `Normal`, `High`, `Urgent`.
`IdentifierScheme`: deterministic — `Docket`, `ExternalCaseId`, `Uri`,
`Uuid`; agency-scoped — `AgencyCaseNumber`, `LocalId`; plus
`Custom(String)`.

Serialisation: struct fields are snake_case; enum unit variants
serialise as their bare PascalCase string (`"Docket"`), and `Custom`
serialises as `{"Custom":"label"}`.

## 7. Configuration

`MatchConfig` weights (default, sum 1.0): title 0.30, subjects 0.25,
case_number 0.15, case_type 0.10, status 0.05, keywords 0.15.
Threshold 0.85. Presets: `strict()` 0.95, `lenient()` 0.70.

## 8. Normalisation

`fold` (trim + NFKC + lowercase, diacritic-preserving); `case_number`
(alphanumeric-only, uppercased — so `"CV-2024-001234"` ≡
`"cv 2024 001234"`); `url` (fold + drop trailing slash); `fold_set`
(sort + dedupe). Subjects and keywords compare via `fold_set` Jaccard.

## 9. Title similarity

Best Jaro-Winkler over `title` + `alternate_titles` (folded), with a
Soundex +0.05 bonus on the primary titles capped at 0.95.

## 10. Subjects

Jaccard over the `fold_set` of involved-party id strings. Shared
subjects are a strong identity signal, so subjects carry the
second-highest weight. Skipped when both sides are empty.

## 11. Case number

Within the same agency key (`agency_id`, or `agency_name` fallback):
1.0 if normalised numbers equal, else 0.0. Across agencies (or missing
agency): `None` (a local number like `CV-2024-001234` is not globally
unique).

## 12. Case type & status

Exact enum match → 1.0 else 0.0. `None` when either side is unset.
Case type weight 0.10, status weight 0.05.

## 13. Keywords

Jaccard over `fold_set`. Skipped when both empty; `0.0` when exactly one
side is populated.

## 14. Data-only fields

`priority` and `opened_date` are carried for downstream consumers and
MUST NOT contribute to the score.

## 15. Deterministic identifier short-circuits

R-0: any shared value on a deterministic scheme → 1.0. Empty values
ignored. `AgencyCaseNumber`/`LocalId`/`Custom` are excluded.

## 16. Agency+number, same_as, and open questions

R-1: shared non-empty agency key + equal normalised `case_number` →
1.0. R-2: any normalised `same_as` URL overlap → 1.0.

Open questions: should a shared subject set alone be a strong pin
(currently probabilistic — many cases per subject)? Should case-type or
status mismatch *penalise* rather than just not corroborate? Should
`opened_date` proximity become a (year-only) scored component?

## 17. Renormalisation

Weighted average over `Some` components only; divisor is the sum of
contributing weights.

## 18. Confidence classification

`High` ≥ 0.95, `Medium` ≥ 0.70, else `Low`. Separate from
`MatchConfig::threshold` (`is_match`).

## 19. Quality goals

Total functions (no `unwrap`/`expect`/`panic`); no `unsafe`;
deterministic; explainable; diacritic-correct.

## 20. Consumption

`case-service` embeds this crate via an adapter and calls
`MatchingEngine::match_cases`. A bridge test in the service pins the
contract.

## 21. Compatibility

Semantic versioning. Re-exports from `lib.rs` are the contract:
`Case`, `CaseIdentifier`, `IdentifierScheme`, `CaseType`, `CaseStatus`,
`Priority`, `MatchingEngine`, `MatchConfig`, `MatchResult`,
`MatchBreakdown`, `Confidence`, `Error`, `Result`.

## 22. Anti-patterns

Do not short-circuit on agency-scoped or `Custom` schemes. Do not score
a `case_number` across agencies. Do not score `priority` or
`opened_date`. Do not strip diacritics. Do not add IO, async, or panics
to library code.

## 23. Tasks (live work queue)

- [ ] Optional year-only `opened_date` proximity component.
- [ ] Optional case-type taxonomy (related types score partial).
- [ ] Split this single `spec/index.md` into the numbered §-per-file
      layout used by the sibling matcher crates.

## 24. Testing strategy

Unit tests embedded per module; an integration suite
(`tests/public_api.rs`) over the re-exported surface; rustdoc examples
run as doctests. Gate: `cargo test`, `cargo clippy --all-targets -- -D
warnings`, `cargo fmt --check`.

## 25. Change control

Update this spec in the same PR as any behavioural change; bump
`CHANGELOG.md` under the latest version.
