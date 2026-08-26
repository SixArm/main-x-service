//! Pure rules for **recorded effort** and **utilisation** (entity spec
//! §5.9.3 / §5.9.6, FR-28 / FR-35). DB-free and exhaustively
//! unit-tested.
//!
//! # Effort is an assertion, not an observation
//!
//! A time entry is typed by a person. The task transition log is a
//! by-product of the work. These are different kinds of evidence and
//! the difference is reported wherever a figure rests on the former —
//! because anti-gaming rests on incidental collection, and a timesheet
//! never had that protection.
//!
//! # Utilisation, and the five obligations
//!
//! Per-person utilisation is computed here by owner decision of
//! 2026-08-25, which **narrowed** the family refusal in
//! `agents/share/time-based-analysis.md` §7.1 rather than repealing it.
//! Per-person cycle time, throughput and flow efficiency stay refused,
//! and nothing in this module computes them.
//!
//! The obligations are enforced, not documented:
//!
//! 1. The denominator is **declared and returned**, never assumed 100%.
//! 2. Non-working time **leaves the denominator** — leave is absence of
//!    capacity, not failure to use it. This is why somebody on leave
//!    all window reports `null` and not `0%`.
//! 3. Small denominators are **suppressed**, which in a clinical
//!    setting is a re-identification control and not merely a
//!    statistical one.
//! 4. It is never the sole ranking key, and ships with its numerator
//!    and denominator.
//! 5. Effort stays labelled **asserted**.

use serde::{Deserialize, Serialize};

/// Basis-point scale.
pub const BASIS_POINTS: i64 = 10_000;

/// Minutes of declared capacity below which a utilisation figure is
/// suppressed. One nominal working week.
pub const DEFAULT_SUPPRESSION_FLOOR_MINUTES: i64 = 2_400;

/// How effort is categorised for the capex / opex split the budget
/// lines already use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffortCategory {
    /// Capitalisable.
    Capex,
    /// Operating.
    Opex,
    /// Not categorised.
    Unclassified,
}

impl EffortCategory {
    /// The wire token.
    #[must_use]
    pub const fn token(self) -> &'static str {
        match self {
            Self::Capex => "capex",
            Self::Opex => "opex",
            Self::Unclassified => "unclassified",
        }
    }

    /// Parse a declared category; anything unrecognised is
    /// `Unclassified` rather than silently one of the other two.
    #[must_use]
    pub fn parse(raw: Option<&str>) -> Self {
        match raw.map(str::trim).map(str::to_ascii_lowercase).as_deref() {
            Some("capex") => Self::Capex,
            Some("opex") => Self::Opex,
            _ => Self::Unclassified,
        }
    }
}

/// One recorded time entry, reduced to what roll-ups need.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffortFact {
    /// Who recorded it against themselves.
    pub actor_ref: String,
    /// Minutes.
    pub minutes: i64,
    /// Capex / opex split.
    pub category: EffortCategory,
    /// Whether it is billable.
    pub billable: bool,
}

/// Effort rolled up for one subject.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffortTotals {
    /// Total minutes.
    pub minutes: i64,
    /// Of which capitalisable.
    pub capex_minutes: i64,
    /// Of which operating.
    pub opex_minutes: i64,
    /// Of which categorised as neither — reported rather than folded
    /// into `opex`, which would flatter the capitalisable share.
    pub unclassified_minutes: i64,
    /// Of which billable.
    pub billable_minutes: i64,
    /// Always true: these are typed by people, not observed.
    pub asserted: bool,
}

/// Roll up effort. Saturating, so an absurd stored value cannot panic.
#[must_use]
pub fn totals(entries: &[EffortFact]) -> EffortTotals {
    let mut out = EffortTotals {
        minutes: 0,
        capex_minutes: 0,
        opex_minutes: 0,
        unclassified_minutes: 0,
        billable_minutes: 0,
        asserted: true,
    };
    for entry in entries {
        out.minutes = out.minutes.saturating_add(entry.minutes);
        match entry.category {
            EffortCategory::Capex => {
                out.capex_minutes = out.capex_minutes.saturating_add(entry.minutes);
            }
            EffortCategory::Opex => {
                out.opex_minutes = out.opex_minutes.saturating_add(entry.minutes);
            }
            EffortCategory::Unclassified => {
                out.unclassified_minutes = out.unclassified_minutes.saturating_add(entry.minutes);
            }
        }
        if entry.billable {
            out.billable_minutes = out.billable_minutes.saturating_add(entry.minutes);
        }
    }
    out
}

/// Why a utilisation figure is absent. Reported beside the `None`, so
/// nobody has to guess whether it is zero, missing, or withheld.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Unavailable {
    /// No declared capacity at all in the window.
    NoDeclaredCapacity,
    /// Every minute of the window was non-working time. **Not zero
    /// percent**: leave is absence of capacity, not failure to use it.
    AllNonWorking,
    /// Below the suppression floor. In a clinical setting this is a
    /// re-identification control, not only a statistical one.
    BelowFloor,
}

