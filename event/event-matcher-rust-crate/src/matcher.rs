//! Event matcher engine: deterministic and probabilistic algorithms.
//!
//! This is the orchestration layer of the crate. It pulls together the data
//! types from [`crate::models`], the text transformations from
//! [`crate::normalizer`], and the similarity primitives from
//! [`crate::scorer`] to produce a single answer about whether two event
//! records refer to the same event.
//!
//! ## Two strategies, one engine
//!
//! - [`MatchingEngine::deterministic_match`] — fast, binary. Returns `true`
//!   iff the two events share any external event-ID pair, or share an
//!   exact normalised primary name plus the same normalised start date.
//! - [`MatchingEngine::match_events`] — weighted probabilistic scoring,
//!   returning a [`MatchResult`] with per-field [`MatchBreakdown`].
//!
//! ## Example
//!
//! ```
//! use event_matcher::{MatchingEngine, Event};
//!
//! let a = Event::builder()
//!     .name("Glastonbury Festival 2024")
//!     .start_date("2024-06-26T09:00:00Z")
//!     .build();
//!
//! let b = Event::builder()
//!     .name("Glasto 2024")
//!     .add_alternate_name("Glastonbury Festival 2024")
//!     .start_date("2024-06-26T09:15:00Z")
//!     .build();
//!
//! let engine = MatchingEngine::default_config();
//! let result = engine.match_events(&a, &b);
//! assert!(result.is_match);
//! ```

use crate::models::{Address, Event, Location, RelationKind, RelationshipRef};
use crate::normalizer::Normalizer;
use crate::scorer::{Scorer, SimilarityAlgorithm};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

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
/// use event_matcher::{MatchConfig, SimilarityAlgorithm};
///
/// let custom = MatchConfig {
///     match_threshold: 0.80,
///     name_weight: 0.20,
///     start_date_weight: 0.25,
///     start_date_scale_seconds: 3600.0,
///     end_date_weight: 0.05,
///     location_weight: 0.15,
///     coordinates_scale_metres: 100.0,
///     category_weight: 0.08,
///     country_code_weight: 0.04,
///     event_ids_weight: 0.15,
///     organizer_weight: 0.04,
///     performers_weight: 0.02,
///     url_weight: 0.02,
///     relationships_weight: 0.05,
///     tags_weight: 0.05,
///     use_phonetic_matching: true,
///     name_algorithm: SimilarityAlgorithm::Combined,
///     strict_mode: false,
/// };
/// assert_eq!(custom.match_threshold, 0.80);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MatchConfig {
    /// Threshold score for considering two events a match (`0.0..=1.0`).
    pub match_threshold: f64,

    /// Weight for name similarity (best-of cartesian product across the
    /// primary `name` and `alternate_names` on both sides).
    pub name_weight: f64,

    /// Weight for `start_date` similarity (Gaussian decay over absolute
    /// seconds difference).
    pub start_date_weight: f64,

    /// Time scale, in seconds, controlling the Gaussian decay of the
    /// `start_date` score. At a separation equal to `scale` the score is
    /// `1/e ~= 0.368`. Defaults to one hour (`3600.0`).
    pub start_date_scale_seconds: f64,

    /// Weight for `end_date` similarity (same Gaussian-decay shape as
    /// `start_date`).
    pub end_date_weight: f64,

    /// Weight for location similarity (weighted blend of venue name,
    /// address, and coordinates).
    pub location_weight: f64,

    /// Distance scale, in metres, controlling the Gaussian decay of the
    /// coordinates sub-score inside `location`. Defaults to `100.0`.
    pub coordinates_scale_metres: f64,

    /// Weight for [`EventCategory`](crate::models::EventCategory) equality
    /// (1.0 / 0.0 when both sides set).
    pub category_weight: f64,

    /// Weight for case-insensitive equality of
    /// `country_code_as_iso_3166_1_alpha_2`.
    pub country_code_weight: f64,

    /// Weight for "shared external event ID" (1.0 if any `(scheme, value)`
    /// pair is shared, 0.0 otherwise).
    pub event_ids_weight: f64,

    /// Weight for organiser-name similarity (Combined string similarity
    /// after name normalisation).
    pub organizer_weight: f64,

    /// Weight for performer-list similarity (best-of cartesian product
    /// after name normalisation).
    pub performers_weight: f64,

    /// Weight for canonical-URL exact match after trimming whitespace.
    pub url_weight: f64,

    /// Weight for relationship-set similarity: typed-set Jaccard over
    /// `(relation, event_id)` pairs (see [`crate::RelationshipRef`]).
    /// Defaults to `0.05` — a **supporting** signal only: two records
    /// referencing the same related events are weakly more likely the
    /// same event, but the field never identifies on its own and does
    /// not participate when either side has no relationships recorded.
    /// See spec §6.11.
    pub relationships_weight: f64,

    /// Weight for tag-set similarity: set Jaccard over the
    /// case-insensitively normalised tag sets. Defaults to `0.05` — a
    /// **supporting** signal only, analogous to
    /// [`Self::relationships_weight`]: two records sharing the same
    /// operator-applied tags are weakly more likely the same event, but
    /// does not participate when either side has no tags recorded. See
    /// spec §6.12.
    pub tags_weight: f64,

    /// Whether to add a phonetic-name bonus when both names sound alike.
    pub use_phonetic_matching: bool,

    /// Similarity algorithm to use when comparing names.
    pub name_algorithm: SimilarityAlgorithm,

    /// Reserved flag for stricter deterministic enforcement. When `true`,
    /// `is_match` requires both a probabilistic score above the threshold
    /// *and* a deterministic match.
    pub strict_mode: bool,
}

