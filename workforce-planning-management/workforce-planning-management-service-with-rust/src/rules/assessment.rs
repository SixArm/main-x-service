//! Pure rules for **assessments** — aptitude, personality,
//! psychometric, and selection tests (WPM-R20). DB-free and clock-free
//! (`as_of` is always supplied), exhaustively unit-tested; the
//! controller only wires these.
//!
//! The four categories and the scales they measure:
//!
//! | Category | Measures | Scales |
//! |---|---|---|
//! | `aptitude` | how a person performs at tasks and reacts to situations | `numerical_reasoning`, `verbal_reasoning`, `problem_solving`, `logical_thinking` |
//! | `personality` | behavioural style and working qualities | `work_style`, `team_compatibility`, `introversion_extraversion` |
//! | `psychometric` | **spans aptitude and personality** | `behavioural_style`, `emotional_intelligence`, `cognitive_ability` (+ every aptitude and personality scale) |
//! | `selection` | suitability for a role during hiring | `job_simulation`, `skills_assessment`, `judgement_test` |
//!
//! [`category_permits`] is the load-bearing rule: a category accepts
//! its own scales, and `psychometric` additionally accepts aptitude and
//! personality scales, because a psychometric test covers both by
//! definition. Everything else is a `422` — the profile views are only
//! honest if a reading's category is trustworthy.
//!
//! **Scores are integers.** Percentiles are `0`–`100` and raw scores are
//! whole points out of a whole maximum, so the module needs no floats
//! (the one exception is the reported *mean*, which is a display ratio
//! and always accompanied by its numerator and denominator).

/// The assessment categories.
pub const ASSESSMENT_CATEGORIES: &[&str] =
    &["aptitude", "personality", "psychometric", "selection", "cognitive"];

/// The scales an `aptitude` test measures.
pub const APTITUDE_SCALES: &[&str] = &[
    "numerical_reasoning",
    "verbal_reasoning",
    "problem_solving",
    "logical_thinking",
];

/// The scales a `personality` test measures.
pub const PERSONALITY_SCALES: &[&str] = &[
    "work_style",
    "team_compatibility",
    "introversion_extraversion",
];

/// The scales a `psychometric` test measures **in its own right** (it
/// also covers every aptitude and personality scale — see
/// [`category_permits`]).
pub const PSYCHOMETRIC_SCALES: &[&str] = &[
    "behavioural_style",
    "emotional_intelligence",
    "cognitive_ability",
];

/// The scales a `cognitive` (IQ-style) test measures — standard
/// index names (WAIS shape), not a single "IQ" number: WPM records
/// per-scale readings and **never** derives a composite ranking
/// (WPM-R20: report, never gate; equality-law review is a deployment
/// duty before any selection use).
pub const COGNITIVE_SCALES: &[&str] = &[
    "verbal_comprehension",
    "working_memory",
    "processing_speed",
    "spatial_reasoning",
    "fluid_reasoning",
];

/// The scales a `selection` test measures.
pub const SELECTION_SCALES: &[&str] =
    &["job_simulation", "skills_assessment", "judgement_test"];

/// Every scale, across all four categories.
pub const ASSESSMENT_SCALES: &[&str] = &[
    "numerical_reasoning",
    "verbal_reasoning",
    "problem_solving",
    "logical_thinking",
    "work_style",
    "team_compatibility",
    "introversion_extraversion",
    "behavioural_style",
    "emotional_intelligence",
    "cognitive_ability",
    "job_simulation",
    "skills_assessment",
    "judgement_test",
    "verbal_comprehension",
    "working_memory",
    "processing_speed",
    "spatial_reasoning",
    "fluid_reasoning",
];

/// Assessment lifecycle statuses.
pub const ASSESSMENT_STATUSES: &[&str] = &[
    "scheduled",
    "in_progress",
    "completed",
    "expired",
    "cancelled",
];

/// Who an assessment is about: a `candidate` (during hiring) or an
/// `employee` (development, internal moves). Both may sit any category
/// — selection tests are typically hiring, but an internal candidate
/// takes them too, so the vocabulary does not gate on it.
pub const ASSESSMENT_SUBJECTS: &[&str] = &["candidate", "employee"];

/// Score bands, weakest to strongest.
pub const SCORE_BANDS: &[&str] = &[
    "low",
    "below_average",
    "average",
    "above_average",
    "high",
];

