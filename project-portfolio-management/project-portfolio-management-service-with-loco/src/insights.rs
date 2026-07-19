//! Pure derivations for the executive insight areas (CEO / CFO / CTO)
//! — the money aggregation, technology-radar tag convention, benefit
//! realization ratio, and dependency fan-out counting behind
//! `controllers::insights`. No I/O; every number a served view shows is
//! derived here (or in [`crate::visibility`]) so the derivations are
//! unit-testable and the honesty rules are pinned in one place:
//! per-currency lines never merge, ratios carry their numerator and
//! denominator, and absent data is `None`, never `0`.

use std::collections::BTreeMap;

use serde::Serialize;
use uuid::Uuid;

/// One money observation: a currency plus planned/actual minor units.
#[derive(Debug, Clone)]
pub struct MoneyLine {
    /// ISO-4217 currency code.
    pub currency: String,
    /// Planned (committed) amount in minor units.
    pub planned_minor: i64,
    /// Actual (spent) amount in minor units.
    pub actual_minor: i64,
}

/// Per-currency variance rollup. Currencies are **never** merged and
/// there is **no** FX conversion — one row per currency.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct VarianceRow {
    /// ISO-4217 currency code.
    pub currency: String,
    /// Sum of planned minor units in this currency.
    pub planned_minor: i64,
    /// Sum of actual minor units in this currency.
    pub actual_minor: i64,
    /// `planned - actual` (negative = overrun), minor units.
    pub remaining_minor: i64,
    /// Whether actual exceeds planned in this currency.
    pub overrun: bool,
    /// How many lines contributed to this row.
    pub line_count: usize,
}

/// Aggregate money lines into one variance row per currency
/// (deterministic order: currency code ascending).
#[must_use]
pub fn variance_by_currency(lines: &[MoneyLine]) -> Vec<VarianceRow> {
    let mut per: BTreeMap<&str, (i64, i64, usize)> = BTreeMap::new();
    for line in lines {
        let entry = per.entry(line.currency.as_str()).or_default();
        entry.0 = entry.0.saturating_add(line.planned_minor);
        entry.1 = entry.1.saturating_add(line.actual_minor);
        entry.2 += 1;
    }
    per.into_iter()
        .map(|(currency, (planned, actual, count))| VarianceRow {
            currency: currency.to_string(),
            planned_minor: planned,
            actual_minor: actual,
            remaining_minor: planned.saturating_sub(actual),
            overrun: actual > planned,
            line_count: count,
        })
        .collect()
}

/// The four technology-radar rings, outermost first.
pub const RADAR_RINGS: [&str; 4] = ["assess", "trial", "adopt", "hold"];

/// Parse one work-item tag against the radar convention:
/// `tech:<name>` or `tech:<name>:<ring>` (ring ∈ [`RADAR_RINGS`]).
/// The name is lowercased and trimmed; blank names and unknown rings
/// are rejected (`None`) rather than guessed.
#[must_use]
pub fn parse_tech_tag(tag: &str) -> Option<(String, Option<String>)> {
    let rest = tag.trim().strip_prefix("tech:")?;
    let (name, ring) = match rest.split_once(':') {
        Some((name, ring)) => {
            let ring = ring.trim().to_lowercase();
            if !RADAR_RINGS.contains(&ring.as_str()) {
                return None;
            }
            (name, Some(ring))
        }
        None => (rest, None),
    };
    let name = name.trim().to_lowercase();
    if name.is_empty() {
        return None;
    }
    Some((name, ring))
}

/// The consensus ring for a technology from its declared rings: the
/// most-declared ring wins; ties break toward the more cautious ring
/// (earlier in [`RADAR_RINGS`]); no declarations ⇒ `unclassified`.
#[must_use]
pub fn ring_consensus(rings: &[String]) -> &'static str {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for ring in rings {
        if let Some(known) = RADAR_RINGS.iter().find(|r| **r == ring.as_str()) {
            *counts.entry(known).or_default() += 1;
        }
    }
    let best = RADAR_RINGS
        .iter()
        .filter_map(|ring| counts.get(ring).map(|n| (*ring, *n)))
        .max_by(|a, b| a.1.cmp(&b.1)); // stable: first max in ring order wins ties
    match best {
        Some((ring, _)) => RADAR_RINGS
            .iter()
            .filter(|r| counts.get(**r) == counts.get(ring))
            .copied()
            .next()
            .unwrap_or("unclassified"),
        None => "unclassified",
    }
}

