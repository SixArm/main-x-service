# Normalisation — organization-matcher

All in `src/normalize.rs`. Pure, deterministic, diacritic-preserving.

## `fold(s)`

Trim → Unicode NFKC → lowercase. The base for every comparison.
Diacritics are **kept** (`Müller` folds to `müller`, not `muller`).

## `legal_name(s)`

The organization-name comparison key:

1. `fold`.
2. Replace every non-alphanumeric char with a space (drops commas,
   periods, ampersands → tokens).
3. Drop legal-form suffix tokens and noise words: `inc`, `incorporated`,
   `corp`, `corporation`, `co`, `company`, `ltd`, `limited`, `llc`,
   `llp`, `lp`, `plc`, `gmbh`, `ag`, `sa`, `sas`, `sasu`, `srl`, `spa`,
   `bv`, `nv`, `oy`, `ab`, `as`, `pty`, `pte`, `kk`, `kg`, `ohg`, `ug`,
   `sl`, `sarl`, `eurl`, `the`, `and`, `&`.
4. Collapse whitespace. Never returns empty (falls back to the cleaned
   form if every token was a suffix, e.g. `"The Co"`).

So `"Acme, Inc."`, `"ACME Corporation"`, and `"Acme"` all key to
`"acme"`.

## `domain(s)`

URL → comparable registered domain: strip scheme, userinfo, port, path,
query, fragment, trailing dot, and a leading `www.`.
`"https://www.Acme.com/about"` → `"acme.com"`.

## `fold_set(items)`

`fold` each, drop empties, sort, dedupe. Used for the keywords Jaccard.

## Rule of thumb

New normalisation behaviour lives **here**, documented, with a unit
test — never inlined into `matcher.rs`.