/// The scales `category` owns — the ones whose home it is. `None` for
/// an unknown category token.
#[must_use]
pub fn own_scales(category: &str) -> Option<&'static [&'static str]> {
    match category {
        "aptitude" => Some(APTITUDE_SCALES),
        "personality" => Some(PERSONALITY_SCALES),
        "psychometric" => Some(PSYCHOMETRIC_SCALES),
        "selection" => Some(SELECTION_SCALES),
        "cognitive" => Some(COGNITIVE_SCALES),
        _ => None,
    }
}

/// The category a scale belongs to; `None` for an unknown scale token.
#[must_use]
pub fn scale_category(scale: &str) -> Option<&'static str> {
    ASSESSMENT_CATEGORIES
        .iter()
        .copied()
        .find(|category| own_scales(category).is_some_and(|scales| scales.contains(&scale)))
}

/// Whether `scale` may be reported on an assessment of `category`.
///
/// A category always permits its own scales; `psychometric`
/// additionally permits every aptitude, personality, and cognitive
/// scale (a full psychometric battery spans them by definition). An
/// unknown category or scale token permits nothing.
#[must_use]
pub fn category_permits(category: &str, scale: &str) -> bool {
    match scale_category(scale) {
        None => false,
        Some(home) => {
            home == category
                || (category == "psychometric"
                    && matches!(home, "aptitude" | "personality" | "cognitive"))
        }
    }
}

/// The lifecycle: `scheduled → in_progress → completed → expired`, with
/// `cancelled` reachable from any open state and a direct
/// `scheduled → completed` for a sitting recorded after the fact.
/// `expired` and `cancelled` are terminal.
///
/// # Errors
///
/// A human-readable refusal naming the legal moves (the controller
/// turns it into a `422`).
pub fn assessment_transition(current: &str, to: &str) -> Result<(), String> {
    if !ASSESSMENT_STATUSES.contains(&to) {
        return Err(format!(
            "unknown status `{to}` (statuses: {ASSESSMENT_STATUSES:?})"
        ));
    }
    let ok = matches!(
        (current, to),
        ("scheduled", "in_progress" | "completed" | "cancelled")
            | ("in_progress", "completed" | "cancelled")
            | ("completed", "expired")
    );
    if ok {
        Ok(())
    } else {
        Err(format!(
            "illegal transition `{current}` → `{to}` \
             (scheduled→in_progress→completed→expired; cancel an open one; \
             a sitting may be recorded straight to completed)"
        ))
    }
}

/// Whether a percentile is a legal norm-referenced value (`0`–`100`).
#[must_use]
pub fn valid_percentile(percentile: i32) -> bool {
    (0..=100).contains(&percentile)
}

/// Whether a raw score is legal against its maximum: the maximum must
/// be positive and the score within `0..=max`.
#[must_use]
pub fn valid_raw_score(raw: i32, max: i32) -> bool {
    max > 0 && (0..=max).contains(&raw)
}

/// The band a percentile falls in, on the conventional
/// norm-referenced split: bottom decile `low`, next fifth
/// `below_average`, middle two-fifths `average`, next fifth
/// `above_average`, top decile `high`. Out-of-range values clamp, so
/// this is total (validation refuses them at the boundary; this is the
/// defensive fallback).
#[must_use]
pub fn band_for_percentile(percentile: i32) -> &'static str {
    match percentile.clamp(0, 100) {
        0..=9 => "low",
        10..=29 => "below_average",
        30..=69 => "average",
        70..=89 => "above_average",
        _ => "high",
    }
}

/// Whether an assessment's results count as **current** on `as_of`:
/// the assessment is `completed`, and either has no expiry or has not
/// passed it. A scheduled, in-progress, cancelled, or explicitly
/// expired assessment is never current.
#[must_use]
pub fn is_current(status: &str, expires_on: Option<chrono::NaiveDate>, as_of: chrono::NaiveDate) -> bool {
    status == "completed" && expires_on.is_none_or(|expiry| as_of <= expiry)
}

