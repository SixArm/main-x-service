//! **Time-based analysis** (TBA) — pure computation over a pathway
//! instance's clock and its recorded segments. No I/O, no clock read
//! (`as_of` is always a parameter), fully deterministic, and therefore
//! unit-testable without a database.
//!
//! See `spec/time-based-analysis.md` for the contract. The three rules
//! this module exists to enforce, and which a later "simplification"
//! would quietly destroy:
//!
//! 1. **The denominator is elapsed calendar time** (§6.3), never the sum
//!    of recorded activity. Otherwise a service that records only its
//!    value-adding work scores 100%, and recording *less* scores
//!    *better* — the exact inversion TBA exists to expose.
//! 2. **Overlapping segments are unioned, not summed** (§6.4), so the
//!    ratios are provably in `[0, 1]`; the raw sum is reported
//!    separately as touch time (φ), which may exceed the lead time when
//!    care is concurrent.
//! 3. **Every millisecond of the clock lands in exactly one bucket**
//!    (§12.3): value-adding, necessary non-value-adding, unnecessary
//!    non-value-adding, or unrecorded. The four sum to the lead time by
//!    construction, so there is no category to hide time in.
//!
//! Vocabulary is the value-stream-mapping one (VA / NNVA / UNVA, LT /
//! VT / PT / %A / #HO) and the queueing-theory one (λ / μ / ρ / τ / κ);
//! see the spec's §2 for provenance.

use serde::Serialize;

/// Milliseconds in one day — the unit every duration is reported in.
pub const DAY_MS: i64 = 86_400_000;

/// Milliseconds in one hour.
pub const HOUR_MS: i64 = 3_600_000;

/// Journey stages (spec §5.1). Closed vocabulary.
pub const STAGES: &[&str] = &[
    "referral",
    "triage",
    "diagnostics",
    "treatment",
    "follow_up",
    "discharge",
    "other",
];

/// The value-adding category: the patient would recognise it as care.
pub const CATEGORY_VALUE_ADDING: &str = "value_adding";

/// Necessary non-value-adding: required, but not care (consent, safety
/// checks, statutory recording, mandated waits).
pub const CATEGORY_NECESSARY: &str = "necessary_non_value_adding";

/// Unnecessary non-value-adding: pure waste.
pub const CATEGORY_UNNECESSARY: &str = "unnecessary_non_value_adding";

/// The VSM categories (spec §2.2). Closed vocabulary.
pub const CATEGORIES: &[&str] = &[
    CATEGORY_VALUE_ADDING,
    CATEGORY_NECESSARY,
    CATEGORY_UNNECESSARY,
];

/// The eight VSM wastes. Recorded only on non-value-adding segments.
pub const WASTES: &[&str] = &[
    "waiting",
    "transportation",
    "motion",
    "over_processing",
    "defects",
    "inventory",
    "overproduction",
    "underutilised_people",
];

/// The bucket name used for clock time no segment covers.
pub const UNRECORDED: &str = "unrecorded";

/// Coverage below which a journey is reported as essentially unmapped.
pub const COVERAGE_UNMAPPED: f64 = 0.20;

/// Coverage at or above which the non-value-adding figure is treated as
/// substantially evidenced rather than inferred.
pub const COVERAGE_MAPPED: f64 = 0.80;

// ---------------------------------------------------------------------
// Vocabulary validation
// ---------------------------------------------------------------------

/// Whether `stage` is in the closed [`STAGES`] vocabulary.
#[must_use]
pub fn is_stage(stage: &str) -> bool {
    STAGES.contains(&stage)
}

/// Whether `category` is in the closed [`CATEGORIES`] vocabulary.
#[must_use]
pub fn is_category(category: &str) -> bool {
    CATEGORIES.contains(&category)
}

/// Whether `waste` is in the closed [`WASTES`] vocabulary.
#[must_use]
pub fn is_waste(waste: &str) -> bool {
    WASTES.contains(&waste)
}

/// Validate a segment's vocabulary and its `waste` coupling (spec §5.1
/// invariants 2–4).
///
/// The two coupling rules are not fussiness. A `waste` on a
/// value-adding segment is a contradiction that would corrupt the waste
/// ranking (§8.3); a missing `waste` on something declared pure waste
/// is an assertion the analysis cannot act on.
///
/// # Errors
///
/// A human-readable refusal naming the offending field and the
/// vocabulary it should have come from.
pub fn validate_classification(
    stage: &str,
    category: &str,
    waste: Option<&str>,
) -> Result<(), String> {
    if !is_stage(stage) {
        return Err(format!("unknown stage `{stage}` (stages: {STAGES:?})"));
    }
    if !is_category(category) {
        return Err(format!(
            "unknown category `{category}` (categories: {CATEGORIES:?})"
        ));
    }
    match (category, waste) {
        (CATEGORY_VALUE_ADDING, Some(w)) => Err(format!(
            "waste `{w}` is not allowed on a `{CATEGORY_VALUE_ADDING}` segment: \
             value-adding waste is a contradiction"
        )),
        (CATEGORY_UNNECESSARY, None) => Err(format!(
            "waste is required on a `{CATEGORY_UNNECESSARY}` segment \
             (wastes: {WASTES:?}): declaring time pure waste without saying \
             what kind gives the analysis nothing to act on"
        )),
        (_, Some(w)) if !is_waste(w) => Err(format!("unknown waste `{w}` (wastes: {WASTES:?})")),
        _ => Ok(()),
    }
}

/// Validate a segment's interval (spec §5.1 invariant 1).
///
/// # Errors
///
/// A human-readable refusal.
pub fn validate_interval(start_ms: i64, end_ms: Option<i64>) -> Result<(), String> {
    match end_ms {
        Some(end) if end <= start_ms => Err(
            "ended_at must be strictly after started_at (a zero-length or \
             reversed segment measures nothing)"
                .to_string(),
        ),
        _ => Ok(()),
    }
}

// ---------------------------------------------------------------------
// Interval algebra — the core of §6.4
// ---------------------------------------------------------------------

/// Merge a set of half-open intervals into a sorted, disjoint set.
///
/// Empty and reversed intervals are dropped. This is the union
/// operation the ratios depend on: without it, two clinicians treating
/// the same patient for the same hour would count as two hours and push
/// the value-adding ratio above 1.
#[must_use]
pub fn merge_intervals(mut spans: Vec<(i64, i64)>) -> Vec<(i64, i64)> {
    spans.retain(|(start, end)| end > start);
    spans.sort_unstable();
    let mut out: Vec<(i64, i64)> = Vec::with_capacity(spans.len());
    for (start, end) in spans {
        match out.last_mut() {
            Some(last) if start <= last.1 => {
                if end > last.1 {
                    last.1 = end;
                }
            }
            _ => out.push((start, end)),
        }
    }
    out
}

/// Subtract `remove` from `keep`. Both must already be merged (sorted
/// and disjoint); the result is merged too.
///
/// This is what turns three overlapping category unions into a genuine
/// partition of the clock (§12.3 invariant 3).
#[must_use]
pub fn subtract_intervals(keep: &[(i64, i64)], remove: &[(i64, i64)]) -> Vec<(i64, i64)> {
    let mut out = Vec::new();
    for &(start, end) in keep {
        let mut cursor = start;
        for &(rem_start, rem_end) in remove {
            if rem_end <= cursor {
                continue;
            }
            if rem_start >= end {
                break;
            }
            if rem_start > cursor {
                out.push((cursor, rem_start.min(end)));
            }
            cursor = cursor.max(rem_end);
            if cursor >= end {
                break;
            }
        }
        if cursor < end {
            out.push((cursor, end));
        }
    }
    out
}

/// Total duration of a merged interval set, saturating rather than
/// overflowing on absurd input (security invariant 2: never panic on
/// attacker-controlled values).
#[must_use]
pub fn total_ms(spans: &[(i64, i64)]) -> i64 {
    spans.iter().fold(0i64, |acc, (start, end)| {
        acc.saturating_add(end.saturating_sub(*start))
    })
}

/// Clip one interval to a window, returning `None` when they do not
/// overlap. Clipping is what makes the ratios provably bounded: no
/// recorded segment, however long, can push value time past lead time.
#[must_use]
pub fn clip(span: (i64, i64), window: (i64, i64)) -> Option<(i64, i64)> {
    let start = span.0.max(window.0);
    let end = span.1.min(window.1);
    (end > start).then_some((start, end))
}

// ---------------------------------------------------------------------
// Inputs
// ---------------------------------------------------------------------

/// The instance clock: the window every ratio is measured against.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Clock {
    /// Clock start, epoch milliseconds.
    pub start_ms: i64,
    /// Clock stop, epoch milliseconds (`as_of` while the clock runs).
    pub stop_ms: i64,
    /// Where `start_ms` came from: `clock_start_at` or `enrolled_on`.
    pub start_source: &'static str,
    /// Where `stop_ms` came from: `clock_stop_at`, `closed_on`, or
    /// `as_of`.
    pub stop_source: &'static str,
    /// Whether the clock is still running (the instance is open).
    pub running: bool,
}

impl Clock {
    /// The window as a span.
    #[must_use]
    pub const fn window(&self) -> (i64, i64) {
        (self.start_ms, self.stop_ms)
    }

    /// Lead time (τ / LT). Zero when the clock is degenerate; callers
    /// check [`Clock::is_measurable`] before dividing.
    #[must_use]
    pub fn lead_time_ms(&self) -> i64 {
        self.stop_ms.saturating_sub(self.start_ms).max(0)
    }

    /// Whether the clock spans a positive duration. A stop at or before
    /// the start is a data error, reported as a stated null rather than
    /// a division by zero or a negative ratio.
    #[must_use]
    pub fn is_measurable(&self) -> bool {
        self.stop_ms > self.start_ms
    }
}

