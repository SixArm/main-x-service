# course-matcher — Living Specification

> **Source of truth.** When code and spec disagree, the spec wins —
> open a task in §13 to bring code in line.

Per-crate `spec.md` for the family's matcher crates uses a §1–§25
shape (research basis, per-component derivations, normalisation
rules, …). The shorter §1–§18 shape is used by the service crates.

## Table of contents

1. [Purpose](#1-purpose)
2. [Scope](#2-scope)
3. [Glossary](#3-glossary)
4. [Research basis](#4-research-basis)
5. [Algorithm overview](#5-algorithm-overview)
6. [Domain model](#6-domain-model)
7. [Configuration](#7-configuration)
8. [Normalisation](#8-normalisation)
9. [Name similarity](#9-name-similarity)
10. [Course code](#10-course-code)
11. [Provider](#11-provider)
12. [Educational level](#12-educational-level)
13. [Keywords](#13-keywords)
14. [Teaches / competencies](#14-teaches--competencies)
15. [Identifier short-circuits](#15-identifier-short-circuits)
16. [Same-as URL short-circuit](#16-same-as-url-short-circuit)
17. [Renormalisation](#17-renormalisation)
18. [Confidence classification](#18-confidence-classification)
19. [Quality goals](#19-quality-goals)
20. [Consumption](#20-consumption)
21. [Compatibility](#21-compatibility)
22. [Anti-patterns](#22-anti-patterns)
23. [Tasks](#23-tasks)
24. [Testing strategy](#24-testing-strategy)
25. [Change control](#25-change-control)

## 1. Purpose

A small, library-friendly, dependency-light crate for pairwise
matching of course records. Score in `[0.0, 1.0]`, per-component
breakdown, deterministic short-circuits for high-precision schemes.

Modelled loosely on [schema.org/Course](https://schema.org/Course),
re-using only the properties that carry signal for identity
matching. The full Course model (Syllabus, EducationalCredential,
CourseInstance, …) lives in the consuming
[`course-service`](../course-service-rust-crate/) crate.

## 2. Scope

### 2.1 In scope

- Pairwise scoring (`match_courses`).
- One-to-many ranking (`rank`, `find_matches`).
- Deterministic short-circuits on identifier schemes, same-provider
  course codes, and `same_as` URLs.
- Probabilistic weighted-average scoring over name, course_code,
  provider, educational level, keywords, teaches.
- Tunable `MatchConfig` (weights + threshold).
- Total functions — no panics on bad input.
- `serde` round-trip for `Course`, `MatchConfig`, `MatchResult`.

### 2.2 Out of scope

- Search / blocking. Callers (e.g. `course-service`) pre-filter
  candidates via Tantivy before calling into the matcher.
- Persistence.
- HTTP / gRPC.
- Cross-language matching. Set `same_as` for that.
- Stemming / synonym expansion.

## 3. Glossary

| Term | Meaning |
|---|---|
| **Deterministic scheme** | An identifier scheme whose values are unique-by-construction (DOI, Wikidata, …). A match short-circuits scoring to 1.0. |
| **Renormalisation** | Weighted sum / sum-of-weights over the present components, not the full configured weight table. |
| **Same-provider** | Two records sharing `provider_id`. Required for the course-code component to contribute. |
| **Confidence band** | Coarse `{High, Medium, Low}` classification of the final score. |
| **`is_match`** | Score ≥ `MatchConfig::threshold` (default 0.85). |

## 4. Research basis

Approach mirrors the sibling matcher crates:

- **Name similarity:** Jaro-Winkler — proven on short titles, handles
  transpositions cheaply, prefix bonus matches catalog conventions
  where the leading discipline tag is preserved across variants
  ("Intro to CS" vs "Introduction to CS").
- **Set similarity:** Jaccard on the lowercased keyword / teaches
  sets — robust to ordering and exact-membership rather than
  fuzzy substring.
- **Renormalisation:** absent-component penalty was retired across
  the family on 2026-06-03 (post-Person Service fix). The matcher
  starts with the same convention.
- **Deterministic short-circuits:** DOI / Wikidata / OER / LOM / URI
  / UUID are globally unique by construction. `provider_id +
  course_code` is unique within a provider's catalogue.

Crate dependencies kept deliberately small:

- `strsim` for Jaro-Winkler.
- `unicode-normalization` for NFKC.
- `serde` + `serde_json` for round-trip.
- `thiserror` for the error enum.

## 5. Algorithm overview

```text
match_courses(A, B):
  if deterministic_match(A, B):
      return Score 1.0 with deterministic_match=true.

  components = [
    (name_score(A, B),               name_weight),
    (course_code_score(A, B),        course_code_weight),
    (provider_score(A, B),           provider_weight),
    (educational_level_score(A, B),  educational_level_weight),
    (set_jaccard(A.keywords, B.k),   keywords_weight),
    (set_jaccard(A.teaches, B.t),    teaches_weight),
  ]

  score = weighted_average(components)   # ignores None entries
  is_match = score >= threshold
  return MatchResult { score, is_match, confidence, breakdown }
```

## 6. Domain model

`src/course.rs`:

- `Course { name, alternate_names, course_code, provider_id,
  provider_name, educational_level, learning_resource_type,
  keywords, teaches, identifiers, same_as, in_language }`.
- `CourseIdentifier { scheme, value }`.
- `IdentifierScheme` — 12 variants (see §15).
- `EducationalLevel` — 12 variants + `Custom(String)` (see §12).
- `LearningResourceType` — 11 variants + `Custom(String)`.

## 7. Configuration

`src/config.rs::MatchConfig`:

| Field | Default |
|---|---|
| `threshold` | 0.85 |
| `name_weight` | 0.35 |
| `course_code_weight` | 0.15 |
| `provider_weight` | 0.15 |
| `educational_level_weight` | 0.10 |
| `keywords_weight` | 0.10 |
| `teaches_weight` | 0.15 |

Sum of weights = 1.00. Per §17 they're renormalised over the
*present* components.

Convenience presets: `MatchConfig::strict()` (threshold = 0.95) and
`MatchConfig::lenient()` (threshold = 0.70). Same weights.

## 8. Normalisation

`src/normalize.rs`:

- `fold(s)` — trim → NFKC → lowercase.
- `course_code(s)` — strip whitespace → uppercase.
- `fold_set(xs)` — fold each → drop empties → sort → dedup.

Detailed rules: [`AGENTS/normalization.md`](AGENTS/normalization.md).

## 9. Name similarity

- `jaro_winkler(fold(a.name), fold(b.name))` is the floor.
- Then try each `(alternate_names_a × b.name)` and `(a.name ×
  alternate_names_b)` and take the max.
- Final score is in `[0.0, 1.0]`. Never `None` (every Course has a
  `name`).

## 10. Course code

- When both records have `provider_id` AND both `provider_id` match
  AND both have `course_code`:
  - `course_code(a.course_code) == course_code(b.course_code)` →
    1.0.
  - Else 0.0.
- Otherwise the component is `None` (skipped from the weighted
  average).

Rationale: `CS101` exists at most universities. Without sharing the
provider it's noise; with shared provider it's identity-grade.

## 11. Provider

- When both have `provider_id`:
  - Equal → 1.0; not equal → 0.0.
- Else when both have `provider_name`:
  - `jaro_winkler(fold(a), fold(b))`.
- Else `None`.

## 12. Educational level

| Pair | Score |
|---|---|
| Same variant | 1.0 |
| Adjacent on the skill ladder (`Beginner < Intermediate < Advanced < Expert`) | 0.5 |
| Adjacent on the school ladder (`Primary < Secondary < Higher`) | 0.5 |
| Adjacent on the degree ladder (`Undergraduate < Graduate < Postgraduate`) | 0.5 |
| Across ladders | 0.0 |
| Either side `None` | component skipped |

`EducationalLevel::Custom(s)` is compared by equality on the inner
string.

## 13. Keywords

`fold_set(a.keywords)` ∩ `fold_set(b.keywords)` → Jaccard. Returns
`None` if both sides are empty. If exactly one side is empty: 0.0.

## 14. Teaches / competencies

Identical algorithm to §13 on the `teaches` lists.

## 15. Identifier short-circuits

`IdentifierScheme::is_deterministic` returns `true` for:

- `Doi`
- `Wikidata`
- `Lom`
- `Oer`
- `Uri`
- `Uuid`

A match on any two deterministic identifiers (same scheme + same
folded value) → score 1.0.

NOT deterministic: `LmsCourseId` (scoped to LMS instance, but the
value alone isn't globally unique), `CourseCode` (scoped to
provider — see §10), `PlatformSlug`, `Isced`, `Ror` (organisation
identifier — same provider on two records, but two courses at the
same provider aren't the same course), `Custom(_)` (unknown
semantics).

## 16. Same-as URL short-circuit

Any pair of `same_as` URLs that fold to the same string short-
circuits. The fold normalises scheme case + host case + path; we do
NOT strip trailing slashes (the URL `/` carrier matters in
canonical schema.org links).

## 17. Renormalisation

```text
weighted_sum = sum(score * weight for (Some(score), weight) in components)
weight_sum   = sum(weight        for (Some(_),     weight) in components)
final        = weight_sum > 0 ? weighted_sum / weight_sum : 0
```

Two records with **identical** name + provider_id + course_code
score 1.0 because the denominator is the *present* weight, not 1.00.

## 18. Confidence classification

- `Confidence::High` for score ≥ 0.95.
- `Confidence::Medium` for score in `[0.70, 0.95)`.
- `Confidence::Low` for score < 0.70.

This is independent of `MatchConfig::threshold` (used by
`is_match`).

## 19. Quality goals

- Zero `unsafe`.
- Zero `unwrap` / `expect` / `panic!` in library code.
- Crate compile time on a warm cache < 5 s.
- `cargo bench` plan covers name match throughput, full
  `match_courses` throughput, and `rank` against N=100.

## 20. Consumption

Embedded as a path dependency by
[`course-service`](../course-service-rust-crate/) via an `adapter`
module. The service's `Course` is the richer schema; the adapter
projects it down to the matcher's `Course` and back-fills missing
fields with defaults.

The adapter lives in the service crate (not here). This crate has no
SeaORM / Axum / Tantivy dependencies.

## 21. Compatibility

- Crate version follows semver. Pre-1.0 we allow patch-level field
  renames; from 1.0 onward field renames are major bumps.
- Adding a new optional field to `Course` is patch-level.
- Changing default weights is **minor** — semantically observable to
  downstream tests.
- Changing `IdentifierScheme::is_deterministic` for an existing
  variant is **major** — could create false positives.

## 22. Anti-patterns

- **Calling `match_courses` in a hot loop with the same `MatchingEngine`
  reconstructed each iteration.** Build the engine once.
- **Adding stop-words / stemming.** Out of scope (§2.2).
- **Setting weights to negative.** Not validated today — caller
  contract.
- **Using `Course::default()` in tests.** Use `Course::new(name)` so
  the required field is set.

## 23. Tasks

- [x] T-1: Scaffold (Cargo.toml, src/, spec, AGENTS, README, CHANGELOG).
- [x] T-2: Implement `match_courses` per §5 with all per-component fns.
- [x] T-3: `MatchConfig` + presets per §7.
- [x] T-4: `normalize::{fold, course_code, fold_set}` per §8.
- [x] T-5: Unit tests covering deterministic short-circuits + probabilistic ordering.
- [x] T-6: Phonetic (Soundex) bonus on `name` component — `src/phonetic.rs` + `+0.05` bonus applied inside `name_score` when both names produce the same Soundex code and Jaro-Winkler is `< 0.95`. Capped at `0.95` so a phonetic hit nudges Medium-band scores but never single-handedly mints High confidence.
- [ ] T-7: `course-service`-side adapter + bridge test
      (`tests/duplicate_detection.rs`).
- [ ] T-8: Criterion benches for name, full match, rank-of-100.
- [ ] T-9: Tighten `IdentifierScheme` documentation comment with examples
      per variant.
- [ ] T-10: Expose `MatchingEngine::match_one_to_many` (currently `rank`
      plus a filter) as a separate method for parity with sibling
      matcher crates.

## 24. Testing strategy

See [`AGENTS/testing.md`](AGENTS/testing.md). Summary:

- Unit tests in `#[cfg(test)] mod tests` blocks.
- Bridge tests live in the embedding service crate.
- Benchmarks under `benches/` (T-8 pending).

## 25. Change control

Material changes to:

- the §7 default weights,
- the §15 deterministic identifier set,
- the §18 confidence bands,
- the §5 algorithm trace,

MUST land in the same PR as the code change AND a bridge-test edit
in the consuming service crate.
