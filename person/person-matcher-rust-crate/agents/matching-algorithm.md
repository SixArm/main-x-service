# Matching Algorithm — Agent Guide

The authoritative description lives in [`../spec.md`](../spec/index.md) §12 and §13. This guide is the practitioner's view.

## Strategies and Surface

`MatchingEngine` exposes four entry points:

1. **`deterministic_match(&p1, &p2) -> bool`** — binary, fast, defensible. Use when a regulator or operator must see a clear yes/no.
2. **`match_persons(&p1, &p2) -> MatchResult`** — score, threshold, per-field breakdown. Use to triage a single pair.
3. **`match_one_to_many(&query, candidates) -> Vec<MatchResult>`** — score the query against a candidate slice; the output is parallel to the input. The building block for an identity-screening workflow (see spec §12.6 / FR-45).
4. **`rank_one_to_many(&query, candidates) -> Vec<(usize, MatchResult)>`** — same scoring, sorted by descending score with deterministic ascending-index tiebreak (FR-46).

The engine is immutable and `Send + Sync`, so consumers can wrap the batch entry points in `rayon::par_iter` / `tokio::task::spawn_blocking` without changes to this crate. Blocking (candidate pre-filtering) is a consumer concern.

In production, downstream services typically call both deterministic and probabilistic forms: deterministic for clinical confirmation, probabilistic for ranking and audit. Batch scoring sits on top of the same single-pair scoring pipeline, so per-field invariants (missing fields, scheme-locality, transposition heuristic, nickname boost) carry through unchanged.

## Deterministic Logic

Returns `true` iff **any one** of the following holds (illustrative subset —
all **42** per-scheme identifiers participate identically, plus
passport-book agreement; see "Deterministic Matching — Full Branch List"
below for the complete, canonical enumeration):

- Both UK United Kingdom National Health Service Numbers parse and are equal.
- Both France NIRs parse and are equal.
- Both España TSIs parse and are equal.
- Both Éire IHIs parse and are equal.
- Both UK Northern Ireland H&C Numbers parse and are equal.
- Both US SSNs parse and are equal.
- … (the remaining 36 schemes, each scheme-local, score the same way).
- At least one `(country, number)` passport-book pair is shared.
- Normalised given name matches AND normalised family name matches AND DOB matches exactly AND gender matches (or at least one is missing).

Identifiers are scheme-local: a United Kingdom National Health Service Number and an H&C Number with the same 10 digits do **not** cross-match. If you change this logic, update spec §12.1 and add an integration test.

## Probabilistic Pipeline

1. Compute component scores (each in `[0.0, 1.0]` or `None` if data missing/unparseable).
2. Compute the weighted sum across fields that scored.
3. Compute the sum of those participating weights.
4. If the optional phonetic-name score exceeds 0.9, add a 0.05-weighted bonus.
5. `score = weighted_sum / total_weight` (or 0.0 if no field scored).
6. `is_match = score >= match_threshold`.
7. `confidence = Confidence::from_score(score)` — see spec §12.5.

The weight-renormalisation step is important: missing data must not penalise the score. A person with name + DOB only must score 1.0 if both fields match — not be dragged down by "missing United Kingdom National Health Service Number" treated as a zero.

The `Confidence` band is **independent of `match_threshold`**: `score >= 0.90 → High`, `>= 0.75 → Medium`, else `Low`. Treat `confidence` as a triage hint; `is_match` (which incorporates the configured threshold) remains the authoritative go/no-go signal.

## Component Scoring At-a-Glance