/// One recorded segment, as the analysis consumes it.
#[derive(Clone, Debug)]
pub struct Segment {
    /// Human name — "MRI", "await triage outcome".
    pub label: String,
    /// One of [`STAGES`].
    pub stage: String,
    /// One of [`CATEGORIES`].
    pub category: String,
    /// One of [`WASTES`], on non-value-adding segments only.
    pub waste: Option<String>,
    /// Interval start, epoch milliseconds.
    pub start_ms: i64,
    /// Interval end, epoch milliseconds; `None` while still running.
    pub end_ms: Option<i64>,
    /// Who — a `worker:` / `organization:` URN.
    pub actor_ref: Option<String>,
    /// Where — a `place:` / `organization:` URN.
    pub location_ref: Option<String>,
}

impl Segment {
    /// The segment's span, treating a running segment as ending at
    /// `as_of`.
    #[must_use]
    pub fn span(&self, as_of_ms: i64) -> (i64, i64) {
        (self.start_ms, self.end_ms.unwrap_or(as_of_ms))
    }
}

// ---------------------------------------------------------------------
// Outputs
// ---------------------------------------------------------------------

/// A ratio reported with the two figures it came from, so a consumer
/// can re-aggregate without trusting our rounding (spec §6.5).
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Ratio {
    /// The ratio in `[0, 1]`, or `null` when undefined.
    pub value: Option<f64>,
    /// The numerator, milliseconds.
    pub numerator_ms: i64,
    /// The denominator, milliseconds.
    pub denominator_ms: i64,
}

impl Ratio {
    /// Build a ratio, yielding `value: None` on a non-positive
    /// denominator rather than a sentinel zero.
    #[must_use]
    pub fn new(numerator_ms: i64, denominator_ms: i64) -> Self {
        #[allow(clippy::cast_precision_loss)] // display ratio, not arithmetic
        let value = (denominator_ms > 0).then(|| numerator_ms as f64 / denominator_ms as f64);
        Self {
            value,
            numerator_ms,
            denominator_ms,
        }
    }
}

/// One bucket of the clock partition (spec §12.3 invariant 3).
#[derive(Clone, Debug, Serialize)]
pub struct CategoryShare {
    /// `value_adding`, `necessary_non_value_adding`,
    /// `unnecessary_non_value_adding`, or `unrecorded`.
    pub category: String,
    /// Milliseconds in this bucket.
    pub ms: i64,
    /// Rounded days, for display.
    pub days: f64,
    /// Share of lead time.
    pub share: Option<f64>,
}

/// Per-stage time. Stages may overlap, so shares need not sum to 1 —
/// unlike [`CategoryShare`], which partitions.
#[derive(Clone, Debug, Serialize)]
pub struct StageShare {
    /// One of [`STAGES`].
    pub stage: String,
    /// Union of this stage's segments, clipped to the clock.
    pub ms: i64,
    /// Non-value-adding time attributed to this stage, including the
    /// gaps spent waiting to reach it (spec §8.2).
    pub non_value_adding_ms: i64,
    /// Share of lead time.
    pub share: Option<f64>,
    /// Segments recorded in this stage.
    pub segments: usize,
}

/// Per-waste-type time, over non-value-adding segments only.
#[derive(Clone, Debug, Serialize)]
pub struct WasteShare {
    /// One of [`WASTES`].
    pub waste: String,
    /// Milliseconds.
    pub ms: i64,
    /// Segments carrying this waste type.
    pub segments: usize,
}

/// A maximal stretch of clock covered by no segment (spec §8.1).
#[derive(Clone, Debug, Serialize)]
pub struct Gap {
    /// Gap start, epoch milliseconds.
    pub start_ms: i64,
    /// Gap end, epoch milliseconds.
    pub end_ms: i64,
    /// Duration, milliseconds.
    pub duration_ms: i64,
    /// Duration in days, for display.
    pub days: f64,
    /// Label of the segment the gap follows, if any.
    pub after: Option<String>,
    /// Label of the segment the gap precedes, if any.
    pub before: Option<String>,
    /// The stage this gap's time is attributed to — the stage of the
    /// segment that follows it (what the patient was waiting to reach),
    /// falling back to the preceding segment for a trailing gap.
    pub stage: Option<String>,
    /// Whether the gap sits at a change of actor or location, making it
    /// the cost of changing hands rather than the cost of the work.
    pub at_handoff: bool,
}

/// Handoff counts and their time cost (spec §8.4).
#[derive(Clone, Debug, Serialize)]
pub struct Handoffs {
    /// Consecutive segments whose `actor_ref` differs.
    pub actor_changes: usize,
    /// Consecutive segments whose `location_ref` differs.
    pub location_changes: usize,
    /// Boundaries where either changed.
    pub total: usize,
    /// Distinct actors touching the journey.
    pub distinct_actors: usize,
    /// Distinct locations the journey visited.
    pub distinct_locations: usize,
    /// Time sitting in gaps at handoff boundaries.
    pub gap_ms_at_handoffs: i64,
}

/// The per-instance analysis (spec §6).
#[derive(Clone, Debug, Serialize)]
pub struct InstanceAnalysis {
    /// The clock the analysis was measured against.
    pub clock: Clock,
    /// Lead time (τ / LT).
    pub lead_time_ms: i64,
    /// Lead time in days, for display.
    pub lead_time_days: f64,
    /// Value time (VT) — union of value-adding segments.
    pub value_time_ms: i64,
    /// Process time (PT) — union of value-adding + necessary segments.
    pub process_time_ms: i64,
    /// Waste time — unnecessary time not already counted as VA or NNVA.
    pub waste_time_ms: i64,
    /// Touch time (φ) — the raw **sum**, which may exceed lead time when
    /// care was concurrent. Resource effort, not elapsed time.
    pub touch_time_ms: i64,
    /// Wait time (ω) = lead time − process time.
    pub wait_time_ms: i64,
    /// Clock time no segment covered.
    pub unrecorded_ms: i64,
    /// %VA — the Barker headline ratio.
    pub value_adding_ratio: Ratio,
    /// %A — the VSM percentage-activity ratio.
    pub activity_ratio: Ratio,
    /// How much of the journey was mapped at all (spec §6.6).
    pub coverage_ratio: Ratio,
    /// `unmapped` | `partial` | `mapped`, derived from coverage, so a
    /// UI cannot render "we do not know" as "catastrophically
    /// inefficient".
    pub confidence: &'static str,
    /// Segments considered (those overlapping the clock).
    pub segments: usize,
    /// The four-bucket partition of the clock.
    pub by_category: Vec<CategoryShare>,
    /// Per-stage time.
    pub by_stage: Vec<StageShare>,
    /// Per-waste-type time.
    pub by_waste: Vec<WasteShare>,
    /// Handoff counts and cost.
    pub handoffs: Handoffs,
    /// Gaps, longest first.
    pub gaps: Vec<Gap>,
    /// Why the analysis is null, when the clock is unmeasurable.
    pub reason: Option<String>,
}

/// Milliseconds as days, rounded to three places for display.
#[must_use]
#[allow(clippy::cast_precision_loss)] // display only
pub fn as_days(ms: i64) -> f64 {
    (ms as f64 / DAY_MS as f64 * 1000.0).round() / 1000.0
}

/// Analyse one instance (spec §6).
///
/// `as_of_ms` bounds any still-running segment and, when the instance is
/// open, the clock itself — passed in rather than read, so the function
/// is deterministic and testable.
#[must_use]
pub fn analyze(clock: Clock, segments: &[Segment], as_of_ms: i64) -> InstanceAnalysis {
    let window = clock.window();
    let lead = clock.lead_time_ms();

    // Clip every segment to the clock; anything outside contributes zero.
    let mut clipped: Vec<(&Segment, (i64, i64))> = segments
        .iter()
        .filter_map(|seg| clip(seg.span(as_of_ms), window).map(|span| (seg, span)))
        .collect();
    clipped.sort_by_key(|(_, span)| *span);

    let spans_where = |pred: &dyn Fn(&Segment) -> bool| -> Vec<(i64, i64)> {
        merge_intervals(
            clipped
                .iter()
                .filter(|(seg, _)| pred(seg))
                .map(|(_, span)| *span)
                .collect(),
        )
    };

    let value = spans_where(&|s: &Segment| s.category == CATEGORY_VALUE_ADDING);
    let necessary = spans_where(&|s: &Segment| s.category == CATEGORY_NECESSARY);
    let unnecessary = spans_where(&|s: &Segment| s.category == CATEGORY_UNNECESSARY);
    let all = merge_intervals(clipped.iter().map(|(_, span)| *span).collect());

    // Partition by priority VA > NNVA > UNVA > unrecorded, so the four
    // buckets sum to the lead time exactly (spec §12.3 invariant 3).
    let value_ms = total_ms(&value);
    let process = merge_intervals([value.clone(), necessary.clone()].concat());
    let process_ms = total_ms(&process);
    let necessary_only_ms = process_ms.saturating_sub(value_ms);
    let unnecessary_only_ms = total_ms(&subtract_intervals(&unnecessary, &process));
    let covered_ms = total_ms(&all);
    let unrecorded_ms = lead.saturating_sub(covered_ms);
    let touch_ms = clipped.iter().fold(0i64, |acc, (_, (start, end))| {
        acc.saturating_add(end.saturating_sub(*start))
    });

    let by_category = vec![
        share(CATEGORY_VALUE_ADDING, value_ms, lead),
        share(CATEGORY_NECESSARY, necessary_only_ms, lead),
        share(CATEGORY_UNNECESSARY, unnecessary_only_ms, lead),
        share(UNRECORDED, unrecorded_ms, lead),
    ];

    let gaps = compute_gaps(&clipped, &all, window);
    let handoffs = compute_handoffs(&clipped, &gaps);
    let by_stage = compute_stages(&clipped, &gaps, &process, lead);
    let by_waste = compute_wastes(&clipped);

    let coverage = Ratio::new(covered_ms, lead);
    let confidence = confidence_label(coverage.value);

    let mut gaps_ranked = gaps;
    gaps_ranked.sort_by_key(|g| std::cmp::Reverse(g.duration_ms));

    InstanceAnalysis {
        clock,
        lead_time_ms: lead,
        lead_time_days: as_days(lead),
        value_time_ms: value_ms,
        process_time_ms: process_ms,
        waste_time_ms: unnecessary_only_ms,
        touch_time_ms: touch_ms,
        wait_time_ms: lead.saturating_sub(process_ms),
        unrecorded_ms,
        value_adding_ratio: Ratio::new(value_ms, lead),
        activity_ratio: Ratio::new(process_ms, lead),
        coverage_ratio: coverage,
        confidence,
        segments: clipped.len(),
        by_category,
        by_stage,
        by_waste,
        handoffs,
        gaps: gaps_ranked,
        reason: (!clock.is_measurable())
            .then(|| "clock stop is at or before clock start; no ratio is defined".to_string()),
    }
}

