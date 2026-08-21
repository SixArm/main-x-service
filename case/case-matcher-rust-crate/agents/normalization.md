# Normalisation — case-matcher

All in `src/normalize.rs`. Pure, deterministic, diacritic-preserving.

## `fold(s)`

Trim → Unicode NFKC → lowercase. The base for title, keyword, and
subject comparison. Diacritics are **kept**.

## `case_number(s)`

Keep only ASCII alphanumerics, uppercased. Drops whitespace, hyphens,
and punctuation, so `"CV-2024-001234"` and `"cv 2024 001234"` both key
to `"CV2024001234"`. Used for the agency-scoped case-number component
and the R-1 short-circuit.

## `url(s)`

`fold` then drop a single trailing slash (unless the whole string is
`"/"`). Used for the R-2 `same_as` overlap rule so trivial formatting
differences do not defeat the comparison.

## `fold_set(items)`

`fold` each, drop empties, sort, dedupe. Used for the subjects and
keywords Jaccards.

## Rule of thumb

New normalisation behaviour lives **here**, documented, with a unit
test — never inlined into `matcher.rs`.
