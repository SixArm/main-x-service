# Normalisation — care-pathway-matcher

All in `src/normalize.rs`. Pure, deterministic, diacritic-preserving.

## `fold(s)`

Trim → Unicode NFKC → lowercase. The base for name, keyword,
intervention, and condition-code comparison. Diacritics are **kept**.

## `pathway_code(s)`

Keep only ASCII alphanumerics, uppercased. Drops whitespace, hyphens,
and punctuation, so `"STROKE-01"` and `"stroke 01"` both key to
`"STROKE01"`. Used for the provider-scoped pathway-code component and
the R-1 short-circuit.

## Condition-code tokens

Built in `matcher.rs` (not a public normalize fn): each `ConditionCode`
renders to a lower-cased `"system:code"` token (`Icd10`→`icd10`,
`Snomed`→`snomed`, `Custom(s)`→`fold(s)`), then compared by Jaccard via
`fold_set`.

## `fold_set(items)`

`fold` each, drop empties, sort, dedupe. Used for the keywords,
interventions, and condition-code Jaccards.

## Rule of thumb

New normalisation behaviour lives **here**, documented, with a unit
test — never inlined into `matcher.rs` (the condition-token helper is the
one documented exception, kept local to the scorer).
