# care-pathway-matcher — Specification

> **Single source of truth.** Code conforms to this spec. A behavioural
> change is a three-part PR: spec edit + code edit + test edit. Live
> work queue is §23; open questions are §16.

## 1. Purpose

`care-pathway-matcher` is a reusable, dependency-light Rust library for
**pairwise care-pathway record matching**. A *care pathway* (also
"clinical pathway", "critical pathway", "integrated care pathway") is a
structured, evidence-based, multidisciplinary plan of care for a
specific clinical condition or patient group over a defined episode.
Given two `CarePathway` records the matcher returns a `MatchResult`:
score in `[0.0, 1.0]`, `Confidence`, `is_match`, and a per-component
`MatchBreakdown`. It is the canonical algorithm embedded in
`care-pathway-service`'s matching layer.

## 2. Scope

In scope: the attributes that distinguish one pathway from another —
name, target clinical condition codes, provider-scoped pathway code,
care setting, key interventions, keywords, and document identifiers.
Out of scope: the full pathway content (intervention timing, variance
tracking, outcomes), patient-level data, and anything requiring IO, a
runtime, or network access.

## 3. Glossary

- **Care pathway** — a standardised, evidence-based care plan for a
  condition/episode.
- **Deterministic identifier** — globally unique (DOI, Wikidata,
  guideline-registry id, URI, UUID). A match pins the score to `1.0`.
- **Provider-scoped code** — `pathway_code`/`PathwayCode`/`LocalId`;
  only unique within the issuing organisation.
- **Condition code** — ICD-10 / ICD-11 / SNOMED CT code of the target
  clinical condition; the defining attribute of a pathway.

## 4. Research basis

Care pathways are largely defined by their **target condition** and
**setting**, published by a provider or a guideline body (e.g. NICE),
and referenced by a guideline/registry id, DOI, or URL. Matching
therefore combines deterministic linkage on those identifiers with
fuzzy comparison of the title and overlap of the target condition codes.

## 5. Algorithm overview

```
Input: CarePathway A, CarePathway B, MatchConfig
  ├─ R-0 deterministic identifier match?        ─yes─> 1.0
  ├─ R-1 same provider + pathway_code?          ─yes─> 1.0
  ├─ R-2 same_as URL overlap?                   ─yes─> 1.0
  │
  ├─ name_score          (always)   Jaro-Winkler + Soundex bonus
  ├─ condition_score     (≥1 set)   Jaccard over "system:code" tokens
  ├─ pathway_code_score  (same provider)  1.0/0.0
  ├─ care_setting_score  (both set) exact enum (1.0/0.0)
  ├─ interventions_score (≥1 set)   Jaccard
  ├─ keywords_score      (≥1 set)   Jaccard
  ├─ relationships_score (≥1 set)   typed-set Jaccard over (relation, pathway_id)
  ├─ tags_score          (both set) set Jaccard over normalised tags
  └─ renormalised weighted average over present components
```

## 6. Domain model

`CarePathway`: `name` (required), `alternate_names`, `pathway_code`,
`provider_id`, `provider_name`, `care_setting`, `condition_codes`
(`ConditionCode { system, code }`), `interventions`, `keywords`, `tags`
(`Vec<String>`, default empty), `identifiers` (`PathwayIdentifier {
scheme, value }`), `same_as`, `in_language`, `relationships`
(`RelationshipRef { relation, pathway_id }`).

`tags: Vec<String>` holds operator-applied free-text labels for grouping
/ workflow (e.g. `vip`, `review`, `fast-track`); each is whitespace-
trimmed, non-empty, and the set is de-duplicated case-insensitively.
Distinct from `keywords` (descriptive / discovery terms about *what the
record is*): tags are user-applied operational labels. A supporting
signal, not an identifying field on its own (§13.2).