/// A person's declared capacity and recorded effort for one window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapacityFact {
    /// The person.
    pub actor_ref: String,
    /// Minutes the roster declares available, before deductions.
    pub declared_minutes: i64,
    /// Minutes of leave, study leave, holiday, or non-project duty.
    /// **Subtracted from the denominator.**
    pub non_working_minutes: i64,
    /// Minutes of effort recorded.
    pub effort_minutes: i64,
}

/// One person's utilisation, with everything needed to check it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Utilisation {
    /// The person.
    pub actor_ref: String,
    /// Effort ÷ available capacity, in basis points. `None` when the
    /// figure is not reportable.
    pub basis_points: Option<i64>,
    /// Why it is absent, when it is.
    pub unavailable: Option<Unavailable>,
    /// The numerator, always returned.
    pub effort_minutes: i64,
    /// The denominator, always returned — never assumed.
    pub available_minutes: i64,
    /// Declared before deductions, so the deduction is visible.
    pub declared_minutes: i64,
    /// What was deducted.
    pub non_working_minutes: i64,
    /// Effort is typed by a person.
    pub asserted: bool,
    /// Whether the reading is at or above capacity — a **warning**, not
    /// an achievement: it is what a queueing system looks like just
    /// before it stops coping.
    pub at_or_over_capacity: bool,
}

/// Compute one person's utilisation under the five obligations.
#[must_use]
pub fn utilisation(fact: &CapacityFact, floor_minutes: i64) -> Utilisation {
    let available = fact
        .declared_minutes
        .saturating_sub(fact.non_working_minutes)
        .max(0);

    let unavailable = if fact.declared_minutes <= 0 {
        Some(Unavailable::NoDeclaredCapacity)
    } else if available == 0 {
        Some(Unavailable::AllNonWorking)
    } else if available < floor_minutes {
        Some(Unavailable::BelowFloor)
    } else {
        None
    };

    let basis_points = if unavailable.is_none() {
        fact.effort_minutes
            .checked_mul(BASIS_POINTS)
            .map(|scaled| scaled / available)
    } else {
        None
    };

    Utilisation {
        actor_ref: fact.actor_ref.clone(),
        basis_points,
        unavailable,
        effort_minutes: fact.effort_minutes,
        available_minutes: available,
        declared_minutes: fact.declared_minutes,
        non_working_minutes: fact.non_working_minutes,
        asserted: true,
        at_or_over_capacity: basis_points.is_some_and(|bp| bp >= BASIS_POINTS),
    }
}