| Field | Function | Notes |
|---|---|---|
| United Kingdom National Health Service Number | Exact equality of parsed `NHSNumber` | Both must parse, else `None`. |
| Given name | `name_algorithm` on normalised strings, then nickname lift to `≥ 0.9` if `nickname_table.are_equivalent(a, b)`; when both sides have a `middle_name`, blended as `0.95 × given + 0.05 × middle` (FR-49) | Default `Combined` (0.7 JW + 0.3 Lev); default table is empty. |
| Family name | Same as above | Default English table contains no family-name entries. |
| Date of birth | Exact equality (`1.0`), or same-year day/month transposition (`0.5`) — see §12.2 / FR-38 | Either both `Some` or `None`. The transposition heuristic only affects the probabilistic score; `deterministic_match` still requires exact equality. |
| Date of death | Same heuristic as date of birth via the shared `score_dob_pair` helper — see §12.4c / FR-83 | Independent from `date_of_birth_score`; default weight `0.10`. |
| Place of birth | City Jaro-Winkler + country exact, blended `0.7 / 0.3` — see §12.4a / FR-81 | Shared `score_named_place` helper. |
| Place of death | Same algorithm as place of birth via the shared `score_named_place` helper — see §12.4b / FR-84 | Independent from `birth_place_score`; default weight `0.05`. |
| Gender | Exact equality | |
| Address | See §12.4 of spec | Weighted average over postcode/city/line-1 contributions (`Σ(score × weight) / Σ(weight)`, weights `0.5 / 0.3 / 0.2`); line-1 is `(house_number, street)` after `parse_address_line`. |
| Phone | E.164 equality (preferred) or legacy national-significant comparison (fallback) | `phone` falls back to `mobile`. |
| Email | Exact equality of canonical form from `Normalizer::normalize_email` | Both must parse, else `None`. Gmail dot/+-folding opt-in via `MatchConfig::gmail_dot_folding`. |
| Phonetic | Mean Soundex equality of given + family | Only contributes if `> 0.9`. |

## When You Change Weights

- The default weight table is part of the documented behaviour (spec §13.1, README). Touch both when you change defaults.
- Add a `### Behaviour Change` subsection under the next CHANGELOG entry.
- Bump the minor version (pre-1.0 minor bumps may carry breaking behaviour by convention).

## Nickname Matching

- The matcher carries a `NicknameTable` on `MatchConfig`. Default is `NicknameTable::empty()` so existing callers see no change.
- When the table considers two normalised names equivalent (e.g. `"michael"` and `"mike"`), `score_name` lifts the per-name component to `max(score, 0.9)`. The boost never lowers a score and never crosses class boundaries.
- The built-in `NicknameTable::english()` is convenient but **not part of the stable contract** — entries may be added in minor releases. Pin a custom table via `with_class` if you need deterministic behaviour across upgrades.
- Family names are passed through the same `score_name` helper. The default English table has no family-name entries, so callers won't see surprise boosts there.

## DOB Transposition Heuristic

The probabilistic DOB sub-score has three outcomes (per spec §12.2, FR-38):

- `1.0` — exact equality.
- `0.5` — same year, and one side is a valid day/month swap of the other (`1995-01-10` vs `1995-10-01`). The swapped form is validated via `NaiveDate::from_ymd_opt`, so the heuristic never fires on a day > 12 or an out-of-range day-in-month.
- `0.0` — anything else.

The heuristic is intentionally narrow: years must agree, and the swap must produce a valid calendar date. It only lifts the probabilistic score. `deterministic_match` is unchanged — the demographic-tuple branch still requires exact `NaiveDate` equality.

Why 0.5 specifically: a transposition is meaningful evidence the records refer to the same person modulo a data-entry bug, but it is not strong enough on its own to clear default thresholds. Stacked with name + gender + phone matches, it can push a record from "Low" into "Medium" confidence.

## Phonetic Matching

- Uses American Soundex via the `soundex` crate. It is tuned for English-language US census names and is known to lose information on digraphs and non-English phonemes. **T-9 spike outcome (§21.4):** keep Soundex as the default; an opt-in `MatchConfig::phonetic_encoder` enum (`DoubleMetaphone`, `DaitchMokotoff`) is the recommended path forward, gated behind a `phonetic-rphonetic` Cargo feature flag and deferred until an empirical multinational person corpus is available. Implementation is tracked as T-9.1.
- The phonetic score is *bonus only* — it can lift a score but never lower it.
- Avoid making phonetic-only matches definitive; they have high false-positive rates for short names.

## Edge Cases to Remember

