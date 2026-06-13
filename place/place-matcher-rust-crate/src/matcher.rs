//! Place matcher engine: deterministic and probabilistic algorithms.
//!
//! This is the orchestration layer of the crate. It pulls together the data
//! types from [`crate::models`], the text transformations from
//! [`crate::normalizer`], and the similarity primitives from
//! [`crate::scorer`] to produce a single answer about whether two place
//! records refer to the same place.
//!
//! ## Two strategies, one engine
//!
//! - [`MatchingEngine::deterministic_match`] — fast, binary. Returns `true`
//!   iff the two places share any external place-ID pair, or share an exact
//!   normalised primary name plus the same normalised postcode.
//! - [`MatchingEngine::match_places`] — weighted probabilistic scoring,
//!   returning a [`MatchResult`] with per-field [`MatchBreakdown`].
//!
//! ## Example
//!
//! ```
//! use place_matcher::{MatchingEngine, Place};
//!
//! let a = Place::builder()
//!     .name("Eiffel Tower")
//!     .latitude(48.858_222)
//!     .longitude(2.294_500)
//!     .build();
//!
//! let b = Place::builder()
//!     .name("La Tour Eiffel")
//!     .add_alternate_name("Eiffel Tower")
//!     .latitude(48.858_3)
//!     .longitude(2.294_5)
//!     .build();
//!
//! let engine = MatchingEngine::default_config();
//! let result = engine.match_places(&a, &b);
//! assert!(result.is_match);
//! ```

use crate::models::{Address, Place};
use crate::normalizer::Normalizer;
use crate::scorer::{Scorer, SimilarityAlgorithm};
use serde::{Deserialize, Serialize};

/// Tunable configuration for the matching engine.
///
/// All weights are dimensionless and contribute to a renormalised weighted
/// sum — they do not need to add to `1.0`. The matching pipeline divides the
/// weighted sum by the sum of *participating* weights so that missing fields
/// neither contribute nor penalise. The score is then compared against
/// [`MatchConfig::match_threshold`] to produce the `is_match` boolean.
///
/// Two presets cover most needs:
///
/// - [`MatchConfig::strict`]  — `match_threshold = 0.95`, `strict_mode = true`.
/// - [`MatchConfig::lenient`] — `match_threshold = 0.65`, phonetic on.
///
/// # Example
///
/// ```
/// use place_matcher::{MatchConfig, SimilarityAlgorithm};
///
/// let custom = MatchConfig {
///     match_threshold: 0.80,
///     name_weight: 0.20,
///     coordinates_weight: 0.30,
///     coordinates_scale_metres: 50.0,
///     address_weight: 0.10,
///     category_weight: 0.10,
///     country_code_weight: 0.05,
///     place_ids_weight: 0.15,
///     phone_weight: 0.03,
///     email_weight: 0.02,
///     use_phonetic_matching: true,
///     name_algorithm: SimilarityAlgorithm::Combined,
///     strict_mode: false,
///     gmail_dot_folding: false,
///     phone_default_country: Some("GB".into()),
/// };
/// assert_eq!(custom.match_threshold, 0.80);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MatchConfig {
    /// Threshold score for considering two places a match (`0.0..=1.0`).
    pub match_threshold: f64,

    /// Weight for name similarity (best-of cartesian product across the
    /// primary `name` and `alternate_names` on both sides).
    pub name_weight: f64,

    /// Weight for geographic-coordinates similarity (Gaussian decay over
    /// Haversine distance).
    pub coordinates_weight: f64,

    /// Distance scale, in metres, controlling the Gaussian decay of the
    /// coordinates score. At a separation equal to `scale` the score is
    /// `1/e ~= 0.368`.
    pub coordinates_scale_metres: f64,

    /// Weight for address similarity.
    pub address_weight: f64,

    /// Weight for [`PlaceCategory`](crate::models::PlaceCategory) equality (1.0 / 0.0 when both sides set).
    pub category_weight: f64,

    /// Weight for case-insensitive equality of
    /// `country_code_as_iso_3166_1_alpha_2`.
    pub country_code_weight: f64,

    /// Weight for "shared external place ID" (1.0 if any `(scheme, value)`
    /// pair is shared, 0.0 otherwise).
    pub place_ids_weight: f64,

    /// Weight for phone-number exact match (after normalisation).
    pub phone_weight: f64,

    /// Weight for email-address exact match (after normalisation via
    /// [`crate::Normalizer::normalize_email`]).
    pub email_weight: f64,

    /// Whether to add a phonetic-name bonus when both names sound alike.
    pub use_phonetic_matching: bool,

    /// Similarity algorithm to use when comparing names.
    pub name_algorithm: SimilarityAlgorithm,

    /// Reserved flag for stricter deterministic enforcement.
    pub strict_mode: bool,

    /// Fold Gmail-specific localpart dots and `+tag` suffixes during
    /// email normalisation. When `true`, addresses on `gmail.com` /
    /// `googlemail.com` have every `.` in the localpart removed and any
    /// `+anything` suffix dropped before comparison.
    pub gmail_dot_folding: bool,

    /// ISO 3166-1 alpha-2 country code applied to phone numbers that lack
    /// an explicit international marker (`+CC` or `00CC`).
    ///
    /// When `Some(cc)`, the matcher converts each phone to E.164 form via
    /// [`crate::Normalizer::normalize_phone_e164`] using `cc` as the
    /// fallback jurisdiction. Defaults to `Some("GB")`.
    pub phone_default_country: Option<String>,
}

