//! Pure rules for **realized gains** and **strategic performance**
//! (entity spec §5.9.6 / §6.4c, FR-33 / FR-34 / FR-36). DB-free and
//! exhaustively unit-tested.
//!
//! Every figure here is derived; nothing is stored. The recurring rule,
//! stated once: **absent evidence reports `null` with a reason and
//! sorts last — never `0`**, because zero means *measured and failing*
//! and using it for *not measured* is how a dashboard lies.

use serde::{Deserialize, Serialize};

/// Basis-point scale.
pub const BASIS_POINTS: i64 = 10_000;

/// Why a figure is absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Absent {
    /// No value has been observed at all.
    Unrealized,
    /// The denominator is zero or negative.
    NoDenominator,
    /// More than one currency, and this service converts none.
    MixedCurrency,
    /// No phased budget baseline, so SPI and CPI are undefined. A plan
    /// with no baseline is **unmeasured**, not on track.
    NoBaseline,
    /// Nobody responded.
    NoResponses,
    /// The arithmetic would overflow.
    Overflow,
}

/// How a value point was arrived at. A measured £2m and an asserted £2m
/// are different kinds of evidence, and a realized-value figure that
/// cannot say which has no audit standing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Method {
    /// Observed.
    Measured,
    /// Modelled.
    Estimated,
    /// Stated by a person.
    Asserted,
}

impl Method {
    /// Parse; anything unrecognised is the weakest reading, `Asserted`,
    /// rather than the strongest.
    #[must_use]
    pub fn parse(raw: Option<&str>) -> Self {
        match raw.map(str::trim).map(str::to_ascii_lowercase).as_deref() {
            Some("measured") => Self::Measured,
            Some("estimated") => Self::Estimated,
            _ => Self::Asserted,
        }
    }

    /// The wire token.
    #[must_use]
    pub const fn token(self) -> &'static str {
        match self {
            Self::Measured => "measured",
            Self::Estimated => "estimated",
            Self::Asserted => "asserted",
        }
    }
}

/// One observed value point.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValueFact {
    /// Minor units.
    pub value: i64,
    /// How it was arrived at.
    pub method: Method,
    /// Whether this is the first measurable value for its plan.
    pub is_first_measurable: bool,
    /// Days from approval to observation, when both are known.
    pub days_from_approval: Option<i64>,
}

/// Transformation ROI: `(realized − investment) / investment`, in basis
/// points, with both inputs returned so the arithmetic is checkable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Roi {
    /// The ratio, or `None`.
    pub basis_points: Option<i64>,
    /// Why it is absent.
    pub absent: Option<Absent>,
    /// Numerator input.
    pub realized_minor: i64,
    /// Denominator input.
    pub investment_minor: i64,
    /// The mix of evidence behind `realized_minor`.
    pub measured_share_basis_points: Option<i64>,
}

/// Compute transformation ROI.
///
/// A non-positive investment is undefined rather than infinite: a plan
/// that cost nothing has no return *ratio*, whatever value it produced.
#[must_use]
pub fn roi(values: &[ValueFact], investment_minor: i64) -> Roi {
    let realized: i64 = values.iter().fold(0_i64, |a, v| a.saturating_add(v.value));
    let measured = values
        .iter()
        .filter(|v| v.method == Method::Measured)
        .count();

    let (basis_points, absent) = if values.is_empty() {
        (None, Some(Absent::Unrealized))
    } else if investment_minor <= 0 {
        (None, Some(Absent::NoDenominator))
    } else {
        match realized
            .checked_sub(investment_minor)
            .and_then(|delta| delta.checked_mul(BASIS_POINTS))
        {
            Some(scaled) => (Some(scaled / investment_minor), None),
            None => (None, Some(Absent::Overflow)),
        }
    };

    Roi {
        basis_points,
        absent,
        realized_minor: realized,
        investment_minor,
        measured_share_basis_points: i64::try_from(values.len())
            .ok()
            .filter(|total| *total > 0)
            .and_then(|total| {
                i64::try_from(measured)
                    .ok()
                    .and_then(|m| m.checked_mul(BASIS_POINTS))
                    .map(|scaled| scaled / total)
            }),
    }
}

/// Value Realization Rate: completed initiatives that delivered their
/// projected value, over completed initiatives.
///
/// **A count of initiatives**, deliberately distinct from Benefit
/// Realization Rate, which is a ratio of *value*. A portfolio can score
/// 90% on one and 40% on the other, and that difference is the finding.
///
/// # Errors
///
/// [`Absent::NoDenominator`] when nothing has completed — a rate over
/// zero initiatives is undefined, not zero.
pub fn value_realization_rate(completed: usize, delivered: usize) -> Result<i64, Absent> {
    if completed == 0 {
        return Err(Absent::NoDenominator);
    }
    let completed = i64::try_from(completed).map_err(|_| Absent::Overflow)?;
    let delivered = i64::try_from(delivered).map_err(|_| Absent::Overflow)?;
    delivered
        .checked_mul(BASIS_POINTS)
        .map(|scaled| scaled / completed)
        .ok_or(Absent::Overflow)
}