- An identifier that fails to parse should yield `<scheme>_score = None`, not `0.0`. The user shouldn't be punished for our parser's limits.
- An address with no comparable subcomponents returns `0.5` (neutral), not `0.0`. This is by design — we don't punish persons for missing address data.
- The address sub-score is the **best of** the cartesian product across `(current ∪ previous_addresses)` on both sides (FR-48 / §12.4.2). `address_score = None` only when at least one side has no address data at all. An unrelated historical entry must not lower a strong current-vs-current score because the engine takes the maximum.

## Strict Mode

When `MatchConfig::strict_mode = true`, the engine still computes the same probabilistic `score` and `confidence`, but it tightens the binary `is_match` decision to **also require `deterministic_match`**. A fuzzy near-match that clears the threshold but has no identifier agreement and no full demographic-tuple agreement is rejected as `is_match = false` (FR-47 / §13.2 / resolves OQ-5).

Consumers reading `MatchBreakdown` directly are unaffected — every per-field score, the overall `score`, and the `Confidence` band are identical across strict and non-strict configurations. Strict mode is a presentation-layer guard for the `is_match` boolean.

## Open Algorithm Questions

See spec §22 for the live list: OQ-7 (phonetic-bonus weighting). Resolved in 0.3.0: OQ-1 (middle-name scoring), OQ-2 (email / local_id scoring), OQ-3 (`#[non_exhaustive]`), OQ-4 (address arithmetic, T-3), OQ-5 (strict-mode enforcement).

When you have an opinion, propose a resolution in `spec.md` as a PR, not as a unilateral code change.

---

## Detailed Algorithm Specifications

The following sections were lifted from `spec.md` §12 to keep the spec terse. They remain canonical for the algorithm's wire-level behaviour.

### Deterministic Matching — Full Branch List

`deterministic_match` returns `true` iff **any** of the following hold:

1. **UK United Kingdom National Health Service Number agreement.** Both records have a `united_kingdom_national_health_service_number`, both parse via `identifiers::parse_united_kingdom_national_health_service_number`, and the canonical forms are equal.
2. **France NIR agreement.** Both records have an `fr_nir`, both parse via `identifiers::parse_fr_nir`, and the canonical forms are equal.
3. **España TSI agreement.** Both records have an `es_tsi`, both parse via `identifiers::parse_es_tsi`, and the canonical forms are equal.
4. **Éire IHI agreement.** Both records have an `ie_ihi`, both parse via `identifiers::parse_ie_ihi`, and the canonical forms are equal.
5. **UK Northern Ireland H&C Number agreement.** Both records have a `uk_hc_number`, both parse via `identifiers::parse_uk_hc_number`, and the canonical forms are equal.
6. **US SSN agreement.** Both records have a `us_ssn`, both parse via `identifiers::parse_us_ssn`, and the canonical forms are equal.
7. **Australia IHI agreement.** Both records have an `au_ihi`, both parse via `identifiers::parse_au_ihi`, and the canonical forms are equal.
8. **Germany KVNR agreement.** Both records have a `de_kvnr`, both parse via `identifiers::parse_de_kvnr`, and the canonical forms are equal.
9. **Italy *Codice Fiscale* agreement.** Both records have an `it_cf`, both parse via `identifiers::parse_it_cf`, and the canonical forms are equal.
10. **Netherlands BSN agreement.** Both records have an `nl_bsn`, both parse via `identifiers::parse_nl_bsn`, and the canonical forms are equal.
11. **Sweden *Personnummer* agreement.** Both records have an `se_personnummer`, both parse via `identifiers::parse_se_personnummer`, and the canonical forms are equal.
12. **UK Scotland CHI Number agreement.** Both records have a `uk_chi_number`, both parse via `identifiers::parse_uk_chi_number`, and the canonical forms are equal.
13. **T-27 schemes agreement.** Same shape for `be_nn`, `bg_egn`, `cz_rc`, `dk_cpr`, `ee_ik`, `es_dni`, `fi_hetu`, `hr_oib`, `is_kt`, `lt_ak`, `lv_pk`, `mt_id`, `no_fnr`, `pl_pesel`, `ro_cnp`, `si_emso`, `sk_rc`, `uk_nino` — each is scheme-local and any pair with equal canonical form fires.
14. **T-28 schemes agreement.** Same shape for `gr_dss`, `li_id`, `nl_id`, `pl_nip`, `pt_nif`.
14a. **T-17.1 schemes agreement.** Same shape for `br_cpf`, `cn_rrn`, `in_aadhaar`, `jp_my_number`, `mx_curp`, `nz_nhi`, `za_id` — each is scheme-local and any pair with equal canonical form fires (FR-85..FR-91).
15. **Passport-book agreement.** At least one `(country, number)` pair is shared across the two persons' `passport_books` lists after the canonicalisation performed by `PassportBook::new` (FR-52). Cross-country values with the same `number` MUST NOT match.
16. **Demographic tuple agreement.**
   - Normalised given names equal AND
   - Normalised family names equal AND
   - Dates of birth are exactly equal AND
   - Genders are equal OR at least one is `None` (missing gender does not fail this branch).

