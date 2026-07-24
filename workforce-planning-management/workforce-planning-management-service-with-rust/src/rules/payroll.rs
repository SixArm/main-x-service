//! Payslip arithmetic (HCM-R13, HCM-D4/D5), DB-free: minor units
//! only, every operation overflow-checked, and the reconciliation
//! invariant `net = gross − Σ deductions` enforced by construction
//! and re-checked before persist.
//!
//! The tax table is a deliberate **stub** (demo software): a flat
//! `TAX_RATE_PERCENT` above a monthly `TAX_FREE_MINOR` allowance.
//! Production requires jurisdiction-correct tables (spec
//! `regulatory.md`; gate HCM-G2).

/// One deduction line on a payslip.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Deduction {
    /// Human label (`tax`, `pension`, a benefit plan name, …).
    pub label: String,
    /// Amount in minor units (non-negative).
    pub amount_minor: i64,
}

/// A computed payslip: gross, the deduction lines, and the net.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Payslip {
    /// Gross pay in minor units.
    pub gross_minor: i64,
    /// The deduction lines.
    pub deductions: Vec<Deduction>,
    /// Net pay: always `gross - Σ deductions`.
    pub net_minor: i64,
}

/// Stub flat tax rate (percent) above the allowance.
pub const TAX_RATE_PERCENT: i64 = 20;
/// Stub monthly tax-free allowance, minor units (£1,047.50 ≈ 12570/12).
pub const TAX_FREE_MINOR: i64 = 104_750;
/// Assumed working minutes per month for the hourly rate
/// (`CONTRACTED_DAY_MINUTES` × ~21.67 working days).
pub const MONTH_MINUTES: i64 = 9750;

/// Monthly base pay: `annual × fte% / 100 / 12`, overflow-refused.
///
/// # Errors
///
/// `"overflow"`-flavoured message when the arithmetic overflows, or a
/// refusal for a non-positive salary / out-of-range FTE.
pub fn monthly_base_minor(annual_salary_minor: i64, fte_percent: i32) -> Result<i64, String> {
    if annual_salary_minor < 0 {
        return Err("salary must be non-negative".to_string());
    }
    if !(1..=100).contains(&fte_percent) {
        return Err(format!("fte_percent {fte_percent} out of range 1-100"));
    }
    annual_salary_minor
        .checked_mul(i64::from(fte_percent))
        .and_then(|v| v.checked_div(100))
        .and_then(|v| v.checked_div(12))
        .ok_or_else(|| "salary arithmetic overflows".to_string())
}

/// Overtime pay for the period: `minutes × hourly`, where hourly
/// derives from the monthly base over [`MONTH_MINUTES`].
///
/// # Errors
///
/// On arithmetic overflow.
pub fn overtime_pay_minor(monthly_base_minor: i64, overtime_minutes: i64) -> Result<i64, String> {
    if overtime_minutes <= 0 {
        return Ok(0);
    }
    monthly_base_minor
        .checked_mul(overtime_minutes)
        .and_then(|v| v.checked_div(MONTH_MINUTES))
        .ok_or_else(|| "overtime arithmetic overflows".to_string())
}

/// The stub tax line: `TAX_RATE_PERCENT`% of gross above
/// [`TAX_FREE_MINOR`], floor zero.
///
/// # Errors
///
/// On arithmetic overflow.
pub fn stub_tax_minor(gross_minor: i64) -> Result<i64, String> {
    let taxable = (gross_minor - TAX_FREE_MINOR).max(0);
    taxable
        .checked_mul(TAX_RATE_PERCENT)
        .map(|v| v / 100)
        .ok_or_else(|| "tax arithmetic overflows".to_string())
}

/// Compute one payslip from the period inputs: monthly base (salary ×
/// FTE), overtime pay, then deductions (stub tax + the benefit
/// employee-costs). The net is derived, never supplied.
///
/// # Errors
///
/// On overflow anywhere, or a negative benefit cost.
pub fn compute_payslip(
    annual_salary_minor: i64,
    fte_percent: i32,
    overtime_minutes: i64,
    benefit_costs: &[(String, i64)],
) -> Result<Payslip, String> {
    let base = monthly_base_minor(annual_salary_minor, fte_percent)?;
    let overtime = overtime_pay_minor(base, overtime_minutes)?;
    let gross = base
        .checked_add(overtime)
        .ok_or_else(|| "gross overflows".to_string())?;
    let mut deductions = vec![Deduction {
        label: "tax".to_string(),
        amount_minor: stub_tax_minor(gross)?,
    }];
    for (label, cost) in benefit_costs {
        if *cost < 0 {
            return Err(format!("benefit cost for {label} is negative"));
        }
        deductions.push(Deduction {
            label: label.clone(),
            amount_minor: *cost,
        });
    }
    let total: i64 = deductions
        .iter()
        .try_fold(0_i64, |acc, d| acc.checked_add(d.amount_minor))
        .ok_or_else(|| "deductions overflow".to_string())?;
    let net = gross
        .checked_sub(total)
        .ok_or_else(|| "net overflows".to_string())?;
    let slip = Payslip {
        gross_minor: gross,
        deductions,
        net_minor: net,
    };
    reconcile(&slip)?;
    Ok(slip)
}