/// Team-level utilisation: effort over capacity across a group.
///
/// Computed from the **summed** numerator and denominator, never as a
/// mean of individual ratios — averaging ratios over unequal
/// denominators is a different (and wrong) number.
#[must_use]
pub fn team_utilisation(facts: &[CapacityFact], floor_minutes: i64) -> Utilisation {
    let combined = CapacityFact {
        actor_ref: "team".to_string(),
        declared_minutes: facts
            .iter()
            .fold(0_i64, |a, f| a.saturating_add(f.declared_minutes)),
        non_working_minutes: facts
            .iter()
            .fold(0_i64, |a, f| a.saturating_add(f.non_working_minutes)),
        effort_minutes: facts
            .iter()
            .fold(0_i64, |a, f| a.saturating_add(f.effort_minutes)),
    };
    utilisation(&combined, floor_minutes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(actor: &str, minutes: i64, category: EffortCategory, billable: bool) -> EffortFact {
        EffortFact {
            actor_ref: actor.to_string(),
            minutes,
            category,
            billable,
        }
    }

    /// Effort rolls up, and uncategorised effort is reported separately
    /// rather than folded into `opex` — which would flatter the
    /// capitalisable share.
    #[test]
    fn effort_totals_keep_uncategorised_visible() {
        let t = totals(&[
            entry("person:a", 120, EffortCategory::Capex, true),
            entry("person:a", 60, EffortCategory::Opex, false),
            entry("person:b", 30, EffortCategory::Unclassified, false),
        ]);
        assert_eq!(t.minutes, 210);
        assert_eq!(t.capex_minutes, 120);
        assert_eq!(t.opex_minutes, 60);
        assert_eq!(t.unclassified_minutes, 30);
        assert_eq!(t.billable_minutes, 120);
        assert!(t.asserted, "effort is always labelled asserted");
    }

    /// An unrecognised category is `Unclassified`, never silently one of
    /// the real two.
    #[test]
    fn an_unknown_category_is_unclassified() {
        assert_eq!(EffortCategory::parse(Some("capex")), EffortCategory::Capex);
        assert_eq!(
            EffortCategory::parse(Some("invented")),
            EffortCategory::Unclassified
        );
        assert_eq!(EffortCategory::parse(None), EffortCategory::Unclassified);
    }

    fn capacity(declared: i64, non_working: i64, effort: i64) -> CapacityFact {
        CapacityFact {
            actor_ref: "person:a".to_string(),
            declared_minutes: declared,
            non_working_minutes: non_working,
            effort_minutes: effort,
        }
    }

    /// Utilisation is effort over **available** capacity, and always
    /// ships its numerator and denominator.
    #[test]
    fn utilisation_returns_its_own_denominator() {
        let u = utilisation(&capacity(2_400, 0, 1_200), 0);
        assert_eq!(u.basis_points, Some(5_000));
        assert_eq!(u.available_minutes, 2_400);
        assert_eq!(u.declared_minutes, 2_400);
        assert_eq!(u.effort_minutes, 1_200);
        assert!(u.asserted);
        assert!(!u.at_or_over_capacity);
    }

    /// **The obligation-2 test.** Non-working time leaves the
    /// denominator rather than sitting in it: somebody on leave for the
    /// whole window reports `null` **with a reason**, never `0%`, which
    /// would read as measured idleness.
    #[test]
    fn leave_leaves_the_denominator_and_never_reads_as_zero() {
        let all_leave = utilisation(&capacity(2_400, 2_400, 0), 0);
        assert_eq!(all_leave.basis_points, None);
        assert_eq!(all_leave.unavailable, Some(Unavailable::AllNonWorking));
        assert_eq!(all_leave.available_minutes, 0);

        // Half the window on leave doubles the utilisation of the same
        // effort, because only half the capacity was ever available.
        let half = utilisation(&capacity(2_400, 1_200, 600), 0);
        assert_eq!(half.basis_points, Some(5_000));
        assert_eq!(half.non_working_minutes, 1_200);
    }

    /// No roster at all is `NoDeclaredCapacity`, distinct from being on
    /// leave — the denominator is unknown, not zero.
    #[test]
    fn no_declared_capacity_is_its_own_reason() {
        let u = utilisation(&capacity(0, 0, 500), 0);
        assert_eq!(u.basis_points, None);
        assert_eq!(u.unavailable, Some(Unavailable::NoDeclaredCapacity));
    }

    /// Below the floor the figure is withheld with a reason. In a
    /// clinical setting this is a re-identification control.
    #[test]
    fn a_small_denominator_is_suppressed() {
        let u = utilisation(&capacity(60, 0, 30), DEFAULT_SUPPRESSION_FLOOR_MINUTES);
        assert_eq!(u.basis_points, None);
        assert_eq!(u.unavailable, Some(Unavailable::BelowFloor));
        // The inputs are still returned, so the suppression is visible
        // rather than looking like missing data.
        assert_eq!(u.effort_minutes, 30);
        assert_eq!(u.available_minutes, 60);
    }

    /// At or over capacity is flagged — a warning about the queue, not
    /// an achievement.
    #[test]
    fn at_capacity_is_flagged_as_a_warning() {
        let at = utilisation(&capacity(2_400, 0, 2_400), 0);
        assert_eq!(at.basis_points, Some(10_000));
        assert!(at.at_or_over_capacity);

        let over = utilisation(&capacity(2_400, 0, 3_600), 0);
        assert_eq!(over.basis_points, Some(15_000), "over 100% is not clamped");
        assert!(over.at_or_over_capacity);
    }

    /// Team utilisation sums the numerator and denominator; it is **not**
    /// a mean of individual ratios, which over unequal denominators is a
    /// different and wrong number.
    #[test]
    fn team_utilisation_sums_rather_than_averaging_ratios() {
        let facts = vec![
            CapacityFact {
                actor_ref: "person:a".to_string(),
                declared_minutes: 2_400,
                non_working_minutes: 0,
                effort_minutes: 2_400, // 100%
            },
            CapacityFact {
                actor_ref: "person:b".to_string(),
                declared_minutes: 240,
                non_working_minutes: 0,
                effort_minutes: 0, // 0%
            },
        ];
        let team = team_utilisation(&facts, 0);
        // Summed: 2400 / 2640 ≈ 90.9%. A mean of the two ratios would
        // have said 50%.
        assert_eq!(team.basis_points, Some(9_090));
        assert_ne!(team.basis_points, Some(5_000));
    }

    /// Untrusted stored values must not panic.
    #[test]
    fn extreme_values_are_total() {
        let u = utilisation(&capacity(i64::MAX, i64::MIN, i64::MAX), 0);
        assert!(u.available_minutes >= 0);
        let _ = utilisation(&capacity(1, 0, i64::MAX), 0);
        let huge: Vec<EffortFact> = (0..3)
            .map(|_| entry("person:a", i64::MAX, EffortCategory::Capex, true))
            .collect();
        assert_eq!(totals(&huge).minutes, i64::MAX, "saturating, not wrapping");
        let _ = team_utilisation(&[], 0);
    }
}