Otherwise it returns `false`.

National identifiers are scheme-local: a UK United Kingdom National Health Service Number is only ever compared against another UK United Kingdom National Health Service Number, never against an H&C Number that happens to share the same 10 digits.

### Component Scoring — Full Table

| Field | Score function | Score domain |
|---|---|---|
| UK United Kingdom National Health Service Number | Exact equality of canonical form from `parse_united_kingdom_national_health_service_number`; both must parse. | `{0.0, 1.0}`, else `None` |
| France NIR | Exact equality of canonical form from `parse_fr_nir`; both must parse. | `{0.0, 1.0}`, else `None` |
| España TSI | Exact equality of canonical form from `parse_es_tsi`; both must parse. | `{0.0, 1.0}`, else `None` |
| Éire IHI | Exact equality of canonical form from `parse_ie_ihi`; both must parse. | `{0.0, 1.0}`, else `None` |
| UK NI H&C Number | Exact equality of canonical form from `parse_uk_hc_number`; both must parse. | `{0.0, 1.0}`, else `None` |
| US SSN | Exact equality of canonical form from `parse_us_ssn`; both must parse. | `{0.0, 1.0}`, else `None` |
| Australia IHI | Exact equality of canonical form from `parse_au_ihi`; both must parse. | `{0.0, 1.0}`, else `None` |
| Germany KVNR | Exact equality of canonical form from `parse_de_kvnr`; both must parse. | `{0.0, 1.0}`, else `None` |
| Italy *Codice Fiscale* | Exact equality of canonical form from `parse_it_cf`; both must parse. | `{0.0, 1.0}`, else `None` |
| Netherlands BSN | Exact equality of canonical form from `parse_nl_bsn`; both must parse. | `{0.0, 1.0}`, else `None` |
| Sweden *Personnummer* | Exact equality of canonical form from `parse_se_personnummer`; both must parse. | `{0.0, 1.0}`, else `None` |
| UK Scotland CHI | Exact equality of canonical form from `parse_uk_chi_number`; both must parse. | `{0.0, 1.0}`, else `None` |
| Remaining 30 schemes (T-27: `be_nn`, `bg_egn`, `cz_rc`, `dk_cpr`, `ee_ik`, `es_dni`, `fi_hetu`, `hr_oib`, `is_kt`, `lt_ak`, `lv_pk`, `mt_id`, `no_fnr`, `pl_pesel`, `ro_cnp`, `si_emso`, `sk_rc`, `uk_nino`; T-28: `gr_dss`, `li_id`, `nl_id`, `pl_nip`, `pt_nif`; T-17.1: `br_cpf`, `cn_rrn`, `in_aadhaar`, `jp_my_number`, `mx_curp`, `nz_nhi`, `za_id`) | Same shape as the rows above: exact equality of the canonical form from the scheme's own `parse_<cc>_<scheme>`; both must parse. Each is scheme-local. | `{0.0, 1.0}`, else `None` |
| Passport book | `Some(1.0)` if any `(country, number)` pair is shared across `passport_books` on both sides; `Some(0.0)` if both non-empty but disjoint; `None` if either empty. | `{0.0, 1.0}`, else `None` |
| Given name | `name_algorithm` applied to normalised strings; raised to `0.9` when both names appear in the same class of `MatchConfig::nickname_table`. When both persons have a `middle_name`, the final score is `0.95 × given_sim + 0.05 × middle_sim` (FR-49). | `[0.0, 1.0]` |
| Family name | Same as given name (table-driven boost applies symmetrically; default English table contains no family-name entries). | `[0.0, 1.0]` |
| Date of birth | Exact equality, or `0.5` for a same-year day/month transposition. | `{0.0, 0.5, 1.0}` |
| Gender | Exact equality. | `{0.0, 1.0}` |
| Blood type | Exact equality of `BloodType` enum value. Stable for life so disagreement is reliable evidence of non-match; weak positive signal because many people share a type. | `{0.0, 1.0}`, else `None` |
| Multiple birth | Exact equality of FHIR `Patient.multipleBirth` integer (1-indexed birth order). Primary purpose: disambiguate identical twins. | `{0.0, 1.0}`, else `None` |
| Place of birth | City Jaro-Winkler blended with country exact match (`0.7 × city + 0.3 × country` when both present; single signal when only one); `None` when no comparable subset exists. | `[0.0, 1.0]`, else `None` |
| Date of death | Exact equality, or `0.5` for a same-year day/month transposition (same heuristic as date of birth). | `{0.0, 0.5, 1.0}`, else `None` |
| Place of death | Same scoring rule as place of birth — city + country blend via the shared `score_named_place` helper. | `[0.0, 1.0]`, else `None` |
| Address | Sub-score; see Address Sub-Score below. | `[0.0, 1.0]` |
| Phone | Exact equality after normalisation. | `{0.0, 1.0}` |
| Email | Exact equality of canonical form from `normalize_email`; both must parse. | `{0.0, 1.0}`, else `None` |
| Phonetic names | Average of given-name and family-name Soundex equality. | `{0.0, 0.5, 1.0}` |

