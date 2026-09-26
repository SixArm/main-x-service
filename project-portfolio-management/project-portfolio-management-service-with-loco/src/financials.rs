//! Pure rules for the phased budget baseline and the EAC/ETC forecast
//! (T-28b, `spec/13-tasks.md`). DB-free and exhaustively unit-tested.
//!
//! ## What this unblocks, and what it deliberately does not
//!
//! A baseline makes an **estimate-at-completion (EAC) forecast**
//! possible: `EAC = AC + ETC`, actual cost plus the estimate to
//! complete. It does **not** make SPI/CPI possible — those need
//! **earned value** (percent complete × baseline), and nothing in this
//! service computes percent complete as a trustworthy figure yet.
//! T-28b's own acceptance criteria test EAC/ETC/forecast/rollup, never
//! a real SPI/CPI number — `GET /plans/{pid}/performance`'s `spi`/`cpi`
//! stay `null` with a reason (`src/controllers/value.rs::performance`),
//! now naming the missing earned-value signal rather than a missing
//! baseline once a plan has one. Wiring a real earned-value signal is
//! a separate, undecided question (what counts as "percent complete"?
//! story points? task count? a phase-weighted schedule?) — not guessed
//! here.
//!
//! ## Money conventions
//!
//! Integer minor units, ISO-4217 currency codes, no float — the same
//! posture as every other money figure in this crate.

use serde::{Deserialize, Serialize};

/// One period of a budget baseline: planned cost for one window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaselinePeriod {
    /// Inclusive.
    pub period_start: chrono::NaiveDate,
    /// Inclusive.
    pub period_end: chrono::NaiveDate,
    /// Minor units.
    pub planned_minor: i64,
}

/// One approved, versioned budget baseline. Frozen at approval —
/// nothing here is ever edited; a re-baseline is a new [`Baseline`] at
/// `version + 1`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Baseline {
    /// `1` for the initial approval, incrementing per re-baseline.
    pub version: i32,
    /// ISO-4217. One baseline, one currency.
    pub currency: String,
    /// At least one, enforced at write.
    pub periods: Vec<BaselinePeriod>,
}

impl Baseline {
    /// Planned cost of periods not yet fully elapsed as of `today` —
    /// the fallback estimate-to-complete when no TPC observation
    /// exists.
    ///
    /// A period ending before `today` is elapsed and contributes
    /// nothing; a period straddling or starting after `today`
    /// contributes its **whole** planned cost, never a pro-rated
    /// share — this service has no record of how much of a period's
    /// work is done, only whether the window has passed, and
    /// inventing a pro-ration would be a fabricated precision.
    #[must_use]
    pub fn remaining_planned_minor(&self, today: chrono::NaiveDate) -> i64 {
        self.periods
            .iter()
            .filter(|p| p.period_end >= today)
            .fold(0_i64, |acc, p| acc.saturating_add(p.planned_minor))
    }
}

/// Where a forecast's estimate-to-complete came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EtcSource {
    /// The plan's own latest TPC observation's cost-estimate-to-complete.
    Tpc,
    /// The baseline's own not-yet-elapsed periods, summed.
    BaselineRemaining,
}

/// Why a forecast is absent. The **only** case: neither a baseline nor
/// a TPC observation names a currency to forecast in. Once a currency
/// is known — from either source — an estimate to complete always
/// resolves too: a TPC observation carries its own CEC, and a
/// baseline's [`Baseline::remaining_planned_minor`] always returns an
/// answer, `0` included (all periods elapsed is a real, meaningful
/// "nothing left", not an absence). There is deliberately no second
/// variant for "currency known, ETC unresolvable" — that state cannot
/// occur given how `currency` and the ETC source are resolved
/// together in [`forecast`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForecastAbsent {
    /// Neither a baseline nor a TPC observation names a currency to
    /// forecast in — there is nothing to forecast *in*.
    NoCurrencySignal,
}

