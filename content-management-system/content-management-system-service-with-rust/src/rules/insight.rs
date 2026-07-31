//! Insight arithmetic (CMS-R21, CMS-D13) — pure, DB-free.
//!
//! Every number a dashboard shows is derived here from recorded facts,
//! and the honesty rules are enforced by the types rather than by
//! remembering:
//!
//! - **A ratio carries its numerator and denominator**, and a zero
//!   denominator produces `null` — never `0%`, and never `100%`. "We
//!   published 100% of our drafts" out of zero drafts is the kind of
//!   number that gets into a board pack and stays there.
//! - **A percentile states its sample size**, and below a floor it
//!   refuses to be a percentile at all, returning the raw durations
//!   instead. A p90 over three observations is not a p90.
//! - **Every finding names the rule that produced it**, so an editor
//!   can argue with the rule rather than guess at it.

use serde::{Deserialize, Serialize};

/// The smallest sample a percentile is willing to summarise.
pub const PERCENTILE_SAMPLE_FLOOR: usize = 5;

/// A ratio that shows its working.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ratio {
    /// The ratio, or `null` when the denominator is zero.
    pub value: Option<f64>,
    /// The numerator, always shown.
    pub numerator: u64,
    /// The denominator, always shown.
    pub denominator: u64,
}

/// Build a ratio, refusing to invent one from nothing.
#[must_use]
pub fn ratio(numerator: u64, denominator: u64) -> Ratio {
    Ratio {
        #[allow(clippy::cast_precision_loss)] // counts, not identifiers
        value: (denominator > 0).then(|| numerator as f64 / denominator as f64),
        numerator,
        denominator,
    }
}

/// A summary of a set of durations (in seconds).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DurationSummary {
    /// How many observations there were — always shown, because a
    /// median of two is a different claim from a median of two hundred.
    pub sample_size: usize,
    /// The median, when the sample is large enough to have one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub median_seconds: Option<i64>,
    /// The 90th percentile, when the sample is large enough.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub p90_seconds: Option<i64>,
    /// The raw observations, returned **instead** of percentiles when
    /// the sample is below the floor.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub observations_seconds: Vec<i64>,
    /// Why percentiles are absent, when they are.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Summarise durations, refusing to report a percentile over a sample
/// too small to have one.
#[must_use]
pub fn summarise(mut durations: Vec<i64>) -> DurationSummary {
    durations.sort_unstable();
    let sample_size = durations.len();
    if sample_size < PERCENTILE_SAMPLE_FLOOR {
        return DurationSummary {
            sample_size,
            median_seconds: None,
            p90_seconds: None,
            observations_seconds: durations,
            note: (sample_size > 0).then(|| {
                format!(
                    "fewer than {PERCENTILE_SAMPLE_FLOOR} observations: the raw durations are \
                     shown instead of percentiles"
                )
            }),
        };
    }
    DurationSummary {
        sample_size,
        median_seconds: percentile(&durations, 0.5),
        p90_seconds: percentile(&durations, 0.9),
        observations_seconds: Vec::new(),
        note: None,
    }
}

/// The `p`-th percentile of a **sorted** slice, by nearest rank.
///
/// Nearest-rank rather than interpolation: it always returns a value
/// that actually happened, which is easier to defend when someone asks
/// which review took that long.
#[must_use]
pub fn percentile(sorted: &[i64], p: f64) -> Option<i64> {
    if sorted.is_empty() || !(0.0..=1.0).contains(&p) {
        return None;
    }
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    let rank = ((p * sorted.len() as f64).ceil() as usize).max(1);
    sorted.get(rank - 1).copied()
}

/// One content-health finding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Finding {
    /// The rule that produced it — the thing to argue with.
    pub rule: &'static str,
    /// What it concerns (an entry key, an asset id, a path).
    pub subject: String,
    /// The locale, where the finding is per-variant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    /// The observed values that triggered it.
    pub detail: String,
    /// Who is best placed to fix it, when that is known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
}

/// Every health rule, with the sentence that explains it. Shipping the
/// explanations with the findings means a dashboard never has to invent
/// its own wording — and an editor reads the same rule the code applied.
pub const HEALTH_RULES: &[(&str, &str)] = &[
    (
        "image_alt_text_missing",
        "a published page shows an image whose alt text is empty",
    ),
    (
        "seo_metadata_missing",
        "a published, indexable page has no meta title or description",
    ),
    (
        "broken_reference",
        "a reference points at something missing, deleted, or unpublished",
    ),
    (
        "orphan_asset",
        "an asset nothing currently references (reported, never deleted)",
    ),
    (
        "stale_content",
        "a published page whose newest revision is older than the staleness window",
    ),
    (
        "stale_translation",
        "a translation whose source has published newer revisions",
    ),
    (
        "stuck_in_review",
        "a variant left in review for longer than the review window",
    ),
    (
        "approved_not_published",
        "a variant approved but neither published nor scheduled",
    ),
    (
        "needs_migration",
        "content written under an older content-type version that today's declaration would reject",
    ),
    (
        "route_hazard",
        "a redirect chain approaching the hop cap, or a noindex page linked from a menu",
    ),
];