/// The mean of the supplied percentiles as `(numerator, denominator,
/// mean)`, or `None` when the slice is empty.
///
/// Reported with its numerator and denominator so a consumer can see
/// how many real scores went into it. Never interpolates: a scale with
/// no percentile simply does not contribute, and no scores at all
/// yields `None` rather than a misleading zero.
#[must_use]
pub fn mean_percentile(percentiles: &[i32]) -> Option<(i64, usize, f64)> {
    if percentiles.is_empty() {
        return None;
    }
    let sum: i64 = percentiles.iter().map(|p| i64::from(*p)).sum();
    let count = percentiles.len();
    #[allow(clippy::cast_precision_loss)] // display mean; the exact terms are returned alongside
    let mean = sum as f64 / count as f64;
    Some((sum, count, mean))
}

/// Which of `category`'s own scales have **no** reading among
/// `measured` — the honest statement of what has not been assessed.
#[must_use]
pub fn scales_not_assessed(category: &str, measured: &[String]) -> Vec<&'static str> {
    own_scales(category)
        .unwrap_or(&[])
        .iter()
        .copied()
        .filter(|scale| !measured.iter().any(|m| m == scale))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(y: i32, m: u32, d: u32) -> chrono::NaiveDate {
        chrono::NaiveDate::from_ymd_opt(y, m, d).expect("valid date")
    }

    /// Every scale has exactly one home category, and the two
    /// directions of the mapping agree.
    #[test]
    fn scale_category_mapping_is_consistent() {
        for scale in ASSESSMENT_SCALES {
            let home = scale_category(scale).expect("every scale has a home");
            assert!(
                own_scales(home).expect("known category").contains(scale),
                "{scale} should be owned by {home}"
            );
            let homes: Vec<&str> = ASSESSMENT_CATEGORIES
                .iter()
                .copied()
                .filter(|c| own_scales(c).expect("known").contains(scale))
                .collect();
            assert_eq!(homes.len(), 1, "{scale} must have exactly one home");
        }
        // The union of the per-category scales is the full list.
        let union: usize = ASSESSMENT_CATEGORIES
            .iter()
            .map(|c| own_scales(c).expect("known").len())
            .sum();
        assert_eq!(union, ASSESSMENT_SCALES.len());
        assert_eq!(scale_category("telepathy"), None);
        assert_eq!(own_scales("astrology"), None);
    }

    /// Psychometric spans aptitude and personality; every other
    /// category stays inside its own scales.
    #[test]
    fn psychometric_spans_aptitude_and_personality() {
        assert!(category_permits("psychometric", "emotional_intelligence"));
        assert!(category_permits("psychometric", "numerical_reasoning"));
        assert!(category_permits("psychometric", "team_compatibility"));
        assert!(
            !category_permits("psychometric", "job_simulation"),
            "selection scales are a hiring instrument, not a psychometric one"
        );

        assert!(category_permits("aptitude", "verbal_reasoning"));
        assert!(!category_permits("aptitude", "work_style"));
        assert!(!category_permits("personality", "cognitive_ability"));
        assert!(category_permits("selection", "judgement_test"));
        assert!(!category_permits("selection", "logical_thinking"));

        // Unknown tokens permit nothing.
        assert!(!category_permits("astrology", "work_style"));
        assert!(!category_permits("aptitude", "vibes"));
    }

    /// The lifecycle: forward moves only, cancel from any open state,
    /// terminal states stuck, no self-transition.
    #[test]
    fn assessment_lifecycle() {
        assert!(assessment_transition("scheduled", "in_progress").is_ok());
        assert!(assessment_transition("in_progress", "completed").is_ok());
        assert!(assessment_transition("completed", "expired").is_ok());
        assert!(
            assessment_transition("scheduled", "completed").is_ok(),
            "a sitting may be recorded after the fact"
        );
        assert!(assessment_transition("scheduled", "cancelled").is_ok());
        assert!(assessment_transition("in_progress", "cancelled").is_ok());

        assert!(assessment_transition("completed", "in_progress").is_err(), "no rewind");
        assert!(assessment_transition("expired", "completed").is_err(), "terminal");
        assert!(assessment_transition("cancelled", "scheduled").is_err(), "terminal");
        assert!(assessment_transition("scheduled", "sideways").is_err(), "unknown");
        for status in ASSESSMENT_STATUSES {
            assert!(
                assessment_transition(status, status).is_err(),
                "{status} → {status} is not a move"
            );
        }
    }

    /// Bands split the percentile range at 10 / 30 / 70 / 90, and
    /// out-of-range values clamp rather than panic.
    #[test]
    fn bands_split_the_percentile_range() {
        assert_eq!(band_for_percentile(0), "low");
        assert_eq!(band_for_percentile(9), "low");
        assert_eq!(band_for_percentile(10), "below_average");
        assert_eq!(band_for_percentile(29), "below_average");
        assert_eq!(band_for_percentile(30), "average");
        assert_eq!(band_for_percentile(69), "average");
        assert_eq!(band_for_percentile(70), "above_average");
        assert_eq!(band_for_percentile(89), "above_average");
        assert_eq!(band_for_percentile(90), "high");
        assert_eq!(band_for_percentile(100), "high");
        assert_eq!(band_for_percentile(-5), "low");
        assert_eq!(band_for_percentile(150), "high");
        // Every band the function can return is in the vocabulary.
        for percentile in 0..=100 {
            assert!(SCORE_BANDS.contains(&band_for_percentile(percentile)));
        }
    }

    /// Score bounds: percentiles are 0–100; a raw score sits in
    /// `0..=max` with a positive maximum.
    #[test]
    fn score_bounds() {
        assert!(valid_percentile(0) && valid_percentile(100));
        assert!(!valid_percentile(-1) && !valid_percentile(101));

        assert!(valid_raw_score(0, 10) && valid_raw_score(10, 10));
        assert!(!valid_raw_score(11, 10), "above the maximum");
        assert!(!valid_raw_score(-1, 10), "negative");
        assert!(!valid_raw_score(0, 0), "the maximum must be positive");
    }

    /// Currency requires completion and respects the expiry date.
    #[test]
    fn currency_requires_completion_and_a_live_expiry() {
        let as_of = day(2026, 7, 23);
        assert!(is_current("completed", None, as_of), "no expiry ⇒ current");
        assert!(is_current("completed", Some(day(2026, 7, 23)), as_of), "current on the expiry date");
        assert!(!is_current("completed", Some(day(2026, 7, 22)), as_of), "expired the day before");
        for status in ["scheduled", "in_progress", "cancelled", "expired"] {
            assert!(!is_current(status, None, as_of), "{status} is not current");
        }
    }

    /// The mean reports its terms and never interpolates.
    #[test]
    fn mean_reports_numerator_and_denominator() {
        assert_eq!(mean_percentile(&[]), None, "no scores ⇒ no figure, not zero");
        let (sum, count, mean) = mean_percentile(&[90, 70]).expect("two scores");
        assert_eq!((sum, count), (160, 2));
        assert!((mean - 80.0).abs() < f64::EPSILON, "got {mean}");
        let (sum, count, _) = mean_percentile(&[0]).expect("one score");
        assert_eq!((sum, count), (0, 1), "a real zero is a score, not an absence");
    }

    /// The gap list names exactly the category's own scales with no
    /// reading.
    #[test]
    fn gaps_name_the_unassessed_scales() {
        let measured = vec!["numerical_reasoning".to_string()];
        let gaps = scales_not_assessed("aptitude", &measured);
        assert_eq!(gaps.len(), APTITUDE_SCALES.len() - 1);
        assert!(!gaps.contains(&"numerical_reasoning"));
        assert!(gaps.contains(&"logical_thinking"));

        // A cross-category reading does not fill an aptitude gap.
        let elsewhere = vec!["work_style".to_string()];
        assert_eq!(
            scales_not_assessed("aptitude", &elsewhere).len(),
            APTITUDE_SCALES.len()
        );
        // Nothing measured for an unknown category.
        assert!(scales_not_assessed("astrology", &measured).is_empty());
    }

    /// The cognitive (IQ-style) category: its own index scales are
    /// permitted, psychometric spans them, and selection does not —
    /// an IQ scale cannot ride into a selection test unreviewed.
    #[test]
    fn cognitive_category_scales_and_overlap() {
        for scale in COGNITIVE_SCALES {
            assert!(category_permits("cognitive", scale));
            assert!(category_permits("psychometric", scale), "battery spans cognitive");
            assert!(!category_permits("selection", scale), "no silent selection use");
            assert!(!category_permits("aptitude", scale));
        }
        assert!(!category_permits("cognitive", "job_simulation"));
        assert!(!category_permits("cognitive", "work_style"));
    }
}