impl Default for MatchConfig {
    /// Production-ready defaults.
    ///
    /// ```
    /// use place_matcher::{MatchConfig, SimilarityAlgorithm};
    /// let c = MatchConfig::default();
    /// assert!((c.match_threshold - 0.80).abs() < 1e-9);
    /// assert!(matches!(c.name_algorithm, SimilarityAlgorithm::Combined));
    /// ```
    fn default() -> Self {
        Self {
            match_threshold: 0.80,
            name_weight: 0.20,
            coordinates_weight: 0.30,
            coordinates_scale_metres: 50.0,
            address_weight: 0.10,
            category_weight: 0.10,
            country_code_weight: 0.05,
            place_ids_weight: 0.15,
            phone_weight: 0.03,
            email_weight: 0.02,
            use_phonetic_matching: false,
            name_algorithm: SimilarityAlgorithm::Combined,
            strict_mode: false,
            gmail_dot_folding: false,
            phone_default_country: Some("GB".to_string()),
        }
    }
}

impl MatchConfig {
    /// A stricter preset: `match_threshold = 0.95`, `strict_mode = true`.
    ///
    /// Use when callers must rely on the answer and false positives are
    /// more dangerous than false negatives.
    ///
    /// ```
    /// use place_matcher::MatchConfig;
    /// let c = MatchConfig::strict();
    /// assert!((c.match_threshold - 0.95).abs() < 1e-9);
    /// assert!(c.strict_mode);
    /// ```
    #[must_use]
    pub fn strict() -> Self {
        Self {
            match_threshold: 0.95,
            strict_mode: true,
            ..Default::default()
        }
    }

    /// A more forgiving preset: `match_threshold = 0.65`, phonetic matching on.
    ///
    /// Use when triaging large candidate sets where false negatives are
    /// worse than false positives.
    ///
    /// ```
    /// use place_matcher::MatchConfig;
    /// let c = MatchConfig::lenient();
    /// assert!((c.match_threshold - 0.65).abs() < 1e-9);
    /// assert!(c.use_phonetic_matching);
    /// ```
    #[must_use]
    pub fn lenient() -> Self {
        Self {
            match_threshold: 0.65,
            use_phonetic_matching: true,
            ..Default::default()
        }
    }
}

/// Qualitative confidence band derived from the probabilistic
/// [`MatchResult::score`].
///
/// The bands are fixed across all `MatchConfig` presets — they do **not**
/// follow `match_threshold`. They are intended for triage UIs and audit
/// logs where a coarse High/Medium/Low summary is more useful than the
/// raw float. The `is_match` boolean remains the authoritative go/no-go
/// signal because it incorporates the configured threshold.
///
/// Boundaries:
///
/// | Score range | Band |
/// |---|---|
/// | `score >= 0.90` | `High` |
/// | `0.75 <= score < 0.90` | `Medium` |
/// | `score < 0.75` | `Low` |
///
/// # Examples
///
/// ```
/// use place_matcher::Confidence;
///
/// assert_eq!(Confidence::from_score(0.99), Confidence::High);
/// assert_eq!(Confidence::from_score(0.90), Confidence::High);   // inclusive
/// assert_eq!(Confidence::from_score(0.85), Confidence::Medium);
/// assert_eq!(Confidence::from_score(0.75), Confidence::Medium); // inclusive
/// assert_eq!(Confidence::from_score(0.50), Confidence::Low);
/// assert_eq!(Confidence::from_score(0.00), Confidence::Low);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Confidence {
    /// Score is at or above `0.90`. Strong match; safe to act on with
    /// minimal review.
    High,
    /// Score is in `0.75..0.90`. Medium-confidence match; the per-field
    /// [`MatchBreakdown`] should be inspected before downstream use.
    Medium,
    /// Score is below `0.75`. Treat as a candidate at best; require
    /// additional evidence before treating as the same place.
    Low,
}

impl Confidence {
    /// Bucket a probabilistic score into one of the three bands.
    ///
    /// The function is total over `f64`: NaN inputs degrade to `Low`,
    /// negative scores degrade to `Low`, scores above `1.0` are treated
    /// as `High`. In practice the matcher only ever produces values in
    /// `[0.0, 1.0]`, so callers shouldn't encounter the degenerate
    /// inputs.
    ///
    /// ```
    /// use place_matcher::Confidence;
    ///
    /// assert_eq!(Confidence::from_score(f64::NAN), Confidence::Low);
    /// assert_eq!(Confidence::from_score(-0.5),     Confidence::Low);
    /// assert_eq!(Confidence::from_score(2.0),      Confidence::High);
    /// ```
    #[must_use]
    pub fn from_score(score: f64) -> Self {
        if score >= 0.90 {
            Confidence::High
        } else if score >= 0.75 {
            Confidence::Medium
        } else {
            Confidence::Low
        }
    }
}

/// Outcome of a probabilistic place match.
///
/// Contains the overall renormalised `score`, the threshold-derived
/// `is_match` boolean, a coarse [`Confidence`] band, and a per-field
/// [`MatchBreakdown`] for audit.
///
/// `MatchResult` implements `Serialize + Deserialize` so it can be persisted
/// or returned over an API.
///
/// ```
/// use place_matcher::{Confidence, MatchingEngine, Place};
///
/// let p = Place::builder().name("Eiffel Tower").build();
/// let q = p.clone();
/// let result = MatchingEngine::default_config().match_places(&p, &q);
/// assert_eq!(result.confidence, Confidence::High);
///
/// // Round-trip through JSON.
/// let json = serde_json::to_string(&result).unwrap();
/// let back: place_matcher::MatchResult = serde_json::from_str(&json).unwrap();
/// assert!((result.score - back.score).abs() < 1e-12);
/// assert_eq!(result.is_match, back.is_match);
/// assert_eq!(result.confidence, back.confidence);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchResult {
    /// Overall match score in `[0.0, 1.0]`.
    pub score: f64,

    /// `true` if `score >= MatchConfig::match_threshold`.
    pub is_match: bool,

    /// Coarse confidence band derived from `score`. Defaults to
    /// [`Confidence::Low`] on legacy JSON payloads that pre-date the field.
    #[serde(default = "default_confidence")]
    pub confidence: Confidence,

    /// Per-field score contributions for explainability.
    pub breakdown: MatchBreakdown,
}

