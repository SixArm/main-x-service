//! Rule-based cohort splits (spec `13-tasks.md` T-14f): `contains=`/
//! `excludes=` predicates over a closed-where-possible vocabulary
//! (`stage:`/`step:`/`event:`/`waste:`/`outcome:`/`setting:`/
//! `urgency:`), and the two-cell [`suppression::Table`] that keeps a
//! withheld side's detail from being recovered by subtraction against
//! the published unsplit total (T-14k).
//!
//! Named `split`, not `rules` (the query parameters this module reads
//! are literally `contains=`/`excludes=` **rules**): `crate::instances`
//! is already aliased `rules` throughout the controller layer, and a
//! second module answering to the same name in the same files would
//! be a standing hazard, not a cosmetic clash.
//!
//! Reuses [`crate::analytics::CaseContext`] /
//! [`crate::analytics::SegmentInput`] / [`crate::analytics::StepInput`] /
//! [`crate::analytics::EventInput`] — the same per-instance inputs
//! T-14a's event-log codec already loads — rather than a second
//! feature-extraction path over the same rows.
//!
//! **Landed for `time-analysis` and `constraints` only.**
//! `process-map` and `variants` each already carry their own,
//! differently-shaped suppression (per node/edge; per variant), and
//! "compare" would mean something different for each (two side-by-side
//! process maps? two variant Paretos?) — wiring this same query
//! contract onto them needs its own design pass, so it stays a
//! documented follow-up rather than a rushed fit.

use std::collections::BTreeSet;

use crate::analytics::{CaseContext, EventInput, SegmentInput, StepInput};
use crate::instances::{EVENT_KINDS, OUTCOMES, URGENCY_LEVELS};
use crate::suppression;
use crate::tba::{STAGES, WASTES};

/// One `contains=`/`excludes=` predicate (spec T-14f).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Predicate {
    /// `stage:<s>` — the instance has at least one segment in this
    /// [`STAGES`] stage.
    Stage(String),
    /// `step:<name>` — the instance completed a step with this label.
    /// Free-form: step labels are declared per pathway/instance, not
    /// a closed vocabulary this module can check against.
    Step(String),
    /// `event:<kind>` — the instance recorded an event of this
    /// [`EVENT_KINDS`] kind.
    Event(String),
    /// `waste:<w>` — the instance has at least one segment tagged
    /// with this [`WASTES`] waste type.
    Waste(String),
    /// `outcome:<o>` — the instance's recorded [`OUTCOMES`] outcome.
    Outcome(String),
    /// `setting:<s>` — the pathway's own declared care setting,
    /// lowercased (`outpatient`, `inpatient`, …) — the same
    /// derivation `care_setting_string` uses elsewhere in this crate.
    /// Free-form here: the value is the matcher's own `CareSetting`
    /// enum, not a vocabulary this module owns.
    Setting(String),
    /// `urgency:<u>` — the instance's [`URGENCY_LEVELS`] urgency.
    Urgency(String),
}

/// Check `value` against a closed vocabulary, naming both the
/// predicate and the bad value in the error so a caller sees exactly
/// what was refused.
fn in_vocabulary(kind: &str, value: &str, vocabulary: &[&str]) -> Result<(), String> {
    if vocabulary.contains(&value) {
        Ok(())
    } else {
        Err(format!(
            "`{kind}:{value}` — `{value}` is not one of {vocabulary:?}"
        ))
    }
}

impl Predicate {
    /// Parse one `type:value` token, e.g. `stage:triage`.
    ///
    /// # Errors
    /// Names the problem — an unrecognised type, a missing `:`/value,
    /// or (where a closed vocabulary exists) an unrecognised value —
    /// so the caller can refuse with a useful message rather than
    /// silently matching nothing.
    pub fn parse(token: &str) -> Result<Self, String> {
        let (kind, value) = token
            .split_once(':')
            .ok_or_else(|| format!("`{token}` is not `type:value`"))?;
        if value.is_empty() {
            return Err(format!("`{token}` has no value after `:`"));
        }
        let value = value.to_string();
        match kind {
            "stage" => in_vocabulary("stage", &value, STAGES).map(|()| Self::Stage(value)),
            "waste" => in_vocabulary("waste", &value, WASTES).map(|()| Self::Waste(value)),
            "urgency" => {
                in_vocabulary("urgency", &value, URGENCY_LEVELS).map(|()| Self::Urgency(value))
            }
            "outcome" => in_vocabulary("outcome", &value, OUTCOMES).map(|()| Self::Outcome(value)),
            "event" => in_vocabulary("event", &value, EVENT_KINDS).map(|()| Self::Event(value)),
            "step" => Ok(Self::Step(value)),
            "setting" => Ok(Self::Setting(value)),
            other => Err(format!(
                "`{other}` is not a recognised predicate type \
                 (stage, step, event, waste, outcome, setting, urgency)"
            )),
        }
    }

