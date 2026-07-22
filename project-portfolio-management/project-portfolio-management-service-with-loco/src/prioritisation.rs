//! **Smart Score** — the pure, explainable prioritisation score behind
//! the data-driven prioritisation views. DB-free and exhaustively
//! unit-tested.
//!
//! The score is a renormalised weighted average over the components a
//! plan actually has evidence for, in `0..=100`, with a full
//! per-component breakdown — deliberately the same shape as the
//! matcher's `MatchBreakdown`, for the same reason: a number that
//! ranks somebody's work has to be able to explain itself.
//!
//! Three honesty rules:
//!
//! - **Absent evidence is absent, not zero.** A plan with no benefit
//!   line does not score 0 for ROI; the ROI component is dropped and
//!   the remaining weights renormalise. Every dropped component is
//!   named in `missing`, and `coverage` reports how much of the weight
//!   was actually backed by data.
//! - **A score with no evidence is `None`.** No components ⇒ no score,
//!   never a confident-looking 0.
//! - **Nothing is stored.** The score is derived on read from rows the
//!   service already owns, so it cannot drift from its inputs.

use std::collections::BTreeMap;

/// A component's identity and default weight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComponentSpec {
    /// Stable machine name (also the weight-config key).
    pub name: &'static str,
    /// Default weight in basis points of the total (sums to 10 000).
    pub default_weight_bp: u32,
}

/// The Smart Score components, in report order. Weights are basis
/// points so the defaults sum exactly to 10 000 with no float drift.
pub const COMPONENTS: &[ComponentSpec] = &[
    ComponentSpec {
        name: "roi",
        default_weight_bp: 2500,
    },
    ComponentSpec {
        name: "strategic_alignment",
        default_weight_bp: 2000,
    },
    ComponentSpec {
        name: "expert_review",
        default_weight_bp: 1500,
    },
    ComponentSpec {
        name: "risk",
        default_weight_bp: 1500,
    },
    ComponentSpec {
        name: "demand",
        default_weight_bp: 1000,
    },
    ComponentSpec {
        name: "priority",
        default_weight_bp: 1000,
    },
    ComponentSpec {
        name: "momentum",
        default_weight_bp: 500,
    },
];

/// Score at or above which a plan is banded `high`.
pub const HIGH_BAND: f64 = 70.0;
/// Score at or above which a plan is banded `medium`.
pub const MEDIUM_BAND: f64 = 40.0;

/// ROI (in basis points) treated as a full-marks return: 200 %.
pub const ROI_FULL_MARKS_BP: i64 = 20_000;
/// Votes treated as full-marks demand.
pub const VOTES_FULL_MARKS: i64 = 20;
/// Days of silence after which momentum has fully decayed.
pub const STALE_DAYS: i64 = 90;
/// Highest possible risk exposure (probability × impact, 5 × 5).
pub const MAX_RISK_EXPOSURE: i64 = 25;

/// The evidence a plan offers the score. Every field is optional: a
/// `None` drops its component rather than scoring it zero.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScoreFacts {
    /// Return on investment in basis points (`realized / cost`), from
    /// the plan's benefit lines.
    pub roi_basis_points: Option<i64>,
    /// Strongest objective-link weight `0..=100`.
    pub objective_weight: Option<i64>,
    /// Mean submitted expert-review score `0..=100`.
    pub review_score: Option<i64>,
    /// Highest open-risk exposure `0..=25` (probability × impact).
    pub risk_exposure: Option<i64>,
    /// Votes carried over from the originating idea.
    pub votes: Option<i64>,
    /// `MoSCoW` band from the plan's `moscow:<band>` tag.
    pub moscow_band: Option<String>,
    /// Days since the plan last changed.
    pub days_since_update: Option<i64>,
}

/// One component's contribution to the score.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ScoreComponent {
    /// The component name.
    pub name: &'static str,
    /// Its renormalised share of the score, `0.0..=1.0`.
    pub weight: f64,
    /// The normalised component value, `0.0..=1.0`.
    pub raw: f64,
    /// `weight × raw × 100` — its points of the final score.
    pub contribution: f64,
}

/// A plan's Smart Score with its full derivation.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct SmartScore {
    /// The score `0.0..=100.0`, or `None` when there is no evidence.
    pub score: Option<f64>,
    /// `high` / `medium` / `low`, or `unscored`.
    pub band: &'static str,
    /// The components that contributed, in report order.
    pub components: Vec<ScoreComponent>,
    /// Components with no evidence, excluded from the average.
    pub missing: Vec<&'static str>,
    /// Share of the configured weight that had evidence behind it.
    pub coverage: f64,
}

/// Component weights, in basis points, keyed by component name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Weights(BTreeMap<&'static str, u32>);

