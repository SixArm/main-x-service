//! Total Project Control (TPC) — pure rules for Stephen Devaux's
//! Devaux's Index of Project Performance (DIPP), Expected Monetary
//! Value (EMV), and Cost Estimate to Complete (CEC). DB-free and
//! exhaustively unit-tested (entity spec §5.9.7 / FR-37).
//!
//! **DIPP = EMV / CEC.** Above 1.0 the value still to come exceeds the
//! money still to spend and continuing is rational; below 1.0 it is
//! not, *whatever has already been spent*. Sunk cost appears nowhere in
//! the formula, which is the whole point of it: earned value asks
//! whether we are conforming to a baseline, DIPP asks whether we should
//! still be here.
//!
//! Three conventions, shared with [`crate::strategy`]:
//!
//! - **Money is minor units in `i64`** and **ratios are basis points**
//!   (`10_000` = 1.0). No float touches a currency figure.
//! - **Undefined returns `None` with a reason**, never a sentinel and
//!   never a panic — `CEC = 0` is the end of a project, not an
//!   infinitely good one.
//! - **Arithmetic is checked.** These are operator-supplied estimates,
//!   so the inputs are untrusted (`agents/share/security.md`
//!   invariant 2).

use serde::{Deserialize, Serialize};

/// Basis-point scale: `10_000` basis points is a ratio of 1.0.
pub const BASIS_POINTS: i64 = 10_000;

/// Break-even DIPP: remaining value exactly equals remaining cost.
pub const BREAK_EVEN_BASIS_POINTS: i64 = BASIS_POINTS;

/// Why a ratio could not be computed. Reported beside the `None` so a
/// reader never has to guess whether the figure is missing, undefined,
/// or zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Undefined {
    /// The denominator is zero: nothing left to spend.
    ZeroDenominator,
    /// The denominator is negative, which no cost estimate can be.
    NegativeDenominator,
    /// The multiplication or subtraction would overflow `i64`.
    Overflow,
}

/// How a DIPP reads against break-even. Deliberately three coarse
/// bands, not a score: 1.0 is the only threshold Devaux's arithmetic
/// actually supplies, and inventing more would dress a judgement as a
/// measurement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DippBand {
    /// EMV is negative: finishing is worth less than nothing.
    ValueDestroying,
    /// Positive value, but less than the cost still to come.
    BelowBreakEven,
    /// Remaining value at least covers remaining cost.
    AtOrAboveBreakEven,
}

/// One TPC observation for a plan, as stored (entity spec §5.9.7).
/// Money in minor units of `currency`; every field is an **asserted**
/// estimate, not an observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TpcFacts {
    /// ISO 4217 code. Comparisons never cross it.
    pub currency: String,
    /// The stored DIPP, which may carry TPC time-value terms (an
    /// acceleration premium or delay cost) that EMV alone does not.
    /// Basis points.
    pub dipp: Option<i64>,
    /// Expected Monetary Value, minor units. May be **negative**.
    pub expected_monetary_value: i64,
    /// Cost Estimate to Complete, minor units. Never negative.
    pub cost_estimate_to_complete: i64,
    /// Actual DIPP at this point in the schedule, basis points.
    pub dipp_progress_index_numerator: Option<i64>,
    /// Baseline DIPP at the same point, basis points.
    pub dipp_progress_index_denominator: Option<i64>,
}