/// Backstop for legacy `MatchResult` JSON payloads that lack the
/// `confidence` field. Returns `Confidence::Low` so a deserialised
/// payload that pre-dates the field is unambiguously flagged as
/// "needs re-scoring".
fn default_confidence() -> Confidence {
    Confidence::Low
}

/// Per-field score breakdown returned with every [`MatchResult`].
///
/// Each field is `Option<f64>`:
///
/// - `Some(score)` — the field was scored; the value is in `[0.0, 1.0]`.
/// - `None` — the field was missing on at least one side and so did not
///   participate in the weighted sum.
///
/// The breakdown exists so an auditor can see *why* a match was
/// flagged. Do not throw it away in downstream services.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchBreakdown {
    /// Best-of-cartesian-product similarity across (primary name +
    /// alternate names) on both sides, using the configured algorithm.
    pub name_score: Option<f64>,
    /// Maximum Soundex match across the same name pairs. `None` when
    /// `use_phonetic_matching` is false or either side has no names.
    pub name_phonetic_score: Option<f64>,
    /// Gaussian-decay coordinates score over Haversine distance. `None`
    /// if either side is missing lat/lon, or any value is non-finite or
    /// outside the conventional `[-90,90]` / `[-180,180]` ranges.
    pub coordinates_score: Option<f64>,
    /// Score for address similarity (weighted blend of postcode, city, line 1).
    pub address_score: Option<f64>,
    /// `1.0` if both categories set and structurally equal; `0.0` if both
    /// set but differ; `None` if either is `None`.
    pub category_score: Option<f64>,
    /// `1.0` if both country codes set and equal after trim + ASCII
    /// lowercase; `0.0` otherwise; `None` if either is `None`.
    pub country_code_score: Option<f64>,
    /// `1.0` if both `place_ids` non-empty and they share any
    /// `(scheme, value)` pair; `0.0` if both non-empty but none shared;
    /// `None` if either side is empty.
    pub place_ids_score: Option<f64>,
    /// Score for normalised phone-number equality (`1.0` or `0.0`).
    pub phone_score: Option<f64>,
    /// Score for normalised email-address equality (`1.0` or `0.0`).
    /// `None` if either side is absent or fails to parse as an email.
    #[serde(default)]
    pub email_score: Option<f64>,
}

/// Place matcher engine.
///
/// The engine is **immutable after construction** and cheap to clone (it
/// owns only a [`MatchConfig`]). Construct one and call its methods from any
/// thread.
///
/// ```
/// use place_matcher::{MatchConfig, MatchingEngine};
///
/// let engine_a = MatchingEngine::default_config();
/// let engine_b = MatchingEngine::new(MatchConfig::strict());
/// # let _ = (engine_a, engine_b);
/// ```
pub struct MatchingEngine {
    config: MatchConfig,
}

impl MatchingEngine {
    /// Construct an engine with the given configuration.
    ///
    /// ```
    /// use place_matcher::{MatchConfig, MatchingEngine};
    /// let engine = MatchingEngine::new(MatchConfig::lenient());
    /// # let _ = engine;
    /// ```
    #[must_use]
    pub fn new(config: MatchConfig) -> Self {
        Self { config }
    }

    /// Construct an engine with [`MatchConfig::default`].
    ///
    /// ```
    /// use place_matcher::MatchingEngine;
    /// let engine = MatchingEngine::default_config();
    /// # let _ = engine;
    /// ```
    #[must_use]
    pub fn default_config() -> Self {
        Self::new(MatchConfig::default())
    }

    /// Compare two places probabilistically and return a [`MatchResult`].
    ///
    /// The score is the weight-renormalised sum of every component that
    /// scored on both records. Missing fields are skipped, not penalised.
    ///
    /// ```
    /// use place_matcher::{MatchingEngine, Place};
    ///
    /// let p = Place::builder()
    ///     .name("Eiffel Tower")
    ///     .latitude(48.858_222)
    ///     .longitude(2.294_500)
    ///     .build();
    ///
    /// let result = MatchingEngine::default_config().match_places(&p, &p);
    /// assert!(result.is_match);
    /// assert!(result.score > 0.99);
    /// ```
    #[must_use]
    pub fn match_places(&self, place1: &Place, place2: &Place) -> MatchResult {
        let breakdown = self.calculate_breakdown(place1, place2);
        let score = self.calculate_weighted_score(&breakdown);
        let above_threshold = score >= self.config.match_threshold;
        // Under strict mode, `is_match` ALSO requires a deterministic match.
        let is_match = if self.config.strict_mode {
            above_threshold && self.deterministic_match(place1, place2)
        } else {
            above_threshold
        };
        let confidence = Confidence::from_score(score);

        MatchResult {
            score,
            is_match,
            confidence,
            breakdown,
        }
    }