/// Coverage → the label a UI may safely render (spec §6.6).
#[must_use]
pub fn confidence_label(coverage: Option<f64>) -> &'static str {
    match coverage {
        None => "unmapped",
        Some(c) if c < COVERAGE_UNMAPPED => "unmapped",
        Some(c) if c < COVERAGE_MAPPED => "partial",
        Some(_) => "mapped",
    }
}

/// One bucket of the category partition.
fn share(category: &str, ms: i64, lead: i64) -> CategoryShare {
    CategoryShare {
        category: category.to_string(),
        ms,
        days: as_days(ms),
        share: Ratio::new(ms, lead).value,
    }
}

/// The uncovered stretches of the clock, in time order.
fn compute_gaps(
    clipped: &[(&Segment, (i64, i64))],
    covered: &[(i64, i64)],
    window: (i64, i64),
) -> Vec<Gap> {
    let uncovered = subtract_intervals(&[window], covered);
    uncovered
        .into_iter()
        .filter(|(start, end)| end > start)
        .map(|(start, end)| {
            // The segment that ends at (or last before) the gap, and the
            // one that starts at (or first after) it.
            let after = clipped
                .iter()
                .filter(|(_, span)| span.1 <= start)
                .max_by_key(|(_, span)| span.1);
            let before = clipped
                .iter()
                .filter(|(_, span)| span.0 >= end)
                .min_by_key(|(_, span)| span.0);
            let at_handoff = match (after, before) {
                (Some((a, _)), Some((b, _))) => {
                    a.actor_ref != b.actor_ref || a.location_ref != b.location_ref
                }
                _ => false,
            };
            Gap {
                start_ms: start,
                end_ms: end,
                duration_ms: end.saturating_sub(start),
                days: as_days(end.saturating_sub(start)),
                after: after.map(|(seg, _)| seg.label.clone()),
                before: before.map(|(seg, _)| seg.label.clone()),
                // Attributed to what the patient was waiting to reach;
                // a trailing gap falls back to what they last left.
                stage: before
                    .map(|(seg, _)| seg.stage.clone())
                    .or_else(|| after.map(|(seg, _)| seg.stage.clone())),
                at_handoff,
            }
        })
        .collect()
}

/// Handoff counts over the time-ordered segments, plus the gap time at
/// those boundaries.
fn compute_handoffs(clipped: &[(&Segment, (i64, i64))], gaps: &[Gap]) -> Handoffs {
    let mut actor_changes = 0usize;
    let mut location_changes = 0usize;
    let mut total = 0usize;
    for pair in clipped.windows(2) {
        let (prev, next) = (pair[0].0, pair[1].0);
        let actor = prev.actor_ref != next.actor_ref;
        let location = prev.location_ref != next.location_ref;
        if actor {
            actor_changes += 1;
        }
        if location {
            location_changes += 1;
        }
        if actor || location {
            total += 1;
        }
    }
    let distinct = |pick: &dyn Fn(&Segment) -> Option<&String>| {
        clipped
            .iter()
            .filter_map(|(seg, _)| pick(seg))
            .collect::<std::collections::BTreeSet<_>>()
            .len()
    };
    Handoffs {
        actor_changes,
        location_changes,
        total,
        distinct_actors: distinct(&|s: &Segment| s.actor_ref.as_ref()),
        distinct_locations: distinct(&|s: &Segment| s.location_ref.as_ref()),
        gap_ms_at_handoffs: gaps
            .iter()
            .filter(|g| g.at_handoff)
            .fold(0i64, |acc, g| acc.saturating_add(g.duration_ms)),
    }
}

/// Per-stage union time, plus the non-value-adding time attributed to
/// each stage (its own non-VA segments, plus the gaps waiting for it).
fn compute_stages(
    clipped: &[(&Segment, (i64, i64))],
    gaps: &[Gap],
    process: &[(i64, i64)],
    lead: i64,
) -> Vec<StageShare> {
    let mut stages: std::collections::BTreeSet<String> =
        clipped.iter().map(|(seg, _)| seg.stage.clone()).collect();
    for gap in gaps {
        if let Some(stage) = &gap.stage {
            stages.insert(stage.clone());
        }
    }
    stages
        .into_iter()
        .map(|stage| {
            let spans: Vec<(i64, i64)> = clipped
                .iter()
                .filter(|(seg, _)| seg.stage == stage)
                .map(|(_, span)| *span)
                .collect();
            let count = spans.len();
            let merged = merge_intervals(spans);
            // This stage's own non-value-adding time: its segments minus
            // anything the process (VA + NNVA) union already covers,
            // plus the gaps attributed to it.
            let non_va_segments: Vec<(i64, i64)> = clipped
                .iter()
                .filter(|(seg, _)| seg.stage == stage && seg.category != CATEGORY_VALUE_ADDING)
                .map(|(_, span)| *span)
                .collect();
            let non_va = total_ms(&subtract_intervals(
                &merge_intervals(non_va_segments),
                &merge_intervals(
                    clipped
                        .iter()
                        .filter(|(seg, _)| seg.category == CATEGORY_VALUE_ADDING)
                        .map(|(_, span)| *span)
                        .collect(),
                ),
            ));
            let gap_ms = gaps
                .iter()
                .filter(|g| g.stage.as_deref() == Some(stage.as_str()))
                .fold(0i64, |acc, g| acc.saturating_add(g.duration_ms));
            let _ = process; // process union informs `non_va` via VA subtraction
            StageShare {
                stage,
                ms: total_ms(&merged),
                non_value_adding_ms: non_va.saturating_add(gap_ms),
                share: Ratio::new(total_ms(&merged), lead).value,
                segments: count,
            }
        })
        .collect()
}

/// Per-waste-type totals over non-value-adding segments.
fn compute_wastes(clipped: &[(&Segment, (i64, i64))]) -> Vec<WasteShare> {
    let mut per: std::collections::BTreeMap<String, (i64, usize)> =
        std::collections::BTreeMap::new();
    for (seg, span) in clipped {
        if seg.category == CATEGORY_VALUE_ADDING {
            continue;
        }
        if let Some(waste) = &seg.waste {
            let entry = per.entry(waste.clone()).or_insert((0, 0));
            entry.0 = entry.0.saturating_add(span.1.saturating_sub(span.0));
            entry.1 += 1;
        }
    }
    let mut out: Vec<WasteShare> = per
        .into_iter()
        .map(|(waste, (ms, segments))| WasteShare {
            waste,
            ms,
            segments,
        })
        .collect();
    out.sort_by_key(|w| std::cmp::Reverse(w.ms));
    out
}

// ---------------------------------------------------------------------
// Cohort statistics (spec §7)
// ---------------------------------------------------------------------

/// Nearest-rank percentile over a **sorted** sample (spec §7.1).
///
/// Nearest-rank always returns an observed value, which matters when
/// someone asks "which patient is the p90?" — with interpolation the
/// answer is nobody.
#[must_use]
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)] // rank arithmetic over a bounded sample
pub fn percentile(sorted: &[i64], p: f64) -> Option<i64> {
    if sorted.is_empty() || !(0.0..=1.0).contains(&p) || p.is_nan() {
        return None;
    }
    let n = sorted.len();
    let rank = (p * n as f64).ceil().max(1.0);
    let index = (rank as usize).min(n).saturating_sub(1);
    sorted.get(index).copied()
}

/// A right-skewed duration distribution, reported by percentile rather
/// than by mean (spec §7.1).
#[derive(Clone, Debug, Serialize)]
pub struct Distribution {
    /// Sample size.
    pub n: usize,
    /// Smallest observation, milliseconds.
    pub min_ms: i64,
    /// Median.
    pub p50_ms: i64,
    /// 75th percentile.
    pub p75_ms: i64,
    /// 90th percentile.
    pub p90_ms: i64,
    /// 95th percentile.
    pub p95_ms: i64,
    /// Largest observation.
    pub max_ms: i64,
    /// Arithmetic mean — reported, but skew-sensitive and describing no
    /// actual patient.
    pub mean_ms: i64,
    /// The median in days, for display.
    pub p50_days: f64,
    /// The 90th percentile in days, for display.
    pub p90_days: f64,
    /// The percentile method in use, stated so a consumer comparing
    /// against `percentile_cont` knows why the figures differ.
    pub method: &'static str,
}

/// Build a distribution from unsorted durations. `None` on an empty
/// sample.
#[must_use]
pub fn distribution(values: &[i64]) -> Option<Distribution> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let n = sorted.len();
    let sum = sorted.iter().fold(0i64, |a, v| a.saturating_add(*v));
    #[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]
    let mean = sum / (n as i64).max(1);
    let at = |p: f64| percentile(&sorted, p).unwrap_or(0);
    Some(Distribution {
        n,
        min_ms: sorted.first().copied().unwrap_or(0),
        p50_ms: at(0.50),
        p75_ms: at(0.75),
        p90_ms: at(0.90),
        p95_ms: at(0.95),
        max_ms: sorted.last().copied().unwrap_or(0),
        mean_ms: mean,
        p50_days: as_days(at(0.50)),
        p90_days: as_days(at(0.90)),
        method: "nearest_rank",
    })
}