impl Default for Weights {
    fn default() -> Self {
        Self(
            COMPONENTS
                .iter()
                .map(|c| (c.name, c.default_weight_bp))
                .collect(),
        )
    }
}

impl Weights {
    /// The weight of one component in basis points (0 when unknown).
    #[must_use]
    pub fn get(&self, name: &str) -> u32 {
        self.0.get(name).copied().unwrap_or(0)
    }
}

/// Parse the `PROJECT_PORTFOLIO_MANAGEMENT_SMART_SCORE_WEIGHTS` JSON —
/// a map of component name → basis points, e.g.
/// `{"roi": 4000, "risk": 1000, …}`. Every component must be present
/// and the weights must sum to 10 000: a partial or unbalanced map is
/// rejected wholesale (`None`) so the caller falls back to the
/// documented defaults rather than silently scoring on a lopsided
/// scale. Absent / blank config is also `None`.
#[must_use]
pub fn parse_weights(raw: Option<&str>) -> Option<Weights> {
    let raw = raw?.trim();
    if raw.is_empty() {
        return None;
    }
    let parsed: BTreeMap<String, i64> = serde_json::from_str(raw).ok()?;
    if parsed.len() != COMPONENTS.len() {
        return None;
    }
    let mut weights = BTreeMap::new();
    let mut total: i64 = 0;
    for spec in COMPONENTS {
        let bp = *parsed.get(spec.name)?;
        if !(0..=10_000).contains(&bp) {
            return None;
        }
        total += bp;
        weights.insert(spec.name, u32::try_from(bp).ok()?);
    }
    if total != 10_000 {
        return None;
    }
    Some(Weights(weights))
}

/// Clamp a ratio into `0.0..=1.0`.
fn clamp01(value: f64) -> f64 {
    value.clamp(0.0, 1.0)
}

/// Normalise one fact into `0.0..=1.0`, or `None` when absent.
#[allow(clippy::cast_precision_loss)]
fn normalise(name: &str, facts: &ScoreFacts) -> Option<f64> {
    match name {
        // More return is better; losses (negative bp) floor at 0.
        "roi" => facts
            .roi_basis_points
            .map(|bp| clamp01(bp as f64 / ROI_FULL_MARKS_BP as f64)),
        "strategic_alignment" => facts.objective_weight.map(|w| clamp01(w as f64 / 100.0)),
        "expert_review" => facts.review_score.map(|s| clamp01(s as f64 / 100.0)),
        // Less exposure is better: an inverted, bounded scale.
        "risk" => facts
            .risk_exposure
            .map(|e| clamp01(1.0 - (e as f64 / MAX_RISK_EXPOSURE as f64))),
        "demand" => facts
            .votes
            .map(|v| clamp01(v as f64 / VOTES_FULL_MARKS as f64)),
        "priority" => facts.moscow_band.as_deref().and_then(|band| match band {
            "must" => Some(1.0),
            "should" => Some(2.0 / 3.0),
            "could" => Some(1.0 / 3.0),
            "wont" => Some(0.0),
            _ => None,
        }),
        // Recent movement is better; fully decayed after STALE_DAYS.
        "momentum" => facts
            .days_since_update
            .map(|days| clamp01(1.0 - (days.max(0) as f64 / STALE_DAYS as f64))),
        _ => None,
    }
}

/// Band a score.
#[must_use]
pub fn band(score: Option<f64>) -> &'static str {
    match score {
        None => "unscored",
        Some(score) if score >= HIGH_BAND => "high",
        Some(score) if score >= MEDIUM_BAND => "medium",
        Some(_) => "low",
    }
}

/// Compute the Smart Score for one plan's evidence.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn smart_score(facts: &ScoreFacts, weights: &Weights) -> SmartScore {
    let mut present: Vec<(&'static str, u32, f64)> = Vec::new();
    let mut missing: Vec<&'static str> = Vec::new();
    let mut configured_bp: u32 = 0;
    for spec in COMPONENTS {
        let weight_bp = weights.get(spec.name);
        configured_bp += weight_bp;
        // A zero-weighted component is switched off by config: it is
        // neither a contributor nor a gap in the evidence.
        if weight_bp == 0 {
            continue;
        }
        match normalise(spec.name, facts) {
            Some(raw) => present.push((spec.name, weight_bp, raw)),
            None => missing.push(spec.name),
        }
    }
    let present_bp: u32 = present.iter().map(|(_, bp, _)| *bp).sum();
    if present_bp == 0 {
        return SmartScore {
            score: None,
            band: band(None),
            components: Vec::new(),
            missing,
            coverage: 0.0,
        };
    }
    let mut components = Vec::with_capacity(present.len());
    let mut score = 0.0;
    for (name, bp, raw) in present {
        let weight = f64::from(bp) / f64::from(present_bp);
        let contribution = weight * raw * 100.0;
        score += contribution;
        components.push(ScoreComponent {
            name,
            weight: round1(weight * 100.0) / 100.0,
            raw: round2(raw),
            contribution: round1(contribution),
        });
    }
    let coverage = if configured_bp == 0 {
        0.0
    } else {
        f64::from(present_bp) / f64::from(configured_bp)
    };
    let score = round1(score.clamp(0.0, 100.0));
    SmartScore {
        score: Some(score),
        band: band(Some(score)),
        components,
        missing,
        coverage: round2(coverage),
    }
}