`relationships: Vec<RelationshipRef>` holds typed pathway-to-pathway
references — `RelationshipRef { relation: RelationKind, pathway_id:
String }` where `RelationKind` is a `#[non_exhaustive]` enum mirroring
the service: `PrecededBy` / `FollowedBy` (sequencing inverses),
`SimilarTo` (symmetric), `Supersedes` / `SupersededBy` (versioning
inverses), plus `Custom(String)`. `pathway_id` is an opaque registry id
(whitespace-trimmed, non-empty); the matcher does **not** resolve,
invert, or transitively close the references — it compares the two
records' relationship **sets** (§13.1). A supporting signal, not an
identifying field on its own.

`CodeSystem`: `Icd10`, `Icd11`, `Snomed`, `Custom(String)`.
`CareSetting`: `Inpatient`, `Outpatient`, `PrimaryCare`,
`EmergencyDepartment`, `Community`, `HomeCare`, `Rehabilitation`,
`MentalHealth`, `Palliative`, `Custom(String)`.
`IdentifierScheme`: deterministic — `Doi`, `Wikidata`, `GuidelineId`,
`Uri`, `Uuid`; provider-scoped — `PathwayCode`, `LocalId`; plus
`Custom(String)`.

`provider_name` is **informational-only**: it is serialized for callers
but never read by the matcher. The pathway-code gate (§11, R-1) keys
solely on `provider_id`, so two records can only share a provider scope
via that opaque id, not via a fuzzy provider-name comparison.

## 7. Configuration

`MatchConfig` core weights (default, sum 1.0): name 0.30, condition
0.25, pathway_code 0.15, care_setting 0.10, interventions 0.10, keywords
0.10. Plus two **supporting** signals layered on top of the core six:
`relationships_weight` 0.05 (§13.1) and `tags_weight` 0.05 (§13.2). The
weighted average is renormalised over the components actually present
(§17), so the extra supporting weights never break the renormalisation.
Threshold 0.85. Presets: `strict()` 0.95, `lenient()` 0.70.

Changing any weight (including adding `relationships_weight` or
`tags_weight`) is a config-section + `CHANGELOG.md` edit in the same PR
(§25).

## 8. Normalisation

`fold` (trim + NFKC + lowercase, diacritic-preserving); `pathway_code`
(alphanumeric-only, uppercased — so `"STROKE-01"` ≡ `"stroke 01"`);
`fold_set` (sort + dedupe). Condition codes render to `"system:code"`
tokens (lower-cased) for the Jaccard.

## 9. Name similarity

Best Jaro-Winkler over `name` + `alternate_names` (folded), with a
Soundex +0.05 bonus on the primary names capped at 0.95.

## 10. Condition codes

Jaccard over the set of `"system:code"` tokens. The defining attribute
of a pathway, so it carries the second-highest weight. Skipped when both
sides are empty.

## 11. Pathway code

Within the same `provider_id`: 1.0 if normalised codes equal, else 0.0.
Across providers (or missing provider): `None` (a local code like
`STROKE-01` is not globally unique).

## 12. Care setting

Exact enum match → 1.0 else 0.0. `None` when either side is unset.

## 13. Interventions & keywords

Jaccard over `fold_set`. Skipped when both empty; `0.0` when exactly one
side is populated.

### 13.1 Relationships — `relationships_score`