/// A named access standard: a threshold on lead time plus the share of
/// patients expected to be inside it (spec §2.4, §7.3).
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Standard {
    /// Stable identifier used in `?standard=`.
    pub id: &'static str,
    /// Human label.
    pub label: &'static str,
    /// The lead-time threshold, milliseconds.
    pub threshold_ms: i64,
    /// The share expected inside the threshold, in `[0, 1]`.
    pub target_ratio: f64,
    /// Who sets it.
    pub authority: &'static str,
    /// When this entry was last checked — targets move, and a stale
    /// threshold silently mis-scoring a cohort is worse than none.
    pub as_of: &'static str,
    /// Anything a reader needs in order not to misapply it.
    pub note: &'static str,
}

/// The catalogue of named standards (spec §7.3). Reference data with a
/// citation date, not an assertion that a given pathway is subject to
/// any of them.
pub const STANDARDS: &[Standard] = &[
    Standard {
        id: "rtt_18_weeks",
        label: "Referral to treatment, incomplete pathway",
        threshold_ms: 126 * DAY_MS,
        target_ratio: 0.92,
        authority: "NHS England",
        as_of: "2026-08",
        note: "18 weeks. The clock stops at first definitive treatment, not at \
               a diagnostic or an outpatient appointment.",
    },
    Standard {
        id: "cancer_fds_28_days",
        label: "Cancer faster diagnosis standard",
        threshold_ms: 28 * DAY_MS,
        target_ratio: 0.80,
        authority: "NHS England",
        as_of: "2026-08",
        note: "Diagnosis given or cancer ruled out within 28 days of referral. \
               Target rose from 75% to 80% in March 2026.",
    },
    Standard {
        id: "cancer_31_days",
        label: "Cancer decision-to-treat to treatment",
        threshold_ms: 31 * DAY_MS,
        target_ratio: 0.96,
        authority: "NHS England",
        as_of: "2026-08",
        note: "Measured from the decision to treat, not from referral.",
    },
    Standard {
        id: "cancer_62_days",
        label: "Cancer referral to first treatment",
        threshold_ms: 62 * DAY_MS,
        target_ratio: 0.85,
        authority: "NHS England",
        as_of: "2026-08",
        note: "An interim 70% operational target has been used in planning \
               guidance while recovery continues.",
    },
    Standard {
        id: "diagnostics_6_weeks",
        label: "Diagnostic waiting time (DM01)",
        threshold_ms: 42 * DAY_MS,
        target_ratio: 0.99,
        authority: "NHS England",
        as_of: "2026-08",
        note: "Expressed nationally as the share waiting *over* 6 weeks \
               (1% by 2028/29); inverted here for comparability.",
    },
    Standard {
        id: "ae_4_hours",
        label: "A&E four-hour standard",
        threshold_ms: 4 * HOUR_MS,
        target_ratio: 0.85,
        authority: "NHS England",
        as_of: "2026-08",
        note: "85% by 2028/29, via 82% by March 2027. Hours, not days — which \
               is why every duration here is reported in milliseconds.",
    },
];

/// Look up a standard by id.
#[must_use]
pub fn standard(id: &str) -> Option<&'static Standard> {
    STANDARDS.iter().find(|s| s.id == id)
}

/// How a cohort scored against a threshold (spec §7.3).
#[derive(Clone, Debug, Serialize)]
pub struct Compliance {
    /// The standard id, or `custom` for an explicit `target_days`.
    pub standard: String,
    /// The threshold applied, milliseconds.
    pub threshold_ms: i64,
    /// The threshold in days, for display.
    pub threshold_days: f64,
    /// Instances inside the threshold.
    pub within: usize,
    /// Instances outside it.
    pub breached: usize,
    /// Achieved share inside the threshold.
    pub achieved_ratio: Option<f64>,
    /// The operational target, where the standard declares one.
    pub target_ratio: Option<f64>,
    /// Whether the target was met. `None` when there is no target.
    pub target_met: Option<bool>,
    /// When the threshold was last checked against its authority.
    pub as_of: Option<&'static str>,
}

/// Score a set of lead times against a threshold.
#[must_use]
pub fn compliance(
    lead_times_ms: &[i64],
    label: &str,
    threshold_ms: i64,
    target_ratio: Option<f64>,
    as_of: Option<&'static str>,
) -> Compliance {
    let within = lead_times_ms
        .iter()
        .filter(|lt| **lt <= threshold_ms)
        .count();
    let breached = lead_times_ms.len().saturating_sub(within);
    #[allow(clippy::cast_precision_loss)] // display ratio
    let achieved = (!lead_times_ms.is_empty()).then(|| within as f64 / lead_times_ms.len() as f64);
    Compliance {
        standard: label.to_string(),
        threshold_ms,
        threshold_days: as_days(threshold_ms),
        within,
        breached,
        achieved_ratio: achieved,
        target_ratio,
        target_met: match (achieved, target_ratio) {
            (Some(a), Some(t)) => Some(a >= t),
            _ => None,
        },
        as_of,
    }
}

/// The cohort analysis (spec §7).
#[derive(Clone, Debug, Serialize)]
pub struct CohortAnalysis {
    /// Instances analysed.
    pub instances: usize,
    /// Lead-time distribution.
    pub lead_time: Option<Distribution>,
    /// `Σ VT / Σ LT` — the system's overall ratio, dominated by the
    /// longest journeys.
    pub aggregate_value_adding_ratio: Ratio,
    /// The median of the per-instance ratios — the typical journey.
    pub median_value_adding_ratio: Option<f64>,
    /// `concentrated` when the two diverge (waste sits in a minority of
    /// journeys), `uniform` when they agree, `insufficient_data`
    /// otherwise. The divergence is itself the finding (spec §7.2).
    pub waste_shape: &'static str,
    /// Cohort-wide coverage.
    pub coverage_ratio: Ratio,
    /// Per-stage non-value-adding time across the cohort.
    pub by_stage: Vec<StageShare>,
    /// Per-waste-type time across the cohort.
    pub by_waste: Vec<WasteShare>,
}

/// Aggregate per-instance analyses into a cohort view.
#[must_use]
pub fn cohort(analyses: &[InstanceAnalysis]) -> CohortAnalysis {
    let lead_times: Vec<i64> = analyses.iter().map(|a| a.lead_time_ms).collect();
    let total_lead = lead_times.iter().fold(0i64, |a, v| a.saturating_add(*v));
    let total_value = analyses
        .iter()
        .fold(0i64, |a, x| a.saturating_add(x.value_time_ms));
    let total_covered = analyses.iter().fold(0i64, |a, x| {
        a.saturating_add(x.lead_time_ms.saturating_sub(x.unrecorded_ms))
    });

    let mut ratios: Vec<f64> = analyses
        .iter()
        .filter_map(|a| a.value_adding_ratio.value)
        .collect();
    ratios.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_ratio = if ratios.is_empty() {
        None
    } else {
        ratios.get(ratios.len() / 2).copied()
    };

    let aggregate = Ratio::new(total_value, total_lead);
    let waste_shape = match (aggregate.value, median_ratio) {
        (Some(agg), Some(med)) => {
            if (agg - med).abs() > 0.05 {
                "concentrated"
            } else {
                "uniform"
            }
        }
        _ => "insufficient_data",
    };

    // Roll the per-instance stage and waste tables up.
    let mut stage_totals: std::collections::BTreeMap<String, (i64, i64, usize)> =
        std::collections::BTreeMap::new();
    for analysis in analyses {
        for stage in &analysis.by_stage {
            let entry = stage_totals.entry(stage.stage.clone()).or_insert((0, 0, 0));
            entry.0 = entry.0.saturating_add(stage.ms);
            entry.1 = entry.1.saturating_add(stage.non_value_adding_ms);
            entry.2 += stage.segments;
        }
    }
    let mut by_stage: Vec<StageShare> = stage_totals
        .into_iter()
        .map(|(stage, (ms, non_va, segments))| StageShare {
            stage,
            ms,
            non_value_adding_ms: non_va,
            share: Ratio::new(ms, total_lead).value,
            segments,
        })
        .collect();
    by_stage.sort_by_key(|s| std::cmp::Reverse(s.non_value_adding_ms));

    let mut waste_totals: std::collections::BTreeMap<String, (i64, usize)> =
        std::collections::BTreeMap::new();
    for analysis in analyses {
        for waste in &analysis.by_waste {
            let entry = waste_totals.entry(waste.waste.clone()).or_insert((0, 0));
            entry.0 = entry.0.saturating_add(waste.ms);
            entry.1 += waste.segments;
        }
    }
    let mut by_waste: Vec<WasteShare> = waste_totals
        .into_iter()
        .map(|(waste, (ms, segments))| WasteShare {
            waste,
            ms,
            segments,
        })
        .collect();
    by_waste.sort_by_key(|w| std::cmp::Reverse(w.ms));

    CohortAnalysis {
        instances: analyses.len(),
        lead_time: distribution(&lead_times),
        aggregate_value_adding_ratio: aggregate,
        median_value_adding_ratio: median_ratio,
        waste_shape,
        coverage_ratio: Ratio::new(total_covered, total_lead),
        by_stage,
        by_waste,
    }
}

// ---------------------------------------------------------------------
// Constraint findings (spec §8)
// ---------------------------------------------------------------------

/// One disclosed constraint finding, ordered by time recoverable
/// (spec §8.5). Every finding names the rule that produced it; there is
/// deliberately no composite score.
#[derive(Clone, Debug, Serialize)]
pub struct Finding {
    /// The rule that fired.
    pub rule: &'static str,
    /// What the rule is about — a stage, a waste type, a gap.
    pub subject: String,
    /// Human explanation, including the threshold that fired.
    pub detail: String,
    /// Time this finding accounts for, milliseconds.
    pub recoverable_ms: i64,
    /// The same in days, for display.
    pub recoverable_days: f64,
}

/// The share of the cohort's non-value-adding time above which a stage
/// is reported as dominating it rather than merely contributing.
const STAGE_DOMINANCE: f64 = 0.40;

/// Instances changing hands at least this many times are reported as
/// handoff-heavy.
const HANDOFF_HEAVY: usize = 5;