/// Round to one decimal place.
fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

/// Round to two decimal places.
fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Compare two scores for practical equality (they are rounded to
    /// 1–2 decimal places before anyone sees them).
    fn approx(left: f64, right: f64) -> bool {
        (left - right).abs() < 1e-9
    }

    fn full_facts() -> ScoreFacts {
        ScoreFacts {
            roi_basis_points: Some(20_000),
            objective_weight: Some(100),
            review_score: Some(100),
            risk_exposure: Some(0),
            votes: Some(20),
            moscow_band: Some("must".to_string()),
            days_since_update: Some(0),
        }
    }

    #[test]
    fn default_weights_sum_to_one_hundred_percent() {
        let total: u32 = COMPONENTS.iter().map(|c| c.default_weight_bp).sum();
        assert_eq!(total, 10_000);
    }

    #[test]
    fn a_perfect_plan_scores_100_and_bands_high() {
        let s = smart_score(&full_facts(), &Weights::default());
        assert_eq!(s.score, Some(100.0));
        assert_eq!(s.band, "high");
        assert!(s.missing.is_empty());
        assert!(approx(s.coverage, 1.0), "{}", s.coverage);
        assert_eq!(s.components.len(), COMPONENTS.len());
    }

    #[test]
    fn a_worst_case_plan_scores_0_and_bands_low() {
        let facts = ScoreFacts {
            roi_basis_points: Some(0),
            objective_weight: Some(0),
            review_score: Some(0),
            risk_exposure: Some(MAX_RISK_EXPOSURE),
            votes: Some(0),
            moscow_band: Some("wont".to_string()),
            days_since_update: Some(STALE_DAYS * 2),
        };
        let s = smart_score(&facts, &Weights::default());
        assert_eq!(s.score, Some(0.0));
        assert_eq!(s.band, "low");
    }

    #[test]
    fn no_evidence_means_no_score_not_a_zero() {
        let s = smart_score(&ScoreFacts::default(), &Weights::default());
        assert_eq!(
            s.score, None,
            "an unscored plan must not look like a bad one"
        );
        assert_eq!(s.band, "unscored");
        assert!(approx(s.coverage, 0.0), "{}", s.coverage);
        assert_eq!(s.missing.len(), COMPONENTS.len());
    }

    #[test]
    fn missing_components_renormalise_rather_than_scoring_zero() {
        // Only ROI has evidence, and it is perfect: the score is 100,
        // not 25 (its default share of the whole).
        let facts = ScoreFacts {
            roi_basis_points: Some(ROI_FULL_MARKS_BP),
            ..ScoreFacts::default()
        };
        let s = smart_score(&facts, &Weights::default());
        assert_eq!(s.score, Some(100.0));
        assert_eq!(s.components.len(), 1);
        assert!(approx(s.components[0].weight, 1.0));
        assert_eq!(s.missing.len(), COMPONENTS.len() - 1);
        assert!(
            approx(s.coverage, 0.25),
            "the thin evidence is disclosed: {}",
            s.coverage
        );
    }

    #[test]
    fn coverage_reports_how_much_evidence_backed_the_score() {
        let facts = ScoreFacts {
            roi_basis_points: Some(0),
            objective_weight: Some(50),
            ..ScoreFacts::default()
        };
        let s = smart_score(&facts, &Weights::default());
        assert!(
            approx(s.coverage, 0.45),
            "roi 0.25 + strategic_alignment 0.20, got {}",
            s.coverage
        );
    }

    #[test]
    fn risk_is_inverted_less_exposure_scores_higher() {
        let low = smart_score(
            &ScoreFacts {
                risk_exposure: Some(1),
                ..ScoreFacts::default()
            },
            &Weights::default(),
        );
        let high = smart_score(
            &ScoreFacts {
                risk_exposure: Some(20),
                ..ScoreFacts::default()
            },
            &Weights::default(),
        );
        assert!(
            low.score > high.score,
            "{:?} should beat {:?}",
            low.score,
            high.score
        );
    }

    #[test]
    fn outsized_inputs_are_clamped_never_overflowing_the_scale() {
        let facts = ScoreFacts {
            roi_basis_points: Some(ROI_FULL_MARKS_BP * 100),
            votes: Some(1_000_000),
            risk_exposure: Some(MAX_RISK_EXPOSURE * 4),
            days_since_update: Some(-5),
            ..ScoreFacts::default()
        };
        let s = smart_score(&facts, &Weights::default());
        let score = s.score.expect("scored");
        assert!((0.0..=100.0).contains(&score), "{score}");
        for c in &s.components {
            assert!((0.0..=1.0).contains(&c.raw), "{c:?}");
        }
    }

    #[test]
    fn a_loss_making_roi_floors_at_zero_rather_than_going_negative() {
        let s = smart_score(
            &ScoreFacts {
                roi_basis_points: Some(-5_000),
                ..ScoreFacts::default()
            },
            &Weights::default(),
        );
        assert_eq!(s.score, Some(0.0));
    }

    #[test]
    fn moscow_bands_order_correctly_and_unknown_bands_are_dropped() {
        let score_for = |band: &str| {
            smart_score(
                &ScoreFacts {
                    moscow_band: Some(band.to_string()),
                    ..ScoreFacts::default()
                },
                &Weights::default(),
            )
            .score
        };
        assert_eq!(score_for("must"), Some(100.0));
        assert_eq!(score_for("wont"), Some(0.0));
        assert!(score_for("should") > score_for("could"));
        assert_eq!(
            score_for("urgent"),
            None,
            "an unknown band is no evidence, not a guess"
        );
    }

    #[test]
    fn components_contributions_sum_to_the_score() {
        let facts = ScoreFacts {
            roi_basis_points: Some(10_000),
            review_score: Some(70),
            risk_exposure: Some(6),
            ..ScoreFacts::default()
        };
        let s = smart_score(&facts, &Weights::default());
        let summed: f64 = s.components.iter().map(|c| c.contribution).sum();
        let score = s.score.expect("scored");
        assert!(
            (summed - score).abs() < 0.2,
            "components {summed} vs score {score}"
        );
    }

    #[test]
    fn weights_config_round_trips_when_it_is_complete_and_balanced() {
        let raw = r#"{"roi":4000,"strategic_alignment":2000,"expert_review":1000,
                      "risk":1000,"demand":1000,"priority":500,"momentum":500}"#;
        let weights = parse_weights(Some(raw)).expect("valid config");
        assert_eq!(weights.get("roi"), 4000);
        // And it actually changes the ranking maths.
        let facts = ScoreFacts {
            roi_basis_points: Some(ROI_FULL_MARKS_BP),
            review_score: Some(0),
            ..ScoreFacts::default()
        };
        let tuned = smart_score(&facts, &weights).score.expect("scored");
        let default = smart_score(&facts, &Weights::default())
            .score
            .expect("scored");
        assert!(tuned > default, "{tuned} vs {default}");
    }

    #[test]
    fn unbalanced_partial_or_junk_weight_config_is_rejected_wholesale() {
        assert_eq!(parse_weights(None), None);
        assert_eq!(parse_weights(Some("   ")), None);
        assert_eq!(parse_weights(Some("not json")), None);
        // Sums to 9 000, not 10 000.
        let unbalanced = r#"{"roi":3000,"strategic_alignment":2000,"expert_review":1000,
                            "risk":1000,"demand":1000,"priority":500,"momentum":500}"#;
        assert_eq!(parse_weights(Some(unbalanced)), None);
        // Missing a component entirely.
        let partial = r#"{"roi":10000}"#;
        assert_eq!(parse_weights(Some(partial)), None);
        // Negative weight.
        let negative = r#"{"roi":10500,"strategic_alignment":-500,"expert_review":0,
                          "risk":0,"demand":0,"priority":0,"momentum":0}"#;
        assert_eq!(parse_weights(Some(negative)), None);
    }

    #[test]
    fn a_zero_weighted_component_is_off_not_missing() {
        let raw = r#"{"roi":10000,"strategic_alignment":0,"expert_review":0,
                      "risk":0,"demand":0,"priority":0,"momentum":0}"#;
        let weights = parse_weights(Some(raw)).expect("valid config");
        let facts = ScoreFacts {
            roi_basis_points: Some(ROI_FULL_MARKS_BP),
            ..ScoreFacts::default()
        };
        let s = smart_score(&facts, &weights);
        assert_eq!(s.score, Some(100.0));
        assert!(
            s.missing.is_empty(),
            "switched-off components are not evidence gaps: {:?}",
            s.missing
        );
    }

    #[test]
    fn bands_split_at_the_documented_thresholds() {
        assert_eq!(band(Some(HIGH_BAND)), "high");
        assert_eq!(band(Some(HIGH_BAND - 0.1)), "medium");
        assert_eq!(band(Some(MEDIUM_BAND)), "medium");
        assert_eq!(band(Some(MEDIUM_BAND - 0.1)), "low");
        assert_eq!(band(None), "unscored");
    }
}
