## 6. Domain model

`src/course.rs` — as shipped:

- `Course { name, alternate_names, course_code, provider_id,
  provider_name, educational_level, learning_resource_type,
  keywords, teaches, identifiers, same_as, in_language,
  relationships, tags }`.
- `CourseIdentifier { scheme, value }`.
- `IdentifierScheme` — 12 variants (see §15).
- `EducationalLevel` — 12 variants + `Custom(String)` (see §12).
- `LearningResourceType` — 11 variants + `Custom(String)`.

`relationships: Vec<RelationshipRef>` (default empty; see §6.1) —
typed references to other courses by registry id. A **supporting**
signal, NOT an identifying field on its own: two records that
reference the **same** related courses (same similar / higher-level /
lower-level course ids) are more likely the same course. Scored by
typed-set overlap (§5.1), weighted `relationships_weight` (§7).

`tags: Vec<String>` (default empty) — operator-applied operational
labels (grouping, triage, workflow). A **supporting** signal, NOT an
identifying field on its own: two records that share tags are somewhat
more likely the same course. Scored by plain set Jaccard over the
case-insensitively normalised sets (§5.2; `None` when either side
empty), weighted `tags_weight` (§7). Distinct from `keywords`
(descriptive subject terms) but scored the same way.

### 6.1 `RelationshipRef` / `RelationKind` (shipped — §23 T-11)

`RelationshipRef { relation: RelationKind, course_id: String }`
references another course in the consuming registry by **opaque id**;
`course_id` is whitespace-trimmed and non-empty. `RelationKind` is an
enum mirroring the service `Course`: `SimilarTo` (symmetric),
`HigherLevelThan` / `LowerLevelThan` (inverses). The matcher does
**not** resolve the references (it has no registry) — it only compares
the two courses' relationship **sets** (§5.1). Derives
`Debug + Clone + PartialEq + Eq + Hash + Serialize + Deserialize`;
re-exported from the crate root.

### 6.2 `MatchBreakdown`

Carries one `Option<f64>` per probabilistic component (`None` = not
scored; `Some(v)` ∈ `[0.0, 1.0]`): `name_score`, `course_code_score`,
`provider_score`, `educational_level_score`, `keywords_score`,
`teaches_score`, `relationships_score`, `tags_score`, plus
`deterministic_match`.

