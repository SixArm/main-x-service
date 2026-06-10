//! Thing matcher engine: deterministic and probabilistic algorithms.
//!
//! This is the orchestration layer of the crate. It pulls together the data
//! types from [`crate::models`], the text transformations from
//! [`crate::normalizer`], and the similarity primitives from
//! [`crate::scorer`] to produce a single answer about whether two `Thing`
//! records refer to the same item.
//!
//! ## Two strategies, one engine
//!
//! - [`MatchingEngine::deterministic_match`] — fast, binary. Returns `true`
//!   iff the two things share any `(property_id, value)` identifier, any
//!   `sameAs` URL, or the same canonical `url`.
//! - [`MatchingEngine::match_things`] — weighted probabilistic scoring,
//!   returning a [`MatchResult`] with per-field [`MatchBreakdown`].
//!
//! ## Example
//!
//! ```
//! use thing_matcher::{MatchingEngine, Thing};
//!
//! let a = Thing::builder()
//!     .name("Eiffel Tower")
//!     .url("https://www.toureiffel.paris/")
//!     .build();
//!
//! let b = Thing::builder()
//!     .name("La Tour Eiffel")
//!     .add_alternate_name("Eiffel Tower")
//!     .url("https://www.toureiffel.paris/")
//!     .build();
//!
//! let engine = MatchingEngine::default_config();
//! let result = engine.match_things(&a, &b);
//! assert!(result.is_match);
//! ```

use crate::models::Thing;
use crate::normalizer::Normalizer;
use crate::scorer::{Scorer, SimilarityAlgorithm};
use serde::{Deserialize, Serialize};

/// Tunable configuration for the matching engine.
///
/// All weights are dimensionless and contribute to a renormalised weighted
/// sum — they do not need to add to `1.0`. The matching pipeline divides
/// the weighted sum by the sum of *participating* weights so that missing
/// fields neither contribute nor penalise. The score is then compared
/// against [`MatchConfig::match_threshold`] to produce the `is_match`
/// boolean.
///
/// Two presets cover most needs:
///
/// - [`MatchConfig::strict`]  — `match_threshold = 0.95`, `strict_mode = true`.
/// - [`MatchConfig::lenient`] — `match_threshold = 0.65`, phonetic on.
///
/// # Example
///
/// ```
/// use thing_matcher::{MatchConfig, SimilarityAlgorithm};
///
/// let custom = MatchConfig {
///     match_threshold: 0.80,
///     name_weight: 0.30,
///     description_weight: 0.10,
///     disambiguating_description_weight: 0.05,
///     identifiers_weight: 0.25,
///     url_weight: 0.05,
///     same_as_weight: 0.15,
///     image_weight: 0.03,
///     main_entity_of_page_weight: 0.02,
///     additional_types_weight: 0.05,
///     use_phonetic_matching: true,
///     name_algorithm: SimilarityAlgorithm::Combined,
///     strict_mode: false,
/// };
/// assert_eq!(custom.match_threshold, 0.80);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MatchConfig {
    /// Threshold score for considering two things a match (`0.0..=1.0`).
    pub match_threshold: f64,

    /// Weight for name similarity (best-of cartesian product across the
    /// primary `name` and `alternate_names` on both sides).
    pub name_weight: f64,

    /// Weight for free-form `description` similarity.
    pub description_weight: f64,

    /// Weight for `disambiguatingDescription` similarity.
    pub disambiguating_description_weight: f64,

    /// Weight for "shared identifier" (1.0 if any `(property_id, value)`
    /// pair is shared, 0.0 otherwise).
    pub identifiers_weight: f64,

    /// Weight for canonical `url` exact match (after URL normalisation).
    pub url_weight: f64,

    /// Weight for `sameAs` URL set similarity (Jaccard).
    pub same_as_weight: f64,

    /// Weight for `image` URL exact match (after URL normalisation).
    pub image_weight: f64,

    /// Weight for `mainEntityOfPage` URL exact match (after URL
    /// normalisation).
    pub main_entity_of_page_weight: f64,

    /// Weight for `additionalType` URI set similarity (Jaccard).
    pub additional_types_weight: f64,

    /// Whether to add a phonetic-name bonus when both names sound alike.
    pub use_phonetic_matching: bool,

    /// Similarity algorithm to use when comparing names.
    pub name_algorithm: SimilarityAlgorithm,

    /// Reserved flag for stricter deterministic enforcement.
    pub strict_mode: bool,
}

