# Normalisation — course-matcher

All matching components apply normalisation before similarity. The
rules are deliberately strict and centralised in `src/normalize.rs`
so a behavioural change is visible in a one-function diff.

## Pure functions

| Function | Rule |
|---|---|
| `fold(s)` | Trim → NFKC → lowercase. Returns an empty string for empty input (never `None`). |
| `course_code(s)` | Strip ALL whitespace → uppercase. `" cs 101 "` → `"CS101"`. |
| `fold_set(xs)` | `fold` each → drop empties → sort → dedup. Used for keyword / teaches set similarity. |

## Why these rules

- **Trim** keeps callers from sending leading/trailing whitespace
  by accident.
- **NFKC** unifies compatibility characters (full-width digits,
  Unicode-decomposed accents) so visually-identical strings score
  as equal.
- **Lowercase** matches schema.org's free-text convention; course
  catalogues rarely care about case beyond the heading.
- **Course code uppercase + no-whitespace** mirrors how registrars
  publish codes ("CS101" / "MATH 221") — strip the layout, keep the
  identity.
- **fold_set sorts + dedupes** so Jaccard similarity is computed on
  set semantics rather than bag semantics.

## What we don't do

- **Stemming.** Out of scope. "introduction" and "introductions" are
  treated as different.
- **Synonym expansion.** No "intro" → "introduction" rewrite. The
  Jaro-Winkler similarity handles those edits naturally.
- **Stopword removal.** No "the" / "of" stripping; titles are short
  enough that the noise is bounded.
- **Translation / transliteration.** Cross-language matching is
  out of scope — set `same_as` to the canonical resource if you need
  cross-language linkage.

## When to extend

Add a new normalisation helper when:

1. Two components want the same transformation. (Don't duplicate.)
2. The transformation is total — never panics, no I/O.
3. You can write a one-line unit test that pins the rule.