/// Group findings by rule, preserving order within each group.
#[must_use]
pub fn group_by_rule(findings: &[Finding]) -> Vec<(&'static str, Vec<&Finding>)> {
    let mut groups: Vec<(&'static str, Vec<&Finding>)> = Vec::new();
    for (rule, _) in HEALTH_RULES {
        let matching: Vec<&Finding> = findings.iter().filter(|f| f.rule == *rule).collect();
        if !matching.is_empty() {
            groups.push((rule, matching));
        }
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rule this module exists for: no inventing a ratio out of
    /// nothing.
    #[test]
    fn a_zero_denominator_yields_null_not_a_percentage() {
        let empty = ratio(0, 0);
        assert!(empty.value.is_none());
        assert_eq!(empty.numerator, 0);
        assert_eq!(empty.denominator, 0);

        let real = ratio(3, 4);
        assert!((real.value.unwrap() - 0.75).abs() < f64::EPSILON);
        // The working is always shown, so a reader can check it.
        assert_eq!((real.numerator, real.denominator), (3, 4));
    }

    #[test]
    fn a_ratio_serializes_its_null_rather_than_omitting_it() {
        let json = serde_json::to_value(ratio(0, 0)).unwrap();
        assert!(json["value"].is_null());
        assert_eq!(json["denominator"], 0);
    }

    /// A p90 over three observations is not a p90.
    #[test]
    fn a_small_sample_returns_observations_instead_of_percentiles() {
        let summary = summarise(vec![10, 20, 30]);
        assert_eq!(summary.sample_size, 3);
        assert!(summary.median_seconds.is_none());
        assert!(summary.p90_seconds.is_none());
        assert_eq!(summary.observations_seconds, vec![10, 20, 30]);
        assert!(summary.note.unwrap().contains("raw durations"));
    }

    #[test]
    fn a_large_enough_sample_is_summarised() {
        let summary = summarise(vec![50, 10, 30, 20, 40]);
        assert_eq!(summary.sample_size, 5);
        assert_eq!(summary.median_seconds, Some(30));
        assert_eq!(summary.p90_seconds, Some(50));
        assert!(summary.observations_seconds.is_empty());
        assert!(summary.note.is_none());
    }

    #[test]
    fn an_empty_sample_says_nothing_at_all() {
        let summary = summarise(Vec::new());
        assert_eq!(summary.sample_size, 0);
        assert!(summary.median_seconds.is_none());
        assert!(summary.observations_seconds.is_empty());
        assert!(
            summary.note.is_none(),
            "nothing to explain when there is nothing"
        );
    }

    /// Nearest rank: the answer is always a value that actually
    /// happened, which is easier to defend than an interpolation.
    #[test]
    fn percentiles_use_nearest_rank() {
        let sorted = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        assert_eq!(percentile(&sorted, 0.5), Some(5));
        assert_eq!(percentile(&sorted, 0.9), Some(9));
        assert_eq!(percentile(&sorted, 1.0), Some(10));
        assert_eq!(percentile(&sorted, 0.0), Some(1));
        assert!(sorted.contains(&percentile(&sorted, 0.9).unwrap()));
    }

    #[test]
    fn percentiles_refuse_nonsense_inputs_without_panicking() {
        assert_eq!(percentile(&[], 0.5), None);
        assert_eq!(percentile(&[1, 2, 3], 1.5), None);
        assert_eq!(percentile(&[1, 2, 3], -0.1), None);
        assert_eq!(percentile(&[1, 2, 3], f64::NAN), None);
    }

    /// Every rule ships its own explanation, so a dashboard never has
    /// to invent wording the code did not use.
    #[test]
    fn every_health_rule_explains_itself() {
        assert!(!HEALTH_RULES.is_empty());
        for (rule, explanation) in HEALTH_RULES {
            assert!(!rule.is_empty());
            assert!(
                explanation.len() > 20,
                "{rule} needs a real explanation, not a label"
            );
        }
    }

    #[test]
    fn findings_group_by_rule_in_declared_order() {
        let findings = vec![
            Finding {
                rule: "orphan_asset",
                subject: "a".to_string(),
                locale: None,
                detail: "unused".to_string(),
                owner: None,
            },
            Finding {
                rule: "image_alt_text_missing",
                subject: "b".to_string(),
                locale: Some("en".to_string()),
                detail: "no alt".to_string(),
                owner: None,
            },
            Finding {
                rule: "orphan_asset",
                subject: "c".to_string(),
                locale: None,
                detail: "unused".to_string(),
                owner: None,
            },
        ];
        let groups = group_by_rule(&findings);
        assert_eq!(groups.len(), 2);
        // Declaration order, not encounter order.
        assert_eq!(groups[0].0, "image_alt_text_missing");
        assert_eq!(groups[1].0, "orphan_asset");
        assert_eq!(groups[1].1.len(), 2);
        // Rules with no findings do not appear as empty groups.
        assert!(groups.iter().all(|(_, items)| !items.is_empty()));
    }
}
