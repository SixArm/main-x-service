# Matching algorithm — course-matcher

## Strategies

- **Probabilistic** — weighted average over the components both
  records actually carry. Absent fields don't penalise the score.
- **Deterministic** — three short-circuit rules pin the score to 1.0.

## Deterministic short-circuits

| Rule | Condition |
|---|---|
| **R-0** | Both records share a value on a deterministic identifier scheme. |
| **R-1** | Both records share `provider_id` AND `course_code` (normalised). |
| **R-2** | A `same_as` URL overlaps (case-folded, NFKC). |

Deterministic schemes (`IdentifierScheme::is_deterministic`): `Doi`,
`Wikidata`, `Lom`, `Oer`, `Uri`, `Uuid`. `LmsCourseId`,
`CourseCode`, `PlatformSlug`, `Isced`, `Ror`, `Custom(_)` are NOT
deterministic — `CS101` exists at many universities.

## Probabilistic components

| Component | Default weight | Algorithm |
|---|---|---|
| Name | 0.35 | `max(Jaro-Winkler(name_a, name_b), …)` over the cross-product of `(name + alternate_names)` |
| Course code | 0.15 | Within same `provider_id`: 1.0 if normalised values equal, else 0.0. Across providers: returns `None` (skipped). |
| Provider | 0.15 | Exact `provider_id` (1.0 / 0.0). When `provider_id` is absent, Jaro-Winkler on `provider_name`. |
| Educational level | 0.10 | Exact enum match = 1.0. One step apart on the same ladder (skill / school / degree) = 0.5. Else 0.0. |
| Keywords | 0.10 | Jaccard on `normalize::fold_set(keywords)`. Skipped if both sides empty. |
| Teaches | 0.15 | Jaccard on `normalize::fold_set(teaches)`. Skipped if both sides empty. |

Weights sum to 1.0.

### Renormalisation

For a pair where (say) only `name` and `provider_id` are present
on both sides, the weighted average runs over weights
`{0.35, 0.15}` and the denominator is `0.50`, not `1.00`. Identical-
name + identical-provider therefore scores 1.0, not 0.50.

### Confidence band

| Band | Score range |
|---|---|
| `High` | ≥ 0.95 |
| `Medium` | ≥ 0.70 |
| `Low` | < 0.70 |

These bands are separate from `MatchConfig::threshold` (used by
`is_match`). The default 0.85 threshold means "probable" matches set
`is_match = true` while still classified as `Medium`.

## Algorithm trace

```text
Input: Course A, Course B, MatchConfig
  │
  ├─ R-0 — deterministic identifier match? ──yes──> 1.0
  ├─ R-1 — same provider + course_code?     ──yes──> 1.0
  ├─ R-2 — same_as URL overlap?             ──yes──> 1.0
  │
  ├─ name_score              (always Some)
  ├─ course_code_score       (Some when same provider)
  ├─ provider_score          (Some when both sides have provider info)
  ├─ educational_level_score (Some when both sides set)
  ├─ keywords_score          (Some when at least one side has keywords)
  ├─ teaches_score           (Some when at least one side has teaches)
  │
  ├─ Renormalised weighted average over Some components
  └─ MatchResult { score, is_match, confidence, breakdown }
```

## Configuration

`MatchConfig::default()` ships the weights documented above and
`threshold = 0.85`. Two convenience presets:

- `MatchConfig::strict()` — threshold = 0.95 (auto-merge only on
  high confidence).
- `MatchConfig::lenient()` — threshold = 0.70 (UI "find similar
  courses").

## Worked example

```rust
use course_matcher::{Course, IdentifierScheme, MatchingEngine, MatchConfig};

let engine = MatchingEngine::new(MatchConfig::default());

let mut a = Course::new("Introduction to Computer Science");
a.course_code   = Some("CS101".into());
a.provider_id   = Some("ror-021nxhr62".into());   // Stanford ROR
a.keywords      = vec!["computer science".into(), "programming".into()];

let mut b = Course::new("CS101");
b.course_code   = Some("cs 101".into());
b.provider_id   = Some("ror-021nxhr62".into());

let r = engine.match_courses(&a, &b);
// R-1 fires: same provider, normalised course-codes match.
assert_eq!(r.score, 1.0);
assert!(r.breakdown.deterministic_match);
```

## Phonetic bonus (T-6)

`src/phonetic.rs` ships the classic American Soundex encoder. Inside
`name_score`:

```
if best < 0.95 && phonetic::same(&name_a, &name_b) {
    best = (best + 0.05).min(0.95);
}
```

Cap at `0.95` is intentional — a phonetic hit nudges Medium-band
scores upward but never single-handedly mints a High-confidence
classification. Soundex is initial-letter-preserving by design, so
the bonus only fires when both course names start with the same
letter (e.g. `Smyth` ↔ `Smith` matches; `Catherine` ↔ `Katheryn`
does not).

## Open questions

- Should `LmsCourseId` become deterministic *within an LMS instance*?
  Today: no. Could be a future field `CourseIdentifier::scope`.