A component scores `None` whenever input data is missing or unparseable on either side.

### Probabilistic Scoring Pipeline

```text
weighted_sum   = Σ_field  score_field × weight_field   (over fields with score = Some)
total_weight   = Σ_field  weight_field                  (over the same fields)
if phonetic_score is Some(s) and s > 0.9:
    weighted_sum  += s × 0.05
    total_weight  += 0.05
score = weighted_sum / total_weight   (or 0.0 if total_weight == 0)
is_match = score >= match_threshold
```

Notes:
- Weights are renormalised against participating fields. A record with only name and DOB does NOT silently get a low score for "missing" United Kingdom National Health Service Number — the missing field is simply not counted.
- The phonetic bonus is asymmetric: it only ever pushes the score up.

### Address Sub-Score

Given both `Address` values, scores are computed where both sides have a value:

| Sub-component | Comparison | Weight in sub-score |
|---|---|---|
| Postcode | Exact equality of normalised postcode (`0.0` or `1.0`). | 0.5 |
| City | Jaro-Winkler on normalised city. | 0.3 |
| Line 1 | Structured sub-score on `(house_number, street)` — see below. | 0.2 |

#### Line 1 Structured Sub-Score

For line 1, each side is parsed via `Normalizer::parse_address_line` into a `ParsedAddressLine { house_number, unit, street }`. The sub-score is computed as:

1. `street_sim` = Jaro-Winkler similarity of `parsed1.street` and `parsed2.street` (both are abbreviation-expanded and name-normalised, so `"High Street"` and `"High St"` produce equal strings and score `1.0`).
2. `house_score` = `Some(1.0)` if both `house_number`s are present and equal; `Some(0.0)` if both are present and differ; `None` if either is absent.
3. If `house_score` is `Some(h)`, the line-1 sub-score is `0.6 * street_sim + 0.4 * h`; otherwise it is `street_sim`.

The `unit` field is parsed and exposed on `ParsedAddressLine` but is intentionally **not** mixed into the line-1 sub-score: real-world data records unit information inconsistently, and weighting it would penalise legitimate matches.

