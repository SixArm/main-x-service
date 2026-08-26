//! Pure rules for **Flow Distribution** — the mix of work types being
//! completed (entity spec §5.9.5 / §1.6 / FR-31). DB-free and
//! exhaustively unit-tested.
//!
//! Flow Distribution is the metric that makes the other four Flow
//! Framework figures legible: a rising Flow Velocity means something
//! entirely different when the mix has shifted to defects. It is the
//! only one of the five that was not already computed under
//! time-based-analysis vocabulary.
//!
//! Two rules that decide whether the number is worth having:
//!
//! - **`unclassified` is counted separately, never folded into
//!   `feature`.** Absorbing it into the largest category would flatter
//!   the one share a reader is most likely to act on, and an
//!   unclassified pile is itself a finding about the board.
//! - **An intended mix is reported against only when a deployment
//!   declares one.** Absent that, the mix is reported without
//!   judgement: an unlabelled target is how a measurement becomes a
//!   quota.

use serde::{Deserialize, Serialize};

/// The Flow Framework's four work-item types, plus the honest fifth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlowType {
    /// Work that adds value a customer asked for.
    Feature,
    /// Work to correct a fault.
    Defect,
    /// Work to protect against a future loss.
    Risk,
    /// Work to repay a shortcut previously taken.
    Debt,
    /// Declared by nobody. **Never folded into `feature`.**
    Unclassified,
}

/// Every type, in report order, so a mix can show each even at zero.
pub const ALL: [FlowType; 5] = [
    FlowType::Feature,
    FlowType::Defect,
    FlowType::Risk,
    FlowType::Debt,
    FlowType::Unclassified,
];

impl FlowType {
    /// The wire token.
    #[must_use]
    pub const fn token(self) -> &'static str {
        match self {
            Self::Feature => "feature",
            Self::Defect => "defect",
            Self::Risk => "risk",
            Self::Debt => "debt",
            Self::Unclassified => "unclassified",
        }
    }

    /// Parse a declared task `flow_type`. Unknown or absent input is
    /// [`FlowType::Unclassified`] — **not** a default of `feature`,
    /// which would silently inflate the share that matters most.
    #[must_use]
    pub fn parse(raw: Option<&str>) -> Self {
        match raw.map(str::trim).map(str::to_ascii_lowercase).as_deref() {
            Some("feature") => Self::Feature,
            Some("defect") => Self::Defect,
            Some("risk") => Self::Risk,
            Some("debt") => Self::Debt,
            _ => Self::Unclassified,
        }
    }

    /// The type a **risk-register** row contributes, from its category.
    ///
    /// Only `tech_debt`, `compliance` and `security` contribute:
    /// `delivery` and `other` are ordinary project risk already visible
    /// in the risk views, and counting them here would double-count
    /// routine delivery work as a distinct flow type.
    #[must_use]
    pub fn from_risk_category(raw: Option<&str>) -> Option<Self> {
        match raw.map(str::trim).map(str::to_ascii_lowercase).as_deref() {
            Some("tech_debt") => Some(Self::Debt),
            Some("compliance" | "security") => Some(Self::Risk),
            _ => None,
        }
    }
}

/// One type's share of the mix.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Share {
    /// The type.
    pub flow_type: String,
    /// Items completed of this type in the window.
    pub count: usize,
    /// Share of the total, in basis points. `None` when the total is
    /// zero — a share of nothing is undefined, not 0%.
    pub basis_points: Option<i64>,
    /// The declared intent for this type, if a deployment set one.
    pub intended_basis_points: Option<i64>,
    /// Actual minus intended, when an intent exists. Positive is over.
    pub gap_basis_points: Option<i64>,
}

/// The full mix.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Distribution {
    /// Items counted.
    pub total: usize,
    /// Every type, **including `unclassified`**, in report order.
    pub shares: Vec<Share>,
    /// Whether a deployment declared an intended mix at all.
    pub intent_declared: bool,
}

