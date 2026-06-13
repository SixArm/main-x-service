## 6. Per-field scoring algorithms

Each component returns `Option<f64>` in `[0.0, 1.0]`. A `None` means the component did not participate (§5.4). Per-algorithm pseudocode, edge cases, and lookup tables are maintained in [`AGENTS/matching-algorithm.md`](../AGENTS/matching-algorithm.md); the contracts below are normative.

### 6.1 Name — `name_score`

`collect_names(p)` gathers `p.name` plus `p.alternate_names`, skipping empty / whitespace-only strings. The matcher computes the similarity for every pair drawn from `collect_names(p1) × collect_names(p2)` (after `normalize_name` on each side) using `MatchConfig::name_algorithm`, and returns the maximum. Algorithm selector values: `JaroWinkler` (prefix-biased; good for short names), `Levenshtein` (`1 - edit_distance / max_len`), `Exact` (binary, case-sensitive), `Combined = 0.7 * JaroWinkler + 0.3 * Levenshtein` (**default**). Two empty strings score `1.0`; empty vs non-empty scores `0.0`. `name_score` is `None` when either side has no usable names.

### 6.2 Phonetic — `name_phonetic_score`

Only computed when `MatchConfig::use_phonetic_matching = true`. For every pair across `collect_names`, compute `Normalizer::phonetic_code(name)` on each side; result is `1.0` if any pair of non-empty codes matches, else `0.0`. Bonus-only: when `> 0.9`, the weighted-sum reducer adds `name_phonetic_score * 0.05` to weighted sum and `0.05` to total weight (§5.2.2). The bonus MUST NOT lower the overall score. `None` when phonetic matching is off, or either side has no usable names.

### 6.3 Coordinates — `coordinates_score`

Compute `d = Scorer::haversine_metres(lat1, lon1, lat2, lon2)`, apply Gaussian decay `exp(-(d / s)^2)` where `s = MatchConfig::coordinates_scale_metres`. Contract: `d == 0 → 1.0`; `d == s → 1/e ≈ 0.368`; `d == 3s → ~0.0001`; non-finite `d`, non-positive `s`, negative `d` → `0.0`. `Scorer::haversine_metres` uses mean Earth radius `6_371_000.0` m on a sphere; total over `f64`; handles equator and date-line crossings. `coordinates_score` is `None` when any of `lat1`, `lon1`, `lat2`, `lon2` is missing, non-finite, or outside `[-90, 90]` / `[-180, 180]`.

### 6.4 Address — `address_score`

When both sides have an `address`, run a weight-renormalised average across populated sub-components with sub-weights **Postcode 0.5**, **City 0.3**, **Line 1 0.2**. Postcode sub-score: `1.0` iff `normalize_postcode(a) == normalize_postcode(b)` else `0.0`. City sub-score: `jaro_winkler_similarity` on normalised city names. Line-1 sub-score: `parse_address_line` on each side; with house numbers present on both, `0.6 * jaro_winkler(street) + 0.4 * (house_number_a == house_number_b)`; otherwise street similarity alone. If no sub-component is populated on both sides, address score is neutral `0.5`. `None` only when at least one side has `Option<Address>` of `None`.

### 6.5 Category — `category_score`

`1.0` if both `category` values are set and structurally equal; `0.0` if both set but differ; `None` if either is `None`.

### 6.6 Country code — `country_code_score`

`1.0` if both `country_code_as_iso_3166_1_alpha_2` values are set and equal after trim + ASCII lowercase; `0.0` otherwise; `None` if either is `None`.

### 6.7 Place IDs — `place_ids_score`

`1.0` if both `place_ids` lists are non-empty AND any `(scheme, value)` pair is shared (under `PlaceId` equality); `0.0` if both lists are non-empty but no pair is shared; `None` if either list is empty.

### 6.8 Phone — `phone_score`

Compute `normalize_phone_e164(phone, cc)` for both sides with `cc = MatchConfig::phone_default_country`. If both canonicalise, `1.0` iff equal, else `0.0`. Otherwise fall back to `normalize_phone(phone)` (UK-centric legacy) on both: `1.0` iff equal, else `0.0`. `None` only when either side has `phone = None`.

### 6.9 Email — `email_score`

Compute `normalize_email(email, gmail_dot_folding)` for both sides. `1.0` iff both canonicalise and are equal; `0.0` if both canonicalise but differ. `None` if either side has `email = None`, or if either side fails to canonicalise.

---