/// Findings for the stage ranking (spec §8.2).
fn stage_findings(summary: &CohortAnalysis) -> Vec<Finding> {
    let total_non_va: i64 = summary
        .by_stage
        .iter()
        .fold(0i64, |a, s| a.saturating_add(s.non_value_adding_ms));
    summary
        .by_stage
        .iter()
        .filter(|stage| stage.non_value_adding_ms > 0)
        .map(|stage| {
            #[allow(clippy::cast_precision_loss)] // display ratio
            let share = if total_non_va > 0 {
                stage.non_value_adding_ms as f64 / total_non_va as f64
            } else {
                0.0
            };
            Finding {
                rule: if share >= STAGE_DOMINANCE {
                    "stage_dominates_waste"
                } else {
                    "stage_non_value_adding"
                },
                subject: stage.stage.clone(),
                detail: format!(
                    "{:.1}% of the cohort's non-value-adding time sits in `{}` \
                     (reported as `stage_dominates_waste` at {:.0}%)",
                    share * 100.0,
                    stage.stage,
                    STAGE_DOMINANCE * 100.0
                ),
                recoverable_ms: stage.non_value_adding_ms,
                recoverable_days: as_days(stage.non_value_adding_ms),
            }
        })
        .collect()
}

/// Findings for the waste ranking (spec §8.3).
fn waste_findings(summary: &CohortAnalysis) -> Vec<Finding> {
    summary
        .by_waste
        .iter()
        .map(|waste| Finding {
            rule: "waste_type",
            subject: waste.waste.clone(),
            detail: format!(
                "{} segments classified `{}` — a `waiting` finding wants \
                 scheduling; a `defects` finding wants process redesign",
                waste.segments, waste.waste
            ),
            recoverable_ms: waste.ms,
            recoverable_days: as_days(waste.ms),
        })
        .collect()
}

/// Rank the cohort's constraints (spec §8).
#[must_use]
pub fn constraints(analyses: &[InstanceAnalysis], summary: &CohortAnalysis) -> Vec<Finding> {
    let mut findings: Vec<Finding> = stage_findings(summary);
    findings.extend(waste_findings(summary));

    // The single longest gap anywhere in the cohort: the named queue.
    if let Some(longest) = analyses
        .iter()
        .flat_map(|a| a.gaps.iter())
        .max_by_key(|g| g.duration_ms)
    {
        findings.push(Finding {
            rule: "longest_gap",
            subject: format!(
                "{} → {}",
                longest.after.as_deref().unwrap_or("clock start"),
                longest.before.as_deref().unwrap_or("clock stop")
            ),
            detail: format!(
                "the longest single stretch in which nothing was recorded: \
                 {:.1} days",
                longest.days
            ),
            recoverable_ms: longest.duration_ms,
            recoverable_days: longest.days,
        });
    }

    let handoff_heavy = analyses
        .iter()
        .filter(|a| a.handoffs.total >= HANDOFF_HEAVY)
        .count();
    if handoff_heavy > 0 {
        let cost = analyses
            .iter()
            .fold(0i64, |a, x| a.saturating_add(x.handoffs.gap_ms_at_handoffs));
        findings.push(Finding {
            rule: "handoff_heavy",
            subject: format!("{handoff_heavy} instances"),
            detail: "instances changing hands five or more times; the recoverable \
                     figure is the time sitting in gaps at those boundaries, \
                     which is the cost of changing hands rather than of the work"
                .to_string(),
            recoverable_ms: cost,
            recoverable_days: as_days(cost),
        });
    }

    let unmapped = analyses
        .iter()
        .filter(|a| a.confidence == "unmapped")
        .count();
    if unmapped > 0 {
        findings.push(Finding {
            rule: "low_coverage",
            subject: format!("{unmapped} instances"),
            detail: format!(
                "coverage below {:.0}% — for these the value-adding ratio is a \
                 floor, not a measurement, and the first improvement is to map \
                 the journey",
                COVERAGE_UNMAPPED * 100.0
            ),
            recoverable_ms: 0,
            recoverable_days: 0.0,
        });
    }

    findings.sort_by_key(|f| std::cmp::Reverse(f.recoverable_ms));
    findings
}

// ---------------------------------------------------------------------
// Flow analysis (spec §9)
// ---------------------------------------------------------------------

/// Queueing-theory flow over a window (spec §9).
#[derive(Clone, Debug, Serialize)]
pub struct Flow {
    /// The window measured, days.
    pub window_days: i64,
    /// Enrolments in the window.
    pub arrivals: usize,
    /// Closures in the window.
    pub closures: usize,
    /// λ — arrivals per day.
    pub arrival_rate_per_day: Option<f64>,
    /// μ — closures per day.
    pub service_rate_per_day: Option<f64>,
    /// ρ = λ/μ. Near 1, expected wait grows without bound — a pathway at
    /// 95% utilisation is not "5% from trouble", it is already in it.
    pub utilisation: Option<f64>,
    /// Why ρ is null, when it is.
    pub utilisation_reason: Option<String>,
    /// κ — open instances now.
    pub work_in_progress: usize,
    /// τ̂ = κ/λ — the lead time Little's Law implies for a journey now
    /// entering the queue.
    pub implied_lead_time_days: Option<f64>,
    /// The observed median lead time of instances closed in the window.
    pub observed_p50_lead_time_days: Option<f64>,
    /// `backlog_growing` | `steady_state` | `queue_draining` |
    /// `insufficient_data` (spec §9.4).
    pub interpretation: &'static str,
    /// What the interpretation means, in words.
    pub detail: String,
}

/// Compute flow analysis. `observed_p50_ms` is the median lead time of
/// instances closed in the window, if any closed.
#[must_use]
#[allow(clippy::cast_precision_loss)] // rates over bounded counts
pub fn flow(
    window_days: i64,
    arrivals: usize,
    closures: usize,
    work_in_progress: usize,
    observed_p50_ms: Option<i64>,
) -> Flow {
    let days = window_days.max(1) as f64;
    let lambda = (window_days > 0).then(|| arrivals as f64 / days);
    let mu = (window_days > 0).then(|| closures as f64 / days);

    let (rho, rho_reason) = match (lambda, mu) {
        (Some(_), Some(m)) if m <= 0.0 => (
            None,
            Some(
                "no instances closed in the window, so the service rate is zero \
                 and utilisation is undefined — not infinite, and not zero"
                    .to_string(),
            ),
        ),
        (Some(l), Some(m)) => (Some(l / m), None),
        _ => (None, Some("window must be at least one day".to_string())),
    };

    let implied = match lambda {
        Some(l) if l > 0.0 => Some(work_in_progress as f64 / l),
        _ => None,
    };
    let observed_days = observed_p50_ms.map(as_days);

    let (interpretation, detail) = match (implied, observed_days) {
        (Some(implied_days), Some(obs)) if obs > 0.0 => {
            let ratio = implied_days / obs;
            if ratio > 1.25 {
                (
                    "backlog_growing",
                    format!(
                        "Little's Law implies {implied_days:.1} days for a journey \
                         entering now, against {obs:.1} days observed for those \
                         just closed. The queue is growing faster than it clears, \
                         so recent completions flatter the system."
                    ),
                )
            } else if ratio < 0.80 {
                (
                    "queue_draining",
                    format!(
                        "Little's Law implies {implied_days:.1} days against {obs:.1} \
                         observed. Either the queue is draining, or the closures in \
                         this window were disproportionately old cases being cleared."
                    ),
                )
            } else {
                (
                    "steady_state",
                    format!(
                        "Little's Law implies {implied_days:.1} days against {obs:.1} \
                         observed. Arrivals and departures are close to balanced, so \
                         the observed lead time is predictive."
                    ),
                )
            }
        }
        _ => (
            "insufficient_data",
            "not enough arrivals or closures in the window to compare the \
             implied lead time against an observed one"
                .to_string(),
        ),
    };

    Flow {
        window_days,
        arrivals,
        closures,
        arrival_rate_per_day: lambda,
        service_rate_per_day: mu,
        utilisation: rho,
        utilisation_reason: rho_reason,
        work_in_progress,
        implied_lead_time_days: implied,
        observed_p50_lead_time_days: observed_days,
        interpretation,
        detail,
    }
}

// ---------------------------------------------------------------------
// Flow gauges (spec §15 TBA-11)
// ---------------------------------------------------------------------

/// Default cap on how many pathways are exported as individual gauge
/// series. Per-pathway labels are a cardinality hazard: a registry that
/// grows one series per record is how a Prometheus install falls over,
/// and a metric that kills the monitoring is worse than no metric.
pub const DEFAULT_METRIC_MAX_PATHWAYS: usize = 50;

/// One pathway's flow figures, ready to be written to a labelled gauge.
#[derive(Clone, Debug, PartialEq)]
pub struct FlowMetricRow {
    /// The pathway's public id — the gauge label.
    pub pathway_pid: String,
    /// Instances behind the figures.
    pub instances: usize,
    /// Cohort %VA, `None` when undefined.
    pub value_adding_ratio: Option<f64>,
    /// p90 lead time in days, `None` when there is no sample.
    pub lead_time_p90_days: Option<f64>,
    /// Cohort coverage.
    pub coverage_ratio: Option<f64>,
}

/// What an export pass decided to publish, and what it held back.
#[derive(Clone, Debug, PartialEq)]
pub struct FlowMetricSet {
    /// The pathways exported as their own series, largest cohort first.
    pub rows: Vec<FlowMetricRow>,
    /// Pathways withheld because their cohort is too small to
    /// aggregate without describing an individual patient.
    pub suppressed_pathways: usize,
    /// Pathways dropped because the series cap was reached.
    pub dropped_pathways: usize,
}