/// Benefit realization ratio: `realized / target`, or `None` when no
/// positive target exists (a ratio against zero would be an invented
/// number).
#[must_use]
pub fn realization_ratio(target_minor: i64, realized_minor: i64) -> Option<f64> {
    if target_minor <= 0 {
        return None;
    }
    #[allow(clippy::cast_precision_loss)] // display ratio, not money math
    Some(realized_minor as f64 / target_minor as f64)
}

/// Dependents per predecessor (fan-out), most-depended-on first, then
/// by pid for determinism. An item many others depend on is a
/// single-point-of-failure candidate.
#[must_use]
pub fn fan_out(edges: &[(Uuid, Uuid)]) -> Vec<(Uuid, usize)> {
    let mut per: BTreeMap<Uuid, usize> = BTreeMap::new();
    for (predecessor, _successor) in edges {
        *per.entry(*predecessor).or_default() += 1;
    }
    let mut out: Vec<(Uuid, usize)> = per.into_iter().collect();
    out.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variance_keeps_currencies_apart_and_flags_overrun() {
        let rows = variance_by_currency(&[
            MoneyLine { currency: "GBP".into(), planned_minor: 10_000, actual_minor: 12_000 },
            MoneyLine { currency: "GBP".into(), planned_minor: 5_000, actual_minor: 1_000 },
            MoneyLine { currency: "USD".into(), planned_minor: 7_000, actual_minor: 7_000 },
        ]);
        assert_eq!(rows.len(), 2, "one row per currency, never merged");
        let gbp = &rows[0];
        assert_eq!((gbp.currency.as_str(), gbp.planned_minor, gbp.actual_minor), ("GBP", 15_000, 13_000));
        assert_eq!(gbp.remaining_minor, 2_000);
        assert!(!gbp.overrun);
        assert_eq!(gbp.line_count, 2);
        let usd = &rows[1];
        assert!(!usd.overrun, "actual == planned is not an overrun");

        let overrun = variance_by_currency(&[MoneyLine {
            currency: "EUR".into(),
            planned_minor: 100,
            actual_minor: 101,
        }]);
        assert!(overrun[0].overrun);
        assert_eq!(overrun[0].remaining_minor, -1);
    }

    #[test]
    fn tech_tags_parse_and_reject() {
        assert_eq!(parse_tech_tag("tech:rust"), Some(("rust".into(), None)));
        assert_eq!(
            parse_tech_tag("tech:Rust:Adopt"),
            Some(("rust".into(), Some("adopt".into())))
        );
        assert_eq!(parse_tech_tag("tech: postgres :trial"), Some(("postgres".into(), Some("trial".into()))));
        assert_eq!(parse_tech_tag("tech:"), None, "blank name refused");
        assert_eq!(parse_tech_tag("tech:x:sideways"), None, "unknown ring refused");
        assert_eq!(parse_tech_tag("owner:alice"), None, "non-tech tag ignored");
    }

    #[test]
    fn ring_consensus_majority_ties_and_default() {
        let r = |xs: &[&str]| ring_consensus(&xs.iter().map(|s| (*s).to_string()).collect::<Vec<_>>());
        assert_eq!(r(&["adopt", "adopt", "trial"]), "adopt");
        assert_eq!(r(&["assess", "adopt"]), "assess", "tie breaks cautious");
        assert_eq!(r(&[]), "unclassified");
        assert_eq!(r(&["nonsense"]), "unclassified", "unknown rings never count");
    }

    #[test]
    fn realization_ratio_never_divides_by_nothing() {
        assert_eq!(realization_ratio(0, 500), None);
        assert_eq!(realization_ratio(-1, 500), None);
        assert!((realization_ratio(1_000, 250).unwrap() - 0.25).abs() < 1e-12);
    }

    #[test]
    fn fan_out_orders_by_dependents_then_pid() {
        let a = Uuid::from_u128(1);
        let b = Uuid::from_u128(2);
        let c = Uuid::from_u128(3);
        let out = fan_out(&[(a, c), (b, c), (a, b)]);
        assert_eq!(out[0], (a, 2));
        assert_eq!(out[1], (b, 1));
    }
}