    /// Whether `features` satisfies this predicate.
    #[must_use]
    pub fn matches(&self, features: &Features) -> bool {
        match self {
            Self::Stage(v) => features.stages.contains(v),
            Self::Step(v) => features.steps.contains(v),
            Self::Event(v) => features.events.contains(v),
            Self::Waste(v) => features.wastes.contains(v),
            Self::Outcome(v) => features.outcome.as_deref() == Some(v.as_str()),
            Self::Setting(v) => features.setting.as_deref() == Some(v.as_str()),
            Self::Urgency(v) => features.urgency == *v,
        }
    }
}

/// One instance's features against which [`Predicate`]s are evaluated
/// (spec T-14f) — built from the same rows
/// [`crate::analytics::event_log_rows`] already loads, so there is one
/// source of truth for "what did this instance do", not two.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Features {
    /// Distinct stages this instance has a segment in.
    pub stages: BTreeSet<String>,
    /// Distinct labels of this instance's *completed* steps.
    pub steps: BTreeSet<String>,
    /// Distinct kinds of this instance's recorded events.
    pub events: BTreeSet<String>,
    /// Distinct waste types across this instance's segments.
    pub wastes: BTreeSet<String>,
    /// The instance's recorded outcome, once closed.
    pub outcome: Option<String>,
    /// The pathway's own declared care setting.
    pub setting: Option<String>,
    /// The instance's urgency.
    pub urgency: String,
}

/// Build one instance's [`Features`] from the same inputs
/// [`crate::analytics::event_log_rows`] takes.
#[must_use]
pub fn features_of(
    ctx: &CaseContext,
    segments: &[SegmentInput],
    steps: &[StepInput],
    events: &[EventInput],
) -> Features {
    Features {
        stages: segments.iter().map(|s| s.stage.clone()).collect(),
        wastes: segments.iter().filter_map(|s| s.waste.clone()).collect(),
        steps: steps
            .iter()
            .filter(|s| s.done_at_ms.is_some())
            .map(|s| s.label.clone())
            .collect(),
        events: events.iter().map(|e| e.kind.clone()).collect(),
        outcome: ctx.outcome.clone(),
        setting: ctx.care_setting.clone(),
        urgency: ctx.urgency.clone(),
    }
}

/// A parsed `contains=`/`excludes=` rule (spec T-14f): an instance is
/// on the **matched** side iff it satisfies every `contains`
/// predicate (AND) and none of the `excludes` predicates. Naming
/// neither is the identity rule — everything matches, nothing is
/// excluded — so an absent query reproduces today's unsplit cohort
/// exactly.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Rule {
    /// Predicates every matched instance must satisfy.
    pub contains: Vec<Predicate>,
    /// Predicates no matched instance may satisfy.
    pub excludes: Vec<Predicate>,
}

/// Parse a comma-separated list of `type:value` tokens (either
/// `contains=` or `excludes=`); `None` or blank is an empty list.
fn parse_tokens(raw: Option<&str>) -> Result<Vec<Predicate>, String> {
    raw.map(|raw| {
        raw.split(',')
            .map(str::trim)
            .filter(|token| !token.is_empty())
            .map(Predicate::parse)
            .collect::<Result<Vec<_>, _>>()
    })
    .transpose()
    .map(Option::unwrap_or_default)
}