    /// Score a single query against many candidates. Returns one
    /// [`MatchResult`] per candidate, in the same order as the input slice.
    ///
    /// The engine is immutable and `Send + Sync`, so call-sites that want
    /// parallel evaluation can wrap the call in `rayon::par_iter` or similar
    /// without further changes to this crate.
    ///
    /// # Examples
    ///
    /// ```
    /// use place_matcher::{MatchingEngine, Place};
    ///
    /// let query = Place::builder().name("Eiffel Tower").build();
    /// let candidates = vec![
    ///     Place::builder().name("Eiffel Tower").build(),
    ///     Place::builder().name("Big Ben").build(),
    /// ];
    ///
    /// let engine = MatchingEngine::default_config();
    /// let results = engine.match_one_to_many(&query, &candidates);
    /// assert_eq!(results.len(), 2);
    /// assert!(results[0].is_match);
    /// assert!(!results[1].is_match);
    /// ```
    ///
    /// Empty candidates yield an empty result:
    ///
    /// ```
    /// # use place_matcher::{MatchingEngine, Place};
    /// let q = Place::builder().name("Solo").build();
    /// let r = MatchingEngine::default_config().match_one_to_many(&q, &[]);
    /// assert!(r.is_empty());
    /// ```
    #[must_use]
    pub fn match_one_to_many(&self, query: &Place, candidates: &[Place]) -> Vec<MatchResult> {
        candidates
            .iter()
            .map(|c| self.match_places(query, c))
            .collect()
    }

