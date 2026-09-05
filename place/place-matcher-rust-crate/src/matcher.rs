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
//!     .latitude_as_decimal_degrees(48.858_222)
//!     .longitude_as_decimal_degrees(2.294_500)
//!     .build();
//!
//! let b = Place::builder()
//!     .name("La Tour Eiffel")
//!     .add_alternate_name("Eiffel Tower")
//!     .latitude_as_decimal_degrees(48.858_3)
//!     .longitude_as_decimal_degrees(2.294_5)
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
///     score_local_id: false,
///     local_id_weight: 0.05,
/// };
/// assert_eq!(custom.match_threshold, 0.80);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
// Four independent feature toggles (`use_phonetic_matching`, `strict_mode`,
// `gmail_dot_folding`, `score_local_id`) trip clippy's pedantic bool-count
// heuristic. Each is a real, independently-documented opt-in with its own
// acceptance criterion (OQ-K added the fourth); collapsing them into an enum
// would obscure rather than clarify, and the acceptance criterion for
// `score_local_id` specifically calls for a `bool`.
#[allow(clippy::struct_excessive_bools)]
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

    /// Opt in to scoring `local_id` (OQ-F/OQ-K). Defaults to `false`,
    /// preserving the crate's long-standing default of never scoring it —
    /// different organisations may issue colliding `local_id` values, so
    /// treating one as identity evidence is only safe when the caller
    /// knows every compared record comes from a single source. When
    /// `true`, [`local_id_weight`](Self::local_id_weight) joins the
    /// weighted sum via an exact-match-after-trim comparison
    /// (`local_id_score`); `false` leaves that score `None` exactly as
    /// before this field existed.
    pub score_local_id: bool,

    /// Weight for `local_id` exact-match equality. Only contributes when
    /// [`score_local_id`](Self::score_local_id) is `true`; inert otherwise
    /// (kept `Default`-derivable rather than `Option`-wrapped so a caller
    /// can pre-set a weight before flipping the flag on).
    pub local_id_weight: f64,
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
            score_local_id: false,
            local_id_weight: 0.05,
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
    /// Score for `local_id` exact-match-after-trim equality (`1.0` or
    /// `0.0`); `None` when either side's trimmed value is empty/absent,
    /// **or** [`MatchConfig::score_local_id`] is `false` (the default) —
    /// see that field's doc comment for why this is opt-in.
    #[serde(default)]
    pub local_id_score: Option<f64>,
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
    /// The immutable scoring configuration: weights, threshold, scale,
    /// algorithm, and feature toggles. Set once at construction and never
    /// mutated, which is what makes the engine `Send + Sync` and safe to
    /// share across threads.
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
    ///     .latitude_as_decimal_degrees(48.858_222)
    ///     .longitude_as_decimal_degrees(2.294_500)
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

    /// Score every field of the two places and assemble the per-field
    /// [`MatchBreakdown`].
    ///
    /// Each component is scored independently; a `None` means the field was
    /// absent on at least one side and so does not participate in the
    /// weighted sum. The phonetic sub-score is only computed when
    /// `use_phonetic_matching` is enabled — otherwise it is `None` and never
    /// contributes its bonus.
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
            local_id_score: if self.config.score_local_id {
                score_local_id(place1, place2)
            } else {
                None
            },
        }
    }

    /// Collapse a [`MatchBreakdown`] into a single overall score in
    /// `[0.0, 1.0]`.
    ///
    /// The score is a **weight-renormalised** sum: each field that scored
    /// (`Some`) contributes `score * weight`, and the running total is
    /// divided by the sum of the weights that actually participated. This
    /// is why missing fields neither help nor hurt — they contribute neither
    /// to `weighted_sum` nor to `total_weight`.
    ///
    /// The phonetic name score is a **bonus only**: it is added (with a small
    /// fixed `0.05` weight) only when it exceeds `0.9`, so it can lift a
    /// borderline match but never drags a score down. When no field scored at
    /// all, `total_weight` is zero and the function returns `0.0` rather than
    /// dividing by zero.
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
        if let Some(score) = breakdown.local_id_score {
            weighted_sum += score * self.config.local_id_weight;
            total_weight += self.config.local_id_weight;
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

    /// Score name similarity as the **best** pairwise score over the
    /// cartesian product of both places' name sets.
    ///
    /// Each place contributes its primary `name` plus every entry in
    /// `alternate_names` (see [`collect_names`]). Every name on the left is
    /// compared against every name on the right, and the single highest
    /// pairwise score wins. This is what lets an alias on one side match a
    /// primary name on the other (e.g. "Big Ben" vs "Elizabeth Tower" +
    /// alternate "Big Ben"). Returns `None` if either side has no usable
    /// names.
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

    /// Score a single pair of raw names.
    ///
    /// Both names are first normalised via
    /// [`Normalizer::normalize_name`] (case-folded, diacritics and ASCII
    /// punctuation stripped, whitespace collapsed) and then compared with
    /// the configured `name_algorithm`. Returns a value in `[0.0, 1.0]`.
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

    /// Score phonetic name similarity: `1.0` if any Soundex code shared
    /// across the two name sets, else `0.0`.
    ///
    /// Each name on both sides is reduced to its Soundex code via
    /// [`Normalizer::phonetic_code`]; if any non-empty code appears on both
    /// sides the names "sound alike" and the score is `1.0`. This catches
    /// spelling variants (Smith/Smyth, Stephen/Steven) that the literal
    /// string scorer might rate lower. Empty codes are ignored so two
    /// unnameable inputs do not spuriously match. Returns `None` if either
    /// side has no usable names. This is an associated (no `self`) function
    /// because the phonetic encoder is fixed and not config-dependent.
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

    /// Score geographic proximity via Gaussian decay over Haversine
    /// distance.
    ///
    /// Both places' coordinates are first validated by [`valid_coords`]
    /// (finite and within the conventional lat/lon ranges); if either side
    /// fails, returns `None` so the field is skipped rather than penalised.
    /// Otherwise the great-circle distance is computed with
    /// [`Scorer::haversine_metres`] and converted to a `[0.0, 1.0]` score by
    /// [`Scorer::coordinates_score`] using the configured
    /// `coordinates_scale_metres`.
    fn score_coordinates(&self, place1: &Place, place2: &Place) -> Option<f64> {
        let (lat1, lon1) = valid_coords(
            place1.latitude_as_decimal_degrees,
            place1.longitude_as_decimal_degrees,
        )?;
        let (lat2, lon2) = valid_coords(
            place2.latitude_as_decimal_degrees,
            place2.longitude_as_decimal_degrees,
        )?;
        let d = Scorer::haversine_metres(lat1, lon1, lat2, lon2);
        Some(Scorer::coordinates_score(
            d,
            self.config.coordinates_scale_metres,
        ))
    }

    /// Score address similarity, or `None` if either place lacks an
    /// address.
    ///
    /// When both addresses are present, delegates to
    /// [`Self::compare_addresses`] for the weighted sub-component comparison.
    /// Associated (no `self`) because address comparison consults no config.
    fn score_address(place1: &Place, place2: &Place) -> Option<f64> {
        match (place1.address.as_ref(), place2.address.as_ref()) {
            (Some(a1), Some(a2)) => Some(Self::compare_addresses(a1, a2)),
            _ => None,
        }
    }

    /// Compare two addresses as a weight-renormalised blend of postcode,
    /// city, and street-line similarity.
    ///
    /// The three sub-components carry fixed weights — postcode `0.5`, city
    /// `0.3`, line 1 `0.2` — but only the components present on *both* sides
    /// participate, and the result is renormalised by the participating
    /// weight so a postcode-only comparison still spans the full `[0.0, 1.0]`
    /// range. Postcode is compared as exact equality after
    /// whitespace/case normalisation; city via Jaro-Winkler on normalised
    /// names; line 1 via a 0.6 street-similarity / 0.4 house-number-equality
    /// blend (falling back to street-only when a house number is absent on
    /// either side). When no sub-component fires, returns the neutral `0.5`
    /// rather than `0.0`, so an empty-vs-empty address is treated as "no
    /// signal" rather than "mismatch".
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

    /// Score phone-number equality: `1.0` if the two numbers canonicalise
    /// to the same value, `0.0` otherwise. `None` if either is absent.
    ///
    /// Prefers the international-aware E.164 form
    /// ([`Normalizer::normalize_phone_e164`], using the configured
    /// `phone_default_country` as the fallback jurisdiction) so numbers from
    /// different countries that share national-significant digits do not
    /// collide. Falls back to the legacy national-significant form
    /// ([`Normalizer::normalize_phone`]) only when either side cannot be
    /// parsed to E.164, preserving behaviour for single-country deployments.
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

    /// Score email-address equality: `1.0` if both addresses canonicalise
    /// to the same value, `0.0` otherwise.
    ///
    /// Both sides are normalised via [`Normalizer::normalize_email`] (trim,
    /// lowercase, optional Gmail dot/`+tag` folding driven by the config's
    /// `gmail_dot_folding`). Returns `None` if either side is absent or
    /// fails to parse as a valid email — an unparseable address is treated
    /// as missing data, not as a mismatch.
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

/// Score category equality: `Some(1.0)` if both categories are set and
/// structurally equal, `Some(0.0)` if both set but differ, `None` if either
/// is absent. Categories are matched by `PartialEq`, so two `Other(_)`
/// variants only match when their carried strings are equal.
fn score_category(p1: &Place, p2: &Place) -> Option<f64> {
    match (&p1.category, &p2.category) {
        (Some(a), Some(b)) => Some(if a == b { 1.0 } else { 0.0 }),
        _ => None,
    }
}

/// Score ISO 3166-1 alpha-2 country-code equality: `Some(1.0)` if both
/// codes are set and equal after trimming whitespace and ASCII-lowercasing,
/// `Some(0.0)` if both set but differ, `None` if either is absent. The
/// case-insensitive trim absorbs the common `"GB"` vs `"gb"` /
/// `" gb "` representation drift between source systems.
fn score_country_code(p1: &Place, p2: &Place) -> Option<f64> {
    let a = p1.country_code_as_iso_3166_1_alpha_2.as_ref()?;
    let b = p2.country_code_as_iso_3166_1_alpha_2.as_ref()?;
    let na = a.trim().to_ascii_lowercase();
    let nb = b.trim().to_ascii_lowercase();
    Some(if na == nb { 1.0 } else { 0.0 })
}

/// Score `local_id` exact-match-after-trim equality: `Some(1.0)` if both
/// sides have a non-empty (after trim) `local_id` and they are equal,
/// `Some(0.0)` if both are non-empty but differ, `None` if either side is
/// absent or trims to empty. Only called when
/// [`MatchConfig::score_local_id`] is `true` (OQ-F/OQ-K) — callers who have
/// not opted in never reach this function, so `local_id` stays unscored by
/// default.
///
/// The empty-after-trim guard mirrors [`shares_place_id`]'s: a blank
/// `local_id` on both sides must never count as shared identity, however
/// it got there.
fn score_local_id(p1: &Place, p2: &Place) -> Option<f64> {
    let a = p1
        .local_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())?;
    let b = p2
        .local_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())?;
    Some(if a == b { 1.0 } else { 0.0 })
}