/// The derived view over [`TpcFacts`]. Every ratio ships its own
/// inputs, so a reader can check the arithmetic rather than trust it
/// (the response convention in entity spec §9.2c).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TpcReport {
    /// Currency of every money figure here.
    pub currency: String,
    /// DIPP computed from this record's own EMV and CEC, basis points.
    pub computed_dipp: Option<i64>,
    /// Why `computed_dipp` is absent, when it is.
    pub computed_dipp_undefined: Option<Undefined>,
    /// The stored DIPP, echoed unchanged.
    pub stored_dipp: Option<i64>,
    /// Stored minus computed, when both exist. Non-zero is a
    /// **finding**, not an error: the stored value legitimately carries
    /// time-value terms EMV does not.
    pub dipp_divergence: Option<i64>,
    /// How `computed_dipp` reads against break-even.
    pub band: Option<DippBand>,
    /// Actual ÷ baseline DIPP, basis points. At or above `10_000` the
    /// project tracks its own plan.
    pub progress_index: Option<i64>,
    /// Why `progress_index` is absent, when it is.
    pub progress_index_undefined: Option<Undefined>,
    /// Echoed inputs, so the ratios are checkable.
    pub expected_monetary_value: i64,
    /// Echoed input.
    pub cost_estimate_to_complete: i64,
    /// Always true: TPC figures are estimates, never observations
    /// (entity spec §5.9.6).
    pub asserted: bool,
}

/// A ratio in basis points: `numerator / denominator * 10_000`.
///
/// A **negative numerator is legitimate** and passes through — a
/// project can be worth less than nothing to finish. A zero or negative
/// denominator is undefined, because no cost estimate to complete is
/// negative and nothing-left-to-spend is the end of a project rather
/// than an infinitely good one.
///
/// # Errors
/// Returns the [`Undefined`] reason rather than a sentinel.
pub fn ratio_basis_points(numerator: i64, denominator: i64) -> Result<i64, Undefined> {
    if denominator == 0 {
        return Err(Undefined::ZeroDenominator);
    }
    if denominator < 0 {
        return Err(Undefined::NegativeDenominator);
    }
    numerator
        .checked_mul(BASIS_POINTS)
        .map(|scaled| scaled / denominator)
        .ok_or(Undefined::Overflow)
}

/// DIPP = EMV / CEC, in basis points.
///
/// # Errors
/// See [`ratio_basis_points`].
pub fn dipp_basis_points(
    expected_monetary_value: i64,
    cost_estimate_to_complete: i64,
) -> Result<i64, Undefined> {
    ratio_basis_points(expected_monetary_value, cost_estimate_to_complete)
}

/// DIPP Progress Index = actual DIPP / baseline DIPP, in basis points.
///
/// # Errors
/// See [`ratio_basis_points`].
pub fn progress_index_basis_points(numerator: i64, denominator: i64) -> Result<i64, Undefined> {
    ratio_basis_points(numerator, denominator)
}

/// Classify a DIPP against break-even. `emv` decides the
/// value-destroying band, because a negative EMV over a positive CEC is
/// already negative and the sign is the finding.
#[must_use]
pub fn band(dipp: i64, expected_monetary_value: i64) -> DippBand {
    if expected_monetary_value < 0 {
        DippBand::ValueDestroying
    } else if dipp < BREAK_EVEN_BASIS_POINTS {
        DippBand::BelowBreakEven
    } else {
        DippBand::AtOrAboveBreakEven
    }
}

/// Build the derived view. Pure: no clock, no I/O, total over every
/// input including the degenerate ones.
#[must_use]
pub fn report(facts: &TpcFacts) -> TpcReport {
    let computed = dipp_basis_points(
        facts.expected_monetary_value,
        facts.cost_estimate_to_complete,
    );
    let (computed_dipp, computed_dipp_undefined) = match computed {
        Ok(value) => (Some(value), None),
        Err(reason) => (None, Some(reason)),
    };

    let progress = match (
        facts.dipp_progress_index_numerator,
        facts.dipp_progress_index_denominator,
    ) {
        (Some(numerator), Some(denominator)) => {
            match progress_index_basis_points(numerator, denominator) {
                Ok(value) => (Some(value), None),
                Err(reason) => (None, Some(reason)),
            }
        }
        _ => (None, None),
    };

    TpcReport {
        currency: facts.currency.clone(),
        computed_dipp,
        computed_dipp_undefined,
        stored_dipp: facts.dipp,
        dipp_divergence: match (facts.dipp, computed_dipp) {
            (Some(stored), Some(derived)) => stored.checked_sub(derived),
            _ => None,
        },
        band: computed_dipp.map(|value| band(value, facts.expected_monetary_value)),
        progress_index: progress.0,
        progress_index_undefined: progress.1,
        expected_monetary_value: facts.expected_monetary_value,
        cost_estimate_to_complete: facts.cost_estimate_to_complete,
        asserted: true,
    }
}