    /// Score and rank: return `(original_index, MatchResult)` tuples
    /// sorted by descending score. Ties are broken by ascending original
    /// index, so the result is deterministic.
    ///
    /// # Examples
    ///
    /// ```
    /// use place_matcher::{MatchingEngine, Place};
    ///
    /// let query = Place::builder().name("Eiffel Tower").build();
    /// let candidates = vec![
    ///     Place::builder().name("Big Ben").build(),                 // index 0
    ///     Place::builder().name("Eiffel Tower").build(),            // index 1 — best match
    ///     Place::builder().name("Statue of Liberty").build(),       // index 2
    /// ];
    ///
    /// let ranked = MatchingEngine::default_config().rank_one_to_many(&query, &candidates);
    /// assert_eq!(ranked.len(), 3);
    /// assert_eq!(ranked[0].0, 1);
    /// assert!(ranked[0].1.score >= ranked[1].1.score);
    /// assert!(ranked[1].1.score >= ranked[2].1.score);
    /// ```
    #[must_use]
    pub fn rank_one_to_many(
        &self,
        query: &Place,
        candidates: &[Place],
    ) -> Vec<(usize, MatchResult)> {
        let mut indexed: Vec<(usize, MatchResult)> = self
            .match_one_to_many(query, candidates)
            .into_iter()
            .enumerate()
            .collect();
        indexed.sort_by(|a, b| {
            b.1.score
                .partial_cmp(&a.1.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });
        indexed
    }

    /// Compare two places deterministically and return a single boolean.
    ///
    /// Returns `true` iff either:
    ///
    /// - the places share any `(scheme, value)` pair in their `place_ids`
    ///   lists, OR
    /// - both have a primary `name` that normalises to the same value AND
    ///   both have an `address.postcode` that normalises to the same value.
    ///
    /// ```
    /// use place_matcher::{MatchingEngine, Place, PlaceId, PlaceIdScheme};
    ///
    /// let id = PlaceId::new(PlaceIdScheme::Wikidata, "Q243").unwrap();
    /// let a = Place::builder().name("Eiffel Tower").add_place_id(id.clone()).build();
    /// let b = Place::builder().name("Tour Eiffel").add_place_id(id).build();
    /// assert!(MatchingEngine::default_config().deterministic_match(&a, &b));
    /// ```
    #[must_use]
    pub fn deterministic_match(&self, place1: &Place, place2: &Place) -> bool {
        if shares_place_id(place1, place2) {
            return true;
        }
        name_and_postcode_match(place1, place2)
    }

    fn calculate_breakdown(&self, place1: &Place, place2: &Place) -> MatchBreakdown {
        MatchBreakdown {
            name_score: self.score_name(place1, place2),
            name_phonetic_score: if self.config.use_phonetic_matching {
                Self::score_phonetic_names(place1, place2)
            } else {
                None
            },
            coordinates_score: self.score_coordinates(place1, place2),
            address_score: Self::score_address(place1, place2),
            category_score: score_category(place1, place2),
            country_code_score: score_country_code(place1, place2),
            place_ids_score: score_place_ids(place1, place2),
            phone_score: self.score_phone(place1, place2),
            email_score: self.score_email(place1, place2),
        }
    }

    fn calculate_weighted_score(&self, breakdown: &MatchBreakdown) -> f64 {
        let mut total_weight = 0.0;
        let mut weighted_sum = 0.0;

        if let Some(score) = breakdown.name_score {
            weighted_sum += score * self.config.name_weight;
            total_weight += self.config.name_weight;
        }
        if let Some(score) = breakdown.coordinates_score {
            weighted_sum += score * self.config.coordinates_weight;
            total_weight += self.config.coordinates_weight;
        }
        if let Some(score) = breakdown.address_score {
            weighted_sum += score * self.config.address_weight;
            total_weight += self.config.address_weight;
        }
        if let Some(score) = breakdown.category_score {
            weighted_sum += score * self.config.category_weight;
            total_weight += self.config.category_weight;
        }
        if let Some(score) = breakdown.country_code_score {
            weighted_sum += score * self.config.country_code_weight;
            total_weight += self.config.country_code_weight;
        }
        if let Some(score) = breakdown.place_ids_score {
            weighted_sum += score * self.config.place_ids_weight;
            total_weight += self.config.place_ids_weight;
        }
        if let Some(score) = breakdown.phone_score {
            weighted_sum += score * self.config.phone_weight;
            total_weight += self.config.phone_weight;
        }
        if let Some(score) = breakdown.email_score {
            weighted_sum += score * self.config.email_weight;
            total_weight += self.config.email_weight;
        }

        // Phonetic match is a bonus only — never lowers the score.
        if let Some(score) = breakdown.name_phonetic_score
            && score > 0.9
        {
            weighted_sum += score * 0.05;
            total_weight += 0.05;
        }

        if total_weight > 0.0 {
            weighted_sum / total_weight
        } else {
            0.0
        }
    }

    fn score_name(&self, place1: &Place, place2: &Place) -> Option<f64> {
        let names1 = collect_names(place1);
        let names2 = collect_names(place2);
        if names1.is_empty() || names2.is_empty() {
            return None;
        }
        let mut best = f64::NEG_INFINITY;
        for n1 in &names1 {
            for n2 in &names2 {
                let s = self.score_name_pair(n1, n2);
                if s > best {
                    best = s;
                }
            }
        }
        Some(best)
    }

    fn score_name_pair(&self, name1: &str, name2: &str) -> f64 {
        let norm1 = Normalizer::normalize_name(name1);
        let norm2 = Normalizer::normalize_name(name2);
        match self.config.name_algorithm {
            SimilarityAlgorithm::JaroWinkler => Scorer::jaro_winkler_similarity(&norm1, &norm2),
            SimilarityAlgorithm::Levenshtein => Scorer::levenshtein_similarity(&norm1, &norm2),
            SimilarityAlgorithm::Exact => Scorer::exact_match(&norm1, &norm2),
            SimilarityAlgorithm::Combined => Scorer::combined_similarity(&norm1, &norm2),
        }
    }

    fn score_phonetic_names(place1: &Place, place2: &Place) -> Option<f64> {
        let names1 = collect_names(place1);
        let names2 = collect_names(place2);
        if names1.is_empty() || names2.is_empty() {
            return None;
        }
        let codes1: Vec<String> = names1
            .iter()
            .map(|n| Normalizer::phonetic_code(n))
            .collect();
        let codes2: Vec<String> = names2
            .iter()
            .map(|n| Normalizer::phonetic_code(n))
            .collect();
        let mut best = 0.0_f64;
        for c1 in &codes1 {
            for c2 in &codes2 {
                if !c1.is_empty() && c1 == c2 {
                    best = 1.0;
                }
            }
        }
        Some(best)
    }

    fn score_coordinates(&self, place1: &Place, place2: &Place) -> Option<f64> {
        let (lat1, lon1) = valid_coords(place1.latitude, place1.longitude)?;
        let (lat2, lon2) = valid_coords(place2.latitude, place2.longitude)?;
        let d = Scorer::haversine_metres(lat1, lon1, lat2, lon2);
        Some(Scorer::coordinates_score(
            d,
            self.config.coordinates_scale_metres,
        ))
    }

    fn score_address(place1: &Place, place2: &Place) -> Option<f64> {
        match (place1.address.as_ref(), place2.address.as_ref()) {
            (Some(a1), Some(a2)) => Some(Self::compare_addresses(a1, a2)),
            _ => None,
        }
    }

    fn compare_addresses(addr1: &Address, addr2: &Address) -> f64 {
        // Each sub-component contributes its own raw score in `[0.0, 1.0]`
        // and a weight. The final sub-score is the weight-renormalised
        // average — `(score x weight) / weight` summed across the
        // sub-components that actually fired. Postcode dominates (`0.5`),
        // then city (`0.3`), then line 1 (`0.2`), regardless of how many
        // components are populated.
        let mut weighted_sum = 0.0_f64;
        let mut total_weight = 0.0_f64;

        if let (Some(pc1), Some(pc2)) = (&addr1.postcode, &addr2.postcode) {
            let norm1 = Normalizer::normalize_postcode(pc1);
            let norm2 = Normalizer::normalize_postcode(pc2);
            weighted_sum += f64::from(norm1 == norm2) * 0.5;
            total_weight += 0.5;
        }

        if let (Some(city1), Some(city2)) = (&addr1.city, &addr2.city) {
            let norm1 = Normalizer::normalize_name(city1);
            let norm2 = Normalizer::normalize_name(city2);
            weighted_sum += Scorer::jaro_winkler_similarity(&norm1, &norm2) * 0.3;
            total_weight += 0.3;
        }

        if let (Some(line1), Some(line2)) = (&addr1.line1, &addr2.line1) {
            let parsed1 = Normalizer::parse_address_line(line1);
            let parsed2 = Normalizer::parse_address_line(line2);
            let street_sim = Scorer::jaro_winkler_similarity(&parsed1.street, &parsed2.street);
            let house_score = match (&parsed1.house_number, &parsed2.house_number) {
                (Some(a), Some(b)) => Some(f64::from(a == b)),
                _ => None,
            };
            let line1_score = match house_score {
                Some(h) => 0.6 * street_sim + 0.4 * h,
                None => street_sim,
            };
            weighted_sum += line1_score * 0.2;
            total_weight += 0.2;
        }

        if total_weight == 0.0 {
            0.5
        } else {
            weighted_sum / total_weight
        }
    }

    fn score_phone(&self, place1: &Place, place2: &Place) -> Option<f64> {
        let phone1 = place1.phone.as_ref()?;
        let phone2 = place2.phone.as_ref()?;

        let default = self.config.phone_default_country.as_deref();
        let e164_1 = Normalizer::normalize_phone_e164(phone1, default);
        let e164_2 = Normalizer::normalize_phone_e164(phone2, default);

        // Prefer the international-aware comparison: a French and a UK
        // number that share the same national-significant digits must not
        // collide. Fall back to the legacy national-significant form when
        // either side cannot be parsed to E.164.
        if let (Some(a), Some(b)) = (&e164_1, &e164_2) {
            return Some(f64::from(a == b));
        }

        let norm1 = Normalizer::normalize_phone(phone1);
        let norm2 = Normalizer::normalize_phone(phone2);
        Some(f64::from(norm1 == norm2))
    }

    fn score_email(&self, place1: &Place, place2: &Place) -> Option<f64> {
        let email1 = place1.email.as_ref()?;
        let email2 = place2.email.as_ref()?;
        let fold = self.config.gmail_dot_folding;
        let canonical1 = Normalizer::normalize_email(email1, fold)?;
        let canonical2 = Normalizer::normalize_email(email2, fold)?;
        Some(f64::from(canonical1 == canonical2))
    }
}

// ---- Free helpers ------------------------------------------------------

/// Collect a place's primary name plus alternate names into a single vec
/// of references. Empty / whitespace-only strings are skipped.
fn collect_names(place: &Place) -> Vec<&String> {
    place
        .name
        .iter()
        .chain(place.alternate_names.iter())
        .filter(|s| !s.trim().is_empty())
        .collect()
}

/// Validate that lat/lon are finite and fall in the conventional ranges.
fn valid_coords(lat: Option<f64>, lon: Option<f64>) -> Option<(f64, f64)> {
    let lat = lat?;
    let lon = lon?;
    if !lat.is_finite() || !lon.is_finite() {
        return None;
    }
    if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
        return None;
    }
    Some((lat, lon))
}

