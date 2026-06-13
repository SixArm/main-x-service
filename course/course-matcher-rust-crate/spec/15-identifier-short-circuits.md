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

