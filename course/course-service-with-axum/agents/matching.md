# Matching algorithm reference — Course Service

This is the service-side adapter view. The canonical algorithm lives
in the sibling [`course-matcher`](../../course-matcher-rust-crate/)
crate; see that crate's `agents/matching-algorithm.md` for the
research basis and component-by-component derivations.

## Strategies

- **Probabilistic** — weighted fuzzy sum of component scores, normalised
  over present components (no penalty for absent ones).
- **Deterministic** — rule-based; identifier-scheme short-circuits.

## Probabilistic weights

| Component | Weight | Algorithm |
|---|---|---|
| Name | 0.35 | Jaro-Winkler + Levenshtein + Soundex (case-insensitive) |
| Course code | 0.15 | Exact (case-insensitive); short-circuit to 1.0 within the same `provider_id` |
| Provider | 0.15 | Exact `provider_id` (1.0/0.0), or `Provider.name` Jaro-Winkler |
| Educational level | 0.10 | Exact enum match (1.0) / one level off (0.5) / else 0.0 |
| Keywords | 0.10 | Jaccard on lowercased keywords set |
| Teaches | 0.15 | Jaccard on lowercased competencies set |
| Identifier | (short-circuit only) | See deterministic below |

Weights sum to 1.0. Only components for which both Courses have data
contribute to the weighted average. The renormalisation rule mirrors
the post-2026-06-03 fix in person-service.

## Deterministic short-circuits

A match on any of the following pins the final score to `1.0`,
`confidence = High`, `breakdown.deterministic_match = true`:

| Rule | Condition |
|---|---|
| **R-0** | Any pair of deterministic identifiers shares a value (DOI / Wikidata / LOM / OER / URI / UUID — see `IdentifierType::is_deterministic`). |
| **R-1** | Both courses share `provider_id` AND `course_code` (case-insensitive normalised). |
| **R-2** | Any `same_as` URL overlaps (scheme-normalised host + path). |

## Match-quality classification

| Quality | Score range |
|---|---|
| Definite | ≥ 0.95 |
| Probable | ≥ threshold (default 0.85) |
| Possible | ≥ 0.50 |
| Unlikely | < 0.50 |

The 0.85 default matches the family-wide convention.

## Component details

### Name

- `Jaro-Winkler` similarity, case-insensitive, prefix bonus.
- Combined with `Levenshtein` (normalised by max length).
- Final = max of the two.
- `Soundex` bonus: +0.05 when codes match and final < 0.95.

### Course code

- Normalised to uppercase, strip whitespace.
- Exact equality only within the same `provider_id`. Across
  providers `CS101` is too noisy to be a signal (false positives at
  every university). The matcher returns `None` for this component
  when `provider_id` differs.

### Provider

- Service-side: exact `provider_id` match (1.0) or `Provider.name`
  Jaro-Winkler (when provider records are loaded).
- Matcher-side: see the `course-matcher` adapter.

### Educational level

- `EducationalLevel::Custom(String)` is compared by case-insensitive
  equality on the inner string.
- Ordered enum variants (Beginner < Intermediate < Advanced <
  Expert; PrimaryEducation < SecondaryEducation < HigherEducation;
  Undergraduate < Graduate < Postgraduate) score:
  - exact = 1.0
  - one level off in the same family = 0.5
  - across families = 0.0

### Keywords / Teaches

- Lowercase + trim each entry.
- Jaccard similarity on the resulting sets: `|A ∩ B| / |A ∪ B|`.
- Returns `None` (skip from weighted sum) when either side is empty.

### Identifier

- Per spec §6 FR-20 a deterministic-identifier match short-circuits
  the entire score. Otherwise the identifier component is `None`.

## Service-side adapter

`src/matching/adapter.rs` (T-6, shipped) maps the service's
`Course` record into `course_matcher::Course`. Field-routing rules:

- `Course.name` → `course_matcher::Course::name`.
- `Course.alternate_names` → `course_matcher::Course::alternate_names`.
- `Course.course_code` + `Course.provider_id` → matcher's
  provider-scoped course-code field.
- `Course.identifiers` → matcher's `Vec<CourseIdentifier>`, with
  `property_id` mapped to the matcher's `IdentifierScheme` enum
  (matcher mirrors but doesn't re-derive the deterministic set).
- `Course.same_as` → matcher's `Vec<String>` of authoritative URLs.
- `Course.educational_level` + `Course.learning_resource_type` →
  matcher fields.
- `Course.keywords` / `Course.teaches` / `Course.assesses` → matcher fields.

The adapter is the pinch point for any field-name drift; bridge
tests in `tests/duplicate_detection.rs` (T-11, 14 tests) pin the
mapping.

## Phonetic bonus (matcher T-6)

The matcher applies a `+0.05` Soundex bonus to `name_score` when
both course names produce the same Soundex code and the
underlying Jaro-Winkler is `< 0.95`. The result is capped at
`0.95` — a phonetic hit lifts a Medium-band score upward but never
single-handedly mints a High-confidence classification.

Soundex is initial-letter-preserving: `Smyth ↔ Smith` matches
(both `S530`); `Catherine ↔ Katheryn` does not.
