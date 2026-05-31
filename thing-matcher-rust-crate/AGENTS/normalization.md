# Normalisation — agent guide

See [`../spec.md`](../spec.md) §4 for the formal rules. This guide is the operational view, with extra examples per rule.

## Why normalise at all?

The research base is unanimous: most accuracy gains come from data standardisation, not from cleverer scoring. Garbage in, garbage out — no Jaro-Winkler trick rescues an apostrophe mismatch or a casing difference in a URL host.

## What we normalise

| Input | Algorithm | Source | Spec |
|---|---|---|---|
| Names | NFKD + drop combining marks + drop ASCII punctuation + lowercase + collapse whitespace | `src/normalizer.rs::normalize_name` | §4 |
| Free-form text | Lowercase + NFKD + collapse whitespace + trim (punctuation retained) | `src/normalizer.rs::normalize_text` | §4 |
| URLs | Lowercase scheme + host; drop trailing slash on path root; preserve path / query / fragment | `src/normalizer.rs::normalize_url` | §4 |
| Phonetic codes | `normalize_name` then 4-character American Soundex | `src/normalizer.rs::phonetic_code` | §4 |

All normalisers are **idempotent**: `f(f(x)) == f(x)`. If you add one that isn't, document the reason in `spec.md` and add a property test.

## Input → output tables

### Names (`Normalizer::normalize_name`)

| Input | Output |
|---|---|
| `"Eiffel Tower"` | `"eiffel tower"` |
| `"  Pride and  Prejudice  "` | `"pride and prejudice"` |
| `"O'Reilly's Practical Rust"` | `"oreillys practical rust"` |
| `"WAR-AND-PEACE"` | `"warandpeace"` |
| `"Café Society"` | `"cafe society"` |
| `"Siân"` | `"sian"` |
| `"Łódź"` | `"łodz"` (stroked `ł` survives NFKD; combining acute on `ó` / `ź` is stripped) |
| `""` / `"   "` | `""` |

### Free-form text (`Normalizer::normalize_text`)

Used for `description` and `disambiguating_description`. Punctuation is **retained** so the text stays readable.

| Input | Output |
|---|---|
| `"  The   Eiffel Tower.  "` | `"the eiffel tower."` |
| `"A 1813 novel by Jane Austen."` | `"a 1813 novel by jane austen."` |
| `"Roman à clef"` | `"roman a clef"` |
| `""` / `"   "` | `""` |

### URLs (`Normalizer::normalize_url`)

| Input | Output |
|---|---|
| `"HTTPS://Example.ORG/"` | `"https://example.org"` |
| `"https://en.wikipedia.org/wiki/Eiffel_Tower"` | `"https://en.wikipedia.org/wiki/Eiffel_Tower"` |
| `"https://Example.org/Path/"` | `"https://example.org/Path/"` (trailing-slash trim only on root path) |
| `"https://example.org/?utm=foo"` | `"https://example.org/?utm=foo"` (query preserved) |
| `"https://example.org#section"` | `"https://example.org#section"` (fragment preserved) |
| `""` / `"   "` | `""` |

### Phonetic codes (`Normalizer::phonetic_code`)

Classic 4-character American Soundex: first letter (uppercased) + three digits, padded with `0` if fewer consonant digits are available. Vowels (`A E I O U`) and `H W Y` are ignored. Adjacent consonants mapping to the same digit collapse.

| Letters | Digit |
|---|---|
| B F P V | 1 |
| C G J K Q S X Z | 2 |
| D T | 3 |
| L | 4 |
| M N | 5 |
| R | 6 |

| Input | Code |
|---|---|
| `"Robert"` | `"R163"` |
| `"Rupert"` | `"R163"` |
| `"Ashcraft"` | `"A261"` |
| `"Smith"` | `"S530"` |
| `"Smyth"` | `"S530"` |
| `""` | `""` |

The name is first run through `normalize_name`, so `"Robért"` and `"Robert"` produce the same code.

## What we do not normalise

- **`local_id`** — intentionally not normalised AND not scored. Different sources may issue colliding values.
- **`identifier.value`** — trimmed at construction by `Identifier::new`, otherwise compared verbatim. Different vocabularies (`isbn`, `doi`, `gtin`, `wikidata`, …) have different canonical forms; the crate makes no per-scheme assumptions. Consumers MUST canonicalise upstream if ISBN-10 ↔ ISBN-13, with-hyphens ↔ without-hyphens, etc., matter.
- **`identifier.property_id`** — trimmed at construction, otherwise case-sensitive. `"wikidata"` and `"WikiData"` are distinct schemes.
- **`subject_of`, `owner`** — preserved as supplied, not scored.

## Adding a new normaliser

1. Add the new public method on `Normalizer` in `src/normalizer.rs`.
2. Add unit tests covering: empty input, all-whitespace input, a realistic happy-path input, and a diacritic case if the field can contain Unicode names.
3. Update [`../spec.md`](../spec.md) §4 with the algorithm steps and a table of examples.
4. If a scoring path uses the new normaliser, document it in `spec.md` §6.

## Pitfalls

- **Smart quotes.** `Normalizer::normalize_name` strips ASCII punctuation only. The curly apostrophe `’` (U+2019) is NOT recognised. Upstream code MUST convert smart quotes to ASCII first, or names like `"O’Brien"` and `"O'Brien"` will not match.
- **Stroked letters.** Latin letters with an integral stroke (`ł`, `ø`, `ð`) do NOT decompose under NFKD. They survive normalisation, lowercased. This is intentional and stable.
- **URL query strings.** `Normalizer::normalize_url` does not strip query parameters. `?utm_source=foo` will defeat URL equality. If you need that, normalise upstream.
- **URL percent-encoding.** Not canonicalised. `%20` vs `+` vs literal space are three different strings to the matcher.
- **Punycode.** Not decoded. `xn--…` and the unicode host form do not match.

## Trade-offs (deliberate)

- **No address parsing.** `Thing` has no address field — addresses belong on `Place` records in the sibling `place-matcher` crate.
- **No phone normalisation.** `Thing` has no phone field — phone numbers belong on `Person` / `Place` records in the sibling crates.
- **No identifier canonicalisation.** Vocabulary-specific rules (ISBN-10 ↔ ISBN-13, GTIN check digits, DOI case) belong upstream; the matcher compares opaque strings. See `spec.md` §10 OQ-B for the open question.