/// Compute the mix over already-classified items.
///
/// `intended` maps a type token to its intended share in basis points;
/// pass an empty slice for no declared intent.
#[must_use]
pub fn distribution(items: &[FlowType], intended: &[(FlowType, i64)]) -> Distribution {
    let total = items.len();
    let shares = ALL
        .iter()
        .map(|flow_type| {
            let count = items.iter().filter(|item| *item == flow_type).count();
            let basis_points = if total == 0 {
                None
            } else {
                i64::try_from(count)
                    .ok()
                    .and_then(|c| c.checked_mul(10_000))
                    .map(|scaled| scaled / i64::try_from(total).unwrap_or(1))
            };
            // An intent outside 0..=10_000 is not a share of anything.
            // `parse_intent` already refuses one, but this function is
            // public and a caller can construct the slice directly — so
            // the guard lives where the arithmetic is, rather than
            // trusting every caller to have come through the parser.
            // Without it, an intent of `i64::MAX` yields a gap of
            // roughly -9.2e18, which is not an overflow and so would
            // pass a `checked_sub` unnoticed: a nonsense number that
            // looks like a measurement.
            let intended_basis_points = intended
                .iter()
                .find(|(t, _)| t == flow_type)
                .map(|(_, bp)| *bp)
                .filter(|bp| (0..=10_000).contains(bp));
            Share {
                flow_type: flow_type.token().to_string(),
                count,
                basis_points,
                intended_basis_points,
                gap_basis_points: match (basis_points, intended_basis_points) {
                    (Some(actual), Some(intent)) => actual.checked_sub(intent),
                    _ => None,
                },
            }
        })
        .collect();

    Distribution {
        total,
        shares,
        intent_declared: !intended.is_empty(),
    }
}

