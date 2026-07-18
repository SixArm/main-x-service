//! Pure PPM Phase-C strategy rules (spec/15-roadmap PPM-2/4/5/11):
//! the idea funnel tokens, scenario evaluation arithmetic, OKR weight
//! bounds, and benefit/ROI math — DB-free and unit-tested.

use uuid::Uuid;

/// Idea funnel statuses (PPM-2).
pub const IDEA_STATUSES: &[&str] = &["open", "converted", "dismissed"];

/// Scenario statuses (PPM-4).
pub const SCENARIO_STATUSES: &[&str] = &["draft", "committed"];

/// Benefit categories (PPM-11).
pub const BENEFIT_CATEGORIES: &[&str] = &[
    "cost_saving",
    "revenue",
    "risk_reduction",
    "quality",
    "compliance",
    "other",
];

/// Benefit statuses (PPM-11).
pub const BENEFIT_STATUSES: &[&str] = &["planned", "on_track", "realized", "missed"];

/// Whether an OKR mapping weight is in bounds (1–5).
#[must_use]
pub fn valid_weight(weight: i32) -> bool {
    (1..=5).contains(&weight)
}

/// One scenario member's prepared facts (aggregated by the caller —
/// the evaluation itself stays pure).
#[derive(Debug, Clone)]
pub struct MemberFact {
    /// The member's pid (work item or proposal).
    pub pid: Uuid,
    /// Planned budget per currency (work items) or the requested
    /// funding (proposals), in minor units.
    pub planned_by_currency: Vec<(String, i64)>,
    /// Sum of open risk exposure (work items; proposals carry 0).
    pub open_exposure: i32,
    /// Sum of OKR mapping weights (work items; proposals carry 0).
    pub alignment_weight: i32,
}

/// The scenario's constraint knobs.
#[derive(Debug, Clone, Default)]
pub struct Constraints {
    /// Budget cap in minor units of [`Constraints::currency`].
    pub budget_cap_minor: Option<i64>,
    /// The cap's currency (required when the cap is set).
    pub currency: Option<String>,
    /// Pids that must appear in the membership.
    pub must_include: Vec<Uuid>,
}

/// A scenario evaluation: totals + named constraint violations.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Evaluation {
    /// Total planned/requested spend per currency (minor units).
    pub planned_by_currency: Vec<(String, i64)>,
    /// Summed open risk exposure across members.
    pub total_exposure: i32,
    /// Summed OKR alignment weight across members.
    pub total_alignment: i32,
    /// Human-readable constraint violations (empty ⇒ feasible).
    pub violations: Vec<String>,
}

/// Evaluate a candidate portfolio: sum per-currency spend, exposure,
/// and alignment; check the budget cap (same-currency only) and the
/// must-include list. Pure arithmetic over prepared facts.
#[must_use]
pub fn evaluate_scenario(members: &[MemberFact], constraints: &Constraints) -> Evaluation {
    let mut planned: Vec<(String, i64)> = Vec::new();
    for member in members {
        for (currency, amount) in &member.planned_by_currency {
            match planned.iter_mut().find(|(c, _)| c == currency) {
                Some((_, total)) => *total = total.saturating_add(*amount),
                None => planned.push((currency.clone(), *amount)),
            }
        }
    }
    planned.sort();
    let mut violations = Vec::new();
    if let (Some(cap), Some(cap_currency)) =
        (constraints.budget_cap_minor, constraints.currency.as_deref())
    {
        let total = planned
            .iter()
            .find(|(c, _)| c == cap_currency)
            .map_or(0, |(_, t)| *t);
        if total > cap {
            violations.push(format!(
                "budget cap exceeded: {total} > {cap} {cap_currency} (minor units)"
            ));
        }
    }
    let member_pids: std::collections::HashSet<Uuid> = members.iter().map(|m| m.pid).collect();
    for required in &constraints.must_include {
        if !member_pids.contains(required) {
            violations.push(format!("must-include member {required} is missing"));
        }
    }
    Evaluation {
        planned_by_currency: planned,
        total_exposure: members.iter().map(|m| m.open_exposure).sum(),
        total_alignment: members.iter().map(|m| m.alignment_weight).sum(),
        violations,
    }
}

/// Simple ROI in basis points: `(realized − cost) / cost × 10_000`.
/// `None` when the cost is zero or negative (undefined, never a
/// divide-by-zero panic).
#[must_use]
pub fn roi_basis_points(realized_minor: i64, cost_minor: i64) -> Option<i64> {
    if cost_minor <= 0 {
        return None;
    }
    let delta = realized_minor.checked_sub(cost_minor)?;
    delta.checked_mul(10_000).map(|scaled| scaled / cost_minor)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pid(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

    fn member(n: u128, planned: &[(&str, i64)], exposure: i32, alignment: i32) -> MemberFact {
        MemberFact {
            pid: pid(n),
            planned_by_currency: planned.iter().map(|(c, a)| ((*c).to_string(), *a)).collect(),
            open_exposure: exposure,
            alignment_weight: alignment,
        }
    }

    /// Totals sum per currency; cap and must-include violations name
    /// themselves; a feasible scenario has none.
    #[test]
    fn scenario_evaluation() {
        let members = vec![
            member(1, &[("GBP", 500_000)], 12, 8),
            member(2, &[("GBP", 300_000), ("EUR", 100_000)], 6, 3),
        ];
        let feasible = evaluate_scenario(
            &members,
            &Constraints {
                budget_cap_minor: Some(1_000_000),
                currency: Some("GBP".to_string()),
                must_include: vec![pid(1)],
            },
        );
        assert_eq!(
            feasible.planned_by_currency,
            vec![("EUR".to_string(), 100_000), ("GBP".to_string(), 800_000)]
        );
        assert_eq!(feasible.total_exposure, 18);
        assert_eq!(feasible.total_alignment, 11);
        assert!(feasible.violations.is_empty());

        let violated = evaluate_scenario(
            &members,
            &Constraints {
                budget_cap_minor: Some(700_000),
                currency: Some("GBP".to_string()),
                must_include: vec![pid(9)],
            },
        );
        assert_eq!(violated.violations.len(), 2);
        assert!(violated.violations[0].contains("budget cap exceeded"));
        assert!(violated.violations[1].contains("missing"));
        // A cap in a currency nobody spends is not violated.
        let other = evaluate_scenario(
            &members,
            &Constraints {
                budget_cap_minor: Some(1),
                currency: Some("USD".to_string()),
                must_include: vec![],
            },
        );
        assert!(other.violations.is_empty());
    }

    /// ROI: positive, negative, and the undefined zero-cost case.
    #[test]
    fn roi_math() {
        assert_eq!(roi_basis_points(15_000, 10_000), Some(5_000)); // +50%
        assert_eq!(roi_basis_points(5_000, 10_000), Some(-5_000)); // −50%
        assert_eq!(roi_basis_points(10_000, 10_000), Some(0));
        assert_eq!(roi_basis_points(1, 0), None);
        assert_eq!(roi_basis_points(1, -5), None);
    }

    /// Weight bounds.
    #[test]
    fn weights() {
        assert!(valid_weight(1));
        assert!(valid_weight(5));
        assert!(!valid_weight(0));
        assert!(!valid_weight(6));
    }
}
