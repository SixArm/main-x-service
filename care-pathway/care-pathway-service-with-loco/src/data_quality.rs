//! Journey data-quality and missingness report (spec `13-tasks.md`
//! T-14h): eight closed-vocabulary defect codes (BNSSG's `bad_date`
//! 1–5, generalised) plus per-stage missingness percentage and
//! entropy. **The report is the finding; it never imputes** — a
//! missing or malformed value is disclosed as a count, never filled
//! in (`agents/share/time-based-analysis.md` §6.6's own refusal,
//! restated here for a whole cohort rather than one instance).
//!
//! Reuses [`tba::Segment`]/[`tba::Clock`]/[`tba::StageAnchor`] rather
//! than a parallel input shape — the same discipline [`crate::split`]
//! already follows for T-14f's rule predicates, and
//! [`crate::data::journeys`]'s own `DEFECT_CODES` names the exact
//! eight conditions this module detects (that generator's injected
//! defects are what this report's own acceptance test proves each
//! code against).

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::instances;
use crate::tba;

/// The closed, ordered vocabulary of data-quality codes (spec T-14h),
/// matching [`crate::data::journeys::DEFECT_CODES`] name for name.
pub const DQ_CODES: &[&str] = &[
    "no_segments",
    "open_segment_past_closure",
    "terminal_without_clock_stop",
    "step_done_before_enrolled",
    "steps_out_of_order",
    "segment_clipped_by_clock",
    "coverage_below_floor",
    "anchors_unreached",
];

/// One instance step's position and completion time — the two facts
/// [`has_steps_out_of_order`] needs that
/// [`crate::analytics::StepInput`] does not carry (that type serves
/// the event-log codec, which needs only a label and a completion
/// time, never a position).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StepOrder {
    /// Declared order among this instance's steps.
    pub position: i32,
    /// Completion time, or `None` if not yet done.
    pub done_at_ms: Option<i64>,
}

/// `code`: no segments recorded at all.
#[must_use]
pub fn has_no_segments(segments: &[tba::Segment]) -> bool {
    segments.is_empty()
}

/// `code`: the instance is closed (spec §5.2's terminal statuses) but
/// still has a segment with no `ended_at` — a running interval that
/// outlived the journey it was measuring.
#[must_use]
pub fn has_open_segment_past_closure(status: &str, segments: &[tba::Segment]) -> bool {
    instances::is_terminal(status) && segments.iter().any(|s| s.end_ms.is_none())
}

/// `code`: the instance is closed but never recorded an explicit
/// clock stop — `clock.stop_source` fell back to `closed_on` (or, in
/// the rarer case of neither being set, `as_of`) rather than reading
/// `clock_stop_at` directly.
#[must_use]
pub fn has_terminal_without_clock_stop(status: &str, clock: &tba::Clock) -> bool {
    instances::is_terminal(status) && clock.stop_source != "clock_stop_at"
}

/// `code`: a step was marked done before the instance was even
/// enrolled — BNSSG's `bad_date` class, generalised to this domain.
#[must_use]
pub fn has_step_done_before_enrolled(steps: &[StepOrder], enrolled_on_ms: i64) -> bool {
    steps
        .iter()
        .any(|s| s.done_at_ms.is_some_and(|d| d < enrolled_on_ms))
}

/// `code`: a later-declared step (by `position`) was completed before
/// an earlier-declared one.
#[must_use]
pub fn has_steps_out_of_order(steps: &[StepOrder]) -> bool {
    let mut done: Vec<&StepOrder> = steps.iter().filter(|s| s.done_at_ms.is_some()).collect();
    done.sort_by_key(|s| s.position);
    done.windows(2)
        .any(|pair| pair[0].done_at_ms > pair[1].done_at_ms)
}

/// `code`: a segment falls partly or wholly outside the resolved
/// clock window, so [`tba::clip`] would shrink (or entirely drop) it
/// — the same clipping `tba::analyze` already applies when scoring
/// this instance, surfaced here as a named, countable condition
/// rather than a silent adjustment. An open segment's effective end
/// is `as_of_ms`, matching how `analyze` itself bounds a still-running
/// interval.
#[must_use]
pub fn has_segment_clipped_by_clock(
    segments: &[tba::Segment],
    clock: &tba::Clock,
    as_of_ms: i64,
) -> bool {
    segments.iter().any(|s| {
        let effective = (s.start_ms, s.end_ms.unwrap_or(as_of_ms));
        tba::clip(effective, (clock.start_ms, clock.stop_ms)) != Some(effective)
    })
}