/// Parse a deployment's intended mix: `feature=6000,debt=2000,…` in
/// basis points.
///
/// Returns `None` for anything malformed — an unknown token, a negative
/// share, a duplicate, or a total over `10_000`. The whole map is
/// rejected rather than half-applied, matching the Smart Score weights
/// and the ABAC policy posture: a partly-understood target is worse
/// than none, because it looks deliberate.
#[must_use]
pub fn parse_intent(raw: Option<&str>) -> Option<Vec<(FlowType, i64)>> {
    let raw = raw?.trim();
    if raw.is_empty() {
        return None;
    }
    let mut out: Vec<(FlowType, i64)> = Vec::new();
    for entry in raw.split(',') {
        let (name, value) = entry.split_once('=')?;
        let flow_type = match name.trim().to_ascii_lowercase().as_str() {
            "feature" => FlowType::Feature,
            "defect" => FlowType::Defect,
            "risk" => FlowType::Risk,
            "debt" => FlowType::Debt,
            // `unclassified` is deliberately not settable as an intent:
            // nobody intends to leave work unclassified, and allowing
            // it would let a deployment budget for its own blind spot.
            _ => return None,
        };
        let basis_points: i64 = value.trim().parse().ok()?;
        if basis_points < 0 || out.iter().any(|(t, _)| *t == flow_type) {
            return None;
        }
        out.push((flow_type, basis_points));
    }
    if out.iter().map(|(_, bp)| bp).sum::<i64>() > 10_000 {
        return None;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An absent or unknown declaration is `unclassified`, **not**
    /// `feature` — the regression test against a mix that flatters the
    /// share a reader acts on.
    #[test]
    fn absent_declaration_is_unclassified_not_feature() {
        assert_eq!(FlowType::parse(None), FlowType::Unclassified);
        assert_eq!(FlowType::parse(Some("")), FlowType::Unclassified);
        assert_eq!(FlowType::parse(Some("nonsense")), FlowType::Unclassified);
        assert_eq!(FlowType::parse(Some(" Feature ")), FlowType::Feature);
    }

    /// Only the three meaningful risk categories contribute; ordinary
    /// delivery risk does not, because counting it here would
    /// double-count routine work as its own flow type.
    #[test]
    fn only_meaningful_risk_categories_contribute() {
        assert_eq!(
            FlowType::from_risk_category(Some("tech_debt")),
            Some(FlowType::Debt)
        );
        assert_eq!(
            FlowType::from_risk_category(Some("compliance")),
            Some(FlowType::Risk)
        );
        assert_eq!(
            FlowType::from_risk_category(Some("security")),
            Some(FlowType::Risk)
        );
        assert_eq!(FlowType::from_risk_category(Some("delivery")), None);
        assert_eq!(FlowType::from_risk_category(Some("other")), None);
        assert_eq!(FlowType::from_risk_category(None), None);
    }

    /// Shares are computed over the total, every type appears, and
    /// `unclassified` is its own row.
    #[test]
    fn every_type_appears_and_unclassified_stands_alone() {
        let items = vec![
            FlowType::Feature,
            FlowType::Feature,
            FlowType::Defect,
            FlowType::Unclassified,
        ];
        let d = distribution(&items, &[]);
        assert_eq!(d.total, 4);
        assert_eq!(d.shares.len(), 5);
        assert_eq!(d.shares[0].count, 2);
        assert_eq!(d.shares[0].basis_points, Some(5_000));
        assert_eq!(d.shares[1].basis_points, Some(2_500));
        assert_eq!(d.shares[2].count, 0);
        assert_eq!(d.shares[2].basis_points, Some(0));
        assert_eq!(d.shares[4].flow_type, "unclassified");
        assert_eq!(d.shares[4].count, 1);
        assert!(!d.intent_declared);
    }

    /// An empty window reports `None` shares, not 0% — a share of
    /// nothing is undefined, and zero would read as measured.
    #[test]
    fn an_empty_window_is_undefined_not_zero() {
        let d = distribution(&[], &[]);
        assert_eq!(d.total, 0);
        assert!(d.shares.iter().all(|s| s.basis_points.is_none()));
        assert!(d.shares.iter().all(|s| s.count == 0));
    }

    /// A declared intent produces a gap; without one, no judgement is
    /// offered.
    #[test]
    fn intent_produces_a_gap_only_when_declared() {
        let items = vec![
            FlowType::Feature,
            FlowType::Feature,
            FlowType::Debt,
            FlowType::Debt,
        ];
        let without = distribution(&items, &[]);
        assert!(without.shares.iter().all(|s| s.gap_basis_points.is_none()));
        assert!(!without.intent_declared);

        let with = distribution(&items, &[(FlowType::Debt, 2_000)]);
        assert!(with.intent_declared);
        let debt = with.shares.iter().find(|s| s.flow_type == "debt").unwrap();
        assert_eq!(debt.basis_points, Some(5_000));
        assert_eq!(debt.intended_basis_points, Some(2_000));
        assert_eq!(debt.gap_basis_points, Some(3_000));
        // A type with no declared intent still reports no gap.
        let feature = with
            .shares
            .iter()
            .find(|s| s.flow_type == "feature")
            .unwrap();
        assert_eq!(feature.gap_basis_points, None);
    }

    /// A malformed intent is rejected **whole**, never half-applied: a
    /// partly-understood target looks deliberate and is worse than none.
    #[test]
    fn a_malformed_intent_is_rejected_whole() {
        assert_eq!(
            parse_intent(Some("feature=6000,debt=2000")),
            Some(vec![(FlowType::Feature, 6_000), (FlowType::Debt, 2_000)])
        );
        assert_eq!(parse_intent(None), None);
        assert_eq!(parse_intent(Some("")), None);
        assert_eq!(parse_intent(Some("feature")), None);
        assert_eq!(parse_intent(Some("invented=100")), None);
        assert_eq!(parse_intent(Some("feature=-1")), None);
        assert_eq!(parse_intent(Some("feature=10,feature=20")), None);
        assert_eq!(parse_intent(Some("feature=9000,debt=9000")), None);
        // Nobody intends to leave work unclassified.
        assert_eq!(parse_intent(Some("unclassified=1000")), None);
    }

    /// Untrusted counts must not panic or divide by zero.
    #[test]
    fn extreme_input_is_total() {
        let many = vec![FlowType::Feature; 1000];
        assert_eq!(
            distribution(&many, &[]).shares[0].basis_points,
            Some(10_000)
        );

        // An out-of-range intent yields **no gap**, rather than a
        // nonsense one. `i64::MAX` does not overflow `checked_sub` here
        // — it produces roughly -9.2e18, a number that looks like a
        // measurement and is not. Caught by this test, fixed in
        // `distribution` rather than by relaxing the assertion.
        for absurd in [i64::MAX, i64::MIN, -1, 10_001] {
            let d = distribution(&many, &[(FlowType::Feature, absurd)]);
            assert_eq!(d.shares[0].intended_basis_points, None, "intent {absurd}");
            assert_eq!(d.shares[0].gap_basis_points, None, "gap for {absurd}");
        }
        // A legitimate intent at the boundaries still works.
        let ok = distribution(&many, &[(FlowType::Feature, 10_000)]);
        assert_eq!(ok.shares[0].gap_basis_points, Some(0));
    }
}
