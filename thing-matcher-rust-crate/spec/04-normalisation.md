## 4. Normalisation

All comparisons are done **after** normalisation. The normalisation routines live in `crate::normalizer` and are exposed through the `Normalizer` unit type. Every routine is **idempotent** (`f(f(x)) == f(x)`), **deterministic**, and allocates at most one new `String`.

| Routine | Use | Behaviour |
|---|---|---|
| `Normalizer::normalize_name(&str)` | Names and alternate names. | NFKD decompose → drop combining marks → drop ASCII punctuation → lowercase → collapse and trim ASCII whitespace. |
| `Normalizer::normalize_text(&str)` | `description`, `disambiguating_description`. | Lowercase → NFKD decompose → collapse whitespace → trim. Punctuation is **retained** so descriptions remain readable. |
| `Normalizer::normalize_url(&str)` | `url`, `image`, `main_entity_of_page`, every entry of `same_as` and `additional_types`. | Lowercase scheme + host; drop trailing slash on the path root. No DNS-aware normalisation, no percent-encoding canonicalisation, no punycode decoding. |
| `Normalizer::phonetic_code(&str)` | Soundex bonus (§6.5). | Classic 4-character Soundex code: first letter + three digits (`0` padding when fewer consonant digits are available). Diacritics are stripped via `normalize_name` first. |

Detailed per-rule behaviour (every NFKD edge case, every URL handling exception, exact whitespace handling) lives in [`AGENTS/normalization.md`](../AGENTS/normalization.md). Behaviours that consumers MUST rely on:

- **Whitespace.** Inputs MAY contain leading, trailing, or internal runs of any whitespace; all of it is canonicalised to single ASCII spaces, then trimmed.
- **Diacritics.** Latin diacritics (Spanish `ó`, German `ü`, French `é`, …) are stripped from names and Soundex codes; this is intentional and stable.
- **Punctuation.** ASCII apostrophes, hyphens, full stops, commas, parentheses, etc. are stripped from names. The curly apostrophe `’` (U+2019) is NOT recognised — upstream code MUST convert smart quotes to ASCII first.
- **URLs.** Equality is host- and scheme-insensitive, but path-, query-, and fragment-sensitive. Two URLs differing only by `?utm_source=…` are NOT equal.
- **Empty handling.** `normalize_name("")` and `normalize_name("   ")` both return `""`. The scoring layer treats empty / whitespace-only fields as "missing".

---