/// Rank plans for triage: **highest DIPP first**, which is the use
/// Devaux intends — scarce resources go to the project returning most
/// per remaining pound.
///
/// Two rules that keep the ranking honest:
///
/// - **One currency only.** Entries in another currency are dropped and
///   returned separately rather than silently compared; this crate
///   never converts (entity spec §6.4a).
/// - **An undefined DIPP is not a zero.** Such entries are excluded
///   from the ranking and reported, rather than sorting last as if
///   measured and bad.
#[must_use]
pub fn triage(
    entries: &[(uuid::Uuid, TpcFacts)],
    currency: &str,
) -> (Vec<(uuid::Uuid, i64)>, Vec<uuid::Uuid>, Vec<uuid::Uuid>) {
    let mut ranked: Vec<(uuid::Uuid, i64)> = Vec::new();
    let mut wrong_currency: Vec<uuid::Uuid> = Vec::new();
    let mut undefined: Vec<uuid::Uuid> = Vec::new();

    for (pid, facts) in entries {
        if facts.currency != currency {
            wrong_currency.push(*pid);
            continue;
        }
        match dipp_basis_points(
            facts.expected_monetary_value,
            facts.cost_estimate_to_complete,
        ) {
            Ok(value) => ranked.push((*pid, value)),
            Err(_) => undefined.push(*pid),
        }
    }

    // Descending by DIPP, then by pid so the order is deterministic
    // across processes rather than merely correct in content.
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    wrong_currency.sort_unstable();
    undefined.sort_unstable();
    (ranked, wrong_currency, undefined)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(emv: i64, cec: i64) -> TpcFacts {
        TpcFacts {
            currency: "GBP".to_string(),
            dipp: None,
            expected_monetary_value: emv,
            cost_estimate_to_complete: cec,
            dipp_progress_index_numerator: None,
            dipp_progress_index_denominator: None,
        }
    }

    /// DIPP is EMV over CEC: twice the value for the money left is 2.0.
    #[test]
    fn dipp_is_emv_over_cec() {
        assert_eq!(dipp_basis_points(2_000_000, 1_000_000), Ok(20_000));
        assert_eq!(
            dipp_basis_points(1_000_000, 1_000_000),
            Ok(BREAK_EVEN_BASIS_POINTS)
        );
        assert_eq!(dipp_basis_points(500_000, 1_000_000), Ok(5_000));
    }

    /// Nothing left to spend is the end of a project, **not** an
    /// infinitely good one — the regression test against a sentinel.
    #[test]
    fn zero_cost_to_complete_is_undefined_not_infinite() {
        assert_eq!(
            dipp_basis_points(1_000_000, 0),
            Err(Undefined::ZeroDenominator)
        );
        let r = report(&facts(1_000_000, 0));
        assert_eq!(r.computed_dipp, None);
        assert_eq!(r.computed_dipp_undefined, Some(Undefined::ZeroDenominator));
        assert_eq!(r.band, None);
    }

    /// A negative EMV is legitimate and is **not** clamped: a project
    /// worth less than nothing to finish is what the metric exists to
    /// expose.
    #[test]
    fn negative_expected_monetary_value_survives() {
        assert_eq!(dipp_basis_points(-500_000, 1_000_000), Ok(-5_000));
        let r = report(&facts(-500_000, 1_000_000));
        assert_eq!(r.computed_dipp, Some(-5_000));
        assert_eq!(r.band, Some(DippBand::ValueDestroying));
    }

    /// No cost estimate to complete is negative.
    #[test]
    fn negative_cost_to_complete_is_undefined() {
        assert_eq!(
            dipp_basis_points(1_000_000, -1),
            Err(Undefined::NegativeDenominator)
        );
    }

    /// Untrusted operator input must never panic (security invariant 2).
    #[test]
    fn extreme_inputs_never_panic() {
        assert_eq!(dipp_basis_points(i64::MAX, 1), Err(Undefined::Overflow));
        assert_eq!(dipp_basis_points(i64::MIN, 1), Err(Undefined::Overflow));
        assert_eq!(dipp_basis_points(0, 1), Ok(0));
        assert_eq!(
            dipp_basis_points(i64::MIN, i64::MAX),
            Err(Undefined::Overflow)
        );
        // A report over the same inputs is total, not panicking.
        let _ = report(&facts(i64::MIN, i64::MAX));
        let _ = report(&facts(i64::MAX, 0));
    }

    /// The progress index is actual over baseline; at or above 1.0 the
    /// project tracks its own plan.
    #[test]
    fn progress_index_compares_actual_to_baseline() {
        let mut f = facts(1_000_000, 1_000_000);
        f.dipp_progress_index_numerator = Some(12_000);
        f.dipp_progress_index_denominator = Some(10_000);
        assert_eq!(report(&f).progress_index, Some(12_000));

        f.dipp_progress_index_numerator = Some(8_000);
        assert_eq!(report(&f).progress_index, Some(8_000));

        // A zero baseline is undefined, not infinite.
        f.dipp_progress_index_denominator = Some(0);
        let r = report(&f);
        assert_eq!(r.progress_index, None);
        assert_eq!(r.progress_index_undefined, Some(Undefined::ZeroDenominator));
    }

    /// A missing half of the progress index reports neither a value nor
    /// an "undefined" reason — it is absent, which is a third thing.
    #[test]
    fn absent_progress_index_is_not_undefined() {
        let r = report(&facts(1_000_000, 1_000_000));
        assert_eq!(r.progress_index, None);
        assert_eq!(r.progress_index_undefined, None);
    }

    /// A stored DIPP carrying time-value terms diverges from EMV/CEC by
    /// design; the report states the gap rather than preferring either.
    #[test]
    fn divergence_is_reported_not_resolved() {
        let mut f = facts(1_000_000, 1_000_000);
        f.dipp = Some(12_000); // an acceleration premium, say
        let r = report(&f);
        assert_eq!(r.computed_dipp, Some(10_000));
        assert_eq!(r.stored_dipp, Some(12_000));
        assert_eq!(r.dipp_divergence, Some(2_000));
    }

    /// Triage ranks by DIPP descending; a foreign currency is set aside
    /// rather than compared, and an undefined DIPP is set aside rather
    /// than sorted last as if it were zero.
    #[test]
    fn triage_ranks_and_sets_aside() {
        let a = uuid::Uuid::from_u128(1);
        let b = uuid::Uuid::from_u128(2);
        let c = uuid::Uuid::from_u128(3);
        let d = uuid::Uuid::from_u128(4);
        let mut euro = facts(9_000_000, 1_000_000);
        euro.currency = "EUR".to_string();

        let (ranked, wrong_currency, undefined) = triage(
            &[
                (a, facts(1_000_000, 1_000_000)), // 1.0
                (b, facts(3_000_000, 1_000_000)), // 3.0
                (c, facts(1_000_000, 0)),         // undefined
                (d, euro),                        // other currency
            ],
            "GBP",
        );

        assert_eq!(ranked, vec![(b, 30_000), (a, 10_000)]);
        assert_eq!(wrong_currency, vec![d]);
        assert_eq!(undefined, vec![c]);
    }

    /// Sunk cost appears nowhere: two plans with identical remaining
    /// value and remaining cost rank identically however much has
    /// already been spent on either. This is the property the whole
    /// metric exists for.
    #[test]
    fn sunk_cost_cannot_influence_the_figure() {
        let cheap = dipp_basis_points(2_000_000, 500_000);
        let ruinous = dipp_basis_points(2_000_000, 500_000);
        assert_eq!(cheap, ruinous);
    }

    /// Every figure is labelled asserted, because a TPC input is an
    /// estimate a person supplied.
    #[test]
    fn report_is_labelled_asserted() {
        assert!(report(&facts(1, 1)).asserted);
    }
}