/// The persist-gate reconciliation: `net = gross − Σ deductions`
/// (HCM-R13). Called by [`compute_payslip`] and again by the
/// controller before insert, so a hand-constructed slip cannot lie.
///
/// # Errors
///
/// When the invariant does not hold or the sum overflows.
pub fn reconcile(slip: &Payslip) -> Result<(), String> {
    let total: i64 = slip
        .deductions
        .iter()
        .try_fold(0_i64, |acc, d| acc.checked_add(d.amount_minor))
        .ok_or_else(|| "deductions overflow".to_string())?;
    let expected = slip
        .gross_minor
        .checked_sub(total)
        .ok_or_else(|| "net overflows".to_string())?;
    if expected == slip.net_minor {
        Ok(())
    } else {
        Err(format!(
            "payslip does not reconcile: gross {} - deductions {} != net {}",
            slip.gross_minor, total, slip.net_minor
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Base pay pro-rates by FTE and refuses junk inputs.
    #[test]
    fn monthly_base_pro_rates() {
        assert_eq!(monthly_base_minor(3_600_000, 100).unwrap(), 300_000);
        assert_eq!(monthly_base_minor(3_600_000, 50).unwrap(), 150_000);
        assert!(monthly_base_minor(-1, 100).is_err());
        assert!(monthly_base_minor(100, 0).is_err());
        assert!(monthly_base_minor(100, 101).is_err());
        assert!(monthly_base_minor(i64::MAX, 99).unwrap_err().contains("overflow"));
    }

    /// Overtime pay derives from the monthly rate; non-positive
    /// minutes cost nothing.
    #[test]
    fn overtime_pay() {
        // £3,000.00 monthly => hourly-ish rate over 9750 min.
        assert_eq!(overtime_pay_minor(300_000, MONTH_MINUTES).unwrap(), 300_000);
        assert_eq!(overtime_pay_minor(300_000, 0).unwrap(), 0);
        assert_eq!(overtime_pay_minor(300_000, -60).unwrap(), 0);
        assert!(overtime_pay_minor(i64::MAX, 2).is_err());
    }

    /// The stub tax kicks in only above the allowance.
    #[test]
    fn stub_tax() {
        assert_eq!(stub_tax_minor(100_000).unwrap(), 0);
        assert_eq!(stub_tax_minor(TAX_FREE_MINOR).unwrap(), 0);
        assert_eq!(stub_tax_minor(TAX_FREE_MINOR + 100_000).unwrap(), 20_000);
    }

    /// A full slip reconciles by construction: gross = base + overtime,
    /// net = gross − tax − benefits.
    #[test]
    fn compute_reconciles() {
        let slip = compute_payslip(
            3_600_000, // £36,000.00
            100,
            0,
            &[("pension".to_string(), 15_000)],
        )
        .unwrap();
        assert_eq!(slip.gross_minor, 300_000);
        let tax = stub_tax_minor(300_000).unwrap();
        assert_eq!(slip.net_minor, 300_000 - tax - 15_000);
        assert!(reconcile(&slip).is_ok());
    }

    /// Tampering with the net fails the persist gate; a negative
    /// benefit cost is refused outright.
    #[test]
    fn reconcile_gate_catches_lies() {
        let mut slip = compute_payslip(3_600_000, 100, 0, &[]).unwrap();
        slip.net_minor += 1;
        assert!(reconcile(&slip).unwrap_err().contains("reconcile"));
        assert!(compute_payslip(3_600_000, 100, 0, &[("x".to_string(), -1)]).is_err());
    }

    /// Overflow anywhere is an error, never a wrap or a panic.
    #[test]
    fn overflow_is_refused() {
        assert!(compute_payslip(i64::MAX, 100, i64::MAX, &[]).is_err());
        let slip = Payslip {
            gross_minor: 0,
            deductions: vec![
                Deduction { label: "a".into(), amount_minor: i64::MAX },
                Deduction { label: "b".into(), amount_minor: 1 },
            ],
            net_minor: 0,
        };
        assert!(reconcile(&slip).unwrap_err().contains("overflow"));
    }
}