/// `code`: this instance's mapped coverage is below
/// [`tba::COVERAGE_UNMAPPED`] — the same threshold `analyze` already
/// uses to label a journey's confidence `"unmapped"`.
#[must_use]
pub fn has_coverage_below_floor(coverage_ratio: Option<f64>) -> bool {
    coverage_ratio.is_some_and(|v| v < tba::COVERAGE_UNMAPPED)
}

/// `code`: the requested `from_anchor`/`to_anchor` pair (reusing
/// T-14d's own mechanism) was never reached — `None` whenever no pair
/// was requested at all, since there is then nothing to check; see
/// [`DataQualityReport`]'s own `anchor_note` field for how that
/// absence is disclosed at the report level rather than silently read
/// as "zero instances have this problem".
#[must_use]
pub fn has_anchors_unreached(
    anchors: &[tba::StageAnchor],
    anchor_pair: Option<(&str, &str)>,
) -> bool {
    match anchor_pair {
        Some((from, to)) => tba::anchor_interval(anchors, from, to).is_none(),
        None => false,
    }
}

/// Everything one instance contributes to a data-quality report —
/// already-loaded or already-derived data, no I/O of its own.
#[derive(Clone, Debug)]
pub struct InstanceDqInputs<'a> {
    /// [`instances::INSTANCE_STATUSES`] value.
    pub status: &'a str,
    /// This instance's recorded segments.
    pub segments: &'a [tba::Segment],
    /// This instance's recorded steps.
    pub steps: &'a [StepOrder],
    /// The resolved clock `tba::analyze` scored this instance against.
    pub clock: &'a tba::Clock,
    /// Epoch milliseconds of `enrolled_on`.
    pub enrolled_on_ms: i64,
    /// The report's own `as_of`, for bounding a still-open segment.
    pub as_of_ms: i64,
    /// This instance's own `coverage_ratio.value`.
    pub coverage_ratio: Option<f64>,
    /// This instance's own stage anchors (spec T-14d).
    pub anchors: &'a [tba::StageAnchor],
}

/// Every [`DQ_CODES`] entry this one instance's inputs exhibit right
/// now (usually zero or one, but never assumed to be — a badly
/// malformed instance can carry more than one condition at once).
#[must_use]
pub fn codes_for_instance(
    inputs: &InstanceDqInputs<'_>,
    anchor_pair: Option<(&str, &str)>,
) -> Vec<&'static str> {
    let mut codes = Vec::new();
    if has_no_segments(inputs.segments) {
        codes.push("no_segments");
    }
    if has_open_segment_past_closure(inputs.status, inputs.segments) {
        codes.push("open_segment_past_closure");
    }
    if has_terminal_without_clock_stop(inputs.status, inputs.clock) {
        codes.push("terminal_without_clock_stop");
    }
    if has_step_done_before_enrolled(inputs.steps, inputs.enrolled_on_ms) {
        codes.push("step_done_before_enrolled");
    }
    if has_steps_out_of_order(inputs.steps) {
        codes.push("steps_out_of_order");
    }
    if has_segment_clipped_by_clock(inputs.segments, inputs.clock, inputs.as_of_ms) {
        codes.push("segment_clipped_by_clock");
    }
    if has_coverage_below_floor(inputs.coverage_ratio) {
        codes.push("coverage_below_floor");
    }
    if has_anchors_unreached(inputs.anchors, anchor_pair) {
        codes.push("anchors_unreached");
    }
    codes
}

/// One [`DQ_CODES`] entry's share across the analysed cohort.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DqCodeShare {
    /// One of [`DQ_CODES`].
    pub code: &'static str,
    /// Instances exhibiting this code right now.
    pub instances: usize,
    /// `instances / n`; `None` only when the cohort itself is empty
    /// (nothing to divide by, not a `0.0` that would read as "clean").
    pub share: Option<f64>,
}