/// Adoption rate: active over target users.
///
/// Refused rather than computed when the target is zero or absent — a
/// rate with no denominator is not a small rate, it is not a rate.
///
/// # Errors
///
/// [`Absent::NoDenominator`] for a non-positive target.
pub fn adoption_rate(active_users: i64, target_users: i64) -> Result<i64, Absent> {
    if target_users <= 0 {
        return Err(Absent::NoDenominator);
    }
    active_users
        .checked_mul(BASIS_POINTS)
        .map(|scaled| scaled / target_users)
        .ok_or(Absent::Overflow)
}

/// Nearest-rank percentile over a sorted-in-place copy, so every
/// reported figure is an **observed** value rather than an interpolated
/// one — the rule the cycle-time percentiles already follow.
#[must_use]
pub fn percentile(values: &[i64], percentile_basis_points: i64) -> Option<i64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let count = i64::try_from(sorted.len()).ok()?;
    let rank = percentile_basis_points
        .clamp(0, BASIS_POINTS)
        .checked_mul(count)?
        / BASIS_POINTS;
    let index = usize::try_from(rank.clamp(0, count - 1)).ok()?;
    sorted.get(index).copied()
}

/// Time to value across a cohort, as a distribution.
///
/// Reported as percentiles, **never as a mean**: these distributions are
/// long-tailed, and an average that nobody experiences is not a promise
/// anyone can keep.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeToValue {
    /// Plans contributing an observation.
    pub observations: usize,
    /// Median days.
    pub p50_days: Option<i64>,
    /// 85th-percentile days — the one a commitment is made on.
    pub p85_days: Option<i64>,
    /// Why absent.
    pub absent: Option<Absent>,
}

/// Build the time-to-value distribution from first-measurable points.
#[must_use]
pub fn time_to_value(values: &[ValueFact]) -> TimeToValue {
    let days: Vec<i64> = values
        .iter()
        .filter(|v| v.is_first_measurable)
        .filter_map(|v| v.days_from_approval)
        .filter(|d| *d >= 0)
        .collect();
    if days.is_empty() {
        return TimeToValue {
            observations: 0,
            p50_days: None,
            p85_days: None,
            absent: Some(Absent::Unrealized),
        };
    }
    TimeToValue {
        observations: days.len(),
        p50_days: percentile(&days, 5_000),
        p85_days: percentile(&days, 8_500),
        absent: None,
    }
}

/// Earned-value indices. Both are `None` without a phased budget
/// baseline: a plan with no baseline is **unmeasured**, not on track,
/// and reporting `1.0` would say the opposite.
///
/// # Errors
///
/// [`Absent::NoBaseline`] when the denominator is non-positive.
pub fn earned_value_index(numerator: i64, denominator: i64) -> Result<i64, Absent> {
    if denominator <= 0 {
        return Err(Absent::NoBaseline);
    }
    numerator
        .checked_mul(BASIS_POINTS)
        .map(|scaled| scaled / denominator)
        .ok_or(Absent::Overflow)
}

/// Net Promoter Score, `-100..=100`, with the response count that
/// produced it.
///
/// An NPS without its count is not reportable — 100 from two
/// respondents is not a finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Nps {
    /// The score, or `None` when nobody responded.
    pub score: Option<i64>,
    /// Why absent.
    pub absent: Option<Absent>,
    /// Responses behind it — always reported.
    pub responses: usize,
    /// Promoters (9–10).
    pub promoters: usize,
    /// Passives (7–8).
    pub passives: usize,
    /// Detractors (0–6).
    pub detractors: usize,
}

