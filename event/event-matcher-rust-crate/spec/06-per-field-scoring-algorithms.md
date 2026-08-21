## 6. Per-field scoring algorithms

Each component returns `Option<f64>` in `[0.0, 1.0]`. A `None` means the component did not participate (§5.4). Per-algorithm pseudocode, edge cases, and lookup tables are maintained in [`agents/matching-algorithm.md`](../agents/matching-algorithm.md); the contracts below are normative.

### 6.1 Name — `name_score`

`collect_names(e)` gathers `e.name` plus `e.alternate_names`, skipping empty / whitespace-only strings. The matcher computes the similarity for every pair drawn from `collect_names(e1) × collect_names(e2)` (after `normalize_name` on each side) using `MatchConfig::name_algorithm`, and returns the maximum. Algorithm selector values: `JaroWinkler` (prefix-biased; good for short names), `Levenshtein` (`1 - edit_distance / max_len`), `Exact` (binary, case-sensitive), `Combined = 0.7 * JaroWinkler + 0.3 * Levenshtein` (**default**). Two empty strings score `1.0`; empty vs non-empty scores `0.0`. `name_score` is `None` when either side has no usable names.

### 6.2 Phonetic — `name_phonetic_score`

Only computed when `MatchConfig::use_phonetic_matching = true`. For every pair across `collect_names`, compute `Normalizer::phonetic_code(name)` on each side; result is `1.0` if any pair of non-empty codes matches, else `0.0`. Bonus-only: when `> 0.9`, the weighted-sum reducer adds `name_phonetic_score * 0.05` to weighted sum and `0.05` to total weight (§5.2.2). The bonus MUST NOT lower the overall score. `None` when phonetic matching is off, or either side has no usable names.

### 6.3 Start / end date — `start_date_score`, `end_date_score`

Compute `d = Scorer::seconds_between(t1, t2)` (absolute difference in seconds; both inputs parsed via `Normalizer::parse_iso8601_unix_seconds`, §4.7), then apply Gaussian decay `exp(-(d / s)^2)` where `s = MatchConfig::start_date_scale_seconds` (`Scorer::start_date_score`). Contract: `d == 0 → 1.0`; `d == s → 1/e ≈ 0.368`; `d == 3s → ~0.0001`; non-finite `d`, non-positive `s`, negative `d` → `0.0`. The **same scale** governs both the start-date and end-date components; only the weights differ. Either score is `None` when the corresponding field is missing on either side or fails to parse as ISO 8601. A date-only value (`"2024-06-26"`) anchors at midnight UTC.

### 6.4 Location — `location_score`

When both sides have a `location`, run a weight-renormalised average across sub-components present on both sides, with sub-weights **Coordinates 0.5**, **Address 0.3**, **Venue name 0.15**, **Virtual URL 0.05**:

- **Coordinates sub-score** — `d = Scorer::haversine_metres(lat1, lon1, lat2, lon2)`, then Gaussian decay `exp(-(d / s)^2)` with `s = MatchConfig::coordinates_scale_metres` (`Scorer::coordinates_score`). `Scorer::haversine_metres` uses mean Earth radius `6_371_000.0` m on a sphere; total over `f64`; handles equator and date-line crossings. Participates only when both sides carry latitude **and** longitude that are finite and inside `[-90, 90]` / `[-180, 180]`.
- **Address sub-score** — weight-renormalised blend across populated address parts with sub-weights **Postcode 0.5**, **City 0.3**, **Line 1 0.2**. Postcode part: `1.0` iff `normalize_postcode(a) == normalize_postcode(b)` else `0.0`. City part: `jaro_winkler_similarity` on normalised city names. Line-1 part: `parse_address_line` on each side; with house numbers present on both, `0.6 * jaro_winkler(street) + 0.4 * (house_number_a == house_number_b)`; otherwise street similarity alone. If no part is populated on both sides, the address sub-score is neutral `0.5`.
- **Venue-name sub-score** — `Scorer::combined_similarity` on `normalize_name(venue_name)` from each side.
- **Virtual-URL sub-score** — `1.0` iff equal after trimming whitespace, else `0.0`.

If no sub-component is populated on both sides, the location score is neutral `0.5`. `location_score` is `None` only when at least one side has `Option<Location>` of `None`.

### 6.5 Category — `category_score`

`1.0` if both `category` values are set and structurally equal; `0.0` if both set but differ; `None` if either is `None`.

### 6.6 Country code — `country_code_score`

`1.0` if both `country_code_as_iso_3166_1_alpha_2` values are set and equal after trim + ASCII lowercase; `0.0` otherwise; `None` if either is `None`.

### 6.7 Event IDs — `event_ids_score`

`1.0` if both `event_ids` lists are non-empty AND any `(scheme, value)` pair is shared (under `EventId` equality); `0.0` if both lists are non-empty but no pair is shared; `None` if either list is empty.

### 6.8 Organizer — `organizer_score`

`Scorer::combined_similarity` on `normalize_name(organizer)` from each side. `None` if either side has `organizer = None`.

### 6.9 Performers — `performers_score`

Best-of cartesian product across `e1.performers × e2.performers`: each pair is compared with `Scorer::combined_similarity` after `normalize_name`; the maximum is returned. `None` if either list is empty.

### 6.10 URL — `url_score`

`1.0` iff both `url` values are set and equal after trimming whitespace, else `0.0`. `None` if either is `None`. No URL canonicalisation (scheme folding, trailing-slash stripping, query-parameter ordering) is performed.

### 6.11 Relationships — `relationships_score` (planned, not yet implemented)

Specified, tracked in `CHANGELOG.md` `[Unreleased]`; no `relationships` field, `RelationshipRef`/`RelationKind` types, `relationships_weight`, or `relationships_score` exist in the code today (§3.1). The contract below is the target design.

Typed-set **Jaccard** over the `(relation, event_id)` pairs: `score = |A ∩ B| / |A ∪ B|`, where each side's set is `{ (r.relation, r.event_id) for r in relationships }`. So an `Outer` reference only agrees with an `Outer` reference to the **same** event id — the relation kind is part of the key; `Inner` / `ImmediatelyBefore` / `ImmediatelyAfter` are compared as opaque, distinct kinds (no inversion or transitive closure). `None` (does not participate) when **either** side has no relationships; otherwise a value in `[0.0, 1.0]`. A **supporting** signal weighted `relationships_weight` (§7, default `0.05`); shared references never single-handedly establish a match.

### 6.12 Tags — `tags_score` (planned, not yet implemented)

Specified, tracked in `CHANGELOG.md` `[Unreleased]`; no `tags` field, `tags_weight`, or `tags_score` exist in the code today (§3.1). The contract below is the target design.

Plain set **Jaccard** over the operator-applied tags: `score = |A ∩ B| / |A ∪ B|`, where each side's set is the case-insensitively normalised tags (trim + ASCII lowercase, empties dropped). Unlike `relationships` (§6.11) the key is the bare normalised string — there is no relation kind. `None` (does not participate) when **either** side has an empty tag set; otherwise a value in `[0.0, 1.0]`. A **supporting** signal weighted `tags_weight` (§7, default `0.05`); shared tags never single-handedly establish a match.

---