/// Return `true` if the two places share any identical `(scheme, value)`
/// external ID. Empty-on-either-side short-circuits to `false`. Equality is
/// scheme-scoped via [`PlaceId`](crate::models::PlaceId)'s `PartialEq`, so a
/// `(Google, "X")` never matches an `(OsmNode, "X")`. The nested loop is
/// `O(n*m)`, which is fine because `place_ids` lists are tiny in practice.
///
/// A `PlaceId` whose trimmed `value` is empty is never identity evidence,
/// however it was constructed. `PlaceId::new` already refuses to build one
/// (see its doc comment), but `PlaceId` is not `#[non_exhaustive]` and both
/// fields are `pub` with `#[derive(Deserialize)]`, so a struct literal or
/// `serde_json` deserialization of untrusted input (e.g. `"value": ""`)
/// bypasses that guard entirely. Without this check, two places each
/// carrying one such id — say both from a JSON payload with an empty
/// `value` — would spuriously satisfy `id1 == id2` and trip
/// `deterministic_match`, the same false-identity class already closed
/// for `name_and_postcode_match` (0.7.0, see `CHANGELOG.md`).
fn shares_place_id(p1: &Place, p2: &Place) -> bool {
    if p1.place_ids.is_empty() || p2.place_ids.is_empty() {
        return false;
    }
    for id1 in &p1.place_ids {
        if id1.value.trim().is_empty() {
            continue;
        }
        for id2 in &p2.place_ids {
            if id2.value.trim().is_empty() {
                continue;
            }
            if id1 == id2 {
                return true;
            }
        }
    }
    false
}