/// One [`tba::STAGES`] entry's missingness across the cohort: the
/// share of instances with no segment in this stage at all, plus the
/// binary entropy of that same share — `H(p) = -p·log2(p) −
/// (1−p)·log2(1−p)` bits, the standard entropy of a two-outcome
/// (present / missing) variable. `0` at `p = 0` or `p = 1` (no
/// disorder — every instance agrees), peaking at `1` bit when exactly
/// half the cohort is missing the stage (maximum disorder).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct StageMissingness {
    /// One of [`tba::STAGES`].
    pub stage: &'static str,
    /// Instances with no segment in this stage.
    pub missing_instances: usize,
    /// `missing_instances / n`; `None` only when the cohort is empty.
    pub missing_share: Option<f64>,
    /// The binary entropy of `missing_share`, bits; `None` under the
    /// same empty-cohort condition as `missing_share`.
    pub entropy_bits: Option<f64>,
}

/// The binary entropy of a Bernoulli variable with success
/// probability `p`, bits. Defined as `0.0` at the boundaries
/// (`p <= 0.0 || p >= 1.0`) rather than evaluating `log2(0)`, which
/// is mathematically the correct limit (`x·log2(x) -> 0` as `x -> 0`)
/// as well as the computationally necessary one.
#[must_use]
fn binary_entropy_bits(p: f64) -> f64 {
    if p <= 0.0 || p >= 1.0 {
        0.0
    } else {
        -(p * p.log2() + (1.0 - p) * (1.0 - p).log2())
    }
}

/// The whole data-quality report over one cohort (spec T-14h).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DataQualityReport {
    /// Instances analysed.
    pub instances: usize,
    /// Every [`DQ_CODES`] entry's share, in the same fixed order —
    /// present at `0` for a clean cohort, never omitted (a code this
    /// crate can detect but a cohort does not exhibit is itself a
    /// finding, the same "an empty cell is a finding" rule T-14g's
    /// attrition trail already lives by).
    pub codes: Vec<DqCodeShare>,
    /// Every [`tba::STAGES`] entry's missingness, in `STAGES` order.
    pub missingness: Vec<StageMissingness>,
    /// Set when no `from_anchor`/`to_anchor` pair was requested, so
    /// `anchors_unreached`'s own `0` reads as "not evaluated", never
    /// as "no instance has this problem" — the same disclosed-but-inert
    /// convention T-14g's `window`/`coverage_floor` steps already use.
    pub anchor_note: Option<&'static str>,
}

/// Disclosed reason `anchors_unreached` is always `0` with no
/// `from_anchor`/`to_anchor` pair requested.
pub const NO_ANCHOR_PAIR_REQUESTED: &str =
    "anchors_unreached is not evaluated without a from_anchor/to_anchor pair";