impl Default for MatchConfig {
    /// Production-ready defaults.
    ///
    /// ```
    /// use thing_matcher::{MatchConfig, SimilarityAlgorithm};
    /// let c = MatchConfig::default();
    /// assert!((c.match_threshold - 0.80).abs() < 1e-9);
    /// assert!(matches!(c.name_algorithm, SimilarityAlgorithm::Combined));
    /// ```
    fn default() -> Self {
        Self {
            match_threshold: 0.80,
            name_weight: 0.30,
            description_weight: 0.10,
            disambiguating_description_weight: 0.05,
            identifiers_weight: 0.25,
            url_weight: 0.05,
            same_as_weight: 0.15,
            image_weight: 0.03,
            main_entity_of_page_weight: 0.02,
            additional_types_weight: 0.05,
            use_phonetic_matching: false,
            name_algorithm: SimilarityAlgorithm::Combined,
            strict_mode: false,
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
    /// use thing_matcher::MatchConfig;
    /// let c = MatchConfig::strict();
    /// assert!((c.match_threshold - 0.95).abs() < 1e-9);
    /// assert!(c.strict_mode);
    /// ```
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
    /// use thing_matcher::MatchConfig;
    /// let c = MatchConfig::lenient();
    /// assert!((c.match_threshold - 0.65).abs() < 1e-9);
    /// assert!(c.use_phonetic_matching);
    /// ```
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
/// use thing_matcher::Confidence;
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
    /// additional evidence before treating as the same thing.
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
    /// use thing_matcher::Confidence;
    ///
    /// assert_eq!(Confidence::from_score(f64::NAN), Confidence::Low);
    /// assert_eq!(Confidence::from_score(-0.5),     Confidence::Low);
    /// assert_eq!(Confidence::from_score(2.0),      Confidence::High);
    /// ```
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

/// Outcome of a probabilistic thing match.
///
/// Contains the overall renormalised `score`, the threshold-derived
/// `is_match` boolean, a coarse [`Confidence`] band, and a per-field
/// [`MatchBreakdown`] for audit.
///
/// `MatchResult` implements `Serialize + Deserialize` so it can be persisted
/// or returned over an API.
///
/// ```
/// use thing_matcher::{Confidence, MatchingEngine, Thing};
///
/// let t = Thing::builder().name("Eiffel Tower").build();
/// let q = t.clone();
/// let result = MatchingEngine::default_config().match_things(&t, &q);
/// assert_eq!(result.confidence, Confidence::High);
///
/// // Round-trip through JSON.
/// let json = serde_json::to_string(&result).unwrap();
/// let back: thing_matcher::MatchResult = serde_json::from_str(&json).unwrap();
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
    /// `Combined` similarity over `description`, after `normalize_text`.
    /// `None` if either side is absent.
    pub description_score: Option<f64>,
    /// `Combined` similarity over `disambiguatingDescription`, after
    /// `normalize_text`. `None` if either side is absent.
    pub disambiguating_description_score: Option<f64>,
    /// `1.0` if both `identifiers` non-empty and they share any
    /// `(property_id, value)` pair; `0.0` if both non-empty but none
    /// shared; `None` if either side is empty.
    pub identifiers_score: Option<f64>,
    /// `1.0` if both `url`s normalise to the same string; `0.0`
    /// otherwise; `None` if either side is absent.
    pub url_score: Option<f64>,
    /// Jaccard set similarity over the union of `sameAs` URLs after
    /// `normalize_url`. `None` if both sides are empty.
    pub same_as_score: Option<f64>,
    /// `1.0` if both `image`s normalise to the same string; `0.0`
    /// otherwise; `None` if either side is absent.
    pub image_score: Option<f64>,
    /// `1.0` if both `mainEntityOfPage`s normalise to the same string;
    /// `0.0` otherwise; `None` if either side is absent.
    pub main_entity_of_page_score: Option<f64>,
    /// Jaccard set similarity over the union of `additionalType` URIs
    /// after `normalize_url`. `None` if both sides are empty.
    pub additional_types_score: Option<f64>,
}

/// Thing matcher engine.
///
/// The engine is **immutable after construction** and cheap to clone (it
/// owns only a [`MatchConfig`]). Construct one and call its methods from
/// any thread.
///
/// ```
/// use thing_matcher::{MatchConfig, MatchingEngine};
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
    /// use thing_matcher::{MatchConfig, MatchingEngine};
    /// let engine = MatchingEngine::new(MatchConfig::lenient());
    /// # let _ = engine;
    /// ```
    pub fn new(config: MatchConfig) -> Self {
        Self { config }
    }

    /// Construct an engine with [`MatchConfig::default`].
    ///
    /// ```
    /// use thing_matcher::MatchingEngine;
    /// let engine = MatchingEngine::default_config();
    /// # let _ = engine;
    /// ```
    pub fn default_config() -> Self {
        Self::new(MatchConfig::default())
    }

    /// Compare two things probabilistically and return a [`MatchResult`].
    ///
    /// The score is the weight-renormalised sum of every component that
    /// scored on both records. Missing fields are skipped, not penalised.
    ///
    /// ```
    /// use thing_matcher::{MatchingEngine, Thing};
    ///
    /// let t = Thing::builder()
    ///     .name("Eiffel Tower")
    ///     .url("https://www.toureiffel.paris/")
    ///     .build();
    ///
    /// let result = MatchingEngine::default_config().match_things(&t, &t);
    /// assert!(result.is_match);
    /// assert!(result.score > 0.99);
    /// ```
    pub fn match_things(&self, thing1: &Thing, thing2: &Thing) -> MatchResult {
        let breakdown = self.calculate_breakdown(thing1, thing2);
        let score = self.calculate_weighted_score(&breakdown);
        let above_threshold = score >= self.config.match_threshold;
        // Under strict mode, `is_match` ALSO requires a deterministic match.
        let is_match = if self.config.strict_mode {
            above_threshold && self.deterministic_match(thing1, thing2)
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
    /// use thing_matcher::{MatchingEngine, Thing};
    ///
    /// let query = Thing::builder().name("Eiffel Tower").build();
    /// let candidates = vec![
    ///     Thing::builder().name("Eiffel Tower").build(),
    ///     Thing::builder().name("Big Ben").build(),
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
    /// # use thing_matcher::{MatchingEngine, Thing};
    /// let q = Thing::builder().name("Solo").build();
    /// let r = MatchingEngine::default_config().match_one_to_many(&q, &[]);
    /// assert!(r.is_empty());
    /// ```
    pub fn match_one_to_many(&self, query: &Thing, candidates: &[Thing]) -> Vec<MatchResult> {
        candidates
            .iter()
            .map(|c| self.match_things(query, c))
            .collect()
    }

    /// Score and rank: return `(original_index, MatchResult)` tuples
    /// sorted by descending score. Ties are broken by ascending original
    /// index, so the result is deterministic.
    ///
    /// # Examples
    ///
    /// ```
    /// use thing_matcher::{MatchingEngine, Thing};
    ///
    /// let query = Thing::builder().name("Eiffel Tower").build();
    /// let candidates = vec![
    ///     Thing::builder().name("Big Ben").build(),                 // index 0
    ///     Thing::builder().name("Eiffel Tower").build(),            // index 1 — best match
    ///     Thing::builder().name("Statue of Liberty").build(),       // index 2
    /// ];
    ///
    /// let ranked = MatchingEngine::default_config().rank_one_to_many(&query, &candidates);
    /// assert_eq!(ranked.len(), 3);
    /// assert_eq!(ranked[0].0, 1);
    /// assert!(ranked[0].1.score >= ranked[1].1.score);
    /// assert!(ranked[1].1.score >= ranked[2].1.score);
    /// ```
    pub fn rank_one_to_many(
        &self,
        query: &Thing,
        candidates: &[Thing],
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

    /// Compare two things deterministically and return a single boolean.
    ///
    /// Returns `true` iff any of the following hold:
    ///
    /// - the things share any `(property_id, value)` pair in their
    ///   `identifiers` lists;
    /// - the things share any `sameAs` URL after URL normalisation;
    /// - both have a `url` that normalises to the same string.
    ///
    /// ```
    /// use thing_matcher::{Identifier, MatchingEngine, Thing};
    ///
    /// let id = Identifier::new("wikidata", "Q243").unwrap();
    /// let a = Thing::builder().name("Eiffel Tower").add_identifier(id.clone()).build();
    /// let b = Thing::builder().name("Tour Eiffel").add_identifier(id).build();
    /// assert!(MatchingEngine::default_config().deterministic_match(&a, &b));
    /// ```
    pub fn deterministic_match(&self, thing1: &Thing, thing2: &Thing) -> bool {
        if shares_identifier(thing1, thing2) {
            return true;
        }
        if shares_same_as(thing1, thing2) {
            return true;
        }
        same_canonical_url(thing1, thing2)
    }

    fn calculate_breakdown(&self, thing1: &Thing, thing2: &Thing) -> MatchBreakdown {
        MatchBreakdown {
            name_score: self.score_name(thing1, thing2),
            name_phonetic_score: if self.config.use_phonetic_matching {
                score_phonetic_names(thing1, thing2)
            } else {
                None
            },
            description_score: score_text(thing1.description.as_ref(), thing2.description.as_ref()),
            disambiguating_description_score: score_text(
                thing1.disambiguating_description.as_ref(),
                thing2.disambiguating_description.as_ref(),
            ),
            identifiers_score: score_identifiers(thing1, thing2),
            url_score: score_url(thing1.url.as_ref(), thing2.url.as_ref()),
            same_as_score: score_url_set(&thing1.same_as, &thing2.same_as),
            image_score: score_url(thing1.image.as_ref(), thing2.image.as_ref()),
            main_entity_of_page_score: score_url(
                thing1.main_entity_of_page.as_ref(),
                thing2.main_entity_of_page.as_ref(),
            ),
            additional_types_score: score_url_set(
                &thing1.additional_types,
                &thing2.additional_types,
            ),
        }
    }

    fn calculate_weighted_score(&self, breakdown: &MatchBreakdown) -> f64 {
        let mut total_weight = 0.0;
        let mut weighted_sum = 0.0;

        let mut add = |score: Option<f64>, weight: f64| {
            if let Some(s) = score {
                weighted_sum += s * weight;
                total_weight += weight;
            }
        };

        add(breakdown.name_score, self.config.name_weight);
        add(breakdown.description_score, self.config.description_weight);
        add(
            breakdown.disambiguating_description_score,
            self.config.disambiguating_description_weight,
        );
        add(breakdown.identifiers_score, self.config.identifiers_weight);
        add(breakdown.url_score, self.config.url_weight);
        add(breakdown.same_as_score, self.config.same_as_weight);
        add(breakdown.image_score, self.config.image_weight);
        add(
            breakdown.main_entity_of_page_score,
            self.config.main_entity_of_page_weight,
        );
        add(
            breakdown.additional_types_score,
            self.config.additional_types_weight,
        );

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

    fn score_name(&self, thing1: &Thing, thing2: &Thing) -> Option<f64> {
        let names1 = collect_names(thing1);
        let names2 = collect_names(thing2);
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
}

// ---- Free helpers ------------------------------------------------------

/// Collect a thing's primary name plus alternate names into a single vec
/// of references. Empty / whitespace-only strings are skipped.
fn collect_names(thing: &Thing) -> Vec<&String> {
    thing
        .name
        .iter()
        .chain(thing.alternate_names.iter())
        .filter(|s| !s.trim().is_empty())
        .collect()
}

/// Maximum Soundex match (`1.0` / `0.0`) across the cartesian product of
/// both things' names. `None` when either side has no usable names.
fn score_phonetic_names(thing1: &Thing, thing2: &Thing) -> Option<f64> {
    let names1 = collect_names(thing1);
    let names2 = collect_names(thing2);
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

/// `Combined` similarity over a pair of optional free-form text fields.
/// Returns `None` if either side is absent.
fn score_text(a: Option<&String>, b: Option<&String>) -> Option<f64> {
    let na = Normalizer::normalize_text(a?);
    let nb = Normalizer::normalize_text(b?);
    Some(Scorer::combined_similarity(&na, &nb))
}

/// Exact match over a pair of optional URL fields, compared after URL
/// normalisation. Returns `None` if either side is absent.
fn score_url(a: Option<&String>, b: Option<&String>) -> Option<f64> {
    let na = Normalizer::normalize_url(a?);
    let nb = Normalizer::normalize_url(b?);
    Some(Scorer::exact_match(&na, &nb))
}

/// Jaccard set similarity over two URL lists. Returns `None` only if both
/// sides are empty; an empty-against-non-empty pair scores `0.0`.
fn score_url_set(a: &[String], b: &[String]) -> Option<f64> {
    if a.is_empty() && b.is_empty() {
        return None;
    }
    let na: Vec<String> = a.iter().map(|s| Normalizer::normalize_url(s)).collect();
    let nb: Vec<String> = b.iter().map(|s| Normalizer::normalize_url(s)).collect();
    Some(Scorer::jaccard_set_similarity(&na, &nb))
}

/// `Some(1.0)` if any `(property_id, value)` pair is shared, `Some(0.0)`
/// if both lists are non-empty but no pair is shared, `None` if either
/// list is empty.
fn score_identifiers(thing1: &Thing, thing2: &Thing) -> Option<f64> {
    if thing1.identifiers.is_empty() || thing2.identifiers.is_empty() {
        return None;
    }
    Some(if shares_identifier(thing1, thing2) {
        1.0
    } else {
        0.0
    })
}

fn shares_identifier(thing1: &Thing, thing2: &Thing) -> bool {
    if thing1.identifiers.is_empty() || thing2.identifiers.is_empty() {
        return false;
    }
    for id1 in &thing1.identifiers {
        for id2 in &thing2.identifiers {
            if id1 == id2 {
                return true;
            }
        }
    }
    false
}

fn shares_same_as(thing1: &Thing, thing2: &Thing) -> bool {
    if thing1.same_as.is_empty() || thing2.same_as.is_empty() {
        return false;
    }
    let set1: std::collections::BTreeSet<String> = thing1
        .same_as
        .iter()
        .map(|s| Normalizer::normalize_url(s))
        .collect();
    for s in &thing2.same_as {
        if set1.contains(&Normalizer::normalize_url(s)) {
            return true;
        }
    }
    false
}

fn same_canonical_url(thing1: &Thing, thing2: &Thing) -> bool {
    let (Some(u1), Some(u2)) = (thing1.url.as_ref(), thing2.url.as_ref()) else {
        return false;
    };
    Normalizer::normalize_url(u1) == Normalizer::normalize_url(u2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Identifier;

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
        assert!((cfg.identifiers_weight - back.identifiers_weight).abs() < 1e-12);
        assert!(matches!(back.name_algorithm, SimilarityAlgorithm::Combined));
        assert_eq!(cfg.strict_mode, back.strict_mode);
    }

    #[test]
    fn config_partial_json_fills_missing_fields_from_default() {
        let partial = r#"{"match_threshold": 0.80, "name_weight": 0.5}"#;
        let cfg: MatchConfig = serde_json::from_str(partial).expect("partial json");
        assert!((cfg.match_threshold - 0.80).abs() < 1e-12);
        assert!((cfg.name_weight - 0.5).abs() < 1e-12);
        assert!(matches!(cfg.name_algorithm, SimilarityAlgorithm::Combined));
    }

    // ---------- probabilistic match ----------

    #[test]
    fn exact_clone_is_a_match() {
        let t = Thing::builder()
            .name("Eiffel Tower")
            .url("https://www.toureiffel.paris/")
            .build();
        let result = MatchingEngine::default_config().match_things(&t, &t.clone());
        assert!(result.is_match);
        assert!(result.score > 0.95);
    }

    #[test]
    fn name_match_takes_best_of_cartesian_product() {
        let t1 = Thing::builder().name("Eiffel Tower").build();
        let t2 = Thing::builder()
            .name("La Tour Eiffel")
            .add_alternate_name("Eiffel Tower")
            .build();
        let r = MatchingEngine::default_config().match_things(&t1, &t2);
        let s = r.breakdown.name_score.expect("scored");
        assert!(
            s > 0.99,
            "best-of cartesian product should pick exact match: {s}"
        );
    }

    #[test]
    fn unrelated_things_do_not_match() {
        let a = Thing::builder().name("Eiffel Tower").build();
        let b = Thing::builder().name("Sydney Opera House").build();
        let r = MatchingEngine::default_config().match_things(&a, &b);
        assert!(!r.is_match);
        assert!(r.score < 0.5);
    }

    #[test]
    fn no_overlapping_fields_returns_zero_score() {
        let a = Thing::builder().description("foo").build();
        let b = Thing::builder()
            .add_same_as("https://example.org/x")
            .build();
        let r = MatchingEngine::default_config().match_things(&a, &b);
        assert_eq!(r.score, 0.0);
    }

    // ---------- description / disambiguating_description ----------

    #[test]
    fn description_identical_scores_one() {
        let t1 = Thing::builder()
            .name("X")
            .description("Iron tower in Paris.")
            .build();
        let t2 = Thing::builder()
            .name("X")
            .description("Iron tower in Paris.")
            .build();
        let r = MatchingEngine::default_config().match_things(&t1, &t2);
        assert!(r.breakdown.description_score.unwrap() > 0.99);
    }

    #[test]
    fn description_score_none_when_either_missing() {
        let t1 = Thing::builder()
            .name("X")
            .description("Iron tower in Paris.")
            .build();
        let t2 = Thing::builder().name("X").build();
        let r = MatchingEngine::default_config().match_things(&t1, &t2);
        assert!(r.breakdown.description_score.is_none());
    }

    // ---------- identifiers ----------

    #[test]
    fn identifiers_shared_scores_one() {
        let id = Identifier::new("wikidata", "Q243").unwrap();
        let a = Thing::builder()
            .name("X")
            .add_identifier(id.clone())
            .build();
        let b = Thing::builder().name("X").add_identifier(id).build();
        let r = MatchingEngine::default_config().match_things(&a, &b);
        assert_eq!(r.breakdown.identifiers_score, Some(1.0));
    }

    #[test]
    fn identifiers_property_scoped_no_cross_match() {
        let a = Thing::builder()
            .name("X")
            .add_identifier(Identifier::new("google", "X").unwrap())
            .build();
        let b = Thing::builder()
            .name("X")
            .add_identifier(Identifier::new("wikidata", "X").unwrap())
            .build();
        let r = MatchingEngine::default_config().match_things(&a, &b);
        assert_eq!(r.breakdown.identifiers_score, Some(0.0));
    }

    #[test]
    fn identifiers_none_when_either_side_empty() {
        let a = Thing::builder().name("X").build();
        let b = Thing::builder()
            .name("X")
            .add_identifier(Identifier::new("wikidata", "Q1").unwrap())
            .build();
        let r = MatchingEngine::default_config().match_things(&a, &b);
        assert!(r.breakdown.identifiers_score.is_none());
    }

    // ---------- url ----------

    #[test]
    fn url_normalised_equality_scores_one() {
        let a = Thing::builder()
            .name("X")
            .url("HTTPS://Example.ORG/")
            .build();
        let b = Thing::builder()
            .name("X")
            .url("https://example.org")
            .build();
        let r = MatchingEngine::default_config().match_things(&a, &b);
        assert_eq!(r.breakdown.url_score, Some(1.0));
    }

    #[test]
    fn url_mismatch_scores_zero() {
        let a = Thing::builder().name("X").url("https://a.org").build();
        let b = Thing::builder().name("X").url("https://b.org").build();
        let r = MatchingEngine::default_config().match_things(&a, &b);
        assert_eq!(r.breakdown.url_score, Some(0.0));
    }

    #[test]
    fn url_none_when_either_side_missing() {
        let a = Thing::builder().name("X").url("https://a.org").build();
        let b = Thing::builder().name("X").build();
        let r = MatchingEngine::default_config().match_things(&a, &b);
        assert!(r.breakdown.url_score.is_none());
    }

    // ---------- sameAs / additional_types ----------

    #[test]
    fn same_as_jaccard_partial_overlap() {
        let a = Thing::builder()
            .name("X")
            .add_same_as("https://example.org/a")
            .add_same_as("https://example.org/b")
            .build();
        let b = Thing::builder()
            .name("X")
            .add_same_as("https://example.org/b")
            .add_same_as("https://example.org/c")
            .build();
        let r = MatchingEngine::default_config().match_things(&a, &b);
        let s = r.breakdown.same_as_score.expect("scored");
        // intersection {b}, union {a,b,c} => 1/3
        assert!((s - 1.0_f64 / 3.0).abs() < 1e-9, "got {s}");
    }

    #[test]
    fn same_as_none_when_both_empty() {
        let a = Thing::builder().name("X").build();
        let b = Thing::builder().name("X").build();
        let r = MatchingEngine::default_config().match_things(&a, &b);
        assert!(r.breakdown.same_as_score.is_none());
    }

    #[test]
    fn additional_types_jaccard_full_overlap() {
        let a = Thing::builder()
            .name("X")
            .add_additional_type("https://schema.org/Landmark")
            .build();
        let b = Thing::builder()
            .name("X")
            .add_additional_type("https://schema.org/Landmark")
            .build();
        let r = MatchingEngine::default_config().match_things(&a, &b);
        assert_eq!(r.breakdown.additional_types_score, Some(1.0));
    }

    // ---------- deterministic match ----------

    #[test]
    fn deterministic_via_shared_identifier() {
        let id = Identifier::new("wikidata", "Q243").unwrap();
        let a = Thing::builder()
            .name("Eiffel Tower")
            .add_identifier(id.clone())
            .build();
        let b = Thing::builder()
            .name("Wholly Different")
            .add_identifier(id)
            .build();
        assert!(MatchingEngine::default_config().deterministic_match(&a, &b));
    }

    #[test]
    fn deterministic_via_shared_same_as() {
        let a = Thing::builder()
            .name("Eiffel Tower")
            .add_same_as("https://www.wikidata.org/wiki/Q243")
            .build();
        let b = Thing::builder()
            .name("Tour Eiffel")
            .add_same_as("https://www.wikidata.org/wiki/Q243")
            .build();
        assert!(MatchingEngine::default_config().deterministic_match(&a, &b));
    }

    #[test]
    fn deterministic_via_shared_url() {
        let a = Thing::builder()
            .name("X")
            .url("https://example.org/")
            .build();
        let b = Thing::builder()
            .name("Y")
            .url("https://example.org")
            .build();
        assert!(MatchingEngine::default_config().deterministic_match(&a, &b));
    }

    #[test]
    fn deterministic_rejects_when_no_shared_identity_signal() {
        let a = Thing::builder().name("X").build();
        let b = Thing::builder().name("X").build();
        assert!(!MatchingEngine::default_config().deterministic_match(&a, &b));
    }

    // ---------- strict_mode enforcement ----------

    #[test]
    fn strict_mode_requires_deterministic_for_is_match() {
        let cfg = MatchConfig {
            match_threshold: 0.40,
            strict_mode: true,
            ..MatchConfig::default()
        };
        let t1 = Thing::builder().name("Cafe Centrale").build();
        let t2 = Thing::builder().name("Cafe Central").build();
        let engine = MatchingEngine::new(cfg);
        let r = engine.match_things(&t1, &t2);
        assert!(r.score >= 0.40, "should clear threshold");
        // No shared identifier / sameAs / url → not deterministic
        assert!(!engine.deterministic_match(&t1, &t2));
        assert!(!r.is_match);
    }

    // ---------- batch APIs ----------

    #[test]
    fn match_one_to_many_empty_candidates_yields_empty_vec() {
        let engine = MatchingEngine::default_config();
        let q = Thing::builder().name("Solo").build();
        assert!(engine.match_one_to_many(&q, &[]).is_empty());
    }

    #[test]
    fn rank_one_to_many_sorts_by_score_descending() {
        let engine = MatchingEngine::default_config();
        let q = Thing::builder().name("Eiffel Tower").build();
        let candidates = vec![
            Thing::builder().name("Big Ben").build(),
            q.clone(),
            Thing::builder().name("Statue of Liberty").build(),
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

    // ---------- phonetic ----------

    #[test]
    fn phonetic_score_none_when_off() {
        let t = Thing::builder().name("Stephen").build();
        let q = Thing::builder().name("Steven").build();
        let r = MatchingEngine::new(MatchConfig {
            use_phonetic_matching: false,
            ..MatchConfig::default()
        })
        .match_things(&t, &q);
        assert!(r.breakdown.name_phonetic_score.is_none());
    }

    #[test]
    fn phonetic_score_some_when_on() {
        let t = Thing::builder().name("Stephen").build();
        let q = Thing::builder().name("Steven").build();
        let r = MatchingEngine::new(MatchConfig {
            use_phonetic_matching: true,
            ..MatchConfig::default()
        })
        .match_things(&t, &q);
        assert!(r.breakdown.name_phonetic_score.is_some());
    }
}