/// One plan's estimate-at-completion forecast: `EAC = AC + ETC`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Forecast {
    /// `None` only when [`ForecastAbsent::NoCurrencySignal`] applies.
    pub currency: Option<String>,
    /// Actual cost (AC): summed `budget_lines.actual_minor` in
    /// `currency`.
    pub actual_minor: i64,
    /// Actual cost recorded in a **different** currency than the
    /// forecast's own — disclosed, never silently dropped or summed
    /// in (the family's mixed-currency rule, `src/value.rs`'s
    /// `Absent::MixedCurrency` precedent, applied here as a named
    /// side-total rather than a withheld whole figure, since AC in
    /// the forecast's own currency is still reportable on its own).
    pub excluded_other_currency_minor: i64,
    /// Estimate to complete (ETC), or `None` when neither source
    /// yields one.
    pub estimate_to_complete_minor: Option<i64>,
    /// Where `estimate_to_complete_minor` came from, when present.
    pub etc_source: Option<EtcSource>,
    /// `AC + ETC`, or `None` when `estimate_to_complete_minor` is.
    pub estimate_at_completion_minor: Option<i64>,
    /// The baseline version this forecast used — as the currency
    /// anchor, the ETC source, or both — when a baseline exists at
    /// all.
    pub baseline_version: Option<i32>,
    /// Why absent, when either the currency or the ETC could not be
    /// resolved.
    pub absent: Option<ForecastAbsent>,
}

/// Build one plan's forecast.
///
/// `actual_lines` is every `(currency, actual_minor)` pair recorded
/// against the plan (from `budget_lines`); `tpc_cec` is the latest TPC
/// observation's own `(currency, cost_estimate_to_complete)`, if any;
/// `baseline` is the plan's latest approved baseline, if any.
///
/// **The baseline is the currency anchor when one exists** — it is the
/// more durable record (frozen at approval; a TPC observation can
/// lapse) — falling back to the latest TPC observation's own currency
/// when there is no baseline at all.
#[must_use]
pub fn forecast(
    actual_lines: &[(String, i64)],
    tpc_cec: Option<(&str, i64)>,
    baseline: Option<&Baseline>,
    today: chrono::NaiveDate,
) -> Forecast {
    let currency = baseline
        .map(|b| b.currency.clone())
        .or_else(|| tpc_cec.map(|(c, _)| c.to_string()));

    let Some(currency) = currency else {
        return Forecast {
            currency: None,
            actual_minor: 0,
            excluded_other_currency_minor: 0,
            estimate_to_complete_minor: None,
            etc_source: None,
            estimate_at_completion_minor: None,
            baseline_version: None,
            absent: Some(ForecastAbsent::NoCurrencySignal),
        };
    };

    let actual_minor = actual_lines
        .iter()
        .filter(|(c, _)| *c == currency)
        .fold(0_i64, |acc, (_, m)| acc.saturating_add(*m));
    let excluded_other_currency_minor = actual_lines
        .iter()
        .filter(|(c, _)| *c != currency)
        .fold(0_i64, |acc, (_, m)| acc.saturating_add(*m));

    let (etc, source) = match tpc_cec.filter(|(c, _)| *c == currency) {
        Some((_, cec)) => (Some(cec), Some(EtcSource::Tpc)),
        None => baseline.map_or((None, None), |b| {
            (
                Some(b.remaining_planned_minor(today)),
                Some(EtcSource::BaselineRemaining),
            )
        }),
    };

    Forecast {
        currency: Some(currency),
        actual_minor,
        excluded_other_currency_minor,
        estimate_to_complete_minor: etc,
        etc_source: source,
        estimate_at_completion_minor: etc.and_then(|e| actual_minor.checked_add(e)),
        baseline_version: baseline.map(|b| b.version),
        absent: None,
    }
}

/// One currency's aggregate forecast across a walked containment
/// subtree. Every plan contributing a row has a fully resolved
/// forecast (see [`ForecastAbsent`]'s doc comment for why a known
/// currency always carries a resolvable ETC too) — there is no
/// separate "contributed AC but not ETC" count to keep.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollupForecastRow {
    /// ISO-4217.
    pub currency: String,
    /// Sum of `actual_minor` across every contributing plan.
    pub actual_minor: i64,
    /// Sum of `estimate_to_complete_minor` across every contributing
    /// plan.
    pub estimate_to_complete_minor: i64,
    /// Sum of `estimate_at_completion_minor` across every contributing
    /// plan.
    pub estimate_at_completion_minor: i64,
    /// Plans contributing to this row.
    pub plans: usize,
}