Typed-set **Jaccard** over the `(relation, pathway_id)` pairs: `score =
|A ∩ B| / |A ∪ B|`, where each side's set is `{ (r.relation,
r.pathway_id) for r in relationships }`. The relation kind is part of
the key, so a `Supersedes` reference only agrees with a `Supersedes`
reference to the **same** pathway id; `PrecededBy` / `FollowedBy` /
`SimilarTo` / `SupersededBy` are compared as opaque, distinct kinds (no
inversion or transitive closure). `None` (does not participate) when
**either** side has no relationships; otherwise a value in `[0.0, 1.0]`.
A **supporting** signal weighted `relationships_weight` (§7, default
`0.05`); shared references never single-handedly establish a match.

### 13.2 Tags — `tags_score`

Plain set **Jaccard** over the case-insensitively normalised tag sets:
`tags_score = |A ∩ B| / |A ∪ B|`, where each side's set is the
`fold`-normalised, de-duplicated `tags`. `None` (does not participate)
when **either** side has an empty tag set; otherwise a value in `[0.0,
1.0]`. Identical in shape to `keywords_score` (§13) — a **supporting**
signal weighted `tags_weight` (§7, default `0.05`); shared tags never
single-handedly establish a match.

## 14. (reserved)

## 15. Deterministic identifier short-circuits

R-0: any shared value on a deterministic scheme → 1.0. Empty values
ignored. `PathwayCode`/`LocalId`/`Custom` are excluded.

## 16. Provider+code, same_as, and open questions

R-1: shared non-empty `provider_id` + equal normalised `pathway_code` →
1.0. R-2: any case-folded `same_as` URL overlap → 1.0.

Open questions: should an exact shared condition code alone be a strong
pin (currently probabilistic — many pathways per condition)? Should
care-setting mismatch *penalise* rather than just not corroborate?

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

`care-pathway-service` embeds this crate via an adapter and calls
`MatchingEngine::match_care_pathways`. A bridge test in the service pins
the contract.

## 21. Compatibility

Semantic versioning. Re-exports from `lib.rs` are the contract:
`CarePathway`, `PathwayIdentifier`, `IdentifierScheme`, `ConditionCode`,
`CodeSystem`, `CareSetting`, `RelationshipRef`, `RelationKind`,
`MatchingEngine`, `MatchConfig`, `MatchResult`, `MatchBreakdown`,
`Confidence`, `Error`, `Result`.

`Error`/`Result` are **reserved for future fallible APIs**: every
current entry point (`match_care_pathways` and all component fns) is
total and returns `MatchResult` directly, so nothing produces an `Error`
today. They remain part of the SemVer surface so a future fallible path
(e.g. validated construction) can be added without a breaking re-export.

## 22. Anti-patterns

Do not short-circuit on provider-scoped or classification codes. Do not
score a `pathway_code` across providers. Do not strip diacritics. Do not
add IO, async, or panics to library code.

## 23. Tasks (live work queue)

- [ ] Implement `relationships` in code: `RelationshipRef { relation:
      RelationKind, pathway_id: String }` + `RelationKind` enum
      (`PrecededBy`, `FollowedBy`, `SimilarTo`, `Supersedes`,
      `SupersededBy`, `Custom(String)`; `#[non_exhaustive]`), the
      typed-set Jaccard component (§13.1), the `relationships_score`
      field on `MatchBreakdown`, and `relationships_weight` (default
      `0.05`) on `MatchConfig`, with the weighted average renormalised
      over present components (§17). Re-export both new types from
      `lib.rs` (§21).
- [ ] Implement `tags` in code: a `tags: Vec<String>` field (default
      empty) on `CarePathway`, the set-Jaccard component (§13.2) over the
      `fold`-normalised tag sets (`None` when either side empty), the
      `tags_score` field on `MatchBreakdown`, and `tags_weight` (default
      `0.05`) on `MatchConfig`, with the weighted average renormalised
      over present components (§17).
- [ ] Optional intervention-sequence (ordered) similarity.
- [ ] Patient-group / age-band component.
- [ ] Split this single `spec/index.md` into the numbered §-per-file
      layout used by the sibling matcher crates.

## 24. Testing strategy

Unit tests embedded per module; an integration suite
(`tests/public_api.rs`) over the re-exported surface; rustdoc examples
run as doctests. Gate (mirrors CI): `cargo test`, `cargo clippy
--all-targets --all-features -- -D warnings`, `cargo fmt --check`.
Library code carries **no** `#[allow(clippy::…)]` attributes — it is
clippy-clean without suppressions (repo-wide invariant).

## 25. Change control

Update this spec in the same PR as any behavioural change; bump
`CHANGELOG.md` under `[Unreleased]`.