fn score_category(p1: &Place, p2: &Place) -> Option<f64> {
    match (&p1.category, &p2.category) {
        (Some(a), Some(b)) => Some(if a == b { 1.0 } else { 0.0 }),
        _ => None,
    }
}

fn score_country_code(p1: &Place, p2: &Place) -> Option<f64> {
    let a = p1.country_code_as_iso_3166_1_alpha_2.as_ref()?;
    let b = p2.country_code_as_iso_3166_1_alpha_2.as_ref()?;
    let na = a.trim().to_ascii_lowercase();
    let nb = b.trim().to_ascii_lowercase();
    Some(if na == nb { 1.0 } else { 0.0 })
}

fn shares_place_id(p1: &Place, p2: &Place) -> bool {
    if p1.place_ids.is_empty() || p2.place_ids.is_empty() {
        return false;
    }
    for id1 in &p1.place_ids {
        for id2 in &p2.place_ids {
            if id1 == id2 {
                return true;
            }
        }
    }
    false
}

fn score_place_ids(p1: &Place, p2: &Place) -> Option<f64> {
    if p1.place_ids.is_empty() || p2.place_ids.is_empty() {
        return None;
    }
    Some(if shares_place_id(p1, p2) { 1.0 } else { 0.0 })
}

fn name_and_postcode_match(p1: &Place, p2: &Place) -> bool {
    let (Some(n1), Some(n2)) = (&p1.name, &p2.name) else {
        return false;
    };
    if Normalizer::normalize_name(n1) != Normalizer::normalize_name(n2) {
        return false;
    }
    let (Some(pc1), Some(pc2)) = (
        p1.address.as_ref().and_then(|a| a.postcode.as_ref()),
        p2.address.as_ref().and_then(|a| a.postcode.as_ref()),
    ) else {
        return false;
    };
    Normalizer::normalize_postcode(pc1) == Normalizer::normalize_postcode(pc2)
}

#[cfg(test)]
mod tests {
    // Tests assert exact scores against representable constants (0.0, 1.0, …).
    #![allow(clippy::float_cmp)]

    use super::*;
    use crate::models::{PlaceCategory, PlaceId, PlaceIdScheme};

    // ---------- MatchConfig presets ----------

    #[test]
    fn config_default_values() {
        let c = MatchConfig::default();
        assert!((c.match_threshold - 0.80).abs() < 1e-9);
        assert!(!c.strict_mode);
    }

    #[test]
    fn config_strict_raises_threshold_and_sets_flag() {
        let c = MatchConfig::strict();
        assert!((c.match_threshold - 0.95).abs() < 1e-9);
        assert!(c.strict_mode);
    }

    #[test]
    fn config_lenient_lowers_threshold() {
        let c = MatchConfig::lenient();
        assert!((c.match_threshold - 0.65).abs() < 1e-9);
        assert!(c.use_phonetic_matching);
    }

    // ---------- MatchConfig serde ----------

    #[test]
    fn config_default_round_trips_through_json() {
        let cfg = MatchConfig::default();
        let json = serde_json::to_string(&cfg).expect("serialise");
        let back: MatchConfig = serde_json::from_str(&json).expect("deserialise");
        assert!((cfg.match_threshold - back.match_threshold).abs() < 1e-12);
        assert!((cfg.name_weight - back.name_weight).abs() < 1e-12);
        assert!((cfg.coordinates_weight - back.coordinates_weight).abs() < 1e-12);
        assert!(matches!(back.name_algorithm, SimilarityAlgorithm::Combined));
        assert_eq!(cfg.strict_mode, back.strict_mode);
    }

    #[test]
    fn config_partial_json_fills_missing_fields_from_default() {
        let partial = r#"{"match_threshold": 0.80, "gmail_dot_folding": true}"#;
        let cfg: MatchConfig = serde_json::from_str(partial).expect("partial json");
        assert!((cfg.match_threshold - 0.80).abs() < 1e-12);
        assert!(cfg.gmail_dot_folding);
        assert!(matches!(cfg.name_algorithm, SimilarityAlgorithm::Combined));
        assert_eq!(cfg.phone_default_country.as_deref(), Some("GB"));
    }

    // ---------- probabilistic match ----------

    #[test]
    fn exact_clone_is_a_match() {
        let p = Place::builder()
            .name("Eiffel Tower")
            .latitude(48.858_222)
            .longitude(2.294_500)
            .build();
        let result = MatchingEngine::default_config().match_places(&p, &p.clone());
        assert!(result.is_match);
        assert!(result.score > 0.95);
    }

    #[test]
    fn name_match_takes_best_of_cartesian_product() {
        let p1 = Place::builder().name("Eiffel Tower").build();
        let p2 = Place::builder()
            .name("La Tour Eiffel")
            .add_alternate_name("Eiffel Tower")
            .build();
        let r = MatchingEngine::default_config().match_places(&p1, &p2);
        let s = r.breakdown.name_score.expect("scored");
        assert!(
            s > 0.99,
            "best-of cartesian product should pick exact match: {s}"
        );
    }