/// Roll up per-plan forecasts into per-currency rows.
///
/// **Currencies are never merged** — one row per currency, the same
/// rule `crate::insights::variance_by_currency` already holds — so a
/// subtree spanning two currencies reports two rows, never one sum.
/// Plans with [`ForecastAbsent::NoCurrencySignal`] (no baseline, no TPC
/// observation) contribute to neither `rows` nor a currency at all;
/// `no_currency_signal` counts them so they are disclosed rather than
/// silently absent from the total.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rollup {
    /// One row per currency found among the walked plans, currency
    /// code ascending.
    pub rows: Vec<RollupForecastRow>,
    /// Plans walked that had neither a baseline nor a TPC observation
    /// — no currency to attribute them to at all.
    pub no_currency_signal: usize,
}

/// Build the per-currency rollup from a walked subtree's individual
/// forecasts.
#[must_use]
pub fn rollup_forecast(forecasts: &[Forecast]) -> Rollup {
    let mut per: std::collections::BTreeMap<&str, RollupForecastRow> =
        std::collections::BTreeMap::new();
    let mut no_currency_signal = 0_usize;
    for f in forecasts {
        let Some(currency) = f.currency.as_deref() else {
            no_currency_signal += 1;
            continue;
        };
        let row = per.entry(currency).or_insert_with(|| RollupForecastRow {
            currency: currency.to_string(),
            actual_minor: 0,
            estimate_to_complete_minor: 0,
            estimate_at_completion_minor: 0,
            plans: 0,
        });
        row.actual_minor = row.actual_minor.saturating_add(f.actual_minor);
        row.estimate_to_complete_minor = row
            .estimate_to_complete_minor
            .saturating_add(f.estimate_to_complete_minor.unwrap_or(0));
        row.estimate_at_completion_minor = row
            .estimate_at_completion_minor
            .saturating_add(f.estimate_at_completion_minor.unwrap_or(0));
        row.plans += 1;
    }
    Rollup {
        rows: per.into_values().collect(),
        no_currency_signal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> chrono::NaiveDate {
        chrono::NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn baseline(
        currency: &str,
        version: i32,
        periods: &[(i32, u32, u32, i32, u32, u32, i64)],
    ) -> Baseline {
        Baseline {
            version,
            currency: currency.to_string(),
            periods: periods
                .iter()
                .map(|&(y1, m1, d1, y2, m2, d2, planned)| BaselinePeriod {
                    period_start: date(y1, m1, d1),
                    period_end: date(y2, m2, d2),
                    planned_minor: planned,
                })
                .collect(),
        }
    }

    /// **A plan without a baseline reports `null` + a reason,
    /// unchanged** by this task existing — the exact acceptance
    /// wording, checked at the pure layer.
    #[test]
    fn no_baseline_and_no_tpc_is_no_currency_signal() {
        let f = forecast(&[], None, None, date(2026, 9, 18));
        assert_eq!(f.currency, None);
        assert_eq!(f.absent, Some(ForecastAbsent::NoCurrencySignal));
        assert_eq!(f.estimate_at_completion_minor, None);
    }

    /// ETC prefers the TPC observation over the baseline's remaining
    /// periods, when both exist.
    #[test]
    fn tpc_cec_wins_over_baseline_remaining() {
        let b = baseline("GBP", 1, &[(2026, 1, 1, 2026, 12, 31, 100_000)]);
        let f = forecast(
            &[("GBP".to_string(), 20_000)],
            Some(("GBP", 30_000)),
            Some(&b),
            date(2026, 6, 1),
        );
        assert_eq!(f.etc_source, Some(EtcSource::Tpc));
        assert_eq!(f.estimate_to_complete_minor, Some(30_000));
        assert_eq!(f.estimate_at_completion_minor, Some(50_000));
        assert_eq!(f.baseline_version, Some(1));
    }

    /// With no TPC observation, ETC falls back to the baseline's
    /// not-yet-elapsed periods, summed.
    #[test]
    fn falls_back_to_baseline_remaining_periods() {
        let b = baseline(
            "GBP",
            1,
            &[
                (2026, 1, 1, 2026, 3, 31, 10_000), // elapsed
                (2026, 4, 1, 2026, 6, 30, 10_000), // straddles "today"
                (2026, 7, 1, 2026, 9, 30, 10_000), // future
            ],
        );
        let f = forecast(&[], None, Some(&b), date(2026, 5, 1));
        assert_eq!(f.etc_source, Some(EtcSource::BaselineRemaining));
        assert_eq!(
            f.estimate_to_complete_minor,
            Some(20_000),
            "the elapsed period contributes nothing; the straddling and future periods contribute their whole planned cost"
        );
    }

    /// A TPC observation in a *different* currency than the baseline
    /// is not used for ETC (mixing currencies silently would be worse
    /// than falling back).
    #[test]
    fn a_tpc_observation_in_another_currency_is_not_used() {
        let b = baseline("GBP", 1, &[(2026, 1, 1, 2026, 12, 31, 100_000)]);
        let f = forecast(&[], Some(("USD", 99_900)), Some(&b), date(2026, 6, 1));
        assert_eq!(f.etc_source, Some(EtcSource::BaselineRemaining));
    }

    /// Actuals in a currency other than the forecast's own are
    /// disclosed, never merged in and never silently dropped.
    #[test]
    fn other_currency_actuals_are_disclosed_not_merged() {
        let b = baseline("GBP", 1, &[(2026, 1, 1, 2026, 12, 31, 100_000)]);
        let f = forecast(
            &[("GBP".to_string(), 10_000), ("USD".to_string(), 5_000)],
            None,
            Some(&b),
            date(2026, 6, 1),
        );
        assert_eq!(f.actual_minor, 10_000);
        assert_eq!(f.excluded_other_currency_minor, 5_000);
    }

    /// An empty-periods baseline (should not occur via valid creation,
    /// but the pure function is never trusted to assume that) still
    /// resolves a real ETC of zero — not an absence. Zero remaining
    /// periods genuinely means "nothing left", which is different from
    /// not knowing.
    #[test]
    fn an_empty_baseline_resolves_a_real_zero_etc_not_an_absence() {
        let b = Baseline {
            version: 1,
            currency: "GBP".to_string(),
            periods: vec![],
        };
        let f = forecast(&[], None, Some(&b), date(2026, 6, 1));
        assert_eq!(f.currency, Some("GBP".to_string()));
        assert_eq!(f.estimate_to_complete_minor, Some(0));
        assert_eq!(f.absent, None);
    }

    /// The rollup never merges two currencies into one row.
    #[test]
    fn rollup_never_merges_two_currencies() {
        let b_gbp = baseline("GBP", 1, &[(2026, 1, 1, 2026, 12, 31, 10_000)]);
        let b_usd = baseline("USD", 1, &[(2026, 1, 1, 2026, 12, 31, 20_000)]);
        let f1 = forecast(&[], None, Some(&b_gbp), date(2026, 6, 1));
        let f2 = forecast(&[], None, Some(&b_usd), date(2026, 6, 1));
        let rollup = rollup_forecast(&[f1, f2]);
        assert_eq!(
            rollup.rows.len(),
            2,
            "two currencies, two rows, never one sum"
        );
        assert_eq!(rollup.rows[0].currency, "GBP");
        assert_eq!(rollup.rows[1].currency, "USD");
    }

    /// A plan with no currency signal at all is counted, not silently
    /// missing from the rollup.
    #[test]
    fn rollup_discloses_plans_with_no_currency_signal() {
        let none_forecast = forecast(&[], None, None, date(2026, 6, 1));
        let rollup = rollup_forecast(&[none_forecast]);
        assert_eq!(rollup.rows.len(), 0);
        assert_eq!(rollup.no_currency_signal, 1);
    }

    /// A mix of forecast and not-yet-baselined plans across a rollup:
    /// the forecast plan contributes a row, the unbaselined one is
    /// counted separately rather than silently missing.
    #[test]
    fn rollup_mixes_forecast_and_unbaselined_plans_correctly() {
        let b = baseline("GBP", 1, &[(2026, 1, 1, 2026, 12, 31, 10_000)]);
        let f_forecast = forecast(
            &[("GBP".to_string(), 1_000)],
            None,
            Some(&b),
            date(2026, 6, 1),
        );
        let f_absent = forecast(&[], None, None, date(2026, 6, 1));
        let rollup = rollup_forecast(&[f_forecast, f_absent]);
        assert_eq!(rollup.rows.len(), 1);
        assert_eq!(rollup.rows[0].plans, 1);
        assert_eq!(rollup.no_currency_signal, 1);
    }

    /// Never panics on untrusted extreme inputs.
    #[test]
    fn extreme_values_never_panic() {
        let b = baseline("GBP", 1, &[(2026, 1, 1, 2026, 12, 31, i64::MAX)]);
        let f = forecast(
            &[("GBP".to_string(), i64::MAX)],
            None,
            Some(&b),
            date(2026, 6, 1),
        );
        // Saturating arithmetic throughout: never a panic, whatever
        // the answer is.
        let _ = f.estimate_at_completion_minor;
    }
}