Address sub-score = `Σ(score × weight) / Σ(weight)` over the contributions that fired, where the per-component weights are postcode = `0.5`, city = `0.3`, line 1 = `0.2`. If nothing fires, **0.5** is returned (neutral). The weighted-average form means postcode dominates as documented and the result is bounded in `[0.0, 1.0]` independent of how many sub-components fired.

#### Best-of Across Historical Addresses

For `MatchBreakdown::address_score`, the engine considers every pair drawn from `(p1.address ∪ p1.previous_addresses) × (p2.address ∪ p2.previous_addresses)`. Each pair is scored via the algorithm above and the **highest** score across the cartesian product is reported. `address_score` is `None` only when **at least one side has no address data at all** (neither current nor historical).

For very large `previous_addresses` lists, the cartesian product can grow quadratically. In practice records carry at most 2–3 historical addresses; consumers that ingest large histories SHOULD trim the list before matching.

### Place-of-Birth / Place-of-Death Sub-Score (shared `score_named_place`)

`MatchingEngine::score_birth_place` and `score_death_place` consume `Option<Address>` and delegate to the shared `score_named_place` free helper. Unlike the current-address sub-score, only the `city` and `country` sub-fields are considered.

Algorithm:

1. If either side has no place, return `None`.
2. Let `city = Jaro-Winkler(normalize_name(p1.city), normalize_name(p2.city))` when both sides have a city, else `None`.
3. Let `country = 1.0` if both `country` strings normalise equal, `0.0` if both are present but differ, `None` if either is absent.
4. Blend:
   - Both present: `0.7 × city + 0.3 × country`.
   - Only city: `city`.
   - Only country: `country`.
   - Neither: `None`.

Diacritics are absorbed by the shared name-normalisation pipeline (so `"Zürich"` and `"Zurich"` score identically). The sub-score is bounded `[0.0, 1.0]`.

### Date-of-Death Sub-Score

`MatchingEngine::score_death_date` consumes `Person::death_date: Option<NaiveDate>` and reuses the existing `score_dob_pair` free helper: exact equality yields `1.0`, a same-year day/month transposition yields `0.5`, otherwise `0.0`. Returns `None` when either side is absent. The transposition heuristic is justified by the same DD/MM ↔ MM/DD data-entry-error mode that motivates FR-38 for the date of birth.

### Confidence Bands

`MatchResult::confidence` is a fixed-band classification of `score`. It is independent of `match_threshold`: the same `score` always maps to the same band regardless of preset.

| Confidence | Score range |
|---|---|
| `High` | `score >= 0.90` |
| `Medium` | `0.75 <= score < 0.90` |
| `Low` | `score < 0.75` |

Boundaries are inclusive on the low side (a score of exactly `0.90` is `High`; exactly `0.75` is `Medium`). `Confidence::from_score(f64) -> Confidence` is total over `f64`: NaN and negative scores degrade to `Low`; scores above `1.0` are `High`. Bands are consultative — `is_match` remains the authoritative go/no-go signal.

### Batch Scoring

`MatchingEngine::match_one_to_many(query, candidates)` iterates `candidates` and produces one `MatchResult` per candidate via the same `match_persons` pipeline. The output `Vec<MatchResult>` is parallel to the input slice; index `i` in the output corresponds to index `i` in `candidates`. Empty candidates yield an empty `Vec`.

`MatchingEngine::rank_one_to_many(query, candidates)` returns a `Vec<(usize, MatchResult)>` where the `usize` is the original index in `candidates`. The vector is sorted by descending `MatchResult::score`. Ties are broken by ascending original index so the ranking is fully deterministic across calls.

Neither function performs blocking (candidate pre-filtering). Consumers that need blocking — e.g. only score candidates whose family-name Soundex equals the query's, or whose postcode outward code matches — MUST pre-filter the slice themselves.

The engine is `Send + Sync`, so parallel batch scoring (`rayon::par_iter`, `tokio::task::spawn_blocking`, …) is the consumer's choice. The crate intentionally does not take a parallelism dependency.
