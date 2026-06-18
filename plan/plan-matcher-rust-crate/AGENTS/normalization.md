# Normalisation — plan-matcher

All in `src/normalize.rs`. Pure, deterministic, diacritic-preserving.

## `fold(s)`

Trim → Unicode NFKC → lowercase. The base for name, goal-title,
keyword, and tag comparison. Diacritics are **kept**.

## `plan_code(s)`

Keep only ASCII alphanumerics, uppercased. Drops whitespace, hyphens,
and punctuation, so `"PLAN-01"` and `"plan 01"` both key to `"PLAN01"`.
Used for the owner-scoped plan-code component and the R-1 short-circuit.

## `fold_set(items)`

`fold` each, drop empties, sort, dedupe. Used for the goal-title,
keywords, and tags Jaccards.

## Timeframe dates

Built in `matcher.rs` (not a public normalize fn): `start_date` /
`target_date` are compared pairwise by day gap, fed through the Gaussian
decay `exp(-(Δdays/σ)²/2)`. No string folding applies — dates are
compared as calendar values.

## Rule of thumb

New normalisation behaviour lives **here**, documented, with a unit
test — never inlined into `matcher.rs` (the timeframe date helper is the
one documented exception, kept local to the scorer).