/// Build the whole report from every instance's already-loaded inputs
/// — no I/O, no DB access. `anchor_pair` is the exact `Result` T-14d's
/// own `resolve_anchor_pair_raw` produces, so this one parameter
/// carries all three cases `anchor_note` needs to distinguish: a
/// valid pair (`Ok(Some(_))`, evaluated, no note), none requested
/// (`Ok(None)`, [`NO_ANCHOR_PAIR_REQUESTED`]), and a malformed or
/// one-sided one (`Err(reason)`, that reason verbatim) — never
/// silently treated as "no problem found" in either of the latter
/// two cases.
#[must_use]
pub fn build_report(
    instances: &[InstanceDqInputs<'_>],
    anchor_pair: Result<Option<(&str, &str)>, &'static str>,
) -> DataQualityReport {
    let n = instances.len();
    #[allow(clippy::cast_precision_loss)] // cohort sizes are far below f64's exact-integer range
    let share = |count: usize| (n > 0).then(|| count as f64 / n as f64);
    let pair = anchor_pair.unwrap_or(None);

    let mut code_counts: BTreeMap<&'static str, usize> =
        DQ_CODES.iter().map(|&code| (code, 0)).collect();
    let mut missing_counts: BTreeMap<&'static str, usize> =
        tba::STAGES.iter().map(|&stage| (stage, 0)).collect();

    for inputs in instances {
        for code in codes_for_instance(inputs, pair) {
            // Every code `codes_for_instance` can return is a
            // `DQ_CODES` entry, and `code_counts` is pre-populated
            // with exactly those keys above.
            *code_counts.entry(code).or_insert(0) += 1;
        }
        let present: BTreeSet<&str> = inputs.segments.iter().map(|s| s.stage.as_str()).collect();
        for &stage in tba::STAGES {
            if !present.contains(stage) {
                *missing_counts.entry(stage).or_insert(0) += 1;
            }
        }
    }

    let codes = DQ_CODES
        .iter()
        .map(|&code| {
            let count = code_counts.get(code).copied().unwrap_or(0);
            DqCodeShare {
                code,
                instances: count,
                share: share(count),
            }
        })
        .collect();

    let missingness = tba::STAGES
        .iter()
        .map(|&stage| {
            let missing = missing_counts.get(stage).copied().unwrap_or(0);
            let missing_share = share(missing);
            StageMissingness {
                stage,
                missing_instances: missing,
                missing_share,
                entropy_bits: missing_share.map(binary_entropy_bits),
            }
        })
        .collect();

    let anchor_note = match anchor_pair {
        Ok(Some(_)) => None,
        Ok(None) => Some(NO_ANCHOR_PAIR_REQUESTED),
        Err(reason) => Some(reason),
    };

    DataQualityReport {
        instances: n,
        codes,
        missingness,
        anchor_note,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: i64 = 1_700_000_000_000;
    const DAY_MS: i64 = 86_400_000;

    fn seg(stage: &str, start_day: i64, end_day: Option<i64>) -> tba::Segment {
        tba::Segment {
            label: stage.to_string(),
            stage: stage.to_string(),
            category: tba::CATEGORY_VALUE_ADDING.to_string(),
            waste: None,
            start_ms: T0 + start_day * DAY_MS,
            end_ms: end_day.map(|d| T0 + d * DAY_MS),
            actor_ref: None,
            location_ref: None,
        }
    }

    fn clock(start_day: i64, stop_day: i64, stop_source: &'static str) -> tba::Clock {
        tba::Clock {
            start_ms: T0 + start_day * DAY_MS,
            stop_ms: T0 + stop_day * DAY_MS,
            start_source: "clock_start_at",
            stop_source,
            running: false,
        }
    }

    fn baseline_inputs<'a>(
        segments: &'a [tba::Segment],
        steps: &'a [StepOrder],
        clock: &'a tba::Clock,
        anchors: &'a [tba::StageAnchor],
    ) -> InstanceDqInputs<'a> {
        InstanceDqInputs {
            status: "active",
            segments,
            steps,
            clock,
            enrolled_on_ms: T0,
            as_of_ms: T0 + 100 * DAY_MS,
            coverage_ratio: Some(1.0),
            anchors,
        }
    }

    #[test]
    fn no_segments_is_detected_only_when_there_are_none() {
        assert!(has_no_segments(&[]));
        assert!(!has_no_segments(&[seg("triage", 0, Some(1))]));
    }

    #[test]
    fn open_segment_past_closure_needs_both_terminal_and_open() {
        let open = [seg("triage", 0, None)];
        let closed = [seg("triage", 0, Some(1))];
        assert!(has_open_segment_past_closure("completed", &open));
        assert!(has_open_segment_past_closure("discontinued", &open));
        assert!(
            !has_open_segment_past_closure("active", &open),
            "still open is fine while the instance itself is still open"
        );
        assert!(!has_open_segment_past_closure("completed", &closed));
    }

    #[test]
    fn terminal_without_clock_stop_reads_the_resolved_source() {
        let via_stop = clock(0, 10, "clock_stop_at");
        let via_closed_on = clock(0, 10, "closed_on");
        assert!(!has_terminal_without_clock_stop("completed", &via_stop));
        assert!(has_terminal_without_clock_stop("completed", &via_closed_on));
        assert!(
            !has_terminal_without_clock_stop("active", &via_closed_on),
            "not terminal, so this code does not apply regardless of source"
        );
    }

    #[test]
    fn step_done_before_enrolled_compares_against_the_enrolment_time() {
        let before = [StepOrder {
            position: 0,
            done_at_ms: Some(T0 - DAY_MS),
        }];
        let after = [StepOrder {
            position: 0,
            done_at_ms: Some(T0 + DAY_MS),
        }];
        let undone = [StepOrder {
            position: 0,
            done_at_ms: None,
        }];
        assert!(has_step_done_before_enrolled(&before, T0));
        assert!(!has_step_done_before_enrolled(&after, T0));
        assert!(!has_step_done_before_enrolled(&undone, T0));
    }

    #[test]
    fn steps_out_of_order_compares_by_position_not_by_input_order() {
        let ordered = [
            StepOrder {
                position: 0,
                done_at_ms: Some(T0),
            },
            StepOrder {
                position: 1,
                done_at_ms: Some(T0 + DAY_MS),
            },
        ];
        let out_of_order = [
            StepOrder {
                position: 0,
                done_at_ms: Some(T0 + DAY_MS),
            },
            StepOrder {
                position: 1,
                done_at_ms: Some(T0),
            },
        ];
        let tied = [
            StepOrder {
                position: 0,
                done_at_ms: Some(T0),
            },
            StepOrder {
                position: 1,
                done_at_ms: Some(T0),
            },
        ];
        assert!(!has_steps_out_of_order(&ordered));
        assert!(has_steps_out_of_order(&out_of_order));
        assert!(!has_steps_out_of_order(&tied), "a tie is not out of order");
    }

    #[test]
    fn segment_clipped_by_clock_detects_any_boundary_overrun() {
        let window = clock(0, 10, "clock_stop_at");
        let inside = [seg("triage", 1, Some(2))];
        let starts_early = [seg("triage", -1, Some(2))];
        let ends_late = [seg("triage", 8, Some(20))];
        let still_open_within_bounds = [seg("triage", 8, None)];
        assert!(!has_segment_clipped_by_clock(
            &inside,
            &window,
            T0 + 5 * DAY_MS
        ));
        assert!(has_segment_clipped_by_clock(
            &starts_early,
            &window,
            T0 + 5 * DAY_MS
        ));
        assert!(has_segment_clipped_by_clock(
            &ends_late,
            &window,
            T0 + 5 * DAY_MS
        ));
        assert!(!has_segment_clipped_by_clock(
            &still_open_within_bounds,
            &window,
            T0 + 9 * DAY_MS
        ));
    }

    #[test]
    fn coverage_below_floor_uses_the_shared_unmapped_threshold() {
        assert!(has_coverage_below_floor(Some(tba::COVERAGE_UNMAPPED / 2.0)));
        assert!(!has_coverage_below_floor(Some(1.0)));
        assert!(!has_coverage_below_floor(None));
    }

    #[test]
    fn anchors_unreached_is_inert_without_a_requested_pair() {
        let anchors = vec![
            tba::StageAnchor {
                stage: "referral".to_string(),
                first_started_at_ms: Some(T0),
            },
            tba::StageAnchor {
                stage: "diagnostics".to_string(),
                first_started_at_ms: None,
            },
        ];
        assert!(has_anchors_unreached(
            &anchors,
            Some(("referral", "diagnostics"))
        ));
        assert!(!has_anchors_unreached(
            &anchors,
            Some(("referral", "referral"))
        ));
        assert!(
            !has_anchors_unreached(&anchors, None),
            "no pair requested means the code never fires"
        );
    }

    /// The literal T-14h acceptance bullet, mirrored against
    /// `data::journeys`'s own eight injected defects (hand-built here
    /// rather than run through the generator, matching every sibling
    /// T-14 pure-layer test's own precedent): each code fires on
    /// exactly the one instance built to carry it, never on the
    /// others.
    #[test]
    #[allow(clippy::too_many_lines)] // nine hand-built instances, one per DQ_CODES entry plus one clean
    fn each_dq_code_fires_on_exactly_the_matching_instance() {
        let clean_clock = clock(0, 10, "clock_stop_at");
        let clean_segments = [seg("referral", 0, Some(1)), seg("diagnostics", 2, Some(3))];
        let clean_steps = [StepOrder {
            position: 0,
            done_at_ms: Some(T0 + DAY_MS),
        }];
        let clean_anchors = vec![
            tba::StageAnchor {
                stage: "referral".to_string(),
                first_started_at_ms: Some(T0),
            },
            tba::StageAnchor {
                stage: "diagnostics".to_string(),
                first_started_at_ms: Some(T0 + 2 * DAY_MS),
            },
        ];

        let no_segments_inputs = InstanceDqInputs {
            segments: &[],
            ..baseline_inputs(&clean_segments, &clean_steps, &clean_clock, &clean_anchors)
        };
        let open_segment = [seg("triage", 0, None)];
        let open_segment_inputs = InstanceDqInputs {
            status: "completed",
            segments: &open_segment,
            // Matches the clock's own 10-day stop, so this open
            // segment's `as_of`-bounded effective end lands exactly
            // at the window edge -- isolating this instance to
            // `open_segment_past_closure` alone, not also
            // `segment_clipped_by_clock`.
            as_of_ms: T0 + 10 * DAY_MS,
            ..baseline_inputs(&clean_segments, &clean_steps, &clean_clock, &clean_anchors)
        };
        let no_stop_clock = clock(0, 10, "closed_on");
        let no_stop_inputs = InstanceDqInputs {
            status: "completed",
            clock: &no_stop_clock,
            ..baseline_inputs(&clean_segments, &clean_steps, &clean_clock, &clean_anchors)
        };
        let early_step = [StepOrder {
            position: 0,
            done_at_ms: Some(T0 - DAY_MS),
        }];
        let early_step_inputs = InstanceDqInputs {
            steps: &early_step,
            ..baseline_inputs(&clean_segments, &clean_steps, &clean_clock, &clean_anchors)
        };
        let out_of_order_steps = [
            StepOrder {
                position: 0,
                done_at_ms: Some(T0 + 5 * DAY_MS),
            },
            StepOrder {
                position: 1,
                done_at_ms: Some(T0 + DAY_MS),
            },
        ];
        let out_of_order_inputs = InstanceDqInputs {
            steps: &out_of_order_steps,
            ..baseline_inputs(&clean_segments, &clean_steps, &clean_clock, &clean_anchors)
        };
        let clipped_segments = [seg("triage", -2, Some(-1))];
        let clipped_inputs = InstanceDqInputs {
            segments: &clipped_segments,
            ..baseline_inputs(&clean_segments, &clean_steps, &clean_clock, &clean_anchors)
        };
        let low_coverage_inputs = InstanceDqInputs {
            coverage_ratio: Some(0.01),
            ..baseline_inputs(&clean_segments, &clean_steps, &clean_clock, &clean_anchors)
        };
        let unreached_anchors = vec![
            tba::StageAnchor {
                stage: "referral".to_string(),
                first_started_at_ms: Some(T0),
            },
            tba::StageAnchor {
                stage: "diagnostics".to_string(),
                first_started_at_ms: None,
            },
        ];
        let unreached_inputs = InstanceDqInputs {
            anchors: &unreached_anchors,
            ..baseline_inputs(&clean_segments, &clean_steps, &clean_clock, &clean_anchors)
        };
        let clean_inputs =
            baseline_inputs(&clean_segments, &clean_steps, &clean_clock, &clean_anchors);

        let all_inputs = [
            clean_inputs,
            no_segments_inputs,
            open_segment_inputs,
            no_stop_inputs,
            early_step_inputs,
            out_of_order_inputs,
            clipped_inputs,
            low_coverage_inputs,
            unreached_inputs,
        ];
        let report = build_report(&all_inputs, Ok(Some(("referral", "diagnostics"))));
        assert_eq!(report.instances, 9);
        assert_eq!(report.codes.len(), DQ_CODES.len(), "every code has a row");
        for expected in DQ_CODES {
            let row = report
                .codes
                .iter()
                .find(|c| c.code == *expected)
                .unwrap_or_else(|| panic!("missing row for {expected}"));
            assert_eq!(
                row.instances, 1,
                "{expected} should fire on exactly one instance: {row:?}"
            );
        }
    }

    /// The literal T-14h acceptance bullet: a clean cohort reports
    /// every code at zero, rows present — never an omitted row.
    #[test]
    fn a_clean_cohort_reports_every_code_at_zero_with_rows_present() {
        let clean_clock = clock(0, 10, "clock_stop_at");
        let clean_segments = [seg("referral", 0, Some(1)), seg("diagnostics", 2, Some(3))];
        let clean_steps = [StepOrder {
            position: 0,
            done_at_ms: Some(T0 + DAY_MS),
        }];
        let clean_anchors = vec![
            tba::StageAnchor {
                stage: "referral".to_string(),
                first_started_at_ms: Some(T0),
            },
            tba::StageAnchor {
                stage: "diagnostics".to_string(),
                first_started_at_ms: Some(T0 + 2 * DAY_MS),
            },
        ];
        let instances: Vec<InstanceDqInputs<'_>> = (0..5)
            .map(|_| baseline_inputs(&clean_segments, &clean_steps, &clean_clock, &clean_anchors))
            .collect();
        let report = build_report(&instances, Ok(Some(("referral", "diagnostics"))));
        assert_eq!(report.instances, 5);
        assert_eq!(report.codes.len(), DQ_CODES.len());
        for row in &report.codes {
            assert_eq!(row.instances, 0, "{row:?}");
            assert_eq!(row.share, Some(0.0), "{row:?}");
        }
        assert_eq!(report.anchor_note, None, "a pair was requested");
    }

    #[test]
    fn anchor_note_discloses_an_absent_pair_rather_than_a_silent_zero() {
        let report = build_report(&[], Ok(None));
        assert_eq!(report.anchor_note, Some(NO_ANCHOR_PAIR_REQUESTED));
        let anchors_row = report
            .codes
            .iter()
            .find(|c| c.code == "anchors_unreached")
            .unwrap();
        assert_eq!(anchors_row.instances, 0);
    }

    /// A malformed or one-sided pair carries its own specific reason
    /// through to `anchor_note` — never silently collapsed to the
    /// generic "no pair requested" note, and never a fabricated `0`
    /// read as "no problem found".
    #[test]
    fn a_malformed_anchor_pair_carries_its_own_reason() {
        let report = build_report(&[], Err("unknown_from_anchor"));
        assert_eq!(report.anchor_note, Some("unknown_from_anchor"));
    }

    #[test]
    fn an_empty_cohort_reports_every_row_with_no_share_to_divide() {
        let report = build_report(&[], Ok(Some(("referral", "diagnostics"))));
        assert_eq!(report.instances, 0);
        assert_eq!(report.codes.len(), DQ_CODES.len());
        for row in &report.codes {
            assert_eq!(row.instances, 0);
            assert_eq!(row.share, None, "nothing to divide by, not a bare 0.0");
        }
        for row in &report.missingness {
            assert_eq!(row.missing_share, None);
            assert_eq!(row.entropy_bits, None);
        }
    }

    #[test]
    fn missingness_reports_every_stage_and_entropy_peaks_at_half() {
        // 2 of 4 instances have no `triage` segment: p = 0.5, the
        // maximum-disorder point, entropy = 1.0 bit exactly.
        let clock = clock(0, 10, "clock_stop_at");
        let steps: [StepOrder; 0] = [];
        let anchors: [tba::StageAnchor; 0] = [];
        let with_triage = [seg("triage", 0, Some(1))];
        let without_triage = [seg("referral", 0, Some(1))];
        let instances = [
            baseline_inputs(&with_triage, &steps, &clock, &anchors),
            baseline_inputs(&with_triage, &steps, &clock, &anchors),
            baseline_inputs(&without_triage, &steps, &clock, &anchors),
            baseline_inputs(&without_triage, &steps, &clock, &anchors),
        ];
        let report = build_report(&instances, Ok(None));
        assert_eq!(report.missingness.len(), tba::STAGES.len());
        let triage = report
            .missingness
            .iter()
            .find(|s| s.stage == "triage")
            .unwrap();
        assert_eq!(triage.missing_instances, 2);
        assert_eq!(triage.missing_share, Some(0.5));
        assert!(
            (triage.entropy_bits.unwrap() - 1.0).abs() < 1e-9,
            "{triage:?}"
        );
        let referral = report
            .missingness
            .iter()
            .find(|s| s.stage == "referral")
            .unwrap();
        assert_eq!(
            referral.missing_instances, 2,
            "missing from the two `with_triage` instances"
        );
    }

    #[test]
    fn binary_entropy_is_zero_at_the_boundaries() {
        assert!(binary_entropy_bits(0.0).abs() < 1e-15);
        assert!(binary_entropy_bits(1.0).abs() < 1e-15);
        assert!((binary_entropy_bits(0.5) - 1.0).abs() < 1e-9);
    }
}