/// Score external-ID overlap for the probabilistic breakdown: `Some(1.0)`
/// if the places share any `(scheme, value)` pair, `Some(0.0)` if both
/// have IDs but none overlap, `None` if either side has no IDs at all (so
/// the field is skipped rather than penalised). Wraps [`shares_place_id`].
fn score_place_ids(p1: &Place, p2: &Place) -> Option<f64> {
    if p1.place_ids.is_empty() || p2.place_ids.is_empty() {
        return None;
    }
    Some(if shares_place_id(p1, p2) { 1.0 } else { 0.0 })
}

/// Return `true` iff both places have a primary `name` that normalises to
/// the same value AND an `address.postcode` that normalises to the same
/// value. This is the second deterministic-match rule (the first being a
/// shared external ID). Any missing name or postcode short-circuits to
/// `false`. Names are compared via [`Normalizer::normalize_name`] and
/// postcodes via [`Normalizer::normalize_postcode`], so cosmetic
/// differences (case, punctuation, internal spacing) do not block a match.
///
/// A value that normalises to the empty string (e.g. `"."`, `"--"`, or a
/// whitespace-only postcode) is NOT identity evidence: if either the
/// normalised name OR the normalised postcode is empty, the rule does not
/// fire. Without this guard two unrelated places whose name and postcode
/// both normalise to `""` would spuriously satisfy `"" == ""` twice and
/// short-circuit to a false 1.0 identity match (SEC-M2).
fn name_and_postcode_match(p1: &Place, p2: &Place) -> bool {
    let (Some(n1), Some(n2)) = (&p1.name, &p2.name) else {
        return false;
    };
    let name1 = Normalizer::normalize_name(n1);
    let name2 = Normalizer::normalize_name(n2);
    if name1.is_empty() || name1 != name2 {
        return false;
    }
    let (Some(pc1), Some(pc2)) = (
        p1.address.as_ref().and_then(|a| a.postcode.as_ref()),
        p2.address.as_ref().and_then(|a| a.postcode.as_ref()),
    ) else {
        return false;
    };
    let postcode1 = Normalizer::normalize_postcode(pc1);
    let postcode2 = Normalizer::normalize_postcode(pc2);
    !postcode1.is_empty() && postcode1 == postcode2
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{PlaceCategory, PlaceId, PlaceIdScheme};

    /// Compare two floats for approximate equality.
    ///
    /// Tests assert scores against representable constants (`0.0`, `1.0`, …);
    /// this avoids exact `==` on floats while staying within one ULP.
    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < f64::EPSILON
    }

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
            .latitude_as_decimal_degrees(48.858_222)
            .longitude_as_decimal_degrees(2.294_500)
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
            .latitude_as_decimal_degrees(48.858_222)
            .longitude_as_decimal_degrees(2.294_500)
            .build();
        let b = Place::builder()
            .name("Sydney Opera House")
            .latitude_as_decimal_degrees(-33.856_8)
            .longitude_as_decimal_degrees(151.215_3)
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
        assert!(approx_eq(r.score, 0.0));
    }

    // ---------- coordinates ----------

    #[test]
    fn coordinates_score_present_when_both_sides_have_coords() {
        let a = Place::builder()
            .name("X")
            .latitude_as_decimal_degrees(0.0)
            .longitude_as_decimal_degrees(0.0)
            .build();
        let b = Place::builder()
            .name("X")
            .latitude_as_decimal_degrees(0.0)
            .longitude_as_decimal_degrees(0.0)
            .build();
        let r = MatchingEngine::default_config().match_places(&a, &b);
        assert!((r.breakdown.coordinates_score.unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn coordinates_score_none_when_out_of_range() {
        let a = Place::builder()
            .name("X")
            .latitude_as_decimal_degrees(91.0)
            .longitude_as_decimal_degrees(0.0)
            .build();
        let b = Place::builder()
            .name("X")
            .latitude_as_decimal_degrees(0.0)
            .longitude_as_decimal_degrees(0.0)
            .build();
        let r = MatchingEngine::default_config().match_places(&a, &b);
        assert!(r.breakdown.coordinates_score.is_none());
    }

    #[test]
    fn coordinates_score_none_when_one_side_missing() {
        let a = Place::builder()
            .name("X")
            .latitude_as_decimal_degrees(48.0)
            .longitude_as_decimal_degrees(2.0)
            .build();
        let b = Place::builder().name("X").build();
        let r = MatchingEngine::default_config().match_places(&a, &b);
        assert!(r.breakdown.coordinates_score.is_none());
    }

    #[test]
    fn coordinates_score_none_when_nonfinite() {
        let a = Place::builder()
            .name("X")
            .latitude_as_decimal_degrees(f64::NAN)
            .longitude_as_decimal_degrees(0.0)
            .build();
        let b = Place::builder()
            .name("X")
            .latitude_as_decimal_degrees(0.0)
            .longitude_as_decimal_degrees(0.0)
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
        assert!(approx_eq(r.breakdown.category_score.unwrap(), 0.0));
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
        assert!(approx_eq(r.breakdown.country_code_score.unwrap(), 1.0));
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
        assert!(approx_eq(r.breakdown.country_code_score.unwrap(), 0.0));
    }

    // ---------- place_ids ----------

    #[test]
    fn place_ids_shared_scores_one() {
        let id = PlaceId::new(PlaceIdScheme::Wikidata, "Q243").unwrap();
        let a = Place::builder().name("X").add_place_id(id.clone()).build();
        let b = Place::builder().name("X").add_place_id(id).build();
        let r = MatchingEngine::default_config().match_places(&a, &b);
        assert!(approx_eq(r.breakdown.place_ids_score.unwrap(), 1.0));
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
        assert!(approx_eq(r.breakdown.place_ids_score.unwrap(), 0.0));
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

    // ---------- local_id (OQ-F/OQ-K: opt-in scoring) ----------

    /// The default `MatchConfig` never scores `local_id`, even when both
    /// sides carry the identical value — behaviour must stay byte-for-byte
    /// unchanged from before this field existed.
    #[test]
    fn local_id_unscored_by_default_even_when_identical() {
        let a = Place::builder().name("X").local_id("REF-1").build();
        let b = Place::builder().name("X").local_id("REF-1").build();
        let r = MatchingEngine::default_config().match_places(&a, &b);
        assert!(
            r.breakdown.local_id_score.is_none(),
            "local_id must stay unscored unless MatchConfig::score_local_id is opted in"
        );
    }

    #[test]
    fn local_id_scores_one_when_opted_in_and_equal() {
        let cfg = MatchConfig {
            score_local_id: true,
            ..MatchConfig::default()
        };
        let a = Place::builder().name("X").local_id(" REF-1 ").build();
        let b = Place::builder().name("X").local_id("REF-1").build();
        let r = MatchingEngine::new(cfg).match_places(&a, &b);
        assert!(approx_eq(r.breakdown.local_id_score.unwrap(), 1.0));
    }

    #[test]
    fn local_id_scores_zero_when_opted_in_and_different() {
        let cfg = MatchConfig {
            score_local_id: true,
            ..MatchConfig::default()
        };
        let a = Place::builder().name("X").local_id("REF-1").build();
        let b = Place::builder().name("X").local_id("REF-2").build();
        let r = MatchingEngine::new(cfg).match_places(&a, &b);
        assert!(approx_eq(r.breakdown.local_id_score.unwrap(), 0.0));
    }

    #[test]
    fn local_id_none_when_opted_in_but_either_side_absent_or_blank() {
        let cfg = MatchConfig {
            score_local_id: true,
            ..MatchConfig::default()
        };
        let engine = MatchingEngine::new(cfg);

        let a = Place::builder().name("X").local_id("REF-1").build();
        let b = Place::builder().name("X").build();
        assert!(
            engine
                .match_places(&a, &b)
                .breakdown
                .local_id_score
                .is_none()
        );

        let blank_a = Place::builder().name("X").local_id("REF-1").build();
        let blank_b = Place::builder().name("X").local_id("   ").build();
        assert!(
            engine
                .match_places(&blank_a, &blank_b)
                .breakdown
                .local_id_score
                .is_none(),
            "a whitespace-only local_id must never count as shared identity"
        );
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

    // spec/10-open-questions.md OQ-I: `PlaceId::new` refuses an empty
    // trimmed value, but `PlaceId` is not `#[non_exhaustive]` and both
    // fields are `pub` with `#[derive(Deserialize)]`, so a struct literal
    // (or `serde_json` deserialization of untrusted input) bypasses that
    // guard entirely. Two unrelated places each carrying one such
    // empty-value id must never spuriously match — this pins the fix.
    #[test]
    fn empty_value_place_id_never_matches_even_via_constructor_bypass() {
        let empty_id = PlaceId {
            scheme: PlaceIdScheme::Google,
            value: String::new(),
        };
        let a = Place::builder()
            .name("Eiffel Tower")
            .add_place_id(empty_id.clone())
            .build();
        let b = Place::builder()
            .name("Wholly Different")
            .add_place_id(empty_id)
            .build();
        assert!(!MatchingEngine::default_config().deterministic_match(&a, &b));
        let r = MatchingEngine::default_config().match_places(&a, &b);
        assert!(approx_eq(r.breakdown.place_ids_score.unwrap(), 0.0));
    }

    // Same bypass, but the empty value is whitespace-only — `PlaceId::new`
    // treats a trimmed-empty value as invalid, so the fix must too.
    #[test]
    fn whitespace_only_value_place_id_never_matches() {
        let ws_id = PlaceId {
            scheme: PlaceIdScheme::Google,
            value: "   ".to_string(),
        };
        let a = Place::builder()
            .name("X")
            .add_place_id(ws_id.clone())
            .build();
        let b = Place::builder().name("Y").add_place_id(ws_id).build();
        assert!(!MatchingEngine::default_config().deterministic_match(&a, &b));
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

    #[test]
    fn deterministic_reject_blank_name_and_postcode() {
        // Two clearly-different places whose only shared values are a name
        // and postcode that both normalise to the empty string ("." → "",
        // whitespace-only postcode → ""). An empty normalisation is not
        // identity evidence, so the deterministic rule must NOT fire (SEC-M2).
        let a = Place::builder()
            .name(".")
            .address(Address::new().with_postcode(" "))
            .latitude_as_decimal_degrees(51.5)
            .longitude_as_decimal_degrees(-0.1)
            .build();
        let b = Place::builder()
            .name("--")
            .address(Address::new().with_postcode("   "))
            .latitude_as_decimal_degrees(40.7)
            .longitude_as_decimal_degrees(-74.0)
            .build();
        // Sanity: both names and both postcodes normalise to empty.
        assert_eq!(Normalizer::normalize_name("."), "");
        assert_eq!(Normalizer::normalize_name("--"), "");
        assert_eq!(Normalizer::normalize_postcode(" "), "");
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
            .latitude_as_decimal_degrees(48.0)
            .longitude_as_decimal_degrees(2.0)
            .build();
        let p2 = Place::builder()
            .name("Cafe Central") // close but not equal under normalisation
            .latitude_as_decimal_degrees(48.0)
            .longitude_as_decimal_degrees(2.0)
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

    // ---------- Confidence::from_score degenerate inputs ----------

    // Pins spec.md §3.6 — Confidence::from_score total-over-f64 contract.
    #[test]
    fn confidence_from_score_degenerate_inputs() {
        // NaN is neither >= 0.90 nor >= 0.75, so it falls through to Low.
        assert_eq!(Confidence::from_score(f64::NAN), Confidence::Low);
        // Negative scores are below every band → Low.
        assert_eq!(Confidence::from_score(-0.5), Confidence::Low);
        assert_eq!(Confidence::from_score(f64::NEG_INFINITY), Confidence::Low);
        // Scores above 1.0 still satisfy the High band (>= 0.90).
        assert_eq!(Confidence::from_score(2.0), Confidence::High);
        assert_eq!(Confidence::from_score(f64::INFINITY), Confidence::High);
    }

    // ---------- address line-1 house-number blend ----------

    // Pins spec.md §6.4 — line 1 sub-score is 0.6*street + 0.4*house_number
    // equality when a house number is present on both sides.
    #[test]
    fn address_line1_blends_street_and_house_number_when_both_present() {
        // Same street, same house number → line1 sub-score 1.0.
        let same = MatchingEngine::compare_addresses(
            &Address::new().with_line1("10 Downing Street"),
            &Address::new().with_line1("10 Downing Street"),
        );
        assert!((same - 1.0).abs() < 1e-9, "got {same}");

        // Same street, different house number → 0.6*1.0 + 0.4*0.0 = 0.6.
        let diff_number = MatchingEngine::compare_addresses(
            &Address::new().with_line1("10 Downing Street"),
            &Address::new().with_line1("12 Downing Street"),
        );
        assert!((diff_number - 0.6).abs() < 1e-9, "got {diff_number}");
    }

    // Pins spec.md §6.4 — when a house number is absent on either side the
    // line 1 sub-score falls back to street similarity alone (no 0.4 term).
    #[test]
    fn address_line1_falls_back_to_street_only_without_house_number() {
        // No house number on either side → street-only, identical → 1.0.
        let s = MatchingEngine::compare_addresses(
            &Address::new().with_line1("Downing Street"),
            &Address::new().with_line1("Downing Street"),
        );
        assert!((s - 1.0).abs() < 1e-9, "got {s}");

        // House number on one side only → still street-only, so the
        // differing numbers do NOT pull the score down to the 0.6 blend.
        let one_sided = MatchingEngine::compare_addresses(
            &Address::new().with_line1("10 Downing Street"),
            &Address::new().with_line1("Downing Street"),
        );
        assert!(
            one_sided > 0.9,
            "street-only fallback expected near 1.0, got {one_sided}"
        );
    }

    // ---------- phone cross-country disambiguation ----------

    // Pins spec.md §6.8 — E.164-preferred path: a UK and a FR number that
    // share the same national-significant digits must NOT collide.
    #[test]
    fn phone_same_nsn_different_country_does_not_collide() {
        let engine = MatchingEngine::default_config();
        // Same trailing digits, different country dial codes.
        let uk = Place::builder().name("X").phone("+44 161 496 0000").build();
        let fr = Place::builder().name("X").phone("+33 161 496 0000").build();
        let r = engine.match_places(&uk, &fr);
        assert_eq!(
            r.breakdown.phone_score,
            Some(0.0),
            "UK and FR numbers sharing NSN must not match"
        );
    }

    // Pins spec.md §6.8 — two identical E.164-parseable numbers score 1.0.
    #[test]
    fn phone_identical_e164_scores_one() {
        let engine = MatchingEngine::default_config();
        let a = Place::builder().name("X").phone("+44 161 496 0000").build();
        let b = Place::builder().name("X").phone("0161 496 0000").build();
        let r = engine.match_places(&a, &b);
        assert_eq!(r.breakdown.phone_score, Some(1.0));
    }

    // Pins spec.md §6.8 — legacy-fallback path: when neither side parses to
    // E.164 the comparison uses the legacy national-significant form.
    #[test]
    fn phone_legacy_fallback_when_not_e164() {
        // phone_default_country = None forces E.164 to fail (no default
        // jurisdiction), so the legacy normaliser decides the outcome.
        let engine = MatchingEngine::new(MatchConfig {
            phone_default_country: None,
            ..MatchConfig::default()
        });
        let a = Place::builder().name("X").phone("0161 496 0000").build();
        let b = Place::builder().name("X").phone("01614960000").build();
        let r = engine.match_places(&a, &b);
        assert_eq!(r.breakdown.phone_score, Some(1.0));
    }

    // ---------- email scoring at the engine level ----------

    // Pins spec.md §6.9 — an unparseable email is treated as missing (None),
    // not as a 0.0 mismatch.
    #[test]
    fn email_unparseable_is_none_not_zero() {
        let engine = MatchingEngine::default_config();
        // Second address lacks an '@' so normalize_email returns None.
        let a = Place::builder().name("X").email("info@example.org").build();
        let b = Place::builder().name("X").email("not-an-email").build();
        let r = engine.match_places(&a, &b);
        assert_eq!(r.breakdown.email_score, None);
    }

    // Pins spec.md §6.9 — two valid but different emails score 0.0.
    #[test]
    fn email_both_parse_but_differ_is_zero() {
        let engine = MatchingEngine::default_config();
        let a = Place::builder().name("X").email("info@example.org").build();
        let b = Place::builder()
            .name("X")
            .email("sales@example.org")
            .build();
        let r = engine.match_places(&a, &b);
        assert_eq!(r.breakdown.email_score, Some(0.0));
    }

    // ---------- phonetic bonus arithmetic ----------

    // Pins spec.md §5.2.2 — the phonetic bonus (score*0.05, weight 0.05) is
    // only applied when name_phonetic_score > 0.9. Compare the same place
    // pair with the bonus arithmetically engaged vs not.
    #[test]
    fn phonetic_bonus_lifts_score_only_when_gate_clears() {
        // Names that sound alike (shared Soundex) but score < 1.0 literally,
        // so the phonetic bonus has room to lift the renormalised score.
        let p = Place::builder().name("Catherine").build();
        let q = Place::builder().name("Kathryn").build();

        let off = MatchingEngine::new(MatchConfig {
            use_phonetic_matching: false,
            ..MatchConfig::default()
        })
        .match_places(&p, &q);

        let on = MatchingEngine::new(MatchConfig {
            use_phonetic_matching: true,
            ..MatchConfig::default()
        })
        .match_places(&p, &q);

        // With phonetic off there is no phonetic sub-score at all.
        assert!(off.breakdown.name_phonetic_score.is_none());

        // With phonetic on the gating sub-score is present.
        let phon = on
            .breakdown
            .name_phonetic_score
            .expect("phonetic score present when enabled");

        if phon > 0.9 {
            // Gate fires: the bonus is added with positive weight, so the
            // renormalised score is strictly higher than with the gate off.
            assert!(
                on.score > off.score,
                "gate fired (phon={phon}) so on.score ({}) should exceed off.score ({})",
                on.score,
                off.score
            );
        } else {
            // Gate does not fire: the only-name pair has identical
            // participating weights, so the score is unchanged.
            assert!(
                approx_eq(on.score, off.score),
                "gate did not fire (phon={phon}) so scores should be equal: {} vs {}",
                on.score,
                off.score
            );
        }
    }
}