/// Choose which pathways get their own gauge series.
///
/// Two bounds, and neither is silent — both counts are exported
/// alongside the rows, because a cap nobody can see reads as "we
/// measured everything" when it did not:
///
/// 1. **Small cohorts are suppressed**, on the same reasoning as §12.2.
///    A p90 lead time over three patients *is* a patient's lead time,
///    and `/metrics.prom` is on the public allow-list — it stays
///    scrapeable when `<ENTITY>_REQUIRE_AUTH` is on, so anything
///    exported there is exported to whoever can reach the port. The
///    API's suppression threshold would be pointless if the same figure
///    left by the side door.
/// 2. **The series count is capped**, largest cohort first, because
///    per-pathway labels are unbounded cardinality.
///
/// The label is the **pid**, never the name: a rename would otherwise
/// fork the series and silently reset its history.
#[must_use]
pub fn flow_metric_rows(
    per_pathway: &[(String, CohortAnalysis)],
    max_pathways: usize,
    min_cohort: usize,
) -> FlowMetricSet {
    let mut eligible: Vec<&(String, CohortAnalysis)> = Vec::new();
    let mut suppressed = 0usize;
    for entry in per_pathway {
        if entry.1.instances == 0 {
            continue;
        }
        if entry.1.instances < min_cohort {
            suppressed += 1;
        } else {
            eligible.push(entry);
        }
    }
    eligible.sort_by_key(|(pid, cohort)| {
        // Largest cohort first; the pid breaks ties so a pass is stable
        // and a dashboard does not reshuffle between scrapes.
        (std::cmp::Reverse(cohort.instances), pid.clone())
    });
    let dropped = eligible.len().saturating_sub(max_pathways);
    eligible.truncate(max_pathways);

    FlowMetricSet {
        rows: eligible
            .into_iter()
            .map(|(pid, cohort)| FlowMetricRow {
                pathway_pid: pid.clone(),
                instances: cohort.instances,
                value_adding_ratio: cohort.aggregate_value_adding_ratio.value,
                lead_time_p90_days: cohort
                    .lead_time
                    .as_ref()
                    .map(|distribution| distribution.p90_days),
                coverage_ratio: cohort.coverage_ratio.value,
            })
            .collect(),
        suppressed_pathways: suppressed,
        dropped_pathways: dropped,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: i64 = 1_700_000_000_000;

    fn seg(label: &str, stage: &str, category: &str, start_day: i64, end_day: i64) -> Segment {
        Segment {
            label: label.to_string(),
            stage: stage.to_string(),
            category: category.to_string(),
            waste: (category == CATEGORY_UNNECESSARY).then(|| "waiting".to_string()),
            start_ms: T0 + start_day * DAY_MS,
            end_ms: Some(T0 + end_day * DAY_MS),
            actor_ref: None,
            location_ref: None,
        }
    }

    fn clock(days: i64) -> Clock {
        Clock {
            start_ms: T0,
            stop_ms: T0 + days * DAY_MS,
            start_source: "clock_start_at",
            stop_source: "clock_stop_at",
            running: false,
        }
    }

    // -- interval algebra ------------------------------------------------

    #[test]
    fn merge_handles_every_arrangement() {
        assert_eq!(merge_intervals(vec![(0, 1), (2, 3)]), vec![(0, 1), (2, 3)]);
        assert_eq!(merge_intervals(vec![(0, 2), (1, 3)]), vec![(0, 3)]);
        assert_eq!(
            merge_intervals(vec![(0, 1), (1, 2)]),
            vec![(0, 2)],
            "touching"
        );
        assert_eq!(
            merge_intervals(vec![(0, 9), (2, 3)]),
            vec![(0, 9)],
            "nested"
        );
        assert_eq!(
            merge_intervals(vec![(5, 6), (0, 1)]),
            vec![(0, 1), (5, 6)],
            "unsorted"
        );
        assert_eq!(merge_intervals(vec![(1, 1)]), Vec::new(), "zero length");
        assert_eq!(merge_intervals(vec![(5, 2)]), Vec::new(), "reversed");
        assert_eq!(
            merge_intervals(vec![(0, 2), (0, 2)]),
            vec![(0, 2)],
            "identical"
        );
    }

    #[test]
    fn subtract_removes_exactly_the_overlap() {
        assert_eq!(
            subtract_intervals(&[(0, 10)], &[(2, 4)]),
            vec![(0, 2), (4, 10)]
        );
        assert_eq!(subtract_intervals(&[(0, 10)], &[(0, 10)]), Vec::new());
        assert_eq!(subtract_intervals(&[(0, 10)], &[]), vec![(0, 10)]);
        assert_eq!(subtract_intervals(&[(0, 10)], &[(20, 30)]), vec![(0, 10)]);
        assert_eq!(
            subtract_intervals(&[(0, 10)], &[(0, 3), (7, 10)]),
            vec![(3, 7)]
        );
        assert_eq!(subtract_intervals(&[], &[(0, 3)]), Vec::new());
    }

    #[test]
    fn clip_bounds_to_the_window() {
        assert_eq!(clip((0, 10), (5, 20)), Some((5, 10)));
        assert_eq!(clip((0, 10), (-5, 5)), Some((0, 5)));
        assert_eq!(clip((0, 10), (20, 30)), None, "wholly after");
        assert_eq!(clip((30, 40), (0, 10)), None, "wholly before");
        assert_eq!(clip((0, 100), (10, 20)), Some((10, 20)), "straddles both");
    }

    // -- the denominator rule (§6.3) -------------------------------------

    #[test]
    fn the_barker_case_and_the_denominator_rule() {
        // A 100-day journey with 14 days of value-adding care.
        let full = vec![
            seg("consult", "treatment", CATEGORY_VALUE_ADDING, 0, 7),
            seg("wait for scan", "diagnostics", CATEGORY_UNNECESSARY, 7, 60),
            seg("scan", "diagnostics", CATEGORY_VALUE_ADDING, 60, 67),
            seg(
                "wait for results",
                "diagnostics",
                CATEGORY_UNNECESSARY,
                67,
                100,
            ),
        ];
        let a = analyze(clock(100), &full, T0 + 100 * DAY_MS);
        assert_eq!(a.value_time_ms, 14 * DAY_MS);
        assert!((a.value_adding_ratio.value.unwrap_or(0.0) - 0.14).abs() < 1e-9);

        // The same journey with ONLY the value-adding segments recorded
        // must still report 0.14, because the denominator is calendar
        // time, not the sum of recorded activity. If this assertion ever
        // fails, someone has "simplified" §6.3 away and under-recording
        // has become a way to score better.
        let sparse: Vec<Segment> = full
            .iter()
            .filter(|s| s.category == CATEGORY_VALUE_ADDING)
            .cloned()
            .collect();
        let b = analyze(clock(100), &sparse, T0 + 100 * DAY_MS);
        assert!((b.value_adding_ratio.value.unwrap_or(0.0) - 0.14).abs() < 1e-9);
        // ...and the coverage figure exposes that it was thinly mapped.
        assert_eq!(b.confidence, "unmapped");
        assert!(b.coverage_ratio.value.unwrap_or(1.0) < COVERAGE_UNMAPPED);
    }

    #[test]
    fn the_four_buckets_partition_the_clock() {
        // Deliberately overlapping categories: without the partition the
        // buckets would sum past the lead time (§12.3 invariant 3).
        let segments = vec![
            seg("care", "treatment", CATEGORY_VALUE_ADDING, 0, 10),
            seg("consent", "treatment", CATEGORY_NECESSARY, 5, 20),
            seg("chasing", "other", CATEGORY_UNNECESSARY, 15, 40),
        ];
        let a = analyze(clock(100), &segments, T0 + 100 * DAY_MS);
        let sum: i64 = a.by_category.iter().map(|c| c.ms).sum();
        assert_eq!(
            sum, a.lead_time_ms,
            "the four buckets must sum to lead time"
        );
        assert_eq!(a.value_time_ms, 10 * DAY_MS);
        assert_eq!(a.process_time_ms, 20 * DAY_MS, "VA ∪ NNVA de-overlapped");
        assert_eq!(a.waste_time_ms, 20 * DAY_MS, "UNVA minus what PT covered");
        assert_eq!(a.unrecorded_ms, 60 * DAY_MS);
    }

    #[test]
    fn overlap_is_unioned_not_summed() {
        // Two clinicians, same hour. Wall-clock stays one day; effort is two.
        let segments = vec![
            seg("clinician a", "treatment", CATEGORY_VALUE_ADDING, 0, 1),
            seg("clinician b", "treatment", CATEGORY_VALUE_ADDING, 0, 1),
        ];
        let a = analyze(clock(10), &segments, T0 + 10 * DAY_MS);
        assert_eq!(a.value_time_ms, DAY_MS, "union");
        assert_eq!(a.touch_time_ms, 2 * DAY_MS, "raw sum — resource effort");
        assert!(a.value_adding_ratio.value.unwrap_or(9.0) <= 1.0);
    }

    #[test]
    fn ratios_stay_bounded_when_segments_exceed_the_clock() {
        let segments = vec![Segment {
            label: "runaway".to_string(),
            stage: "other".to_string(),
            category: CATEGORY_VALUE_ADDING.to_string(),
            waste: None,
            start_ms: T0 - 500 * DAY_MS,
            end_ms: Some(T0 + 500 * DAY_MS),
            actor_ref: None,
            location_ref: None,
        }];
        let a = analyze(clock(10), &segments, T0 + 10 * DAY_MS);
        assert_eq!(a.value_time_ms, 10 * DAY_MS);
        assert!((a.value_adding_ratio.value.unwrap_or(0.0) - 1.0).abs() < 1e-9);
        assert_eq!(a.unrecorded_ms, 0);
    }

    #[test]
    fn unmapped_journey_reads_as_unknown_not_as_terrible() {
        let a = analyze(clock(50), &[], T0 + 50 * DAY_MS);
        assert_eq!(a.value_adding_ratio.value, Some(0.0));
        assert_eq!(a.confidence, "unmapped");
        assert_eq!(a.coverage_ratio.value, Some(0.0));
        assert_eq!(a.gaps.len(), 1, "the whole clock is one gap");
    }

    #[test]
    fn degenerate_clocks_yield_a_stated_null_not_a_panic() {
        for (start, stop) in [(T0, T0), (T0 + DAY_MS, T0)] {
            let c = Clock {
                start_ms: start,
                stop_ms: stop,
                start_source: "clock_start_at",
                stop_source: "clock_stop_at",
                running: false,
            };
            let a = analyze(c, &[seg("x", "other", CATEGORY_VALUE_ADDING, 0, 1)], T0);
            assert_eq!(a.value_adding_ratio.value, None);
            assert!(a.reason.is_some());
            assert_eq!(a.lead_time_ms, 0);
        }
    }

    #[test]
    fn value_time_never_exceeds_process_time_never_exceeds_lead_time() {
        let segments = vec![
            seg("a", "treatment", CATEGORY_VALUE_ADDING, 0, 3),
            seg("b", "triage", CATEGORY_NECESSARY, 2, 9),
            seg("c", "other", CATEGORY_UNNECESSARY, 8, 30),
        ];
        let a = analyze(clock(40), &segments, T0 + 40 * DAY_MS);
        assert!(a.value_time_ms <= a.process_time_ms);
        assert!(a.process_time_ms <= a.lead_time_ms);
        assert!(a.touch_time_ms >= a.value_time_ms);
    }

    // -- gaps, stages, handoffs ------------------------------------------

    #[test]
    fn gaps_are_found_at_every_position() {
        let segments = vec![
            seg("mid", "triage", CATEGORY_VALUE_ADDING, 10, 20),
            seg("late", "treatment", CATEGORY_VALUE_ADDING, 40, 50),
        ];
        let a = analyze(clock(100), &segments, T0 + 100 * DAY_MS);
        assert_eq!(a.gaps.len(), 3, "leading, interior, trailing");
        // Ranked longest first: 50→100 is the biggest.
        assert_eq!(a.gaps[0].duration_ms, 50 * DAY_MS);
        assert_eq!(a.gaps[0].after.as_deref(), Some("late"));
        assert_eq!(a.gaps[0].before, None, "trailing gap has nothing after it");
        let leading = a
            .gaps
            .iter()
            .find(|g| g.start_ms == T0)
            .expect("leading gap");
        assert_eq!(leading.before.as_deref(), Some("mid"));
        assert_eq!(
            leading.stage.as_deref(),
            Some("triage"),
            "waiting to reach triage"
        );
    }

    #[test]
    fn a_fully_covered_clock_has_no_gaps() {
        let segments = vec![seg("all of it", "treatment", CATEGORY_VALUE_ADDING, 0, 30)];
        let a = analyze(clock(30), &segments, T0 + 30 * DAY_MS);
        assert!(a.gaps.is_empty());
        assert_eq!(a.coverage_ratio.value, Some(1.0));
        assert_eq!(a.confidence, "mapped");
    }

    #[test]
    fn handoffs_count_actor_and_location_changes() {
        let mut segments = vec![
            seg("a", "referral", CATEGORY_VALUE_ADDING, 0, 1),
            seg("b", "triage", CATEGORY_VALUE_ADDING, 5, 6),
            seg("c", "treatment", CATEGORY_VALUE_ADDING, 10, 11),
        ];
        segments[0].actor_ref = Some("worker:a".to_string());
        segments[1].actor_ref = Some("worker:b".to_string());
        segments[2].actor_ref = Some("worker:b".to_string());
        segments[2].location_ref = Some("place:x".to_string());
        let a = analyze(clock(20), &segments, T0 + 20 * DAY_MS);
        assert_eq!(a.handoffs.actor_changes, 1);
        assert_eq!(a.handoffs.location_changes, 1);
        assert_eq!(a.handoffs.total, 2);
        assert_eq!(a.handoffs.distinct_actors, 2);
        assert_eq!(a.handoffs.distinct_locations, 1);
        assert!(
            a.handoffs.gap_ms_at_handoffs > 0,
            "gaps sit at both boundaries"
        );
    }

    #[test]
    fn no_actors_means_no_handoffs() {
        let segments = vec![
            seg("a", "referral", CATEGORY_VALUE_ADDING, 0, 1),
            seg("b", "triage", CATEGORY_VALUE_ADDING, 5, 6),
        ];
        let a = analyze(clock(20), &segments, T0 + 20 * DAY_MS);
        assert_eq!(a.handoffs.total, 0);
        assert_eq!(a.handoffs.gap_ms_at_handoffs, 0);
    }

    #[test]
    fn stage_attribution_charges_the_wait_to_what_you_waited_for() {
        // Day 0-1 referral sent, then nothing until triage on day 30,
        // then nothing until the clock stops on day 40.
        let segments = vec![
            seg("referral sent", "referral", CATEGORY_VALUE_ADDING, 0, 1),
            seg("triaged", "triage", CATEGORY_VALUE_ADDING, 30, 31),
        ];
        let a = analyze(clock(40), &segments, T0 + 40 * DAY_MS);
        let stage_of = |name: &str| {
            a.by_stage
                .iter()
                .find(|s| s.stage == name)
                .map(|s| s.non_value_adding_ms)
        };
        // The 29-day wait is charged to triage, not to referral: the
        // patient was waiting *to reach* triage, and naming the stage
        // they were stuck before is what makes the finding actionable.
        assert_eq!(stage_of("referral"), Some(0));
        assert_eq!(
            stage_of("triage"),
            Some(29 * DAY_MS + 9 * DAY_MS),
            "the 29-day wait to reach triage, plus the 9-day trailing gap, \
             which falls back to the last stage left because nothing follows it"
        );
        // The two gaps between them account for all the unrecorded time.
        let attributed: i64 = a.by_stage.iter().map(|s| s.non_value_adding_ms).sum();
        assert_eq!(attributed, a.unrecorded_ms, "every gap lands on some stage");
    }

    // -- validation -------------------------------------------------------

    #[test]
    fn classification_vocabularies_are_closed() {
        assert!(validate_classification("triage", CATEGORY_VALUE_ADDING, None).is_ok());
        assert!(validate_classification("sideways", CATEGORY_VALUE_ADDING, None).is_err());
        assert!(validate_classification("triage", "fast", None).is_err());
        assert!(
            validate_classification("triage", CATEGORY_NECESSARY, Some("nonsense")).is_err(),
            "unknown waste"
        );
    }

    #[test]
    fn waste_coupling_rules_hold() {
        assert!(
            validate_classification("triage", CATEGORY_VALUE_ADDING, Some("waiting")).is_err(),
            "value-adding waste is a contradiction"
        );
        assert!(
            validate_classification("triage", CATEGORY_UNNECESSARY, None).is_err(),
            "pure waste must say what kind"
        );
        assert!(validate_classification("triage", CATEGORY_UNNECESSARY, Some("waiting")).is_ok());
        assert!(
            validate_classification("triage", CATEGORY_NECESSARY, None).is_ok(),
            "necessary non-value-adding needs no waste type"
        );
    }

    #[test]
    fn intervals_must_move_forward() {
        assert!(validate_interval(100, Some(200)).is_ok());
        assert!(validate_interval(100, None).is_ok(), "still running");
        assert!(validate_interval(100, Some(100)).is_err(), "zero length");
        assert!(validate_interval(100, Some(50)).is_err(), "reversed");
    }

    // -- percentiles ------------------------------------------------------

    #[test]
    fn nearest_rank_percentiles_return_observed_values() {
        let sample = [10, 20, 30, 40, 50, 60, 70, 80, 90, 100];
        assert_eq!(percentile(&sample, 0.0), Some(10));
        assert_eq!(percentile(&sample, 0.5), Some(50));
        assert_eq!(percentile(&sample, 0.9), Some(90));
        assert_eq!(percentile(&sample, 1.0), Some(100));
        for p in [0.0, 0.25, 0.5, 0.75, 0.9, 1.0] {
            let v = percentile(&sample, p).expect("value");
            assert!(sample.contains(&v), "every percentile is an observation");
        }
    }

    #[test]
    fn percentiles_on_degenerate_samples() {
        assert_eq!(percentile(&[], 0.5), None);
        assert_eq!(percentile(&[42], 0.5), Some(42));
        assert_eq!(percentile(&[42], 0.0), Some(42));
        assert_eq!(percentile(&[1, 2], 0.5), Some(1));
        assert_eq!(percentile(&[1, 2], 0.51), Some(2));
        assert_eq!(percentile(&[1, 2], 1.5), None, "out of range");
        assert_eq!(percentile(&[1, 2], f64::NAN), None);
    }

    #[test]
    fn distribution_reports_the_shape() {
        let d = distribution(&[5, 1, 3, 2, 4]).expect("distribution");
        assert_eq!(d.n, 5);
        assert_eq!(d.min_ms, 1);
        assert_eq!(d.max_ms, 5);
        assert_eq!(d.p50_ms, 3);
        assert_eq!(d.mean_ms, 3);
        assert_eq!(d.method, "nearest_rank");
        assert!(distribution(&[]).is_none());
    }

    // -- standards --------------------------------------------------------

    #[test]
    fn the_standards_catalogue_is_well_formed() {
        assert!(standard("rtt_18_weeks").is_some());
        assert!(standard("not_a_standard").is_none());
        for s in STANDARDS {
            assert!(s.threshold_ms > 0, "{} has no threshold", s.id);
            assert!(
                (0.0..=1.0).contains(&s.target_ratio),
                "{} target out of range",
                s.id
            );
            assert!(!s.as_of.is_empty(), "{} must carry a citation date", s.id);
        }
        let rtt = standard("rtt_18_weeks").expect("rtt");
        assert_eq!(rtt.threshold_ms, 126 * DAY_MS, "18 weeks");
        let ae = standard("ae_4_hours").expect("ae");
        assert_eq!(ae.threshold_ms, 4 * HOUR_MS, "hours, not days");
    }

    #[test]
    fn compliance_scores_against_a_threshold() {
        let leads = [10 * DAY_MS, 20 * DAY_MS, 200 * DAY_MS];
        let c = compliance(
            &leads,
            "rtt_18_weeks",
            126 * DAY_MS,
            Some(0.92),
            Some("2026-08"),
        );
        assert_eq!(c.within, 2);
        assert_eq!(c.breached, 1);
        assert_eq!(c.target_met, Some(false), "67% is short of 92%");
        let empty = compliance(&[], "rtt_18_weeks", 126 * DAY_MS, Some(0.92), None);
        assert_eq!(empty.achieved_ratio, None);
        assert_eq!(empty.target_met, None, "no target verdict on no data");
    }

    // -- cohort -----------------------------------------------------------

    #[test]
    fn cohort_separates_the_typical_journey_from_the_system_ratio() {
        // Nine efficient short journeys and one enormous slow one: the
        // aggregate is dragged down, the median is not. The divergence is
        // the finding (§7.2).
        let mut analyses: Vec<InstanceAnalysis> = (0..9)
            .map(|_| {
                analyze(
                    clock(10),
                    &[seg("care", "treatment", CATEGORY_VALUE_ADDING, 0, 9)],
                    T0 + 10 * DAY_MS,
                )
            })
            .collect();
        analyses.push(analyze(
            clock(1000),
            &[seg("care", "treatment", CATEGORY_VALUE_ADDING, 0, 1)],
            T0 + 1000 * DAY_MS,
        ));
        let c = cohort(&analyses);
        assert_eq!(c.instances, 10);
        let agg = c.aggregate_value_adding_ratio.value.expect("aggregate");
        let med = c.median_value_adding_ratio.expect("median");
        assert!(
            agg < 0.20,
            "the long journey dominates the aggregate: {agg}"
        );
        assert!(med > 0.80, "the typical journey is efficient: {med}");
        assert_eq!(c.waste_shape, "concentrated");
    }

    #[test]
    fn an_empty_cohort_is_null_not_zero() {
        let c = cohort(&[]);
        assert_eq!(c.instances, 0);
        assert!(c.lead_time.is_none());
        assert_eq!(c.aggregate_value_adding_ratio.value, None);
        assert_eq!(c.waste_shape, "insufficient_data");
        assert!(constraints(&[], &c).is_empty());
    }

    #[test]
    fn constraints_rank_by_recoverable_time_and_name_their_rule() {
        let analyses = vec![analyze(
            clock(100),
            &[
                seg("scan", "diagnostics", CATEGORY_VALUE_ADDING, 60, 61),
                seg("chase results", "diagnostics", CATEGORY_UNNECESSARY, 61, 90),
            ],
            T0 + 100 * DAY_MS,
        )];
        let c = cohort(&analyses);
        let findings = constraints(&analyses, &c);
        assert!(!findings.is_empty());
        for pair in findings.windows(2) {
            assert!(
                pair[0].recoverable_ms >= pair[1].recoverable_ms,
                "ordered by recoverable time"
            );
        }
        assert!(
            findings.iter().any(|f| f.rule == "longest_gap"),
            "the biggest queue is always named"
        );
        assert!(findings.iter().all(|f| !f.rule.is_empty()));
    }

    // -- flow -------------------------------------------------------------

    #[test]
    fn littles_law_labels_the_three_regimes() {
        // κ=100, λ=1/day ⇒ τ̂=100 days, against 10 observed: growing.
        let growing = flow(100, 100, 10, 100, Some(10 * DAY_MS));
        assert_eq!(growing.interpretation, "backlog_growing");
        assert!((growing.implied_lead_time_days.unwrap_or(0.0) - 100.0).abs() < 1e-9);

        // κ=10, λ=1/day ⇒ τ̂=10 days, against 10 observed: steady.
        let steady = flow(100, 100, 100, 10, Some(10 * DAY_MS));
        assert_eq!(steady.interpretation, "steady_state");

        // κ=1, λ=1/day ⇒ τ̂=1 day, against 10 observed: draining.
        let draining = flow(100, 100, 100, 1, Some(10 * DAY_MS));
        assert_eq!(draining.interpretation, "queue_draining");
    }

    #[test]
    fn flow_refuses_to_invent_numbers_it_cannot_have() {
        let no_closures = flow(30, 10, 0, 10, None);
        assert_eq!(no_closures.utilisation, None);
        assert!(
            no_closures.utilisation_reason.is_some(),
            "a null must say why"
        );
        assert_eq!(no_closures.interpretation, "insufficient_data");

        let no_arrivals = flow(30, 0, 5, 10, Some(DAY_MS));
        assert_eq!(no_arrivals.implied_lead_time_days, None);
        assert_eq!(no_arrivals.arrival_rate_per_day, Some(0.0));

        let zero_window = flow(0, 5, 5, 5, Some(DAY_MS));
        assert_eq!(zero_window.arrival_rate_per_day, None);
        assert_eq!(zero_window.utilisation, None);
    }

    #[test]
    fn utilisation_is_the_ratio_of_the_two_rates() {
        let f = flow(10, 20, 10, 5, Some(DAY_MS));
        assert_eq!(f.arrival_rate_per_day, Some(2.0));
        assert_eq!(f.service_rate_per_day, Some(1.0));
        assert_eq!(f.utilisation, Some(2.0), "demand at twice capacity");
    }

    // -- flow gauges (§15 TBA-11) -----------------------------------------

    /// A cohort of `n` identical short journeys.
    fn cohort_of(n: usize) -> CohortAnalysis {
        let analyses: Vec<InstanceAnalysis> = (0..n)
            .map(|_| {
                analyze(
                    clock(10),
                    &[seg("care", "treatment", CATEGORY_VALUE_ADDING, 0, 1)],
                    T0 + 10 * DAY_MS,
                )
            })
            .collect();
        cohort(&analyses)
    }

    #[test]
    fn small_cohorts_are_suppressed_not_exported() {
        // `/metrics.prom` is on the public allow-list, so a p90 lead
        // time over three patients would leave by the side door the API
        // closed. It is counted, never labelled.
        let per_pathway = vec![
            ("big".to_string(), cohort_of(20)),
            ("tiny".to_string(), cohort_of(2)),
            ("empty".to_string(), cohort_of(0)),
        ];
        let set = flow_metric_rows(&per_pathway, 50, 5);
        assert_eq!(set.rows.len(), 1);
        assert_eq!(set.rows[0].pathway_pid, "big");
        assert_eq!(set.suppressed_pathways, 1, "`tiny` is withheld");
        assert_eq!(set.dropped_pathways, 0);
        assert!(
            !set.rows.iter().any(|row| row.pathway_pid == "tiny"),
            "a suppressed pathway must not appear under any label"
        );
        // An empty pathway is neither exported nor counted as
        // suppressed: there is nothing to withhold.
        assert!(!set.rows.iter().any(|row| row.pathway_pid == "empty"));
    }

    #[test]
    fn the_series_cap_keeps_the_largest_cohorts_and_says_what_it_dropped() {
        let per_pathway: Vec<(String, CohortAnalysis)> = (1..=10)
            .map(|i| (format!("p{i:02}"), cohort_of(i * 10)))
            .collect();
        let set = flow_metric_rows(&per_pathway, 3, 5);
        assert_eq!(set.rows.len(), 3);
        assert_eq!(
            set.rows.iter().map(|r| r.instances).collect::<Vec<_>>(),
            vec![100, 90, 80],
            "largest cohort first"
        );
        assert_eq!(set.dropped_pathways, 7, "the cap is never silent");
    }

    #[test]
    fn selection_is_stable_across_passes() {
        // A dashboard must not reshuffle between scrapes, so ties break
        // on the pid rather than on map iteration order.
        let per_pathway: Vec<(String, CohortAnalysis)> = ["c", "a", "b"]
            .iter()
            .map(|pid| ((*pid).to_string(), cohort_of(9)))
            .collect();
        let first = flow_metric_rows(&per_pathway, 2, 5);
        let mut shuffled = per_pathway.clone();
        shuffled.reverse();
        let second = flow_metric_rows(&shuffled, 2, 5);
        assert_eq!(first.rows, second.rows);
        assert_eq!(
            first
                .rows
                .iter()
                .map(|r| r.pathway_pid.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "b"]
        );
    }

    #[test]
    fn an_empty_estate_exports_nothing_rather_than_zeroes() {
        let set = flow_metric_rows(&[], 50, 5);
        assert!(set.rows.is_empty());
        assert_eq!(set.suppressed_pathways, 0);
        assert_eq!(set.dropped_pathways, 0);
    }

    // -- property-style sweeps -------------------------------------------

    #[test]
    fn invariants_hold_over_a_generated_sweep() {
        // A deterministic pseudo-random sweep: cheap here, and it covers
        // the shapes the hand-written cases do not (security invariant 2:
        // never panic on arbitrary input).
        let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let categories = [
            CATEGORY_VALUE_ADDING,
            CATEGORY_NECESSARY,
            CATEGORY_UNNECESSARY,
        ];
        for _ in 0..500 {
            let span_days = i64::try_from(next() % 200).unwrap_or(1);
            let c = clock(span_days);
            let count = usize::try_from(next() % 12).unwrap_or(0);
            let segments: Vec<Segment> = (0..count)
                .map(|_| {
                    let start = i64::try_from(next() % 250).unwrap_or(0) - 25;
                    let len = i64::try_from(next() % 60).unwrap_or(0);
                    let category = categories[usize::try_from(next() % 3).unwrap_or(0)];
                    let mut s = seg("s", "other", category, start, start + len);
                    if next() % 2 == 0 {
                        s.end_ms = None;
                    }
                    s
                })
                .collect();
            let a = analyze(c, &segments, c.stop_ms);
            let sum: i64 = a.by_category.iter().map(|x| x.ms).sum();
            assert_eq!(sum, a.lead_time_ms, "partition must be exact");
            assert!(a.value_time_ms <= a.process_time_ms);
            assert!(a.process_time_ms <= a.lead_time_ms);
            assert!(a.touch_time_ms >= a.value_time_ms, "sum ≥ union");
            assert!(a.unrecorded_ms >= 0);
            for r in [&a.value_adding_ratio, &a.activity_ratio, &a.coverage_ratio] {
                if let Some(v) = r.value {
                    assert!((0.0..=1.0).contains(&v), "ratio out of range: {v}");
                }
            }
            let gap_total: i64 = a.gaps.iter().map(|g| g.duration_ms).sum();
            assert_eq!(
                gap_total, a.unrecorded_ms,
                "gaps account for all unrecorded time"
            );
        }
    }
}