    #[test]
    fn unrelated_places_do_not_match() {
        let a = Place::builder()
            .name("Eiffel Tower")
            .latitude(48.858_222)
            .longitude(2.294_500)
            .build();
        let b = Place::builder()
            .name("Sydney Opera House")
            .latitude(-33.856_8)
            .longitude(151.215_3)
            .build();
        let r = MatchingEngine::default_config().match_places(&a, &b);
        assert!(!r.is_match);
        assert!(r.score < 0.5);
    }

    #[test]
    fn no_overlapping_fields_returns_zero_score() {
        let a = Place::builder().phone("111").build();
        let b = Place::builder().email("x@example.org").build();
        let r = MatchingEngine::default_config().match_places(&a, &b);
        assert_eq!(r.score, 0.0);
    }

    // ---------- coordinates ----------

    #[test]
    fn coordinates_score_present_when_both_sides_have_coords() {
        let a = Place::builder()
            .name("X")
            .latitude(0.0)
            .longitude(0.0)
            .build();
        let b = Place::builder()
            .name("X")
            .latitude(0.0)
            .longitude(0.0)
            .build();
        let r = MatchingEngine::default_config().match_places(&a, &b);
        assert!((r.breakdown.coordinates_score.unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn coordinates_score_none_when_out_of_range() {
        let a = Place::builder()
            .name("X")
            .latitude(91.0)
            .longitude(0.0)
            .build();
        let b = Place::builder()
            .name("X")
            .latitude(0.0)
            .longitude(0.0)
            .build();
        let r = MatchingEngine::default_config().match_places(&a, &b);
        assert!(r.breakdown.coordinates_score.is_none());
    }

    #[test]
    fn coordinates_score_none_when_one_side_missing() {
        let a = Place::builder()
            .name("X")
            .latitude(48.0)
            .longitude(2.0)
            .build();
        let b = Place::builder().name("X").build();
        let r = MatchingEngine::default_config().match_places(&a, &b);
        assert!(r.breakdown.coordinates_score.is_none());
    }

    #[test]
    fn coordinates_score_none_when_nonfinite() {
        let a = Place::builder()
            .name("X")
            .latitude(f64::NAN)
            .longitude(0.0)
            .build();
        let b = Place::builder()
            .name("X")
            .latitude(0.0)
            .longitude(0.0)
            .build();
        let r = MatchingEngine::default_config().match_places(&a, &b);
        assert!(r.breakdown.coordinates_score.is_none());
    }

    // ---------- category ----------

    #[test]
    fn category_equality_scores_one_else_zero() {
        let a = Place::builder()
            .name("X")
            .category(PlaceCategory::Hotel)
            .build();
        let b = Place::builder()
            .name("X")
            .category(PlaceCategory::Hotel)
            .build();
        let c = Place::builder()
            .name("X")
            .category(PlaceCategory::Cafe)
            .build();
        let engine = MatchingEngine::default_config();
        assert_eq!(
            engine.match_places(&a, &b).breakdown.category_score,
            Some(1.0)
        );
        assert_eq!(
            engine.match_places(&a, &c).breakdown.category_score,
            Some(0.0)
        );
    }

    #[test]
    fn category_other_distinct_strings_score_zero() {
        let a = Place::builder()
            .name("X")
            .category(PlaceCategory::Other("foo".into()))
            .build();
        let b = Place::builder()
            .name("X")
            .category(PlaceCategory::Other("bar".into()))
            .build();
        let r = MatchingEngine::default_config().match_places(&a, &b);
        assert_eq!(r.breakdown.category_score, Some(0.0));
    }

    #[test]
    fn category_score_none_when_either_missing() {
        let a = Place::builder()
            .name("X")
            .category(PlaceCategory::Hotel)
            .build();
        let b = Place::builder().name("X").build();
        let r = MatchingEngine::default_config().match_places(&a, &b);
        assert!(r.breakdown.category_score.is_none());
    }

    // ---------- country code ----------

    #[test]
    fn country_code_case_insensitive_equality() {
        let a = Place::builder()
            .name("X")
            .country_code_as_iso_3166_1_alpha_2("gb")
            .build();
        let b = Place::builder()
            .name("X")
            .country_code_as_iso_3166_1_alpha_2("GB")
            .build();
        let r = MatchingEngine::default_config().match_places(&a, &b);
        assert_eq!(r.breakdown.country_code_score, Some(1.0));
    }

    #[test]
    fn country_code_mismatch_scores_zero() {
        let a = Place::builder()
            .name("X")
            .country_code_as_iso_3166_1_alpha_2("GB")
            .build();
        let b = Place::builder()
            .name("X")
            .country_code_as_iso_3166_1_alpha_2("FR")
            .build();
        let r = MatchingEngine::default_config().match_places(&a, &b);
        assert_eq!(r.breakdown.country_code_score, Some(0.0));
    }

    // ---------- place_ids ----------

    #[test]
    fn place_ids_shared_scores_one() {
        let id = PlaceId::new(PlaceIdScheme::Wikidata, "Q243").unwrap();
        let a = Place::builder().name("X").add_place_id(id.clone()).build();
        let b = Place::builder().name("X").add_place_id(id).build();
        let r = MatchingEngine::default_config().match_places(&a, &b);
        assert_eq!(r.breakdown.place_ids_score, Some(1.0));
    }

    #[test]
    fn place_ids_scheme_local_no_cross_match() {
        let a = Place::builder()
            .name("X")
            .add_place_id(PlaceId::new(PlaceIdScheme::Google, "X").unwrap())
            .build();
        let b = Place::builder()
            .name("X")
            .add_place_id(PlaceId::new(PlaceIdScheme::OsmNode, "X").unwrap())
            .build();
        let r = MatchingEngine::default_config().match_places(&a, &b);
        assert_eq!(r.breakdown.place_ids_score, Some(0.0));
    }

    #[test]
    fn place_ids_none_when_either_side_empty() {
        let a = Place::builder().name("X").build();
        let b = Place::builder()
            .name("X")
            .add_place_id(PlaceId::new(PlaceIdScheme::Wikidata, "Q1").unwrap())
            .build();
        let r = MatchingEngine::default_config().match_places(&a, &b);
        assert!(r.breakdown.place_ids_score.is_none());
    }

    // ---------- deterministic match ----------

    #[test]
    fn deterministic_via_shared_place_id() {
        let id = PlaceId::new(PlaceIdScheme::Wikidata, "Q243").unwrap();
        let a = Place::builder()
            .name("Eiffel Tower")
            .add_place_id(id.clone())
            .build();
        let b = Place::builder()
            .name("Wholly Different")
            .add_place_id(id)
            .build();
        assert!(MatchingEngine::default_config().deterministic_match(&a, &b));
    }

    #[test]
    fn deterministic_via_name_and_postcode() {
        let addr = Address::new().with_postcode("SW1A 2AA");
        let a = Place::builder()
            .name("10 Downing Street")
            .address(addr.clone())
            .build();
        let b = Place::builder()
            .name("10 Downing Street")
            .address(addr)
            .build();
        assert!(MatchingEngine::default_config().deterministic_match(&a, &b));
    }

    #[test]
    fn deterministic_rejects_when_name_differs_and_no_shared_id() {
        let a = Place::builder()
            .name("X")
            .address(Address::new().with_postcode("AA"))
            .build();
        let b = Place::builder()
            .name("Y")
            .address(Address::new().with_postcode("AA"))
            .build();
        assert!(!MatchingEngine::default_config().deterministic_match(&a, &b));
    }

    #[test]
    fn deterministic_rejects_when_postcode_missing_and_no_shared_id() {
        let a = Place::builder().name("X").build();
        let b = Place::builder().name("X").build();
        assert!(!MatchingEngine::default_config().deterministic_match(&a, &b));
    }

    // ---------- strict_mode enforcement ----------

    #[test]
    fn strict_mode_requires_deterministic_for_is_match() {
        let cfg = MatchConfig {
            match_threshold: 0.50,
            strict_mode: true,
            ..MatchConfig::default()
        };
        let p1 = Place::builder()
            .name("Cafe Centrale")
            .latitude(48.0)
            .longitude(2.0)
            .build();
        let p2 = Place::builder()
            .name("Cafe Central") // close but not equal under normalisation
            .latitude(48.0)
            .longitude(2.0)
            .build();
        let engine = MatchingEngine::new(cfg);
        let r = engine.match_places(&p1, &p2);
        assert!(r.score >= 0.50, "should clear threshold");
        // No shared place_id; names differ after normalisation; so deterministic_match is false.
        assert!(!engine.deterministic_match(&p1, &p2));
        assert!(!r.is_match);
    }

    // ---------- batch APIs ----------

    #[test]
    fn match_one_to_many_empty_candidates_yields_empty_vec() {
        let engine = MatchingEngine::default_config();
        let q = Place::builder().name("Solo").build();
        assert!(engine.match_one_to_many(&q, &[]).is_empty());
    }

    #[test]
    fn rank_one_to_many_sorts_by_score_descending() {
        let engine = MatchingEngine::default_config();
        let q = Place::builder().name("Eiffel Tower").build();
        let candidates = vec![
            Place::builder().name("Big Ben").build(),
            q.clone(),
            Place::builder().name("Statue of Liberty").build(),
        ];
        let ranked = engine.rank_one_to_many(&q, &candidates);
        assert_eq!(ranked[0].0, 1);
        for w in ranked.windows(2) {
            assert!(w[0].1.score >= w[1].1.score);
        }
    }

    // ---------- Confidence ----------

    #[test]
    fn confidence_band_boundaries_are_inclusive_on_the_low_side() {
        assert_eq!(Confidence::from_score(0.90), Confidence::High);
        assert_eq!(Confidence::from_score(0.89), Confidence::Medium);
        assert_eq!(Confidence::from_score(0.75), Confidence::Medium);
        assert_eq!(Confidence::from_score(0.74), Confidence::Low);
    }

    // ---------- address ----------

    #[test]
    fn address_with_no_subfields_is_neutral_half() {
        let a = Address::new();
        let b = Address::new();
        let score = MatchingEngine::compare_addresses(&a, &b);
        assert!((score - 0.5).abs() < 1e-9, "got {score}");
    }

    #[test]
    fn address_postcode_match_dominates() {
        let a = Address::new().with_postcode("CF10 1AA");
        let b = Address::new().with_postcode("CF10 1AA");
        let s = MatchingEngine::compare_addresses(&a, &b);
        assert!((s - 1.0).abs() < 1e-9);
    }

    // ---------- phonetic ----------

    #[test]
    fn phonetic_score_none_when_off() {
        let p = Place::builder().name("Stephen").build();
        let q = Place::builder().name("Steven").build();
        let r = MatchingEngine::new(MatchConfig {
            use_phonetic_matching: false,
            ..MatchConfig::default()
        })
        .match_places(&p, &q);
        assert!(r.breakdown.name_phonetic_score.is_none());
    }

    #[test]
    fn phonetic_score_some_when_on() {
        let p = Place::builder().name("Stephen").build();
        let q = Place::builder().name("Steven").build();
        let r = MatchingEngine::new(MatchConfig {
            use_phonetic_matching: true,
            ..MatchConfig::default()
        })
        .match_places(&p, &q);
        assert!(r.breakdown.name_phonetic_score.is_some());
    }
}