impl Rule {
    /// Parse `?contains=`/`?excludes=` — each a comma-separated list
    /// of `type:value` tokens, either or both absent.
    ///
    /// # Errors
    /// The first malformed or unrecognised token's own error, from
    /// [`Predicate::parse`].
    pub fn parse(contains: Option<&str>, excludes: Option<&str>) -> Result<Self, String> {
        Ok(Self {
            contains: parse_tokens(contains)?,
            excludes: parse_tokens(excludes)?,
        })
    }

    /// The identity rule matches everything — the case an absent
    /// query must produce, so behaviour is unchanged with no filter.
    #[must_use]
    pub fn is_identity(&self) -> bool {
        self.contains.is_empty() && self.excludes.is_empty()
    }

    /// Whether `features` is on the matched side of this rule.
    #[must_use]
    pub fn matches(&self, features: &Features) -> bool {
        self.contains.iter().all(|p| p.matches(features))
            && !self.excludes.iter().any(|p| p.matches(features))
    }
}

/// Partition `features` (parallel to whatever cohort slice the caller
/// is filtering) by `rule`, returning the matched indices and the
/// complement indices. Every index appears in exactly one of the two
/// — the acceptance criterion "split and complement sizes sum to the
/// unsplit cohort" holds by construction, not merely by convention.
#[must_use]
pub fn partition(rule: &Rule, features: &[Features]) -> (Vec<usize>, Vec<usize>) {
    let mut matched = Vec::new();
    let mut complement = Vec::new();
    for (index, item) in features.iter().enumerate() {
        if rule.matches(item) {
            matched.push(index);
        } else {
            complement.push(index);
        }
    }
    (matched, complement)
}