/// Compute NPS from raw 0–10 scores.
#[must_use]
pub fn nps(scores: &[u8]) -> Nps {
    let total = scores.len();
    if total == 0 {
        return Nps {
            score: None,
            absent: Some(Absent::NoResponses),
            responses: 0,
            promoters: 0,
            passives: 0,
            detractors: 0,
        };
    }
    let promoters = scores.iter().filter(|s| **s >= 9).count();
    let passives = scores.iter().filter(|s| (7..=8).contains(*s)).count();
    let detractors = total - promoters - passives;
    let pct = |n: usize| -> i64 {
        i64::try_from(n)
            .ok()
            .and_then(|n| n.checked_mul(100))
            .map_or(0, |scaled| scaled / i64::try_from(total).unwrap_or(1))
    };
    Nps {
        score: Some(pct(promoters) - pct(detractors)),
        absent: None,
        responses: total,
        promoters,
        passives,
        detractors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(v: i64, method: Method) -> ValueFact {
        ValueFact {
            value: v,
            method,
            is_first_measurable: false,
            days_from_approval: None,
        }
    }

    /// ROI ships its inputs and the evidence mix behind them.
    #[test]
    fn roi_reports_its_inputs_and_evidence_mix() {
        let r = roi(
            &[
                value(1_500_000, Method::Measured),
                value(500_000, Method::Asserted),
            ],
            1_000_000,
        );
        assert_eq!(
            r.basis_points,
            Some(10_000),
            "2m realized on 1m invested = 100%"
        );
        assert_eq!(r.realized_minor, 2_000_000);
        assert_eq!(r.investment_minor, 1_000_000);
        assert_eq!(
            r.measured_share_basis_points,
            Some(5_000),
            "half the value is measured; the reader should see that"
        );
    }

    /// **No value points is `unrealized`, never `-100%`** — the plan has
    /// not failed to deliver, it has not been measured.
    #[test]
    fn no_value_points_is_unrealized_not_a_total_loss() {
        let r = roi(&[], 1_000_000);
        assert_eq!(r.basis_points, None);
        assert_eq!(r.absent, Some(Absent::Unrealized));
    }

    /// A non-positive investment has no ratio, however much value there
    /// was.
    #[test]
    fn a_zero_investment_has_no_ratio() {
        let r = roi(&[value(1, Method::Measured)], 0);
        assert_eq!(r.absent, Some(Absent::NoDenominator));
    }

    /// An unrecognised method reads as the **weakest** evidence, not the
    /// strongest.
    #[test]
    fn an_unknown_method_is_asserted_not_measured() {
        assert_eq!(Method::parse(Some("measured")), Method::Measured);
        assert_eq!(Method::parse(Some("invented")), Method::Asserted);
        assert_eq!(Method::parse(None), Method::Asserted);
    }

    /// Value Realization Rate counts **initiatives**, and refuses an
    /// empty denominator.
    #[test]
    fn value_realization_rate_counts_initiatives() {
        assert_eq!(value_realization_rate(10, 9), Ok(9_000));
        assert_eq!(value_realization_rate(0, 0), Err(Absent::NoDenominator));
    }

    /// Adoption with no target is refused rather than computed.
    #[test]
    fn adoption_needs_a_denominator() {
        assert_eq!(adoption_rate(50, 200), Ok(2_500));
        assert_eq!(adoption_rate(50, 0), Err(Absent::NoDenominator));
        assert_eq!(adoption_rate(50, -1), Err(Absent::NoDenominator));
    }

    /// Percentiles are nearest-rank, so every figure is an observed
    /// value rather than an interpolation.
    #[test]
    fn percentiles_are_observed_values() {
        let days = [10, 20, 30, 40, 100];
        assert_eq!(percentile(&days, 5_000), Some(30));
        assert_eq!(percentile(&days, 8_500), Some(100));
        assert_eq!(percentile(&[], 5_000), None);
        // Every answer is a member of the input.
        assert!(days.contains(&percentile(&days, 5_000).unwrap()));
    }

    /// Time to value is a distribution, and empty is `unrealized`.
    #[test]
    fn time_to_value_is_a_distribution_not_a_mean() {
        let facts: Vec<ValueFact> = [10_i64, 20, 30, 40, 100]
            .iter()
            .map(|d| ValueFact {
                value: 1,
                method: Method::Measured,
                is_first_measurable: true,
                days_from_approval: Some(*d),
            })
            .collect();
        let ttv = time_to_value(&facts);
        assert_eq!(ttv.observations, 5);
        assert_eq!(ttv.p50_days, Some(30));
        assert_eq!(ttv.p85_days, Some(100));

        let empty = time_to_value(&[value(1, Method::Measured)]);
        assert_eq!(empty.absent, Some(Absent::Unrealized));
        assert_eq!(empty.p50_days, None);
    }

    /// **SPI/CPI without a baseline is `null` with a reason, never
    /// `1.0`** — a project with no baseline is unmeasured, and `1.0`
    /// would say "exactly on plan".
    #[test]
    fn no_baseline_is_never_reported_as_on_plan() {
        assert_eq!(earned_value_index(500, 1_000), Ok(5_000));
        assert_eq!(earned_value_index(500, 0), Err(Absent::NoBaseline));
        assert_ne!(earned_value_index(500, 0), Ok(BASIS_POINTS));
    }

    /// NPS always carries its response count; zero responses is `None`,
    /// not a score of zero.
    #[test]
    fn nps_always_carries_its_response_count() {
        let n = nps(&[10, 10, 9, 8, 7, 6, 0]);
        assert_eq!(n.responses, 7);
        assert_eq!(n.promoters, 3);
        assert_eq!(n.passives, 2);
        assert_eq!(n.detractors, 2);
        // 42% promoters − 28% detractors = 14.
        assert_eq!(n.score, Some(14));

        let none = nps(&[]);
        assert_eq!(none.score, None);
        assert_eq!(none.absent, Some(Absent::NoResponses));
        assert_eq!(none.responses, 0);
    }

    /// Untrusted values must not panic.
    #[test]
    fn extreme_values_are_total() {
        let r = roi(&[value(i64::MAX, Method::Measured); 1], 1);
        assert!(r.basis_points.is_none() || r.basis_points.is_some());
        assert_eq!(adoption_rate(i64::MAX, 1), Err(Absent::Overflow));
        assert_eq!(earned_value_index(i64::MAX, 1), Err(Absent::Overflow));
        let _ = percentile(&[i64::MIN, i64::MAX], 10_000);
        let _ = nps(&[255; 3]);
    }
}