impl Default for MatchConfig {
    /// Production-ready defaults.
    ///
    /// ```
    /// use event_matcher::{MatchConfig, SimilarityAlgorithm};
    /// let c = MatchConfig::default();
    /// assert!((c.match_threshold - 0.80).abs() < 1e-9);
    /// assert!(matches!(c.name_algorithm, SimilarityAlgorithm::Combined));
    /// ```
    fn default() -> Self {
        Self {
            match_threshold: 0.80,
            name_weight: 0.20,
            start_date_weight: 0.25,
            start_date_scale_seconds: 3600.0,
            end_date_weight: 0.05,
            location_weight: 0.15,
            coordinates_scale_metres: 100.0,
            category_weight: 0.08,
            country_code_weight: 0.04,
            event_ids_weight: 0.15,
            organizer_weight: 0.04,
            performers_weight: 0.02,
            url_weight: 0.02,
            relationships_weight: 0.05,
            tags_weight: 0.05,
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
    /// use event_matcher::MatchConfig;
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
    /// use event_matcher::MatchConfig;
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
/// raw float.
///
/// Boundaries:
///
/// | Score range | Band |
/// |---|---|
/// | `score >= 0.90` | `High` |
/// | `0.75 <= score < 0.90` | `Medium` |
/// | `score < 0.75` | `Low` |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Confidence {
    /// Score is at or above `0.90`. Strong match.
    High,
    /// Score is in `0.75..0.90`. Medium-confidence match.
    Medium,
    /// Score is below `0.75`. Candidate at best.
    Low,
}

impl Confidence {
    /// Bucket a probabilistic score into one of the three bands.
    ///
    /// NaN inputs and negatives degrade to `Low`; scores above `1.0` are
    /// treated as `High`.
    ///
    /// ```
    /// use event_matcher::Confidence;
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

/// Outcome of a probabilistic event match.
///
/// Contains the overall renormalised `score`, the threshold-derived
/// `is_match` boolean, a coarse [`Confidence`] band, and a per-field
/// [`MatchBreakdown`] for audit.
///
/// `MatchResult` implements `Serialize + Deserialize` so it can be persisted
/// or returned over an API.
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

/// Serde default for [`MatchResult::confidence`].
///
/// Used by `#[serde(default = "default_confidence")]` so that JSON
/// payloads written before the `confidence` field existed deserialise
/// to the most conservative band ([`Confidence::Low`]) instead of
/// failing. New results always carry a freshly-computed band.
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchBreakdown {
    /// Best-of-cartesian-product similarity across primary name +
    /// alternate names on both sides, using the configured algorithm.
    pub name_score: Option<f64>,
    /// Maximum Soundex match across the same name pairs. `None` when
    /// `use_phonetic_matching` is false or either side has no names.
    pub name_phonetic_score: Option<f64>,
    /// Gaussian-decay score over the absolute seconds difference between
    /// the two `start_date` values. `None` if either is missing or fails
    /// to parse as ISO 8601.
    pub start_date_score: Option<f64>,
    /// Gaussian-decay score over the absolute seconds difference between
    /// the two `end_date` values. `None` if either is missing or fails
    /// to parse as ISO 8601.
    pub end_date_score: Option<f64>,
    /// Weighted blend of venue-name similarity, address similarity, and
    /// coordinates similarity. `None` if either side has no location.
    pub location_score: Option<f64>,
    /// `1.0` if both categories set and structurally equal; `0.0` if both
    /// set but differ; `None` if either is `None`.
    pub category_score: Option<f64>,
    /// `1.0` if both country codes set and equal after trim + ASCII
    /// lowercase; `0.0` otherwise; `None` if either is `None`.
    pub country_code_score: Option<f64>,
    /// `1.0` if both `event_ids` non-empty and they share any
    /// `(scheme, value)` pair; `0.0` if both non-empty but none shared;
    /// `None` if either side is empty.
    pub event_ids_score: Option<f64>,
    /// Combined string similarity for the organiser, after name
    /// normalisation. `None` if either side is absent.
    pub organizer_score: Option<f64>,
    /// Best-of cartesian product across performer lists, after name
    /// normalisation. `None` if either side has no performers.
    pub performers_score: Option<f64>,
    /// `1.0` if both URLs set and equal after trimming, else `0.0`. `None`
    /// if either is absent.
    pub url_score: Option<f64>,
    /// Score for relationship-set similarity: typed-set Jaccard over
    /// `(relation, event_id)` pairs, `|A ∩ B| / |A ∪ B|`. `None` when
    /// either side has no relationships recorded. See
    /// [`crate::RelationshipRef`]; spec §6.11.
    #[serde(default)]
    pub relationships_score: Option<f64>,
    /// Score for tag-set similarity: set Jaccard over the
    /// case-insensitively normalised tag sets, `|A ∩ B| / |A ∪ B|`.
    /// `None` when either side has no tags recorded. Spec §6.12.
    #[serde(default)]
    pub tags_score: Option<f64>,
}

/// Event matcher engine.
///
/// The engine is **immutable after construction** and cheap to clone (it
/// owns only a [`MatchConfig`]). Construct one and call its methods from any
/// thread.
///
/// ```
/// use event_matcher::{MatchConfig, MatchingEngine};
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
    #[must_use]
    pub fn new(config: MatchConfig) -> Self {
        Self { config }
    }

    /// Construct an engine with [`MatchConfig::default`].
    #[must_use]
    pub fn default_config() -> Self {
        Self::new(MatchConfig::default())
    }

    /// Compare two events probabilistically and return a [`MatchResult`].
    ///
    /// The score is the weight-renormalised sum of every component that
    /// scored on both records. Missing fields are skipped, not penalised.
    ///
    /// ```
    /// use event_matcher::{MatchingEngine, Event};
    ///
    /// let e = Event::builder()
    ///     .name("RustConf 2024")
    ///     .start_date("2024-09-10T09:00:00Z")
    ///     .build();
    ///
    /// let result = MatchingEngine::default_config().match_events(&e, &e);
    /// assert!(result.is_match);
    /// assert!(result.score > 0.99);
    /// ```
    #[must_use]
    pub fn match_events(&self, event1: &Event, event2: &Event) -> MatchResult {
        let breakdown = self.calculate_breakdown(event1, event2);
        let score = self.calculate_weighted_score(&breakdown);
        let above_threshold = score >= self.config.match_threshold;
        let is_match = if self.config.strict_mode {
            above_threshold && self.deterministic_match(event1, event2)
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
    /// ```
    /// use event_matcher::{MatchingEngine, Event};
    ///
    /// let query = Event::builder().name("RustConf 2024").build();
    /// let candidates = vec![
    ///     Event::builder().name("RustConf 2024").build(),
    ///     Event::builder().name("GoConf 2024").build(),
    /// ];
    ///
    /// let results = MatchingEngine::default_config().match_one_to_many(&query, &candidates);
    /// assert_eq!(results.len(), 2);
    /// assert!(results[0].is_match);
    /// assert!(!results[1].is_match);
    /// ```
    #[must_use]
    pub fn match_one_to_many(&self, query: &Event, candidates: &[Event]) -> Vec<MatchResult> {
        candidates
            .iter()
            .map(|c| self.match_events(query, c))
            .collect()
    }

    /// Score and rank: return `(original_index, MatchResult)` tuples
    /// sorted by descending score. Ties are broken by ascending original
    /// index, so the result is deterministic.
    #[must_use]
    pub fn rank_one_to_many(
        &self,
        query: &Event,
        candidates: &[Event],
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

    /// Compare two events deterministically and return a single boolean.
    ///
    /// Returns `true` iff either:
    ///
    /// - the events share any `(scheme, value)` pair in their `event_ids`
    ///   lists, OR
    /// - both have a primary `name` that normalises to the same value AND
    ///   both have a `start_date` that parses to the same instant.
    ///
    /// ```
    /// use event_matcher::{MatchingEngine, Event, EventId, EventIdScheme};
    ///
    /// let id = EventId::new(EventIdScheme::Eventbrite, "123456789").unwrap();
    /// let a = Event::builder().name("RustConf 2024").add_event_id(id.clone()).build();
    /// let b = Event::builder().name("RC '24").add_event_id(id).build();
    /// assert!(MatchingEngine::default_config().deterministic_match(&a, &b));
    /// ```
    #[must_use]
    pub fn deterministic_match(&self, event1: &Event, event2: &Event) -> bool {
        if shares_event_id(event1, event2) {
            return true;
        }
        name_and_start_date_match(event1, event2)
    }

    /// Score every field of the two events and assemble the per-field
    /// [`MatchBreakdown`].
    ///
    /// Each component scorer returns `Some(score)` when both sides supplied
    /// the field, or `None` when either side is missing (so the field is
    /// later skipped by [`MatchingEngine::calculate_weighted_score`] rather
    /// than counted as a zero). The phonetic name score is only computed
    /// when `use_phonetic_matching` is enabled, since it is a pure bonus.
    fn calculate_breakdown(&self, event1: &Event, event2: &Event) -> MatchBreakdown {
        MatchBreakdown {
            name_score: self.score_name(event1, event2),
            name_phonetic_score: if self.config.use_phonetic_matching {
                Self::score_phonetic_names(event1, event2)
            } else {
                None
            },
            start_date_score: self.score_start_date(event1, event2),
            end_date_score: self.score_end_date(event1, event2),
            location_score: self.score_location(event1, event2),
            category_score: score_category(event1, event2),
            country_code_score: score_country_code(event1, event2),
            event_ids_score: score_event_ids(event1, event2),
            organizer_score: Self::score_organizer(event1, event2),
            performers_score: Self::score_performers(event1, event2),
            url_score: score_url(event1, event2),
            relationships_score: score_relationships(&event1.relationships, &event2.relationships),
            tags_score: score_tags(&event1.tags, &event2.tags),
        }
    }

    /// Collapse a [`MatchBreakdown`] into a single overall score in
    /// `[0.0, 1.0]`.
    ///
    /// The score is a **weight-renormalised** average: each field that was
    /// actually scored (`Some`) contributes `score * weight` to the
    /// numerator and `weight` to the denominator; fields that were missing
    /// (`None`) contribute nothing to either. Dividing by the sum of
    /// *participating* weights — rather than the sum of all configured
    /// weights — is what lets missing data neither reward nor penalise the
    /// pair. When no field participated (the denominator is zero) the score
    /// is `0.0`.
    fn calculate_weighted_score(&self, breakdown: &MatchBreakdown) -> f64 {
        let mut total_weight = 0.0;
        let mut weighted_sum = 0.0;

        // Fold one optional component into the running numerator/denominator.
        // A `None` component is silently skipped so it does not dilute the
        // average toward zero.
        let mut accumulate = |opt: Option<f64>, weight: f64| {
            if let Some(score) = opt {
                weighted_sum += score * weight;
                total_weight += weight;
            }
        };

        accumulate(breakdown.name_score, self.config.name_weight);
        accumulate(breakdown.start_date_score, self.config.start_date_weight);
        accumulate(breakdown.end_date_score, self.config.end_date_weight);
        accumulate(breakdown.location_score, self.config.location_weight);
        accumulate(breakdown.category_score, self.config.category_weight);
        accumulate(
            breakdown.country_code_score,
            self.config.country_code_weight,
        );
        accumulate(breakdown.event_ids_score, self.config.event_ids_weight);
        accumulate(breakdown.organizer_score, self.config.organizer_weight);
        accumulate(breakdown.performers_score, self.config.performers_weight);
        accumulate(breakdown.url_score, self.config.url_weight);
        accumulate(
            breakdown.relationships_score,
            self.config.relationships_weight,
        );
        accumulate(breakdown.tags_score, self.config.tags_weight);

        // Phonetic match is a bonus only — never lowers the score. It is
        // added with a deliberately small weight (`0.05`) so a phonetic
        // agreement nudges a borderline pair upward without dominating the
        // primary fields. The `> 0.9` gate means only a near-perfect
        // Soundex agreement counts; weak phonetic signals are ignored.
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

    /// Score the two events' names as the **best-of cartesian product**
    /// across `name` + `alternate_names` on each side.
    ///
    /// Comparing every name on side one against every name on side two and
    /// keeping the maximum means an alias on either record can rescue a
    /// match (e.g. `"Glasto 2024"` vs the alternate name
    /// `"Glastonbury Festival 2024"`). Returns `None` when either side has
    /// no usable (non-blank) names, so the field is skipped entirely.
    fn score_name(&self, e1: &Event, e2: &Event) -> Option<f64> {
        let names1 = collect_names(e1);
        let names2 = collect_names(e2);
        if names1.is_empty() || names2.is_empty() {
            return None;
        }
        // Seed with negative infinity so the first real comparison always
        // wins; every produced similarity is in [0.0, 1.0].
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

    /// Score a single pair of raw name strings.
    ///
    /// Both names are first run through [`Normalizer::normalize_name`]
    /// (case-fold, drop diacritics and punctuation, collapse whitespace)
    /// so the comparison is robust to cosmetic differences, then scored
    /// with whichever [`SimilarityAlgorithm`] the config selects.
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

    /// Phonetic (Soundex) agreement across the name cartesian product.
    ///
    /// Returns `Some(1.0)` if any name on side one shares a non-empty
    /// Soundex code with any name on side two, otherwise `Some(0.0)`;
    /// `None` when either side has no usable names. This is a binary
    /// signal — it catches "sounds-alike" spelling variants (Stephen /
    /// Steven) that string similarity alone might rate too low — and is
    /// only consumed as a small bonus in
    /// [`MatchingEngine::calculate_weighted_score`].
    ///
    /// Empty codes are skipped so two unrelated names that both encode to
    /// the empty string do not spuriously "match".
    fn score_phonetic_names(e1: &Event, e2: &Event) -> Option<f64> {
        let names1 = collect_names(e1);
        let names2 = collect_names(e2);
        if names1.is_empty() || names2.is_empty() {
            return None;
        }
        // Pre-encode each side once, then cross-compare the codes.
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
                // Require a non-empty code so two name-less / unencodable
                // inputs do not collide on the empty string.
                if !c1.is_empty() && c1 == c2 {
                    best = 1.0;
                }
            }
        }
        Some(best)
    }

    /// Score the two `start_date` values by Gaussian decay over their
    /// absolute difference in seconds.
    ///
    /// Returns `None` if either date is absent or unparseable (the `?`
    /// short-circuits), so an event with no start date neither helps nor
    /// hurts. The decay scale comes from `start_date_scale_seconds`
    /// (default one hour), so events starting within an hour of each other
    /// still score high while events days apart score near zero.
    fn score_start_date(&self, e1: &Event, e2: &Event) -> Option<f64> {
        let d = Scorer::seconds_between(e1.start_date.as_deref()?, e2.start_date.as_deref()?)?;
        // `d` is a non-negative seconds magnitude. `u32::MAX` seconds is
        // ~136 years, exact through the conversion for every realistic
        // event gap; absurd gaps saturate and already score ~0.
        let d = f64::from(u32::try_from(d).unwrap_or(u32::MAX));
        Some(Scorer::start_date_score(
            d,
            self.config.start_date_scale_seconds,
        ))
    }

    /// Score the two `end_date` values by Gaussian decay over their
    /// absolute difference in seconds.
    ///
    /// Identical in shape to [`MatchingEngine::score_start_date`] and
    /// deliberately reuses the same `start_date_scale_seconds` scale and
    /// the [`Scorer::start_date_score`] kernel, since the end of an event
    /// is just another point on the same timeline. Returns `None` when
    /// either end date is missing or unparseable.
    fn score_end_date(&self, e1: &Event, e2: &Event) -> Option<f64> {
        let d = Scorer::seconds_between(e1.end_date.as_deref()?, e2.end_date.as_deref()?)?;
        // Same saturating-lossless seconds magnitude conversion as
        // `score_start_date`.
        let d = f64::from(u32::try_from(d).unwrap_or(u32::MAX));
        Some(Scorer::start_date_score(
            d,
            self.config.start_date_scale_seconds,
        ))
    }

    /// Score the two events' locations, or `None` if either lacks one.
    ///
    /// Delegates to [`MatchingEngine::compare_locations`] only when both
    /// sides carry a [`Location`]; otherwise the field is skipped.
    fn score_location(&self, e1: &Event, e2: &Event) -> Option<f64> {
        match (e1.location.as_ref(), e2.location.as_ref()) {
            (Some(l1), Some(l2)) => Some(self.compare_locations(l1, l2)),
            _ => None,
        }
    }

    /// Compare two [`Location`] values into a single `[0.0, 1.0]` score.
    ///
    /// The score is a weight-renormalised blend over whichever sub-fields
    /// both locations share: coordinates (weight `0.5`), postal address
    /// (`0.3`), venue name (`0.15`), and virtual join URL (`0.05`).
    /// Coordinates dominate because exact geo agreement is the strongest
    /// "same place" evidence; the virtual URL contributes least because it
    /// is an exact string equality that rarely disambiguates. When the two
    /// locations share no comparable sub-field the function returns the
    /// neutral `0.5` (neither evidence for nor against).
    fn compare_locations(&self, l1: &Location, l2: &Location) -> f64 {
        // Each sub-component contributes a raw score in `[0.0, 1.0]` and a
        // weight. Final score is the weight-renormalised average across
        // sub-components that fired. Coordinates dominate (`0.5`), then
        // address (`0.3`), then venue name (`0.15`), then virtual URL
        // (`0.05`).
        let mut weighted_sum = 0.0_f64;
        let mut total_weight = 0.0_f64;

        if let (Some(lat1), Some(lon1), Some(lat2), Some(lon2)) = (
            l1.latitude_as_decimal_degrees,
            l1.longitude_as_decimal_degrees,
            l2.latitude_as_decimal_degrees,
            l2.longitude_as_decimal_degrees,
        ) && let (Some((la1, lo1)), Some((la2, lo2))) = (
            valid_coords(Some(lat1), Some(lon1)),
            valid_coords(Some(lat2), Some(lon2)),
        ) {
            let d = Scorer::haversine_metres(la1, lo1, la2, lo2);
            weighted_sum +=
                Scorer::coordinates_score(d, self.config.coordinates_scale_metres) * 0.5;
            total_weight += 0.5;
        }

        if let (Some(a1), Some(a2)) = (l1.address.as_ref(), l2.address.as_ref()) {
            weighted_sum += compare_addresses(a1, a2) * 0.3;
            total_weight += 0.3;
        }

        if let (Some(v1), Some(v2)) = (l1.venue_name.as_deref(), l2.venue_name.as_deref()) {
            let n1 = Normalizer::normalize_name(v1);
            let n2 = Normalizer::normalize_name(v2);
            weighted_sum += Scorer::combined_similarity(&n1, &n2) * 0.15;
            total_weight += 0.15;
        }

        if let (Some(u1), Some(u2)) = (l1.virtual_url.as_deref(), l2.virtual_url.as_deref()) {
            weighted_sum += f64::from(u1.trim() == u2.trim()) * 0.05;
            total_weight += 0.05;
        }

        if total_weight == 0.0 {
            0.5
        } else {
            weighted_sum / total_weight
        }
    }

    /// Score the two organiser names with [`Scorer::combined_similarity`]
    /// after name normalisation, or `None` if either organiser is absent.
    ///
    /// Organiser is a single string (not a list), so this is a plain
    /// pairwise comparison rather than a cartesian product.
    fn score_organizer(e1: &Event, e2: &Event) -> Option<f64> {
        let o1 = e1.organizer.as_deref()?;
        let o2 = e2.organizer.as_deref()?;
        let n1 = Normalizer::normalize_name(o1);
        let n2 = Normalizer::normalize_name(o2);
        Some(Scorer::combined_similarity(&n1, &n2))
    }

    /// Score the two performer lists as the best-of cartesian product of
    /// normalised, [`Scorer::combined_similarity`]-compared names.
    ///
    /// Returns `None` when either list is empty. Taking the maximum over
    /// all pairs means a single shared headliner (in any list position) is
    /// enough to score the field high, which suits line-ups that overlap
    /// only partially or are listed in a different order.
    fn score_performers(e1: &Event, e2: &Event) -> Option<f64> {
        if e1.performers.is_empty() || e2.performers.is_empty() {
            return None;
        }
        let mut best = 0.0_f64;
        for a in &e1.performers {
            for b in &e2.performers {
                let na = Normalizer::normalize_name(a);
                let nb = Normalizer::normalize_name(b);
                let s = Scorer::combined_similarity(&na, &nb);
                if s > best {
                    best = s;
                }
            }
        }
        Some(best)
    }
}

// ---- Free helpers ------------------------------------------------------

/// Collect an event's primary name plus alternate names into a single vec
/// of references. Empty / whitespace-only strings are skipped.
fn collect_names(event: &Event) -> Vec<&String> {
    event
        .name
        .iter()
        .chain(event.alternate_names.iter())
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

/// Binary category equality: `Some(1.0)` if both categories are set and
/// structurally equal, `Some(0.0)` if both are set but differ, `None` if
/// either is absent.
///
/// Category is a coarse, controlled-vocabulary field, so fuzzy matching
/// would add noise; exact equality is the right comparison here.
fn score_category(e1: &Event, e2: &Event) -> Option<f64> {
    match (&e1.category, &e2.category) {
        (Some(a), Some(b)) => Some(if a == b { 1.0 } else { 0.0 }),
        _ => None,
    }
}

/// Binary country-code equality after trimming and ASCII-lowercasing both
/// sides; `None` if either code is absent.
///
/// ISO 3166-1 alpha-2 codes are a fixed two-letter vocabulary, so a
/// case-insensitive exact match (e.g. `"gb"` == `"GB"`) is appropriate
/// rather than string similarity.
fn score_country_code(e1: &Event, e2: &Event) -> Option<f64> {
    let a = e1.country_code_as_iso_3166_1_alpha_2.as_ref()?;
    let b = e2.country_code_as_iso_3166_1_alpha_2.as_ref()?;
    let na = a.trim().to_ascii_lowercase();
    let nb = b.trim().to_ascii_lowercase();
    Some(if na == nb { 1.0 } else { 0.0 })
}

/// Return `true` if the two events share any identical `(scheme, value)`
/// [`EventId`](crate::models::EventId) pair.
///
/// [`EventId`](crate::models::EventId) equality is scheme-scoped, so an
/// Eventbrite id and a Meetup id with the same string value never collide.
/// Returns `false` immediately if either list is empty (nothing to share).
fn shares_event_id(e1: &Event, e2: &Event) -> bool {
    if e1.event_ids.is_empty() || e2.event_ids.is_empty() {
        return false;
    }
    // Nested loops are fine: external-id lists are tiny in practice.
    for id1 in &e1.event_ids {
        for id2 in &e2.event_ids {
            if id1 == id2 {
                return true;
            }
        }
    }
    false
}

/// Probabilistic-pipeline view of [`shares_event_id`]: `Some(1.0)` when a
/// pair is shared, `Some(0.0)` when both lists are non-empty but disjoint,
/// `None` when either list is empty (so the field is skipped).
fn score_event_ids(e1: &Event, e2: &Event) -> Option<f64> {
    if e1.event_ids.is_empty() || e2.event_ids.is_empty() {
        return None;
    }
    Some(if shares_event_id(e1, e2) { 1.0 } else { 0.0 })
}

/// Binary URL equality after trimming surrounding whitespace; `None` if
/// either URL is absent.
///
/// URLs are compared exactly (no scheme/host canonicalisation): two
/// differently-written URLs for the same page score `0.0`, which is the
/// conservative choice for an explainable matcher.
fn score_url(e1: &Event, e2: &Event) -> Option<f64> {
    let u1 = e1.url.as_deref()?;
    let u2 = e2.url.as_deref()?;
    Some(f64::from(u1.trim() == u2.trim()))
}

/// Score a pair of relationship-reference lists for the probabilistic
/// breakdown: typed-set **Jaccard** over `(relation, event_id)` pairs —
/// `|A ∩ B| / |A ∪ B|` — so an `Outer` reference only agrees with an
/// `Outer` reference to the **same** event id. Returns `None` if either
/// side has no relationships recorded at all (the field is irrelevant
/// for this pair, not evidence of non-match). See spec §6.11.
fn score_relationships(a: &[RelationshipRef], b: &[RelationshipRef]) -> Option<f64> {
    if a.is_empty() || b.is_empty() {
        return None;
    }
    let set_a: HashSet<(RelationKind, &str)> = a
        .iter()
        .map(|r| (r.relation, r.event_id.as_str()))
        .collect();
    let set_b: HashSet<(RelationKind, &str)> = b
        .iter()
        .map(|r| (r.relation, r.event_id.as_str()))
        .collect();
    Some(jaccard(&set_a, &set_b))
}

/// Score a pair of tag lists for the probabilistic breakdown: set
/// **Jaccard** over the case-insensitively normalised tag sets —
/// `|A ∩ B| / |A ∪ B|`. Normalisation happens here, at scoring time,
/// consistent with the crate's verbatim-storage convention for names /
/// organizer / performers — [`crate::Event::tags`] stores tags exactly
/// as provided. Returns `None` if either side has no tags recorded at
/// all. See spec §6.12.
fn score_tags(a: &[String], b: &[String]) -> Option<f64> {
    if a.is_empty() || b.is_empty() {
        return None;
    }
    let set_a: HashSet<String> = a.iter().map(|t| t.to_lowercase()).collect();
    let set_b: HashSet<String> = b.iter().map(|t| t.to_lowercase()).collect();
    Some(jaccard(&set_a, &set_b))
}

/// Jaccard index `|A ∩ B| / |A ∪ B|` over two hash sets. Callers are
/// required to have already checked both inputs are non-empty (see
/// [`score_relationships`] / [`score_tags`]), so the union here is
/// never zero-sized.
///
/// Set sizes in practice are small (an event's relationship or tag
/// list), so the `usize` counts are routed through `u32` (exact,
/// lint-free `u32 -> f64`) rather than a direct `usize -> f64` cast;
/// `unwrap_or(u32::MAX)` is a non-panicking saturating fallback for a
/// pathologically large set rather than a realistic code path.
fn jaccard<T: Eq + std::hash::Hash>(a: &HashSet<T>, b: &HashSet<T>) -> f64 {
    let intersection = u32::try_from(a.intersection(b).count()).unwrap_or(u32::MAX);
    let union = u32::try_from(a.union(b).count()).unwrap_or(u32::MAX);
    f64::from(intersection) / f64::from(union)
}

/// Deterministic rule: do both events share an identical normalised
/// primary `name` **and** a `start_date` that parses to the same Unix
/// instant?
///
/// Both conditions are required. Names are compared after
/// [`Normalizer::normalize_name`]; start dates are compared as parsed Unix
/// seconds so that differently-offset ISO 8601 strings denoting the same
/// instant (e.g. `…T09:00:00Z` and `…T11:00:00+02:00`) count as equal.
/// Any missing or unparseable component yields `false`.
fn name_and_start_date_match(e1: &Event, e2: &Event) -> bool {
    let (Some(n1), Some(n2)) = (&e1.name, &e2.name) else {
        return false;
    };
    let norm1 = Normalizer::normalize_name(n1);
    let norm2 = Normalizer::normalize_name(n2);
    // Guard against empty-name matches (SEC-M2): a name that normalises to an
    // empty string (e.g. "###", "  ") must not satisfy the name leg, else two
    // unrelated events could deterministically match on an empty name.
    if norm1.is_empty() || norm1 != norm2 {
        return false;
    }
    let (Some(sd1), Some(sd2)) = (&e1.start_date, &e2.start_date) else {
        return false;
    };
    match (
        Normalizer::parse_iso8601_unix_seconds(sd1),
        Normalizer::parse_iso8601_unix_seconds(sd2),
    ) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

/// Compare two postal addresses; same blend rule as the previous
/// place-matcher implementation: postcode dominates (0.5), then city
/// (0.3), then line 1 (0.2).
fn compare_addresses(addr1: &Address, addr2: &Address) -> f64 {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{EventCategory, EventId, EventIdScheme};

    /// Approximate float equality for assertions on sentinel scores.
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
        assert!((cfg.start_date_weight - back.start_date_weight).abs() < 1e-12);
        assert!(matches!(back.name_algorithm, SimilarityAlgorithm::Combined));
        assert_eq!(cfg.strict_mode, back.strict_mode);
    }

    #[test]
    fn config_partial_json_fills_missing_fields_from_default() {
        let partial = r#"{"match_threshold": 0.80}"#;
        let cfg: MatchConfig = serde_json::from_str(partial).expect("partial json");
        assert!((cfg.match_threshold - 0.80).abs() < 1e-12);
        assert!(matches!(cfg.name_algorithm, SimilarityAlgorithm::Combined));
    }

    // ---------- probabilistic match ----------

    #[test]
    fn exact_clone_is_a_match() {
        let e = Event::builder()
            .name("RustConf 2024")
            .start_date("2024-09-10T09:00:00Z")
            .build();
        let result = MatchingEngine::default_config().match_events(&e, &e.clone());
        assert!(result.is_match);
        assert!(result.score > 0.95);
    }

    #[test]
    fn name_match_takes_best_of_cartesian_product() {
        let p1 = Event::builder().name("RustConf 2024").build();
        let p2 = Event::builder()
            .name("Rust Conference 2024")
            .add_alternate_name("RustConf 2024")
            .build();
        let r = MatchingEngine::default_config().match_events(&p1, &p2);
        let s = r.breakdown.name_score.expect("scored");
        assert!(s > 0.99, "got {s}");
    }

    #[test]
    fn unrelated_events_do_not_match() {
        let a = Event::builder()
            .name("RustConf 2024")
            .start_date("2024-09-10T09:00:00Z")
            .build();
        let b = Event::builder()
            .name("Sydney Opera Concert")
            .start_date("2025-03-15T20:00:00Z")
            .build();
        let r = MatchingEngine::default_config().match_events(&a, &b);
        assert!(!r.is_match);
        assert!(r.score < 0.5);
    }

    #[test]
    fn no_overlapping_fields_returns_zero_score() {
        let a = Event::builder().url("https://example.org/a").build();
        let b = Event::builder().url("https://example.org/b").build();
        let r = MatchingEngine::default_config().match_events(&a, &b);
        assert!(approx_eq(r.score, 0.0));
    }

    // ---------- start_date ----------

    #[test]
    fn start_date_score_one_when_identical() {
        let a = Event::builder()
            .name("X")
            .start_date("2024-06-26T09:00:00Z")
            .build();
        let b = a.clone();
        let r = MatchingEngine::default_config().match_events(&a, &b);
        assert!((r.breakdown.start_date_score.unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn start_date_score_decays_with_time_gap() {
        let a = Event::builder()
            .name("X")
            .start_date("2024-06-26T09:00:00Z")
            .build();
        let b = Event::builder()
            .name("X")
            .start_date("2024-07-26T09:00:00Z")
            .build();
        let r = MatchingEngine::default_config().match_events(&a, &b);
        assert!(r.breakdown.start_date_score.unwrap() < 1e-3);
    }

    #[test]
    fn start_date_score_none_when_one_side_missing() {
        let a = Event::builder().name("X").start_date("2024-06-26").build();
        let b = Event::builder().name("X").build();
        let r = MatchingEngine::default_config().match_events(&a, &b);
        assert!(r.breakdown.start_date_score.is_none());
    }

    #[test]
    fn start_date_score_none_when_garbage() {
        let a = Event::builder().name("X").start_date("not-a-date").build();
        let b = Event::builder().name("X").start_date("2024-06-26").build();
        let r = MatchingEngine::default_config().match_events(&a, &b);
        assert!(r.breakdown.start_date_score.is_none());
    }

    // ---------- category ----------

    #[test]
    fn category_equality_scores_one_else_zero() {
        let a = Event::builder()
            .name("X")
            .category(EventCategory::MusicEvent)
            .build();
        let b = Event::builder()
            .name("X")
            .category(EventCategory::MusicEvent)
            .build();
        let c = Event::builder()
            .name("X")
            .category(EventCategory::ComedyEvent)
            .build();
        let engine = MatchingEngine::default_config();
        assert!(approx_eq(
            engine
                .match_events(&a, &b)
                .breakdown
                .category_score
                .unwrap(),
            1.0
        ));
        assert!(approx_eq(
            engine
                .match_events(&a, &c)
                .breakdown
                .category_score
                .unwrap(),
            0.0
        ));
    }

    #[test]
    fn category_score_none_when_either_missing() {
        let a = Event::builder()
            .name("X")
            .category(EventCategory::MusicEvent)
            .build();
        let b = Event::builder().name("X").build();
        let r = MatchingEngine::default_config().match_events(&a, &b);
        assert!(r.breakdown.category_score.is_none());
    }

    // ---------- country code ----------

    #[test]
    fn country_code_case_insensitive_equality() {
        let a = Event::builder()
            .name("X")
            .country_code_as_iso_3166_1_alpha_2("gb")
            .build();
        let b = Event::builder()
            .name("X")
            .country_code_as_iso_3166_1_alpha_2("GB")
            .build();
        let r = MatchingEngine::default_config().match_events(&a, &b);
        assert!(approx_eq(r.breakdown.country_code_score.unwrap(), 1.0));
    }

    #[test]
    fn country_code_mismatch_scores_zero() {
        let a = Event::builder()
            .name("X")
            .country_code_as_iso_3166_1_alpha_2("GB")
            .build();
        let b = Event::builder()
            .name("X")
            .country_code_as_iso_3166_1_alpha_2("FR")
            .build();
        let r = MatchingEngine::default_config().match_events(&a, &b);
        assert!(approx_eq(r.breakdown.country_code_score.unwrap(), 0.0));
    }

    // ---------- event_ids ----------

    #[test]
    fn event_ids_shared_scores_one() {
        let id = EventId::new(EventIdScheme::Eventbrite, "12345").unwrap();
        let a = Event::builder().name("X").add_event_id(id.clone()).build();
        let b = Event::builder().name("X").add_event_id(id).build();
        let r = MatchingEngine::default_config().match_events(&a, &b);
        assert!(approx_eq(r.breakdown.event_ids_score.unwrap(), 1.0));
    }

    #[test]
    fn event_ids_scheme_scoped_no_cross_match() {
        let a = Event::builder()
            .name("X")
            .add_event_id(EventId::new(EventIdScheme::Eventbrite, "X").unwrap())
            .build();
        let b = Event::builder()
            .name("X")
            .add_event_id(EventId::new(EventIdScheme::Meetup, "X").unwrap())
            .build();
        let r = MatchingEngine::default_config().match_events(&a, &b);
        assert!(approx_eq(r.breakdown.event_ids_score.unwrap(), 0.0));
    }

    #[test]
    fn event_ids_none_when_either_side_empty() {
        let a = Event::builder().name("X").build();
        let b = Event::builder()
            .name("X")
            .add_event_id(EventId::new(EventIdScheme::Eventbrite, "Q1").unwrap())
            .build();
        let r = MatchingEngine::default_config().match_events(&a, &b);
        assert!(r.breakdown.event_ids_score.is_none());
    }

    // ---------- deterministic match ----------

    #[test]
    fn deterministic_via_shared_event_id() {
        let id = EventId::new(EventIdScheme::Eventbrite, "12345").unwrap();
        let a = Event::builder()
            .name("RustConf 2024")
            .add_event_id(id.clone())
            .build();
        let b = Event::builder()
            .name("Wholly Different")
            .add_event_id(id)
            .build();
        assert!(MatchingEngine::default_config().deterministic_match(&a, &b));
    }

    #[test]
    fn deterministic_via_name_and_start_date() {
        let a = Event::builder()
            .name("RustConf 2024")
            .start_date("2024-09-10T09:00:00Z")
            .build();
        let b = Event::builder()
            .name("RustConf 2024")
            .start_date("2024-09-10T09:00:00Z")
            .build();
        assert!(MatchingEngine::default_config().deterministic_match(&a, &b));
    }

    #[test]
    fn deterministic_via_name_and_start_date_accepts_equivalent_offsets() {
        let a = Event::builder()
            .name("RustConf 2024")
            .start_date("2024-09-10T09:00:00Z")
            .build();
        let b = Event::builder()
            .name("RustConf 2024")
            .start_date("2024-09-10T11:00:00+02:00")
            .build();
        assert!(MatchingEngine::default_config().deterministic_match(&a, &b));
    }

    #[test]
    fn deterministic_rejects_when_name_differs_and_no_shared_id() {
        let a = Event::builder()
            .name("X")
            .start_date("2024-09-10T09:00:00Z")
            .build();
        let b = Event::builder()
            .name("Y")
            .start_date("2024-09-10T09:00:00Z")
            .build();
        assert!(!MatchingEngine::default_config().deterministic_match(&a, &b));
    }

    #[test]
    fn deterministic_rejects_when_start_date_missing_and_no_shared_id() {
        let a = Event::builder().name("X").build();
        let b = Event::builder().name("X").build();
        assert!(!MatchingEngine::default_config().deterministic_match(&a, &b));
    }

    #[test]
    fn deterministic_rejects_when_name_normalises_to_empty() {
        // SEC-M2: two DIFFERENT events whose only shared field is a name that
        // normalises to empty (plus the same start_date) must NOT match.
        let a = Event::builder()
            .name("###")
            .start_date("2024-09-10T09:00:00Z")
            .build();
        let b = Event::builder()
            .name("  ")
            .start_date("2024-09-10T09:00:00Z")
            .build();
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
        let e1 = Event::builder()
            .name("Cafe Centrale Concert")
            .start_date("2024-09-10T09:00:00Z")
            .build();
        let e2 = Event::builder()
            .name("Cafe Central Concert") // close but not equal under normalisation
            .start_date("2024-09-10T09:00:00Z")
            .build();
        let engine = MatchingEngine::new(cfg);
        let r = engine.match_events(&e1, &e2);
        assert!(r.score >= 0.50);
        assert!(!engine.deterministic_match(&e1, &e2));
        assert!(!r.is_match);
    }

    // ---------- batch APIs ----------

    #[test]
    fn match_one_to_many_empty_candidates_yields_empty_vec() {
        let engine = MatchingEngine::default_config();
        let q = Event::builder().name("Solo").build();
        assert!(engine.match_one_to_many(&q, &[]).is_empty());
    }

    #[test]
    fn rank_one_to_many_sorts_by_score_descending() {
        let engine = MatchingEngine::default_config();
        let q = Event::builder().name("RustConf 2024").build();
        let candidates = vec![
            Event::builder().name("PyConf 2024").build(),
            q.clone(),
            Event::builder().name("GoConf 2024").build(),
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

    // ---------- location ----------

    #[test]
    fn location_postcode_match_dominates() {
        let l1 = Location::new().with_address(Address::new().with_postcode("BA4 4BY"));
        let l2 = Location::new().with_address(Address::new().with_postcode("BA4 4BY"));
        let s = MatchingEngine::default_config().compare_locations(&l1, &l2);
        assert!((s - 1.0).abs() < 1e-9, "got {s}");
    }

    #[test]
    fn location_score_none_when_either_side_absent() {
        let a = Event::builder()
            .name("X")
            .location(Location::new().with_venue_name("Worthy Farm"))
            .build();
        let b = Event::builder().name("X").build();
        let r = MatchingEngine::default_config().match_events(&a, &b);
        assert!(r.breakdown.location_score.is_none());
    }

    // ---------- organizer / performers / url ----------

    #[test]
    fn organizer_match_after_normalisation() {
        let a = Event::builder()
            .name("X")
            .organizer("Rust Foundation")
            .build();
        let b = Event::builder()
            .name("X")
            .organizer("rust foundation")
            .build();
        let r = MatchingEngine::default_config().match_events(&a, &b);
        assert!(r.breakdown.organizer_score.unwrap() > 0.99);
    }

    #[test]
    fn performers_match_takes_best_of_cartesian_product() {
        let a = Event::builder()
            .name("X")
            .add_performer("Niko Matsakis")
            .add_performer("Tyler Mandry")
            .build();
        let b = Event::builder()
            .name("X")
            .add_performer("Carol Nichols")
            .add_performer("Niko Matsakis")
            .build();
        let r = MatchingEngine::default_config().match_events(&a, &b);
        assert!(r.breakdown.performers_score.unwrap() > 0.99);
    }

    #[test]
    fn url_match_is_exact_after_trim() {
        let a = Event::builder()
            .name("X")
            .url("https://rustconf.com")
            .build();
        let b = Event::builder()
            .name("X")
            .url("  https://rustconf.com  ")
            .build();
        let r = MatchingEngine::default_config().match_events(&a, &b);
        assert!(approx_eq(r.breakdown.url_score.unwrap(), 1.0));
    }

    // ---------- phonetic ----------

    #[test]
    fn phonetic_score_none_when_off() {
        let p = Event::builder().name("Stephen Concert").build();
        let q = Event::builder().name("Steven Concert").build();
        let r = MatchingEngine::new(MatchConfig {
            use_phonetic_matching: false,
            ..MatchConfig::default()
        })
        .match_events(&p, &q);
        assert!(r.breakdown.name_phonetic_score.is_none());
    }

    #[test]
    fn phonetic_score_some_when_on() {
        let p = Event::builder().name("Stephen").build();
        let q = Event::builder().name("Steven").build();
        let r = MatchingEngine::new(MatchConfig {
            use_phonetic_matching: true,
            ..MatchConfig::default()
        })
        .match_events(&p, &q);
        assert!(r.breakdown.name_phonetic_score.is_some());
    }

    // ---------- relationships & tags ----------

    #[test]
    fn config_default_relationships_and_tags_weight_is_005() {
        let c = MatchConfig::default();
        assert!((c.relationships_weight - 0.05).abs() < 1e-9);
        assert!((c.tags_weight - 0.05).abs() < 1e-9);
    }

    #[test]
    fn score_relationships_identical_sets_scores_one() {
        let a = vec![RelationshipRef::new(RelationKind::Outer, "e1").unwrap()];
        let b = a.clone();
        assert_eq!(score_relationships(&a, &b), Some(1.0));
    }

    #[test]
    fn score_relationships_disjoint_sets_scores_zero() {
        let a = vec![RelationshipRef::new(RelationKind::Outer, "e1").unwrap()];
        let b = vec![RelationshipRef::new(RelationKind::Inner, "e1").unwrap()];
        // Same event id, different relation kind: not the same pair.
        assert_eq!(score_relationships(&a, &b), Some(0.0));
    }

    #[test]
    fn score_relationships_partial_overlap_scores_jaccard_ratio() {
        let a = vec![
            RelationshipRef::new(RelationKind::Outer, "e1").unwrap(),
            RelationshipRef::new(RelationKind::ImmediatelyBefore, "e2").unwrap(),
        ];
        let b = vec![
            RelationshipRef::new(RelationKind::Outer, "e1").unwrap(),
            RelationshipRef::new(RelationKind::ImmediatelyBefore, "e3").unwrap(),
        ];
        // intersection = {(Outer, e1)} = 1; union = 3.
        let score = score_relationships(&a, &b).unwrap();
        assert!(approx_eq(score, 1.0 / 3.0));
    }

    #[test]
    fn score_relationships_empty_either_side_is_none() {
        let empty: Vec<RelationshipRef> = vec![];
        let some = vec![RelationshipRef::new(RelationKind::Outer, "e1").unwrap()];
        assert_eq!(score_relationships(&empty, &some), None);
        assert_eq!(score_relationships(&some, &empty), None);
        assert_eq!(score_relationships(&empty, &empty), None);
    }

    #[test]
    fn score_tags_case_insensitive_identical_scores_one() {
        let a = vec!["VIP".to_string(), "Review".to_string()];
        let b = vec!["vip".to_string(), "review".to_string()];
        assert_eq!(score_tags(&a, &b), Some(1.0));
    }

    #[test]
    fn score_tags_disjoint_sets_scores_zero() {
        let a = vec!["vip".to_string()];
        let b = vec!["fast-track".to_string()];
        assert_eq!(score_tags(&a, &b), Some(0.0));
    }

    #[test]
    fn score_tags_partial_overlap_scores_jaccard_ratio() {
        let a = vec!["vip".to_string(), "review".to_string()];
        let b = vec!["VIP".to_string(), "fast-track".to_string()];
        // intersection = {"vip"} = 1; union = {"vip", "review", "fast-track"} = 3.
        let score = score_tags(&a, &b).unwrap();
        assert!(approx_eq(score, 1.0 / 3.0));
    }

    #[test]
    fn score_tags_empty_either_side_is_none() {
        let empty: Vec<String> = vec![];
        let some = vec!["vip".to_string()];
        assert_eq!(score_tags(&empty, &some), None);
        assert_eq!(score_tags(&some, &empty), None);
        assert_eq!(score_tags(&empty, &empty), None);
    }

    #[test]
    fn relationships_and_tags_absent_do_not_enter_the_weighted_average() {
        // Renormalisation sanity check: with neither field populated on
        // either side, `relationships_score`/`tags_score` are `None` and
        // neither weight enters the denominator — an exact name +
        // start_date match still scores a clean 1.0, not diluted by two
        // "missing" components treated as zero.
        let engine = MatchingEngine::default_config();
        let a = Event::builder()
            .name("RustConf 2024")
            .start_date("2024-09-10T09:00:00Z")
            .build();
        let b = a.clone();
        let result = engine.match_events(&a, &b);
        assert_eq!(result.breakdown.relationships_score, None);
        assert_eq!(result.breakdown.tags_score, None);
        assert!(approx_eq(result.score, 1.0));
    }

    #[test]
    fn relationships_and_tags_participate_when_present_and_agree() {
        let engine = MatchingEngine::default_config();
        let rel = RelationshipRef::new(RelationKind::Outer, "event-42").unwrap();
        let a = Event::builder()
            .name("RustConf 2024")
            .start_date("2024-09-10T09:00:00Z")
            .add_relationship(rel.clone())
            .add_tag("vip")
            .build();
        let b = Event::builder()
            .name("RustConf 2024")
            .start_date("2024-09-10T09:00:00Z")
            .add_relationship(rel)
            .add_tag("VIP")
            .build();
        let result = engine.match_events(&a, &b);
        assert_eq!(result.breakdown.relationships_score, Some(1.0));
        assert_eq!(result.breakdown.tags_score, Some(1.0));
        assert!(approx_eq(result.score, 1.0));
    }
}