/// The two-cell [`suppression::Table`] for a matched/complement split
/// (spec T-14f, T-14k). The unsplit cohort size is a *published*
/// margin — every cohort endpoint already shows its own `instances`
/// count — so if the matched side's detail were shown suppressed
/// while the complement's detail (aggregate sums like per-stage time)
/// stayed visible, the matched side's sums would be exactly
/// recoverable as `unsplit − complement`. [`suppression::decide`] on
/// this table decides which side's *detail* to withhold to prevent
/// that — never the bare counts, which this family never hides
/// (`agents/share/time-based-analysis.md` §12.2: hide detail, not the
/// count).
#[must_use]
pub fn split_table(n_matched: usize, n_complement: usize) -> suppression::Table {
    suppression::Table {
        cells: vec![
            suppression::Cell {
                coords: vec!["matched".to_string()],
                count: n_matched,
            },
            suppression::Cell {
                coords: vec!["complement".to_string()],
                count: n_complement,
            },
        ],
        partitions: vec![suppression::Partition {
            label: "split".to_string(),
            cell_indices: vec![0, 1],
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn features(
        stages: &[&str],
        steps: &[&str],
        events: &[&str],
        wastes: &[&str],
        outcome: Option<&str>,
        setting: Option<&str>,
        urgency: &str,
    ) -> Features {
        Features {
            stages: stages.iter().map(ToString::to_string).collect(),
            steps: steps.iter().map(ToString::to_string).collect(),
            events: events.iter().map(ToString::to_string).collect(),
            wastes: wastes.iter().map(ToString::to_string).collect(),
            outcome: outcome.map(ToString::to_string),
            setting: setting.map(ToString::to_string),
            urgency: urgency.to_string(),
        }
    }

    #[test]
    fn predicate_parses_recognised_types_and_values() {
        assert_eq!(
            Predicate::parse("stage:triage"),
            Ok(Predicate::Stage("triage".to_string()))
        );
        assert_eq!(
            Predicate::parse("step:consent"),
            Ok(Predicate::Step("consent".to_string())),
            "step is free-form, no vocabulary"
        );
        assert_eq!(
            Predicate::parse("setting:outpatient"),
            Ok(Predicate::Setting("outpatient".to_string())),
            "setting is free-form, no vocabulary"
        );
        assert_eq!(
            Predicate::parse("urgency:routine"),
            Ok(Predicate::Urgency("routine".to_string()))
        );
        assert_eq!(
            Predicate::parse("outcome:improved"),
            Ok(Predicate::Outcome("improved".to_string()))
        );
        assert_eq!(
            Predicate::parse("event:review"),
            Ok(Predicate::Event("review".to_string()))
        );
        assert_eq!(
            Predicate::parse("waste:waiting"),
            Ok(Predicate::Waste("waiting".to_string()))
        );
    }

    #[test]
    fn predicate_parse_refuses_malformed_or_unrecognised_input() {
        assert!(Predicate::parse("nocolon").is_err());
        assert!(Predicate::parse("stage:").is_err(), "no value");
        assert!(Predicate::parse("sideways:triage").is_err(), "no such type");
        assert!(
            Predicate::parse("stage:not_a_stage").is_err(),
            "closed vocabulary rejects an unknown value"
        );
        assert!(Predicate::parse("waste:not_a_waste").is_err());
        assert!(Predicate::parse("urgency:not_a_level").is_err());
        assert!(Predicate::parse("outcome:not_an_outcome").is_err());
        assert!(Predicate::parse("event:not_a_kind").is_err());
    }

    #[test]
    fn rule_with_no_predicates_is_the_identity() {
        let rule = Rule::parse(None, None).unwrap();
        assert!(rule.is_identity());
        let f = features(&[], &[], &[], &[], None, None, "routine");
        assert!(rule.matches(&f), "identity rule matches everything");
    }

    #[test]
    fn contains_is_and_excludes_is_none_of() {
        let rule = Rule::parse(
            Some("stage:triage,urgency:routine"),
            Some("outcome:deceased"),
        )
        .unwrap();
        assert!(!rule.is_identity());

        let both_contains_no_excluded = features(
            &["triage"],
            &[],
            &[],
            &[],
            Some("improved"),
            None,
            "routine",
        );
        assert!(rule.matches(&both_contains_no_excluded));

        let missing_one_contains = features(&["triage"], &[], &[], &[], None, None, "urgent");
        assert!(
            !rule.matches(&missing_one_contains),
            "contains is AND, not OR"
        );

        let hits_an_exclude = features(
            &["triage"],
            &[],
            &[],
            &[],
            Some("deceased"),
            None,
            "routine",
        );
        assert!(
            !rule.matches(&hits_an_exclude),
            "any excludes predicate disqualifies, even with every contains met"
        );
    }

    #[test]
    fn partition_sizes_sum_to_the_whole_and_every_index_appears_once() {
        let rule = Rule::parse(Some("stage:triage"), None).unwrap();
        let pool = [
            features(&["triage"], &[], &[], &[], None, None, "routine"),
            features(&["referral"], &[], &[], &[], None, None, "routine"),
            features(&["triage", "referral"], &[], &[], &[], None, None, "urgent"),
            features(&[], &[], &[], &[], None, None, "routine"),
        ];
        let (matched, complement) = partition(&rule, &pool);
        assert_eq!(matched.len() + complement.len(), pool.len());
        let mut all: Vec<usize> = matched.iter().chain(&complement).copied().collect();
        all.sort_unstable();
        assert_eq!(all, vec![0, 1, 2, 3], "every index appears exactly once");
        assert_eq!(matched, vec![0, 2]);
        assert_eq!(complement, vec![1, 3]);
    }

    #[test]
    fn split_table_suppresses_neither_side_when_both_clear_the_floor() {
        let table = split_table(10, 20);
        assert!(suppression::decide(&table, 5).is_empty());
    }

    #[test]
    fn split_table_suppresses_the_complement_too_when_matched_is_small() {
        // Matched alone is below the floor; without protecting the
        // complement too, matched's suppressed sums would be exactly
        // `unsplit - complement` (spec T-14k's own scenario).
        let table = split_table(2, 30);
        let suppressed = suppression::decide(&table, 5);
        assert_eq!(
            suppressed,
            std::collections::BTreeSet::from([0, 1]),
            "the lone small side always recruits its only sibling"
        );
    }

    #[test]
    fn split_table_needs_no_secondary_suppression_when_both_sides_are_already_small() {
        let table = split_table(2, 3);
        let suppressed = suppression::decide(&table, 5);
        assert_eq!(
            suppressed,
            std::collections::BTreeSet::from([0, 1]),
            "already two unknowns, one equation -- no further recruitment needed"
        );
    }
}
