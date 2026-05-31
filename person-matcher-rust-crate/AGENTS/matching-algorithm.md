# Matching Algorithm — Agent Guide

The authoritative description lives in [`../spec.md`](../spec.md) §12 and §13. This guide is the practitioner's view.

## Strategies and Surface

`MatchingEngine` exposes four entry points:

1. **`deterministic_match(&p1, &p2) -> bool`** — binary, fast, defensible. Use when a regulator or clinician must see a clear yes/no.
2. **`match_persons(&p1, &p2) -> MatchResult`** — score, threshold, per-field breakdown. Use to triage a single pair.
3. **`match_one_to_many(&query, candidates) -> Vec<MatchResult>`** — score the query against a candidate slice; the output is parallel to the input. The building block for an MPI screening workflow (see spec §12.6 / FR-45).
4. **`rank_one_to_many(&query, candidates) -> Vec<(usize, MatchResult)>`** — same scoring, sorted by descending score with deterministic ascending-index tiebreak (FR-46).

The engine is immutable and `Send + Sync`, so consumers can wrap the batch entry points in `rayon::par_iter` / `tokio::task::spawn_blocking` without changes to this crate. Blocking (candidate pre-filtering) is a consumer concern.

In production, downstream services typically call both deterministic and probabilistic forms: deterministic for clinical confirmation, probabilistic for ranking and audit. Batch scoring sits on top of the same single-pair scoring pipeline, so per-field invariants (missing fields, scheme-locality, transposition heuristic, nickname boost) carry through unchanged.

## Deterministic Logic

Returns `true` iff **any one** of the following holds:

- Both UK NHS Numbers parse and are equal.
- Both France NIRs parse and are equal.
- Both España TSIs parse and are equal.
- Both Éire IHIs parse and are equal.
- Both UK Northern Ireland H&C Numbers parse and are equal.
- Both US SSNs parse and are equal.
- Normalised given name matches AND normalised family name matches AND DOB matches exactly AND gender matches (or at least one is missing).

Identifiers are scheme-local: an NHS Number and an H&C Number with the same 10 digits do **not** cross-match. If you change this logic, update spec §12.1 and add an integration test.

## Probabilistic Pipeline

1. Compute component scores (each in `[0.0, 1.0]` or `None` if data missing/unparseable).
2. Compute the weighted sum across fields that scored.
3. Compute the sum of those participating weights.
4. If the optional phonetic-name score exceeds 0.9, add a 0.05-weighted bonus.
5. `score = weighted_sum / total_weight` (or 0.0 if no field scored).
6. `is_match = score >= match_threshold`.
7. `confidence = Confidence::from_score(score)` — see spec §12.5.

The weight-renormalisation step is important: missing data must not penalise the score. A person with name + DOB only must score 1.0 if both fields match — not be dragged down by "missing NHS number" treated as a zero.

The `Confidence` band is **independent of `match_threshold`**: `score >= 0.90 → High`, `>= 0.75 → Medium`, else `Low`. Treat `confidence` as a triage hint; `is_match` (which incorporates the configured threshold) remains the authoritative go/no-go signal.

## Component Scoring At-a-Glance

| Field | Function | Notes |
|---|---|---|
| NHS number | Exact equality of parsed `NHSNumber` | Both must parse, else `None`. |
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

- The default weight table is part of the documented behaviour (spec §13.1, README, IMPLEMENTATION_SUMMARY). Touch all three when you change defaults.
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
