//! **Time-based analysis** (TBA) — pure computation over a task's
//! status-transition log. No I/O, no clock read (`as_of` is always a
//! parameter), fully deterministic, and therefore unit-testable without
//! a database.
//!
//! See `spec/time-based-analysis.md` for the contract. The rules this
//! module exists to enforce, and which a later "simplification" would
//! quietly destroy:
//!
//! 1. **Cycle time and lead time are different numbers** (§6.1). An
//!    item that sat in `todo` for three weeks and was built in two days
//!    has a cycle time of 2 days and a lead time of 23. Reporting the
//!    first as "our delivery time" is a tenfold flattering misreport,
//!    and it is the commonest error in the field — so both are always
//!    returned, and never one labelled as the other.
//! 2. **The denominator is elapsed calendar time** (§6.3), never the
//!    sum of recorded activity. Here that is structural: the intervals
//!    come from one ordered log and partition the elapsed time exactly,
//!    so there is no unrecorded remainder to omit.
//! 3. **No status is excluded and no clock pauses** (§12.3). Every
//!    millisecond from creation to finish lands in exactly one status
//!    and one category, and they sum to the lead time by construction —
//!    a property test, not a promise. Lead time rather than cycle time
//!    is the partitioned span, because the backlog dwell is real time
//!    the requester waited and must land somewhere; flow efficiency is
//!    still measured against cycle time, since the team cannot be held
//!    to the backlog's dwell.
//! 4. **Nothing here is per-person** (§12.4). Handoff counts describe
//!    the item's journey; there is deliberately no per-assignee cycle
//!    time, and adding one would destroy the data quality the free-at-
//!    collection design depends on.
//!
//! Vocabulary is the value-stream-mapping one (VA / NNVA / UNVA, LT /
//! PT / VT / %A / #HO / RFPY) and the queueing-theory one (λ / μ / ρ /
//! τ / κ); see the spec's §2 for provenance.

use std::collections::BTreeMap;

use serde::Serialize;

/// Milliseconds in one day.
pub const DAY_MS: i64 = 86_400_000;

/// The value-adding category: the product is being built.
pub const CATEGORY_VALUE_ADDING: &str = "value_adding";

/// Necessary non-value-adding: required, but not building.
pub const CATEGORY_NECESSARY: &str = "necessary_non_value_adding";

/// Unnecessary non-value-adding: queueing, blocking, rework.
pub const CATEGORY_UNNECESSARY: &str = "unnecessary_non_value_adding";

/// The VSM categories. Closed vocabulary.
pub const CATEGORIES: &[&str] = &[
    CATEGORY_VALUE_ADDING,
    CATEGORY_NECESSARY,
    CATEGORY_UNNECESSARY,
];

/// The board status that means finished. Time stops here.
pub const FINISHED_STATUS: &str = "done";

/// The board status that means not yet started — the backlog.
pub const BACKLOG_STATUS: &str = "todo";

/// The board status that means externally halted.
pub const BLOCKED_STATUS: &str = "blocked";

/// The board order used to decide whether a move was **backwards**
/// (§6.5). `blocked` is deliberately absent: it is orthogonal to
/// progress, so moving into or out of it is never rework.
pub const BOARD_ORDER: &[&str] = &["todo", "in_progress", "in_review", "done"];

/// The **disclosed default** status classification (§5.3).
///
/// `todo` is `inventory` waste rather than merely "not started": work
/// bought and not yet used, aging while it waits. `in_review` is
/// necessary — review is not waste — even though review *waiting* is,
/// and the board cannot tell the two apart (spec §17).
pub const DEFAULT_CLASSES: &[(&str, &str)] = &[
    (BACKLOG_STATUS, CATEGORY_UNNECESSARY),
    ("in_progress", CATEGORY_VALUE_ADDING),
    ("in_review", CATEGORY_NECESSARY),
    (BLOCKED_STATUS, CATEGORY_UNNECESSARY),
];

/// The VSM waste type a non-value-adding status represents.
#[must_use]
pub fn waste_for(status: &str) -> Option<&'static str> {
    match status {
        BACKLOG_STATUS => Some("inventory"),
        BLOCKED_STATUS => Some("waiting"),
        _ => None,
    }
}

/// The default classification as a map.
#[must_use]
pub fn default_classes() -> BTreeMap<String, String> {
    DEFAULT_CLASSES
        .iter()
        .map(|(status, category)| ((*status).to_string(), (*category).to_string()))
        .collect()
}

/// Parse the `PROJECT_PORTFOLIO_MANAGEMENT_FLOW_CLASSES` override — a
/// map of status → VSM category.
///
/// Absent / blank / unparsable / an unknown category ⇒ `None`, and the
/// caller falls back to [`default_classes`] **whole**. Half-applying an
/// override would produce a figure that matches no stated
/// classification at all, which is worse than ignoring it — the same
/// posture as the existing WIP-limit parser.
#[must_use]
pub fn parse_classes(raw: Option<&str>) -> Option<BTreeMap<String, String>> {
    let raw = raw?.trim();
    if raw.is_empty() {
        return None;
    }
    let parsed: BTreeMap<String, String> = serde_json::from_str(raw).ok()?;
    if parsed.is_empty() {
        return None;
    }
    for category in parsed.values() {
        if !CATEGORIES.contains(&category.as_str()) {
            return None;
        }
    }
    Some(parsed)
}

/// Classify a status, falling back to `unnecessary_non_value_adding`
/// for anything the map does not name.
///
/// The fallback is deliberately the *pessimistic* one: an unclassified
/// status counts against you, so adding a board column cannot silently
/// improve the flow efficiency.
#[must_use]
pub fn classify<'a>(classes: &'a BTreeMap<String, String>, status: &str) -> &'a str {
    classes
        .get(status)
        .map_or(CATEGORY_UNNECESSARY, String::as_str)
}

/// Whether a status counts as started (anything past the backlog).
#[must_use]
pub fn is_started(status: &str) -> bool {
    status != BACKLOG_STATUS
}

/// Whether a move went backwards on the board (§6.5). Moves into or out
/// of `blocked` are never rework.
#[must_use]
pub fn is_backwards(from: &str, to: &str) -> bool {
    if from == BLOCKED_STATUS || to == BLOCKED_STATUS {
        return false;
    }
    match (
        BOARD_ORDER.iter().position(|s| *s == from),
        BOARD_ORDER.iter().position(|s| *s == to),
    ) {
        (Some(a), Some(b)) => b < a,
        _ => false,
    }
}

// ---------------------------------------------------------------------
// Inputs
// ---------------------------------------------------------------------

/// One recorded status change, as the analysis consumes it.
#[derive(Clone, Debug)]
pub struct Transition {
    /// The status left; `None` marks the task's creation.
    pub from_status: Option<String>,
    /// The status entered.
    pub to_status: String,
    /// When, epoch milliseconds.
    pub at_ms: i64,
    /// Who it was assigned to at that moment.
    pub assignee_ref: Option<String>,
    /// Synthesised by the migration rather than observed.
    pub backfilled: bool,
}

/// The task facts the analysis needs beyond its transitions.
#[derive(Clone, Copy, Debug)]
pub struct TaskClock {
    /// When the task was created, epoch milliseconds.
    pub created_ms: i64,
    /// When it first reached `done`, if it has.
    pub done_ms: Option<i64>,
}

// ---------------------------------------------------------------------
// Outputs
// ---------------------------------------------------------------------

/// A ratio reported with the two figures it came from.
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

/// Time spent in one board status.
#[derive(Clone, Debug, Serialize)]
pub struct StatusShare {
    /// The status.
    pub status: String,
    /// The VSM category it was classified as, in the map in force.
    pub category: String,
    /// The VSM waste type, where the status represents one.
    pub waste: Option<String>,
    /// Milliseconds spent there.
    pub ms: i64,
    /// The same in days, for display.
    pub days: f64,
    /// Share of lead time — the span the statuses partition.
    pub share: Option<f64>,
    /// How many times the item entered this status.
    pub entries: usize,
}

/// Time in one VSM category. These partition the lead time.
#[derive(Clone, Debug, Serialize)]
pub struct CategoryShare {
    /// One of [`CATEGORIES`].
    pub category: String,
    /// Milliseconds.
    pub ms: i64,
    /// The same in days, for display.
    pub days: f64,
    /// Share of lead time.
    pub share: Option<f64>,
}

/// The per-task analysis (spec §6).
#[derive(Clone, Debug, Serialize)]
pub struct TaskAnalysis {
    /// Created → finished (or `as_of`). What the requester waits.
    pub lead_time_ms: i64,
    /// The same in days.
    pub lead_time_days: f64,
    /// First started → finished (or `as_of`). What the team controls.
    /// `None` when the item never left the backlog.
    pub cycle_time_ms: Option<i64>,
    /// The same in days.
    pub cycle_time_days: Option<f64>,
    /// Why the cycle time is null, when it is.
    pub cycle_time_reason: Option<String>,
    /// Time in value-adding statuses (VT / φ).
    pub work_time_ms: i64,
    /// Time in value-adding + necessary statuses (PT).
    pub process_time_ms: i64,
    /// Cycle time − process time (ω).
    pub wait_time_ms: i64,
    /// Time in `blocked` — the waste with the shortest path to a fix.
    pub blocked_time_ms: i64,
    /// Time in `todo` — the backlog dwell, reported separately so it
    /// cannot vanish behind the cycle-time denominator (§6.1).
    pub queue_time_ms: i64,
    /// Work time over cycle time (%A). The headline 5–15% ratio.
    pub flow_efficiency: Ratio,
    /// Per-status time; partitions the lead time.
    pub by_status: Vec<StatusShare>,
    /// Per-category time; partitions the lead time.
    pub by_category: Vec<CategoryShare>,
    /// Transitions considered.
    pub transitions: usize,
    /// How many of them were synthesised by the migration (§5.4).
    pub backfilled: usize,
    /// Backwards moves on the board.
    pub rework_count: usize,
    /// Whether the item never moved backwards.
    pub first_pass: bool,
    /// Distinct assignees over the item's life.
    pub distinct_assignees: usize,
    /// Assignee changes (#HO).
    pub handoffs: usize,
    /// Whether the item has finished.
    pub finished: bool,
    /// For an open item: how long since it started. The only figure
    /// here that is actionable today.
    pub age_ms: Option<i64>,
    /// The same in days.
    pub age_days: Option<f64>,
}

/// Milliseconds as days, rounded to three places for display.
#[must_use]
#[allow(clippy::cast_precision_loss)] // display only
pub fn as_days(ms: i64) -> f64 {
    (ms as f64 / DAY_MS as f64 * 1000.0).round() / 1000.0
}

/// One derived interval: the task sat in `status` from `from_ms` until
/// `to_ms`.
#[derive(Clone, Debug)]
pub struct Interval {
    /// The board status occupied.
    pub status: String,
    /// Start, epoch milliseconds.
    pub from_ms: i64,
    /// End, epoch milliseconds.
    pub to_ms: i64,
}

impl Interval {
    /// The duration, never negative.
    #[must_use]
    pub fn duration_ms(&self) -> i64 {
        self.to_ms.saturating_sub(self.from_ms).max(0)
    }
}

/// Derive one task's intervals, in time order, **partitioning the whole
/// span from creation to finish** (spec §5.2, §6.3).
///
/// Three edges are handled here rather than left to the caller, because
/// each of them would otherwise produce time that belongs to no status
/// — and time that belongs to no status is time a report can quietly
/// lose:
///
/// - **Before the first recorded transition.** A backfilled row (§5.4)
///   is stamped at `status_changed_at`, which is later than the task's
///   creation, so the span between them is unattributed. It is charged
///   to the transition's `from_status` where one is recorded, and to
///   `todo` otherwise — the pessimistic choice, matching [`classify`]'s
///   fallback, so unknown history can never flatter the figures.
/// - **`done` is terminal.** The clock stops there; a finished task
///   analysed a year later must not have accrued a year.
/// - **A clock skew**, where `as_of` precedes the last transition,
///   yields zero-length intervals rather than negative ones.
#[must_use]
pub fn intervals(transitions: &[Transition], clock: TaskClock, as_of_ms: i64) -> Vec<Interval> {
    let mut sorted: Vec<&Transition> = transitions.iter().collect();
    sorted.sort_by_key(|t| t.at_ms);
    let end = clock.done_ms.unwrap_or(as_of_ms).max(clock.created_ms);
    let mut out: Vec<Interval> = Vec::with_capacity(sorted.len() + 1);
    let mut cursor = clock.created_ms;

    for (index, transition) in sorted.iter().enumerate() {
        let start = transition.at_ms.clamp(clock.created_ms, end);
        if start > cursor {
            out.push(Interval {
                status: transition
                    .from_status
                    .clone()
                    .unwrap_or_else(|| BACKLOG_STATUS.to_string()),
                from_ms: cursor,
                to_ms: start,
            });
            cursor = start;
        }
        if transition.to_status == FINISHED_STATUS {
            cursor = end;
            break;
        }
        let next = sorted
            .get(index + 1)
            .map_or(end, |next| next.at_ms)
            .clamp(cursor, end);
        if next > cursor {
            out.push(Interval {
                status: transition.to_status.clone(),
                from_ms: cursor,
                to_ms: next,
            });
        }
        cursor = next;
    }

    // Anything left — an empty log, or a task whose last recorded move
    // was long ago — is unknown history, charged to `todo` for the same
    // reason as above.
    if cursor < end {
        out.push(Interval {
            status: BACKLOG_STATUS.to_string(),
            from_ms: cursor,
            to_ms: end,
        });
    }
    out
}

/// Total the intervals per board status, shared over the lead time.
fn roll_up_statuses(
    spans: &[Interval],
    classes: &BTreeMap<String, String>,
    lead: i64,
) -> Vec<StatusShare> {
    let mut per_status: BTreeMap<String, (i64, usize)> = BTreeMap::new();
    for span in spans {
        let entry = per_status.entry(span.status.clone()).or_insert((0, 0));
        entry.0 = entry.0.saturating_add(span.duration_ms());
        entry.1 += 1;
    }
    per_status
        .iter()
        .map(|(status, (ms, entries))| StatusShare {
            status: status.clone(),
            category: classify(classes, status).to_string(),
            waste: waste_for(status).map(ToString::to_string),
            ms: *ms,
            days: as_days(*ms),
            share: Ratio::new(*ms, lead).value,
            entries: *entries,
        })
        .collect()
}

/// Fold the per-status totals into the three VSM categories. Every
/// category is emitted even at zero, so a consumer never has to decide
/// whether a missing key means zero or means unknown.
fn roll_up_categories(by_status: &[StatusShare], lead: i64) -> Vec<CategoryShare> {
    CATEGORIES
        .iter()
        .map(|category| {
            let ms = by_status
                .iter()
                .filter(|s| s.category == *category)
                .fold(0i64, |acc, s| acc.saturating_add(s.ms));
            CategoryShare {
                category: (*category).to_string(),
                ms,
                days: as_days(ms),
                share: Ratio::new(ms, lead).value,
            }
        })
        .collect()
}

/// Analyse one task (spec §6).
#[must_use]
pub fn analyze(
    transitions: &[Transition],
    clock: TaskClock,
    classes: &BTreeMap<String, String>,
    as_of_ms: i64,
) -> TaskAnalysis {
    let mut sorted: Vec<&Transition> = transitions.iter().collect();
    sorted.sort_by_key(|t| t.at_ms);

    let end = clock.done_ms.unwrap_or(as_of_ms).max(clock.created_ms);
    let spans = intervals(transitions, clock, as_of_ms);

    // Lead time is what the requester waits: creation to finish. The
    // intervals partition it exactly, which is why it is the
    // denominator for every per-status share (§6.3).
    let lead = end.saturating_sub(clock.created_ms).max(0);

    // Cycle time is what the team controls: from the moment the item
    // first occupied a started status.
    //
    // Taken as the earliest of two candidates, because either alone is
    // wrong in a case that really happens:
    //
    // - the first **started interval**, which catches a pre-history
    //   span (§5.2) that was already started — otherwise work time
    //   could begin before the cycle it is measured against;
    // - the first **transition** into a started status, which catches
    //   an item that started and finished inside one millisecond. Such
    //   an item has no interval at all, and reporting its cycle time as
    //   "never started" rather than as zero would be plainly false.
    let started_interval = spans
        .iter()
        .find(|span| is_started(&span.status))
        .map(|span| span.from_ms);
    let started_transition = sorted
        .iter()
        // A transition **after** the window is filtered out rather than
        // clamped into it: clamping a future move back to `end` would
        // let it retroactively start the clock.
        .filter(|t| t.at_ms <= end)
        .find(|t| is_started(&t.to_status))
        .map(|t| t.at_ms.max(clock.created_ms));
    let first_started = match (started_interval, started_transition) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (only @ Some(_), None) | (None, only) => only,
    };
    let cycle = first_started.map(|start| end.saturating_sub(start).max(0));

    let by_status = roll_up_statuses(&spans, classes, lead);
    let by_category = roll_up_categories(&by_status, lead);
    let total_for = |category: &str| -> i64 {
        by_status
            .iter()
            .filter(|s| s.category == category)
            .fold(0i64, |acc, s| acc.saturating_add(s.ms))
    };
    let work = total_for(CATEGORY_VALUE_ADDING);
    let process = work.saturating_add(total_for(CATEGORY_NECESSARY));

    let status_ms = |status: &str| -> i64 {
        by_status
            .iter()
            .find(|s| s.status == status)
            .map_or(0, |s| s.ms)
    };

    // Rework: backwards moves, excluding anything through `blocked`.
    let rework_count = sorted
        .iter()
        .filter(|t| {
            t.from_status
                .as_deref()
                .is_some_and(|from| is_backwards(from, &t.to_status))
        })
        .count();

    // Handoffs: assignee changes across the item's life.
    let mut handoffs = 0usize;
    for pair in sorted.windows(2) {
        if pair[0].assignee_ref != pair[1].assignee_ref {
            handoffs += 1;
        }
    }
    let distinct_assignees = sorted
        .iter()
        .filter_map(|t| t.assignee_ref.as_ref())
        .collect::<std::collections::BTreeSet<_>>()
        .len();

    let finished = clock.done_ms.is_some();
    let age = (!finished)
        .then(|| first_started.map(|start| as_of_ms.saturating_sub(start).max(0)))
        .flatten();
    let denominator = cycle.unwrap_or(0);

    TaskAnalysis {
        lead_time_ms: lead,
        lead_time_days: as_days(lead),
        cycle_time_ms: cycle,
        cycle_time_days: cycle.map(as_days),
        cycle_time_reason: cycle.is_none().then(|| {
            format!("the task never left `{BACKLOG_STATUS}`, so it has no cycle time yet")
        }),
        work_time_ms: work,
        process_time_ms: process,
        wait_time_ms: denominator.saturating_sub(process).max(0),
        blocked_time_ms: status_ms(BLOCKED_STATUS),
        queue_time_ms: status_ms(BACKLOG_STATUS),
        flow_efficiency: Ratio::new(work, denominator),
        by_status,
        by_category,
        transitions: sorted.len(),
        backfilled: sorted.iter().filter(|t| t.backfilled).count(),
        rework_count,
        first_pass: rework_count == 0,
        distinct_assignees,
        handoffs,
        finished,
        age_ms: age,
        age_days: age.map(as_days),
    }
}

// ---------------------------------------------------------------------
// Cohort statistics (spec §7)
// ---------------------------------------------------------------------

/// Nearest-rank percentile over a **sorted** sample (spec §7.1).
///
/// Nearest-rank always returns an observed value, so "which item is the
/// p85?" has an answer — with interpolation it does not.
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

/// A long-tailed duration distribution, reported by percentile rather
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
    /// 85th percentile — the service-level-expectation convention.
    pub p85_ms: i64,
    /// 95th percentile.
    pub p95_ms: i64,
    /// Largest observation.
    pub max_ms: i64,
    /// Arithmetic mean — reported, but skew-sensitive and describing no
    /// actual item.
    pub mean_ms: i64,
    /// The median in days, for display.
    pub p50_days: f64,
    /// The 85th percentile in days, for display.
    pub p85_days: f64,
    /// The percentile method in use.
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
        p85_ms: at(0.85),
        p95_ms: at(0.95),
        max_ms: sorted.last().copied().unwrap_or(0),
        mean_ms: mean,
        p50_days: as_days(at(0.50)),
        p85_days: as_days(at(0.85)),
        method: "nearest_rank",
    })
}

/// The minimum finished-item sample below which a service level
/// expectation is refused rather than computed from noise (spec §7.3).
pub const MIN_SLE_SAMPLE: usize = 10;

/// A service level expectation: "p% of items finish within N days",
/// derived from the team's own history (spec §7.3).
#[derive(Clone, Debug, Serialize)]
pub struct ServiceLevelExpectation {
    /// The percentile the expectation is stated at.
    pub percentile: f64,
    /// The cycle time at that percentile, milliseconds.
    pub within_ms: Option<i64>,
    /// The same in days — the number a team actually quotes.
    pub within_days: Option<f64>,
    /// Finished items the expectation was computed from.
    pub sample: usize,
    /// Why the expectation is null, when it is.
    pub reason: Option<String>,
    /// A caller-supplied commitment to score against, in days.
    pub target_days: Option<f64>,
    /// The share of items that met that commitment.
    pub target_achieved_ratio: Option<f64>,
    /// Whether the commitment was met at the stated percentile.
    pub target_met: Option<bool>,
}

/// Compute the service level expectation from finished items' cycle
/// times, optionally scoring an explicit commitment.
#[must_use]
#[allow(clippy::cast_precision_loss)] // display ratio over a bounded sample
pub fn service_level_expectation(
    cycle_times_ms: &[i64],
    percentile_p: f64,
    target_days: Option<f64>,
) -> ServiceLevelExpectation {
    let mut sorted = cycle_times_ms.to_vec();
    sorted.sort_unstable();
    let sample = sorted.len();
    let enough = sample >= MIN_SLE_SAMPLE;
    let within = enough.then(|| percentile(&sorted, percentile_p)).flatten();

    let (achieved, met) = match target_days {
        Some(days) if days > 0.0 && !sorted.is_empty() => {
            #[allow(clippy::cast_possible_truncation)] // bounded by the guard
            let threshold = (days * DAY_MS as f64) as i64;
            let within_target = sorted.iter().filter(|ms| **ms <= threshold).count();
            let ratio = within_target as f64 / sorted.len() as f64;
            (Some(ratio), Some(ratio >= percentile_p))
        }
        _ => (None, None),
    };

    ServiceLevelExpectation {
        percentile: percentile_p,
        within_ms: within,
        within_days: within.map(as_days),
        sample,
        reason: (!enough).then(|| {
            format!(
                "only {sample} finished items; an expectation needs at least \
                 {MIN_SLE_SAMPLE} or it is a number computed from noise"
            )
        }),
        target_days,
        target_achieved_ratio: achieved,
        target_met: met,
    }
}

/// The plan-level analysis (spec §7).
#[derive(Clone, Debug, Serialize)]
pub struct PlanAnalysis {
    /// Tasks analysed.
    pub tasks: usize,
    /// Tasks finished.
    pub finished: usize,
    /// Tasks started and not finished (κ / WIP).
    pub work_in_progress: usize,
    /// Tasks still in the backlog.
    pub not_started: usize,
    /// Cycle-time distribution over finished tasks.
    pub cycle_time: Option<Distribution>,
    /// Lead-time distribution over finished tasks — always reported
    /// beside the cycle time so the flattering number cannot travel
    /// alone (spec §12.3).
    pub lead_time: Option<Distribution>,
    /// `Σ work / Σ cycle` — the system's ratio, dominated by the
    /// longest-running items.
    pub aggregate_flow_efficiency: Ratio,
    /// The median of the per-item ratios — the typical item.
    pub median_flow_efficiency: Option<f64>,
    /// `concentrated` when the two diverge (the waste sits in a
    /// minority of items), `uniform` when they agree.
    pub waste_shape: &'static str,
    /// Rolled first pass yield: the share of finished items that never
    /// moved backwards. Always reported beside throughput, so shipping
    /// work back to yourself cannot read as going faster.
    pub rolled_first_pass_yield: Option<f64>,
    /// Backwards moves across the plan.
    pub rework_count: usize,
    /// Per-status time across the plan.
    pub by_status: Vec<StatusShare>,
    /// Share of transitions synthesised by the migration (§5.4).
    pub backfilled_ratio: Option<f64>,
}

/// Aggregate per-task analyses into a plan view.
#[must_use]
pub fn plan(analyses: &[TaskAnalysis]) -> PlanAnalysis {
    let finished: Vec<&TaskAnalysis> = analyses.iter().filter(|a| a.finished).collect();
    let cycle_times: Vec<i64> = finished.iter().filter_map(|a| a.cycle_time_ms).collect();
    let lead_times: Vec<i64> = finished.iter().map(|a| a.lead_time_ms).collect();

    let total_work = analyses
        .iter()
        .fold(0i64, |a, x| a.saturating_add(x.work_time_ms));
    let total_cycle = analyses
        .iter()
        .fold(0i64, |a, x| a.saturating_add(x.cycle_time_ms.unwrap_or(0)));

    let mut ratios: Vec<f64> = analyses
        .iter()
        .filter_map(|a| a.flow_efficiency.value)
        .collect();
    ratios.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_ratio = if ratios.is_empty() {
        None
    } else {
        ratios.get(ratios.len() / 2).copied()
    };

    let aggregate = Ratio::new(total_work, total_cycle);
    let waste_shape = match (aggregate.value, median_ratio) {
        (Some(agg), Some(med)) if (agg - med).abs() > 0.05 => "concentrated",
        (Some(_), Some(_)) => "uniform",
        _ => "insufficient_data",
    };

    #[allow(clippy::cast_precision_loss)] // display ratio over bounded counts
    let rfpy = (!finished.is_empty())
        .then(|| finished.iter().filter(|a| a.first_pass).count() as f64 / finished.len() as f64);

    // Roll the per-task status tables up.
    let mut totals: BTreeMap<String, (String, i64, usize)> = BTreeMap::new();
    for analysis in analyses {
        for status in &analysis.by_status {
            let entry =
                totals
                    .entry(status.status.clone())
                    .or_insert((status.category.clone(), 0, 0));
            entry.1 = entry.1.saturating_add(status.ms);
            entry.2 += status.entries;
        }
    }
    let mut by_status: Vec<StatusShare> = totals
        .into_iter()
        .map(|(status, (category, ms, entries))| StatusShare {
            category,
            waste: waste_for(&status).map(ToString::to_string),
            status,
            ms,
            days: as_days(ms),
            share: Ratio::new(ms, total_cycle).value,
            entries,
        })
        .collect();
    by_status.sort_by_key(|s| std::cmp::Reverse(s.ms));

    let total_transitions: usize = analyses.iter().map(|a| a.transitions).sum();
    let total_backfilled: usize = analyses.iter().map(|a| a.backfilled).sum();
    #[allow(clippy::cast_precision_loss)] // display ratio over bounded counts
    let backfilled_ratio =
        (total_transitions > 0).then(|| total_backfilled as f64 / total_transitions as f64);

    PlanAnalysis {
        tasks: analyses.len(),
        finished: finished.len(),
        work_in_progress: analyses
            .iter()
            .filter(|a| !a.finished && a.cycle_time_ms.is_some())
            .count(),
        not_started: analyses
            .iter()
            .filter(|a| !a.finished && a.cycle_time_ms.is_none())
            .count(),
        cycle_time: distribution(&cycle_times),
        lead_time: distribution(&lead_times),
        aggregate_flow_efficiency: aggregate,
        median_flow_efficiency: median_ratio,
        waste_shape,
        rolled_first_pass_yield: rfpy,
        rework_count: analyses.iter().map(|a| a.rework_count).sum(),
        by_status,
        backfilled_ratio,
    }
}

// ---------------------------------------------------------------------
// Constraint findings (spec §8)
// ---------------------------------------------------------------------

/// The share of non-value-adding time above which a status is reported
/// as dominating it rather than merely contributing.
pub const STATUS_DOMINANCE: f64 = 0.40;

/// Assignee changes at or above which an item is reported as
/// handoff-heavy.
pub const HANDOFF_HEAVY: usize = 3;

/// One disclosed constraint finding, ordered by time recoverable
/// (spec §8). Every finding names its rule; there is no composite
/// score, and nothing here is per-person (§12.4).
#[derive(Clone, Debug, Serialize)]
pub struct Finding {
    /// The rule that fired.
    pub rule: &'static str,
    /// What the rule is about.
    pub subject: String,
    /// Human explanation, including the threshold that fired.
    pub detail: String,
    /// Time this finding accounts for, milliseconds.
    pub recoverable_ms: i64,
    /// The same in days.
    pub recoverable_days: f64,
}

/// Findings for the per-status ranking (spec §8).
fn status_findings(summary: &PlanAnalysis) -> Vec<Finding> {
    let non_va: i64 = summary
        .by_status
        .iter()
        .filter(|s| s.category != CATEGORY_VALUE_ADDING)
        .fold(0i64, |a, s| a.saturating_add(s.ms));
    summary
        .by_status
        .iter()
        .filter(|s| s.category != CATEGORY_VALUE_ADDING && s.ms > 0)
        .map(|s| {
            #[allow(clippy::cast_precision_loss)] // display ratio
            let share = if non_va > 0 {
                s.ms as f64 / non_va as f64
            } else {
                0.0
            };
            Finding {
                rule: if share >= STATUS_DOMINANCE {
                    "status_dominates_wait"
                } else {
                    "status_wait"
                },
                subject: s.status.clone(),
                detail: format!(
                    "{:.1}% of the plan's non-value-adding time sits in `{}` \
                     (reported as `status_dominates_wait` at {:.0}%)",
                    share * 100.0,
                    s.status,
                    STATUS_DOMINANCE * 100.0
                ),
                recoverable_ms: s.ms,
                recoverable_days: as_days(s.ms),
            }
        })
        .collect()
}

/// Rank the plan's constraints (spec §8).
#[must_use]
pub fn constraints(analyses: &[TaskAnalysis], summary: &PlanAnalysis) -> Vec<Finding> {
    let mut findings = status_findings(summary);

    let blocked = analyses
        .iter()
        .fold(0i64, |a, x| a.saturating_add(x.blocked_time_ms));
    if blocked > 0 {
        findings.push(Finding {
            rule: "blocked_time",
            subject: BLOCKED_STATUS.to_string(),
            detail: "time halted on something external. Separated from the \
                     other waste because it has the shortest path to a fix: \
                     something specific is in the way, and it has a name"
                .to_string(),
            recoverable_ms: blocked,
            recoverable_days: as_days(blocked),
        });
    }

    let dwell = analyses
        .iter()
        .fold(0i64, |a, x| a.saturating_add(x.queue_time_ms));
    if dwell > 0 {
        findings.push(Finding {
            rule: "backlog_dwell",
            subject: BACKLOG_STATUS.to_string(),
            detail: "time between creation and start. A large dwell means work \
                     is being started too early, which is a WIP decision \
                     rather than a capacity one"
                .to_string(),
            recoverable_ms: dwell,
            recoverable_days: as_days(dwell),
        });
    }

    if summary.rework_count > 0 {
        findings.push(Finding {
            rule: "rework",
            subject: format!("{} backwards moves", summary.rework_count),
            detail: format!(
                "rolled first pass yield {}. Throughput rising while this falls \
                 is not going faster — it is shipping work back to yourself",
                summary
                    .rolled_first_pass_yield
                    .map_or_else(|| "n/a".to_string(), |y| format!("{:.0}%", y * 100.0))
            ),
            recoverable_ms: 0,
            recoverable_days: 0.0,
        });
    }

    let heavy = analyses
        .iter()
        .filter(|a| a.handoffs >= HANDOFF_HEAVY)
        .count();
    if heavy > 0 {
        findings.push(Finding {
            rule: "handoff_heavy",
            subject: format!("{heavy} items"),
            detail: format!(
                "items whose assignee changed {HANDOFF_HEAVY} or more times. A \
                 property of the item's journey, never of the people — see \
                 spec §12.4"
            ),
            recoverable_ms: 0,
            recoverable_days: 0.0,
        });
    }

    if let Some(ratio) = summary.backfilled_ratio
        && ratio > 0.0
    {
        findings.push(Finding {
            rule: "backfilled_history",
            subject: format!("{:.0}% of transitions", ratio * 100.0),
            detail: "synthesised by the migration rather than observed, because \
                     the board kept no history before it. These figures firm up \
                     as real moves accumulate"
                .to_string(),
            recoverable_ms: 0,
            recoverable_days: 0.0,
        });
    }

    findings.sort_by_key(|f| std::cmp::Reverse(f.recoverable_ms));
    findings
}

/// One open item ranked against the service level expectation
/// (spec §8 `aging_wip`) — the only finding about work that can still
/// be helped.
#[derive(Clone, Debug, Serialize)]
pub struct AgingItem {
    /// How long since the item started, milliseconds.
    pub age_ms: i64,
    /// The same in days.
    pub age_days: f64,
    /// Whether it is already past the expectation.
    pub past_sle: bool,
    /// Age as a fraction of the expectation.
    pub sle_ratio: Option<f64>,
}

/// Score one open item's age against the expectation.
#[must_use]
pub fn aging(age_ms: i64, sle_ms: Option<i64>) -> AgingItem {
    #[allow(clippy::cast_precision_loss)] // display ratio
    let ratio = sle_ms
        .filter(|ms| *ms > 0)
        .map(|ms| age_ms as f64 / ms as f64);
    AgingItem {
        age_ms,
        age_days: as_days(age_ms),
        past_sle: ratio.is_some_and(|r| r > 1.0),
        sle_ratio: ratio,
    }
}

// ---------------------------------------------------------------------
// Flow analysis (spec §9)
// ---------------------------------------------------------------------

/// Queueing-theory flow over a window (spec §9).
#[derive(Clone, Debug, Serialize)]
pub struct Flow {
    /// The window measured, days.
    pub window_days: i64,
    /// Tasks created in the window.
    pub arrivals: usize,
    /// Tasks finished in the window.
    pub completions: usize,
    /// λ — arrivals per day.
    pub arrival_rate_per_day: Option<f64>,
    /// μ — completions per day (throughput).
    pub throughput_per_day: Option<f64>,
    /// ρ = λ/μ. Near 1, expected wait grows without bound — a team at
    /// 95% utilisation is not 5% from trouble, it is already in it.
    pub utilisation: Option<f64>,
    /// Why ρ is null, when it is.
    pub utilisation_reason: Option<String>,
    /// κ — items started and not finished.
    pub work_in_progress: usize,
    /// τ̂ = κ/μ — the cycle time Little's Law implies for an item
    /// starting now.
    pub implied_cycle_time_days: Option<f64>,
    /// The observed median cycle time of items finished in the window.
    pub observed_p50_cycle_time_days: Option<f64>,
    /// `wip_growing` | `steady_state` | `queue_draining` |
    /// `insufficient_data` (spec §9.1).
    pub interpretation: &'static str,
    /// What the interpretation means, in words.
    pub detail: String,
}

/// Compute flow analysis. `observed_p50_ms` is the median cycle time of
/// items finished in the window, if any finished.
#[must_use]
#[allow(clippy::cast_precision_loss)] // rates over bounded counts
pub fn flow(
    window_days: i64,
    arrivals: usize,
    completions: usize,
    work_in_progress: usize,
    observed_p50_ms: Option<i64>,
) -> Flow {
    let days = window_days.max(1) as f64;
    let lambda = (window_days > 0).then(|| arrivals as f64 / days);
    let mu = (window_days > 0).then(|| completions as f64 / days);

    let (rho, rho_reason) = match (lambda, mu) {
        (Some(_), Some(m)) if m <= 0.0 => (
            None,
            Some(
                "nothing finished in the window, so throughput is zero and \
                 utilisation is undefined — not infinite, and not zero"
                    .to_string(),
            ),
        ),
        (Some(l), Some(m)) => (Some(l / m), None),
        _ => (None, Some("window must be at least one day".to_string())),
    };

    // Little's Law rearranged: cycle time = WIP / throughput.
    let implied = match mu {
        Some(m) if m > 0.0 => Some(work_in_progress as f64 / m),
        _ => None,
    };
    let observed_days = observed_p50_ms.map(as_days);

    let (interpretation, detail) = match (implied, observed_days) {
        (Some(implied_days), Some(obs)) if obs > 0.0 => {
            let ratio = implied_days / obs;
            if ratio > 1.25 {
                (
                    "wip_growing",
                    format!(
                        "Little's Law implies {implied_days:.1} days for an item \
                         starting now, against {obs:.1} days observed for those \
                         just finished. Work in progress is growing faster than \
                         it clears, so recent completions flatter the board. \
                         Lowering the WIP limit shortens cycle time without \
                         anyone working faster."
                    ),
                )
            } else if ratio < 0.80 {
                (
                    "queue_draining",
                    format!(
                        "Little's Law implies {implied_days:.1} days against \
                         {obs:.1} observed. Either work in progress is draining, \
                         or this window's completions were disproportionately \
                         old items being cleared."
                    ),
                )
            } else {
                (
                    "steady_state",
                    format!(
                        "Little's Law implies {implied_days:.1} days against \
                         {obs:.1} observed. Arrivals and departures are close to \
                         balanced, so the observed cycle time is predictive."
                    ),
                )
            }
        }
        _ => (
            "insufficient_data",
            "not enough arrivals or completions in the window to compare the \
             implied cycle time against an observed one"
                .to_string(),
        ),
    };

    Flow {
        window_days,
        arrivals,
        completions,
        arrival_rate_per_day: lambda,
        throughput_per_day: mu,
        utilisation: rho,
        utilisation_reason: rho_reason,
        work_in_progress,
        implied_cycle_time_days: implied,
        observed_p50_cycle_time_days: observed_days,
        interpretation,
        detail,
    }
}

// ---------------------------------------------------------------------
// Cumulative flow (spec §10.2)
// ---------------------------------------------------------------------

/// The most samples a cumulative-flow request may produce, so a wide
/// window cannot become an unbounded response (security invariant 3).
pub const MAX_FLOW_SAMPLES: usize = 400;

/// One task's history, as the cumulative-flow derivation consumes it.
#[derive(Clone, Debug)]
pub struct TaskHistory {
    /// When the task was created, epoch milliseconds.
    pub created_ms: i64,
    /// Its transitions, in any order.
    pub transitions: Vec<Transition>,
}

/// One sample of the board: how many tasks stood in each status at an
/// instant.
#[derive(Clone, Debug, Serialize)]
pub struct FlowSample {
    /// The instant sampled, epoch milliseconds.
    pub at_ms: i64,
    /// Status → count. Every board status is present, including at
    /// zero, so a consumer stacking the bands never has to decide
    /// whether a missing key means zero or means unknown.
    pub counts: BTreeMap<String, usize>,
    /// Tasks that existed by then.
    pub total: usize,
    /// Of those, finished.
    pub done: usize,
    /// Of those, started and not finished — the band a cumulative-flow
    /// diagram's vertical gap represents.
    pub work_in_progress: usize,
}

/// The status one task stood in at `at_ms`, or `None` if it did not
/// exist yet.
///
/// A task that existed but has no transition at or before the instant
/// is reported as `todo` — the same pessimistic treatment of unknown
/// history that [`intervals`] applies, so a backfilled board does not
/// show items appearing from nowhere mid-chart.
#[must_use]
pub fn status_at(history: &TaskHistory, at_ms: i64) -> Option<&str> {
    if at_ms < history.created_ms {
        return None;
    }
    Some(
        history
            .transitions
            .iter()
            .filter(|t| t.at_ms <= at_ms)
            .max_by_key(|t| t.at_ms)
            .map_or(BACKLOG_STATUS, |t| t.to_status.as_str()),
    )
}

/// Derive a cumulative flow diagram: the board's composition sampled at
/// a fixed interval across a window.
///
/// This is the one view here that cannot be assembled client-side — it
/// needs every task's whole history at once, and an API that returned
/// that would be shipping the log to the browser to re-derive what the
/// server already indexes.
///
/// Samples are inclusive of both ends and capped at
/// [`MAX_FLOW_SAMPLES`]; a `step_ms` of zero or less yields no samples
/// rather than looping.
#[must_use]
pub fn cumulative_flow(
    histories: &[TaskHistory],
    from_ms: i64,
    to_ms: i64,
    step_ms: i64,
) -> Vec<FlowSample> {
    if step_ms <= 0 || to_ms < from_ms {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut at = from_ms;
    while at <= to_ms && out.len() < MAX_FLOW_SAMPLES {
        let mut counts: BTreeMap<String, usize> = BOARD_ORDER
            .iter()
            .chain(std::iter::once(&BLOCKED_STATUS))
            .map(|status| ((*status).to_string(), 0))
            .collect();
        let (mut total, mut done, mut wip) = (0usize, 0usize, 0usize);
        for history in histories {
            let Some(status) = status_at(history, at) else {
                continue;
            };
            total += 1;
            *counts.entry(status.to_string()).or_insert(0) += 1;
            if status == FINISHED_STATUS {
                done += 1;
            } else if is_started(status) {
                wip += 1;
            }
        }
        out.push(FlowSample {
            at_ms: at,
            counts,
            total,
            done,
            work_in_progress: wip,
        });
        // The final sample lands exactly on `to_ms` even when the step
        // does not divide the window, so the chart ends at "now" rather
        // than at some point before it.
        if at == to_ms {
            break;
        }
        at = at.saturating_add(step_ms).min(to_ms);
    }
    out
}

// ---------------------------------------------------------------------
// Cross-plan rollup (spec §15 TBA-9)
// ---------------------------------------------------------------------

/// Hard cap on how deep a containment walk goes.
pub const MAX_ROLLUP_DEPTH: usize = 32;

/// Hard cap on how many plans one rollup covers.
pub const MAX_ROLLUP_NODES: usize = 500;

/// One plan reached by a rollup walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RollupNode {
    /// The plan.
    pub pid: uuid::Uuid,
    /// Hops from the root; the root itself is `0`.
    pub depth: usize,
}

/// The result of walking a containment tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RollupWalk {
    /// The root and every descendant reached, breadth-first.
    pub nodes: Vec<RollupNode>,
    /// Whether a cap stopped the walk before it ran out of plans.
    pub truncated: bool,
    /// Plans reached by more than one path, or that pointed back into
    /// the tree. Non-zero means the containment data holds a cycle the
    /// write-path validation should have refused.
    pub revisits: usize,
}

/// Walk a containment tree breadth-first from `root`.
///
/// Pure over an adjacency map, so the caller loads the plans in **one**
/// query and this decides the shape — rather than a query per level,
/// which is the N+1 this shape exists to avoid.
///
/// Three bounds, and the reasons differ:
///
/// - **A visited set**, because a cycle in `parent_pid` would otherwise
///   revisit nodes and expand exponentially, not merely loop. The write
///   path refuses a cycle, but a rollup that *trusts* that is one bulk
///   import or one direct `UPDATE` away from hanging the service — and
///   `revisits` reports it rather than silently absorbing it.
/// - **A depth cap**, matching the existing ancestor walk's posture.
/// - **A node cap**, so one enormous portfolio cannot become an
///   unbounded response.
///
/// `truncated` says a cap fired, because a rollup that quietly covers
/// half an estate reads as if it covered all of it.
#[must_use]
pub fn walk_descendants(
    children: &BTreeMap<uuid::Uuid, Vec<uuid::Uuid>>,
    root: uuid::Uuid,
    max_nodes: usize,
    max_depth: usize,
) -> RollupWalk {
    let mut seen: std::collections::BTreeSet<uuid::Uuid> = std::collections::BTreeSet::new();
    let mut nodes: Vec<RollupNode> = Vec::new();
    let mut queue: std::collections::VecDeque<RollupNode> = std::collections::VecDeque::new();
    let mut revisits = 0usize;
    let mut truncated = false;

    queue.push_back(RollupNode {
        pid: root,
        depth: 0,
    });
    seen.insert(root);

    while let Some(node) = queue.pop_front() {
        if nodes.len() >= max_nodes {
            truncated = true;
            break;
        }
        nodes.push(node);
        if node.depth >= max_depth {
            // Deeper plans exist but are not walked; that is a cap
            // firing, not an exhausted tree.
            if children.get(&node.pid).is_some_and(|kids| !kids.is_empty()) {
                truncated = true;
            }
            continue;
        }
        for child in children.get(&node.pid).into_iter().flatten() {
            if seen.insert(*child) {
                queue.push_back(RollupNode {
                    pid: *child,
                    depth: node.depth + 1,
                });
            } else {
                revisits += 1;
            }
        }
    }
    if !queue.is_empty() {
        truncated = true;
    }

    RollupWalk {
        nodes,
        truncated,
        revisits,
    }
}

// ---------------------------------------------------------------------
// Monte-Carlo delivery forecasting (spec §15 TBA-11)
// ---------------------------------------------------------------------

/// Days in a forecasting period. A week is the conventional unit:
/// shorter buckets are dominated by weekday effects, longer ones give
/// too few samples to draw from.
pub const DEFAULT_PERIOD_DAYS: i64 = 7;

/// Periods of history below which a forecast is refused.
///
/// Six weeks is not a principled number, but forecasting from two is
/// arithmetic on noise, and a confident-looking figure derived from
/// nothing is what discredits the whole method.
pub const MIN_THROUGHPUT_PERIODS: usize = 6;

/// Simulation trials, and the ceiling a caller may ask for.
pub const DEFAULT_TRIALS: usize = 10_000;

/// The most trials a caller may request.
pub const MAX_TRIALS: usize = 100_000;

/// Per-trial period ceiling. A history that contains only zeroes would
/// otherwise accumulate forever; this turns a hang into a stated
/// refusal.
pub const MAX_PERIODS_PER_TRIAL: usize = 520;

/// A deterministic xorshift64\* generator.
///
/// Determinism is a **feature**, not a testing convenience: a forecast
/// that changes every time you reload it is not one anybody will trust
/// or act on. The same history and the same question give the same
/// answer, and the seed is an input so a caller can vary it
/// deliberately.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        // A zero seed would make xorshift emit only zeroes.
        Self(if seed == 0 {
            0x9E37_79B9_7F4A_7C15
        } else {
            seed
        })
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    /// A uniform index into a slice of length `len` (`len > 0`).
    fn index(&mut self, len: usize) -> usize {
        #[allow(clippy::cast_possible_truncation)] // modulo of a bounded len
        {
            (self.next() % len as u64) as usize
        }
    }
}

/// Bucket completion instants into fixed periods, oldest first.
///
/// This is the input a batch forecast actually needs — **not** the
/// cycle-time distribution (see [`forecast_batch`] for why).
#[must_use]
pub fn throughput_history(
    completed_ms: &[i64],
    from_ms: i64,
    to_ms: i64,
    period_ms: i64,
) -> Vec<usize> {
    if period_ms <= 0 || to_ms <= from_ms {
        return Vec::new();
    }
    let span = to_ms.saturating_sub(from_ms);
    // A partial trailing period would understate throughput, so only
    // whole periods are counted.
    let periods = usize::try_from(span / period_ms).unwrap_or(0);
    let mut buckets = vec![0usize; periods];
    for at in completed_ms {
        if *at < from_ms
            || *at
                >= from_ms.saturating_add(
                    i64::try_from(periods)
                        .unwrap_or(0)
                        .saturating_mul(period_ms),
                )
        {
            continue;
        }
        let index = usize::try_from((at - from_ms) / period_ms).unwrap_or(0);
        if let Some(bucket) = buckets.get_mut(index) {
            *bucket += 1;
        }
    }
    buckets
}

/// "How long will these N items take?"
#[derive(Clone, Debug, Serialize)]
pub struct BatchForecast {
    /// Items asked about.
    pub items: usize,
    /// Simulation trials run.
    pub trials: usize,
    /// Periods of history drawn from.
    pub history_periods: usize,
    /// Days per period.
    pub period_days: i64,
    /// The typical outcome — a coin-flip, not a commitment.
    pub p50_days: Option<f64>,
    /// The figure to quote: 85% of trials finished by here.
    pub p85_days: Option<f64>,
    /// The cautious figure.
    pub p95_days: Option<f64>,
    /// Trials that hit the per-trial ceiling without finishing. A
    /// non-zero count means the percentiles above are floors.
    pub trials_hit_ceiling: usize,
    /// Why the forecast is null, when it is.
    pub reason: Option<String>,
    /// Which direction is conservative, stated in the payload so it
    /// cannot be read the wrong way round.
    pub note: &'static str,
}

/// Forecast how long a batch of `items` will take, by sampling the
/// team's own **throughput** history.
///
/// **Throughput, not cycle time — and the difference is the whole
/// point.** The cycle-time distribution answers a question about *one
/// item* ("this will finish within 11 days at 85% confidence"), which
/// is what the service level expectation already reports. Using it for
/// a batch means implicitly assuming items are worked one at a time:
/// sum twenty cycle times for a team running five in parallel and the
/// answer is roughly five times too pessimistic. Throughput sampling
/// makes no such assumption, because parallelism is already baked into
/// how many items the team actually finished per week.
///
/// Each trial draws periods at random **with replacement** from the
/// history and accumulates until the batch is covered. Sampling with
/// replacement is what makes the result a distribution rather than a
/// replay of the past in order.
///
/// For "how long", the **higher** percentile is the more conservative
/// one — the opposite of [`forecast_items`], which is why both carry a
/// `note` saying so.
#[must_use]
pub fn forecast_batch(
    history: &[usize],
    items: usize,
    trials: usize,
    period_days: i64,
    seed: u64,
) -> BatchForecast {
    let trials = trials.clamp(1, MAX_TRIALS);
    let base = BatchForecast {
        items,
        trials,
        history_periods: history.len(),
        period_days,
        p50_days: None,
        p85_days: None,
        p95_days: None,
        trials_hit_ceiling: 0,
        reason: None,
        note: "higher percentile = more conservative: 85% of simulated runs                finished by the p85 figure",
    };

    if history.len() < MIN_THROUGHPUT_PERIODS {
        return BatchForecast {
            reason: Some(format!(
                "only {} periods of throughput history; a forecast needs at least                  {MIN_THROUGHPUT_PERIODS} or it is arithmetic on noise",
                history.len()
            )),
            ..base
        };
    }
    if items == 0 {
        return BatchForecast {
            p50_days: Some(0.0),
            p85_days: Some(0.0),
            p95_days: Some(0.0),
            ..base
        };
    }
    if history.iter().sum::<usize>() == 0 {
        return BatchForecast {
            reason: Some(
                "nothing was completed in the sample window, so there is no                  throughput to forecast from — the honest answer is `never`,                  not a number"
                    .to_string(),
            ),
            ..base
        };
    }

    let mut rng = Rng::new(seed);
    let mut results: Vec<i64> = Vec::with_capacity(trials);
    let mut hit_ceiling = 0usize;
    for _ in 0..trials {
        let mut done = 0usize;
        let mut periods = 0usize;
        while done < items && periods < MAX_PERIODS_PER_TRIAL {
            done = done.saturating_add(history[rng.index(history.len())]);
            periods += 1;
        }
        if done < items {
            hit_ceiling += 1;
        }
        results.push(i64::try_from(periods).unwrap_or(i64::MAX));
    }
    results.sort_unstable();

    #[allow(clippy::cast_precision_loss)] // display figures
    let as_days = |p: f64| percentile(&results, p).map(|periods| (periods * period_days) as f64);
    BatchForecast {
        p50_days: as_days(0.50),
        p85_days: as_days(0.85),
        p95_days: as_days(0.95),
        trials_hit_ceiling: hit_ceiling,
        ..base
    }
}

/// "How many items will we finish in the next N periods?"
#[derive(Clone, Debug, Serialize)]
pub struct ItemsForecast {
    /// Periods asked about.
    pub periods: usize,
    /// The same as days.
    pub days: i64,
    /// Simulation trials run.
    pub trials: usize,
    /// Periods of history drawn from.
    pub history_periods: usize,
    /// The typical outcome — a coin-flip, not a commitment.
    pub median_items: Option<i64>,
    /// **At least** this many, with 85% confidence.
    ///
    /// This is the **15th** percentile of the simulated distribution,
    /// not the 85th. For "how many", the conservative end is the low
    /// one, and quoting the p85 here would promise the best case while
    /// sounding careful.
    pub at_least_items: Option<i64>,
    /// The optimistic end, labelled as such.
    pub at_most_items: Option<i64>,
    /// Why the forecast is null, when it is.
    pub reason: Option<String>,
    /// Which direction is conservative, stated in the payload.
    pub note: &'static str,
}

/// Forecast how many items a period window will deliver.
///
/// The mirror of [`forecast_batch`], and the percentile direction
/// **reverses**: 85% confidence of finishing *at least* N items is the
/// 15th percentile of the distribution, not the 85th. Getting this
/// backwards produces a forecast that reads as cautious and promises
/// the best case, so both figures are named for what they mean rather
/// than for the percentile they came from.
#[must_use]
pub fn forecast_items(
    history: &[usize],
    periods: usize,
    trials: usize,
    period_days: i64,
    seed: u64,
) -> ItemsForecast {
    let trials = trials.clamp(1, MAX_TRIALS);
    let base = ItemsForecast {
        periods,
        days: i64::try_from(periods)
            .unwrap_or(0)
            .saturating_mul(period_days),
        trials,
        history_periods: history.len(),
        median_items: None,
        at_least_items: None,
        at_most_items: None,
        reason: None,
        note: "at_least_items is the 15th percentile: for `how many`, the                conservative end is the low one",
    };

    if history.len() < MIN_THROUGHPUT_PERIODS {
        return ItemsForecast {
            reason: Some(format!(
                "only {} periods of throughput history; a forecast needs at least                  {MIN_THROUGHPUT_PERIODS} or it is arithmetic on noise",
                history.len()
            )),
            ..base
        };
    }
    if periods == 0 || periods > MAX_PERIODS_PER_TRIAL {
        return ItemsForecast {
            reason: Some(format!(
                "periods must be between 1 and {MAX_PERIODS_PER_TRIAL}"
            )),
            ..base
        };
    }

    let mut rng = Rng::new(seed);
    let mut results: Vec<i64> = Vec::with_capacity(trials);
    for _ in 0..trials {
        let mut done = 0usize;
        for _ in 0..periods {
            done = done.saturating_add(history[rng.index(history.len())]);
        }
        results.push(i64::try_from(done).unwrap_or(i64::MAX));
    }
    results.sort_unstable();

    ItemsForecast {
        median_items: percentile(&results, 0.50),
        at_least_items: percentile(&results, 0.15),
        at_most_items: percentile(&results, 0.85),
        ..base
    }
}

// ---------------------------------------------------------------------
// Flow gauges (spec §15 TBA-10)
// ---------------------------------------------------------------------

/// Default cap on how many plans are exported as individual gauge
/// series. Per-plan labels are unbounded cardinality, and a metric that
/// takes the monitoring down is worse than no metric.
pub const DEFAULT_METRIC_MAX_PLANS: usize = 50;

/// Default minimum task count for a plan to be labelled at all.
pub const DEFAULT_METRIC_MIN_TASKS: usize = 5;

/// One plan's figures, as the exporter consumes them.
#[derive(Clone, Debug)]
pub struct PlanFlowSample {
    /// The plan's public id.
    pub plan_pid: String,
    /// Its rollup.
    pub analysis: PlanAnalysis,
    /// Its service level expectation.
    pub sle: ServiceLevelExpectation,
    /// How many board columns are over their configured WIP cap.
    pub columns_over_limit: usize,
}

/// One plan's flow figures, ready to be written to a labelled gauge.
#[derive(Clone, Debug, PartialEq)]
pub struct FlowMetricRow {
    /// The plan's public id — the gauge label.
    pub plan_pid: String,
    /// Tasks behind the figures.
    pub tasks: usize,
    /// Started and not finished (κ).
    pub work_in_progress: usize,
    /// Aggregate work-over-cycle ratio.
    pub flow_efficiency: Option<f64>,
    /// p85 cycle time in days. `None` below the SLE's own minimum
    /// sample, so the gauge inherits the refusal to forecast from noise
    /// rather than re-deciding it.
    pub cycle_time_p85_days: Option<f64>,
    /// Share of finished items that never moved backwards.
    pub rolled_first_pass_yield: Option<f64>,
    /// Board columns over their configured cap.
    pub columns_over_limit: usize,
}

/// What an export pass decided to publish, and what it held back.
#[derive(Clone, Debug, PartialEq)]
pub struct FlowMetricSet {
    /// The plans exported as their own series, largest board first.
    pub rows: Vec<FlowMetricRow>,
    /// Plans withheld for having too few tasks to aggregate.
    pub suppressed_plans: usize,
    /// Plans dropped because the series cap was reached.
    pub dropped_plans: usize,
}

/// Choose which plans get their own gauge series.
///
/// Two bounds, and neither is silent — both counts ship alongside the
/// rows, because a cap nobody can see reads as "we measured everything".
///
/// 1. **Small boards are suppressed.** A flow efficiency over two tasks
///    describes two people's week, and §12.4 refuses per-person
///    measurement; a label that reaches it by arithmetic is the same
///    thing through a side door. `/metrics.prom` is on the public
///    allow-list, so it stays scrapeable with
///    `PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH` on.
/// 2. **The series count is capped**, largest board first.
///
/// The label is the **pid**, never the name: a rename would fork the
/// series and silently reset its history.
///
/// **Per-column occupancy is deliberately not exported.** It would be
/// the most useful detail and also five series per plan — the single
/// biggest cardinality contributor here. The over-cap *count* carries
/// the alertable fact ("a column on this plan is over its limit") in
/// one series, and the detail is one API call away.
#[must_use]
pub fn flow_metric_rows(
    samples: &[PlanFlowSample],
    max_plans: usize,
    min_tasks: usize,
) -> FlowMetricSet {
    let mut eligible: Vec<&PlanFlowSample> = Vec::new();
    let mut suppressed = 0usize;
    for sample in samples {
        if sample.analysis.tasks == 0 {
            continue;
        }
        if sample.analysis.tasks < min_tasks {
            suppressed += 1;
        } else {
            eligible.push(sample);
        }
    }
    // Largest board first; the pid breaks ties so a pass is stable and a
    // dashboard does not reshuffle between scrapes.
    eligible.sort_by_key(|sample| {
        (
            std::cmp::Reverse(sample.analysis.tasks),
            sample.plan_pid.clone(),
        )
    });
    let dropped = eligible.len().saturating_sub(max_plans);
    eligible.truncate(max_plans);

    FlowMetricSet {
        rows: eligible
            .into_iter()
            .map(|sample| FlowMetricRow {
                plan_pid: sample.plan_pid.clone(),
                tasks: sample.analysis.tasks,
                work_in_progress: sample.analysis.work_in_progress,
                flow_efficiency: sample.analysis.aggregate_flow_efficiency.value,
                cycle_time_p85_days: sample.sle.within_days,
                rolled_first_pass_yield: sample.analysis.rolled_first_pass_yield,
                columns_over_limit: sample.columns_over_limit,
            })
            .collect(),
        suppressed_plans: suppressed,
        dropped_plans: dropped,
    }
}

// ---------------------------------------------------------------------
// Persistence seam
// ---------------------------------------------------------------------

/// Build the `task_transitions` row for one status change.
///
/// Kept here rather than in the controller so the two call sites — task
/// creation and the board move — cannot drift, and so the log's shape
/// is defined next to the analysis that reads it. The caller inserts it
/// **inside the same transaction as the change that caused it**
/// (spec §5.1 invariant 3): a committed move without its transition
/// would silently shorten the item's recorded life, and nothing
/// downstream could tell.
#[must_use]
pub fn transition_row(
    task_pid: uuid::Uuid,
    plan_pid: uuid::Uuid,
    from_status: Option<String>,
    to_status: String,
    at: chrono::DateTime<chrono::Utc>,
    actor_ref: Option<String>,
    assignee_ref: Option<String>,
) -> crate::models::_entities::task_transitions::ActiveModel {
    use sea_orm::ActiveValue;
    crate::models::_entities::task_transitions::ActiveModel {
        pid: ActiveValue::set(uuid::Uuid::new_v4()),
        task_pid: ActiveValue::set(task_pid),
        plan_pid: ActiveValue::set(plan_pid),
        from_status: ActiveValue::set(from_status),
        to_status: ActiveValue::set(to_status),
        at: ActiveValue::set(at.into()),
        actor_ref: ActiveValue::set(actor_ref),
        assignee_ref: ActiveValue::set(assignee_ref),
        backfilled: ActiveValue::set(false),
        ..Default::default()
    }
}

/// A stored transition row → the pure analysis input.
#[must_use]
pub fn to_transition(row: &crate::models::_entities::task_transitions::Model) -> Transition {
    Transition {
        from_status: row.from_status.clone(),
        to_status: row.to_status.clone(),
        at_ms: row.at.timestamp_millis(),
        assignee_ref: row.assignee_ref.clone(),
        backfilled: row.backfilled,
    }
}

/// The classification map in force: the deployment override where it
/// parses whole, else the disclosed default (spec §5.3).
#[must_use]
pub fn classes_in_force() -> BTreeMap<String, String> {
    parse_classes(
        std::env::var("PROJECT_PORTFOLIO_MANAGEMENT_FLOW_CLASSES")
            .ok()
            .as_deref(),
    )
    .unwrap_or_else(default_classes)
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: i64 = 1_700_000_000_000;

    fn at(days: i64) -> i64 {
        T0 + days * DAY_MS
    }

    fn tr(from: Option<&str>, to: &str, day: i64) -> Transition {
        Transition {
            from_status: from.map(ToString::to_string),
            to_status: to.to_string(),
            at_ms: at(day),
            assignee_ref: None,
            backfilled: false,
        }
    }

    fn classes() -> BTreeMap<String, String> {
        default_classes()
    }

    // -- the cycle-versus-lead-time distinction (§6.1) --------------------

    #[test]
    fn cycle_time_and_lead_time_are_different_numbers() {
        // Created day 0, sat in `todo` for 21 days, built in 2.
        // Cycle 2, lead 23. Quoting the cycle time as the delivery time
        // is a tenfold flattering misreport — this is the regression
        // test against ever conflating them.
        let transitions = vec![
            tr(None, "todo", 0),
            tr(Some("todo"), "in_progress", 21),
            tr(Some("in_progress"), "done", 23),
        ];
        let clock = TaskClock {
            created_ms: at(0),
            done_ms: Some(at(23)),
        };
        let a = analyze(&transitions, clock, &classes(), at(23));
        assert_eq!(a.lead_time_ms, 23 * DAY_MS);
        assert_eq!(a.cycle_time_ms, Some(2 * DAY_MS));
        assert_eq!(
            a.queue_time_ms,
            21 * DAY_MS,
            "the backlog dwell is reported"
        );
        assert!(a.finished);
    }

    #[test]
    fn a_task_that_never_started_has_no_cycle_time() {
        let transitions = vec![tr(None, "todo", 0)];
        let clock = TaskClock {
            created_ms: at(0),
            done_ms: None,
        };
        let a = analyze(&transitions, clock, &classes(), at(10));
        assert_eq!(a.cycle_time_ms, None);
        assert!(a.cycle_time_reason.is_some(), "a null must say why");
        assert_eq!(
            a.lead_time_ms,
            10 * DAY_MS,
            "but it has been waiting 10 days"
        );
        assert_eq!(
            a.age_ms, None,
            "age is measured from the start, and it has none"
        );
        assert_eq!(a.flow_efficiency.value, None);
    }

    // -- the partition (§6.3, §12.3) --------------------------------------

    #[test]
    fn statuses_partition_the_cycle_time() {
        let transitions = vec![
            tr(None, "todo", 0),
            tr(Some("todo"), "in_progress", 10),
            tr(Some("in_progress"), "in_review", 12),
            tr(Some("in_review"), "blocked", 15),
            tr(Some("blocked"), "in_progress", 20),
            tr(Some("in_progress"), "done", 21),
        ];
        let clock = TaskClock {
            created_ms: at(0),
            done_ms: Some(at(21)),
        };
        let a = analyze(&transitions, clock, &classes(), at(21));
        let cycle = a.cycle_time_ms.expect("cycle time");
        assert_eq!(cycle, 11 * DAY_MS, "day 10 → 21");
        assert_eq!(a.lead_time_ms, 21 * DAY_MS, "creation → done");
        // The statuses partition the **lead** time, not the cycle time:
        // the 10 days in the backlog are real time the requester waited
        // and have to land somewhere (§6.3).
        let status_total: i64 = a.by_status.iter().map(|s| s.ms).sum();
        assert_eq!(status_total, a.lead_time_ms, "statuses sum to lead time");
        let category_total: i64 = a.by_category.iter().map(|c| c.ms).sum();
        assert_eq!(category_total, a.lead_time_ms, "categories sum too");
        assert_eq!(a.queue_time_ms, 10 * DAY_MS, "and the dwell is named");
        // in_progress: day 10-12 and 20-21 = 3 days of the 11.
        assert_eq!(a.work_time_ms, 3 * DAY_MS);
        assert_eq!(a.blocked_time_ms, 5 * DAY_MS, "day 15 → 20");
        let efficiency = a.flow_efficiency.value.expect("efficiency");
        assert!((efficiency - 3.0 / 11.0).abs() < 1e-9, "got {efficiency}");
        assert!((0.0..=1.0).contains(&efficiency));
    }

    #[test]
    fn done_is_terminal_and_does_not_accrue() {
        let transitions = vec![
            tr(None, "todo", 0),
            tr(Some("todo"), "in_progress", 1),
            tr(Some("in_progress"), "done", 3),
        ];
        let clock = TaskClock {
            created_ms: at(0),
            done_ms: Some(at(3)),
        };
        // Analysed a hundred days later: the clock must not have run on.
        let a = analyze(&transitions, clock, &classes(), at(103));
        assert_eq!(a.cycle_time_ms, Some(2 * DAY_MS));
        assert!(
            !a.by_status.iter().any(|s| s.status == FINISHED_STATUS),
            "`done` is not an interval"
        );
    }

    #[test]
    fn an_open_item_accrues_to_as_of_and_reports_its_age() {
        let transitions = vec![tr(None, "todo", 0), tr(Some("todo"), "in_progress", 2)];
        let clock = TaskClock {
            created_ms: at(0),
            done_ms: None,
        };
        let a = analyze(&transitions, clock, &classes(), at(9));
        assert!(!a.finished);
        assert_eq!(a.cycle_time_ms, Some(7 * DAY_MS), "day 2 → now");
        assert_eq!(a.age_ms, Some(7 * DAY_MS));
        assert_eq!(a.work_time_ms, 7 * DAY_MS, "still in progress");
    }

    // -- classification ---------------------------------------------------

    #[test]
    fn the_default_classification_is_the_disclosed_one() {
        let c = default_classes();
        assert_eq!(classify(&c, "in_progress"), CATEGORY_VALUE_ADDING);
        assert_eq!(classify(&c, "in_review"), CATEGORY_NECESSARY);
        assert_eq!(classify(&c, BACKLOG_STATUS), CATEGORY_UNNECESSARY);
        assert_eq!(classify(&c, BLOCKED_STATUS), CATEGORY_UNNECESSARY);
        assert_eq!(
            classify(&c, "something_new"),
            CATEGORY_UNNECESSARY,
            "an unclassified status counts against you, so adding a board \
             column cannot silently improve the flow efficiency"
        );
        assert_eq!(waste_for(BACKLOG_STATUS), Some("inventory"));
        assert_eq!(waste_for(BLOCKED_STATUS), Some("waiting"));
        assert_eq!(waste_for("in_progress"), None);
    }

    #[test]
    fn an_override_applies_whole_or_not_at_all() {
        let good = parse_classes(Some(r#"{"in_review":"value_adding"}"#)).expect("parsed");
        assert_eq!(classify(&good, "in_review"), CATEGORY_VALUE_ADDING);
        // An unknown category invalidates the whole override rather than
        // half-applying it — a figure matching no stated classification
        // is worse than one matching the documented default.
        assert_eq!(parse_classes(Some(r#"{"in_review":"lovely"}"#)), None);
        assert_eq!(parse_classes(Some("not json")), None);
        assert_eq!(parse_classes(Some("  ")), None);
        assert_eq!(parse_classes(Some("{}")), None);
        assert_eq!(parse_classes(None), None);
    }

    // -- rework (§6.5) ----------------------------------------------------

    #[test]
    fn backwards_moves_are_rework_but_blocking_is_not() {
        assert!(is_backwards("in_review", "in_progress"));
        assert!(is_backwards("done", "todo"));
        assert!(!is_backwards("todo", "in_progress"), "forwards");
        assert!(!is_backwards("in_progress", "in_progress"), "no move");
        assert!(
            !is_backwards("in_progress", BLOCKED_STATUS),
            "blocking is orthogonal to progress"
        );
        assert!(
            !is_backwards(BLOCKED_STATUS, "in_progress"),
            "and so is unblocking"
        );
        assert!(!is_backwards("mystery", "in_progress"), "unknown status");
    }

    #[test]
    fn rework_and_first_pass_are_counted_per_item() {
        let clean = vec![
            tr(None, "todo", 0),
            tr(Some("todo"), "in_progress", 1),
            tr(Some("in_progress"), "done", 2),
        ];
        let clock = TaskClock {
            created_ms: at(0),
            done_ms: Some(at(2)),
        };
        let a = analyze(&clean, clock, &classes(), at(2));
        assert_eq!(a.rework_count, 0);
        assert!(a.first_pass);

        let bounced = vec![
            tr(None, "todo", 0),
            tr(Some("todo"), "in_progress", 1),
            tr(Some("in_progress"), "in_review", 2),
            tr(Some("in_review"), "in_progress", 3),
            tr(Some("in_progress"), "in_review", 4),
            tr(Some("in_review"), "done", 5),
        ];
        let clock = TaskClock {
            created_ms: at(0),
            done_ms: Some(at(5)),
        };
        let b = analyze(&bounced, clock, &classes(), at(5));
        assert_eq!(b.rework_count, 1);
        assert!(!b.first_pass);
    }

    // -- handoffs ---------------------------------------------------------

    #[test]
    fn handoffs_count_assignee_changes_not_people() {
        let mut transitions = vec![
            tr(None, "todo", 0),
            tr(Some("todo"), "in_progress", 1),
            tr(Some("in_progress"), "in_review", 2),
        ];
        transitions[1].assignee_ref = Some("worker:a".to_string());
        transitions[2].assignee_ref = Some("worker:b".to_string());
        let clock = TaskClock {
            created_ms: at(0),
            done_ms: None,
        };
        let a = analyze(&transitions, clock, &classes(), at(3));
        assert_eq!(a.handoffs, 2, "none → a, a → b");
        assert_eq!(a.distinct_assignees, 2);
    }

    // -- degenerate input -------------------------------------------------

    #[test]
    fn degenerate_input_yields_stated_nulls_not_panics() {
        let clock = TaskClock {
            created_ms: at(0),
            done_ms: None,
        };
        // No transitions at all: five days of unknown history, charged
        // to `todo` rather than left unattributed, so it cannot be lost
        // from a report.
        let empty = analyze(&[], clock, &classes(), at(5));
        assert_eq!(empty.cycle_time_ms, None);
        assert_eq!(empty.transitions, 0);
        assert_eq!(empty.queue_time_ms, 5 * DAY_MS);
        assert_eq!(empty.lead_time_ms, 5 * DAY_MS);

        // `as_of` before the last transition (clock skew): a zero-length
        // final interval, never a negative one.
        let skewed = vec![tr(None, "todo", 0), tr(Some("todo"), "in_progress", 10)];
        let a = analyze(&skewed, clock, &classes(), at(3));
        assert!(a.by_status.iter().all(|s| s.ms >= 0));
        assert_eq!(a.lead_time_ms, 3 * DAY_MS);
        assert_eq!(
            a.cycle_time_ms, None,
            "as of day 3 the day-10 move has not happened, so the item has \
             not started — a future transition must not retroactively start \
             the clock"
        );

        // Out-of-order input is sorted, not trusted.
        let jumbled = vec![
            tr(Some("in_progress"), "done", 5),
            tr(None, "todo", 0),
            tr(Some("todo"), "in_progress", 2),
        ];
        let clock = TaskClock {
            created_ms: at(0),
            done_ms: Some(at(5)),
        };
        let b = analyze(&jumbled, clock, &classes(), at(5));
        assert_eq!(b.cycle_time_ms, Some(3 * DAY_MS), "day 2 → 5");

        // Duplicate timestamps.
        let duplicated = vec![
            tr(None, "todo", 0),
            tr(Some("todo"), "in_progress", 1),
            tr(Some("in_progress"), "in_review", 1),
        ];
        let clock = TaskClock {
            created_ms: at(0),
            done_ms: None,
        };
        let c = analyze(&duplicated, clock, &classes(), at(4));
        let total: i64 = c.by_status.iter().map(|s| s.ms).sum();
        assert_eq!(total, c.lead_time_ms);
    }

    // -- percentiles and the SLE ------------------------------------------

    #[test]
    fn nearest_rank_percentiles_return_observed_values() {
        let sample = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        assert_eq!(percentile(&sample, 0.5), Some(5));
        assert_eq!(percentile(&sample, 0.85), Some(9));
        assert_eq!(percentile(&sample, 1.0), Some(10));
        assert_eq!(percentile(&sample, 0.0), Some(1));
        assert_eq!(percentile(&[], 0.5), None);
        assert_eq!(percentile(&[7], 0.85), Some(7));
        assert_eq!(percentile(&sample, 1.5), None);
        assert_eq!(percentile(&sample, f64::NAN), None);
        for p in [0.0, 0.5, 0.85, 0.95, 1.0] {
            let v = percentile(&sample, p).expect("value");
            assert!(sample.contains(&v), "every percentile is an observation");
        }
    }

    #[test]
    fn the_sle_refuses_to_forecast_from_noise() {
        let few: Vec<i64> = (1..=5).map(|d| d * DAY_MS).collect();
        let thin = service_level_expectation(&few, 0.85, None);
        assert_eq!(thin.within_ms, None);
        assert!(thin.reason.is_some(), "must say why, not return a number");
        assert_eq!(thin.sample, 5);

        let enough: Vec<i64> = (1..=20).map(|d| d * DAY_MS).collect();
        let sle = service_level_expectation(&enough, 0.85, None);
        assert_eq!(sle.within_ms, Some(17 * DAY_MS), "p85 of 1..=20");
        assert_eq!(sle.reason, None);
    }

    #[test]
    fn the_sle_scores_an_explicit_commitment() {
        let cycles: Vec<i64> = (1..=20).map(|d| d * DAY_MS).collect();
        let met = service_level_expectation(&cycles, 0.85, Some(20.0));
        assert_eq!(met.target_achieved_ratio, Some(1.0));
        assert_eq!(met.target_met, Some(true));
        let missed = service_level_expectation(&cycles, 0.85, Some(5.0));
        assert_eq!(missed.target_achieved_ratio, Some(0.25));
        assert_eq!(missed.target_met, Some(false));
        let none = service_level_expectation(&cycles, 0.85, None);
        assert_eq!(none.target_met, None, "no commitment, no verdict");
    }

    #[test]
    fn distribution_reports_the_shape_and_names_its_method() {
        let d = distribution(&[5, 1, 3, 2, 4]).expect("distribution");
        assert_eq!((d.n, d.min_ms, d.max_ms, d.p50_ms), (5, 1, 5, 3));
        assert_eq!(d.method, "nearest_rank");
        assert!(distribution(&[]).is_none());
    }

    // -- plan rollup ------------------------------------------------------

    fn finished_task(queue_days: i64, work_days: i64, bounced: bool) -> TaskAnalysis {
        let mut transitions = vec![
            tr(None, "todo", 0),
            tr(Some("todo"), "in_progress", queue_days),
        ];
        let mut day = queue_days + work_days;
        if bounced {
            transitions.push(tr(Some("in_progress"), "in_review", day));
            transitions.push(tr(Some("in_review"), "in_progress", day + 1));
            day += 2;
        }
        transitions.push(tr(Some("in_progress"), "done", day));
        analyze(
            &transitions,
            TaskClock {
                created_ms: at(0),
                done_ms: Some(at(day)),
            },
            &classes(),
            at(day),
        )
    }

    #[test]
    fn the_plan_rollup_reports_throughput_beside_first_pass_yield() {
        let analyses: Vec<TaskAnalysis> = (0..8).map(|i| finished_task(3, 2, i % 4 == 0)).collect();
        let p = plan(&analyses);
        assert_eq!(p.tasks, 8);
        assert_eq!(p.finished, 8);
        assert_eq!(p.rework_count, 2, "two of eight bounced");
        let rfpy = p.rolled_first_pass_yield.expect("rfpy");
        assert!((rfpy - 0.75).abs() < 1e-9, "got {rfpy}");
        assert!(p.cycle_time.is_some() && p.lead_time.is_some());
        // Lead time must always be reported beside cycle time, so the
        // flattering number cannot travel alone (§12.3).
        let cycle = p.cycle_time.as_ref().expect("cycle");
        let lead = p.lead_time.as_ref().expect("lead");
        assert!(
            lead.p50_ms > cycle.p50_ms,
            "the backlog dwell is in the lead time"
        );
    }

    /// One item that started, blocked for nearly a year, and finished:
    /// a 300-day cycle with 2 days of work in it.
    fn long_stalled_task() -> TaskAnalysis {
        let transitions = vec![
            tr(None, "todo", 0),
            tr(Some("todo"), "in_progress", 1),
            tr(Some("in_progress"), BLOCKED_STATUS, 2),
            tr(Some(BLOCKED_STATUS), "in_progress", 300),
            tr(Some("in_progress"), "done", 301),
        ];
        analyze(
            &transitions,
            TaskClock {
                created_ms: at(0),
                done_ms: Some(at(301)),
            },
            &classes(),
            at(301),
        )
    }

    #[test]
    fn the_plan_rollup_separates_the_typical_item_from_the_system_ratio() {
        // Nine quick items and one that sat for a year: the aggregate is
        // dragged down, the median is not, and the divergence is itself
        // the finding (§7.2).
        let mut analyses: Vec<TaskAnalysis> = (0..9).map(|_| finished_task(0, 5, false)).collect();
        analyses.push(long_stalled_task());
        let p = plan(&analyses);
        assert_eq!(p.waste_shape, "concentrated");
        let agg = p.aggregate_flow_efficiency.value.expect("aggregate");
        let med = p.median_flow_efficiency.expect("median");
        assert!(agg < med, "the long item drags the aggregate down");
    }

    #[test]
    fn an_empty_plan_is_null_not_zero() {
        let p = plan(&[]);
        assert_eq!(p.tasks, 0);
        assert!(p.cycle_time.is_none());
        assert_eq!(p.aggregate_flow_efficiency.value, None);
        assert_eq!(p.rolled_first_pass_yield, None);
        assert_eq!(p.waste_shape, "insufficient_data");
        assert!(constraints(&[], &p).is_empty());
    }

    #[test]
    fn constraints_rank_by_recoverable_time_and_name_their_rule() {
        let analyses = vec![finished_task(30, 2, true)];
        let p = plan(&analyses);
        let findings = constraints(&analyses, &p);
        assert!(!findings.is_empty());
        for pair in findings.windows(2) {
            assert!(pair[0].recoverable_ms >= pair[1].recoverable_ms);
        }
        assert!(findings.iter().any(|f| f.rule == "backlog_dwell"));
        assert!(findings.iter().any(|f| f.rule == "rework"));
        assert!(findings.iter().all(|f| !f.rule.is_empty()));
    }

    #[test]
    fn aging_scores_an_open_item_against_the_expectation() {
        let past = aging(20 * DAY_MS, Some(11 * DAY_MS));
        assert!(past.past_sle);
        assert!(past.sle_ratio.unwrap_or(0.0) > 1.0);
        let within = aging(3 * DAY_MS, Some(11 * DAY_MS));
        assert!(!within.past_sle);
        let unknown = aging(3 * DAY_MS, None);
        assert_eq!(unknown.sle_ratio, None);
        assert!(!unknown.past_sle, "no expectation means no breach");
    }

    // -- flow -------------------------------------------------------------

    #[test]
    fn littles_law_labels_the_three_regimes() {
        // κ=100, μ=1/day ⇒ τ̂=100 days, against 10 observed.
        let growing = flow(100, 200, 100, 100, Some(10 * DAY_MS));
        assert_eq!(growing.interpretation, "wip_growing");
        assert!((growing.implied_cycle_time_days.unwrap_or(0.0) - 100.0).abs() < 1e-9);

        let steady = flow(100, 100, 100, 10, Some(10 * DAY_MS));
        assert_eq!(steady.interpretation, "steady_state");

        let draining = flow(100, 100, 100, 1, Some(10 * DAY_MS));
        assert_eq!(draining.interpretation, "queue_draining");
    }

    #[test]
    fn flow_refuses_to_invent_numbers_it_cannot_have() {
        let nothing_done = flow(30, 10, 0, 10, None);
        assert_eq!(nothing_done.utilisation, None);
        assert!(nothing_done.utilisation_reason.is_some());
        assert_eq!(nothing_done.implied_cycle_time_days, None);
        assert_eq!(nothing_done.interpretation, "insufficient_data");

        let zero_window = flow(0, 5, 5, 5, Some(DAY_MS));
        assert_eq!(zero_window.arrival_rate_per_day, None);
        assert_eq!(zero_window.utilisation, None);
    }

    #[test]
    fn utilisation_is_demand_over_capacity() {
        let f = flow(10, 20, 10, 5, Some(DAY_MS));
        assert_eq!(f.arrival_rate_per_day, Some(2.0));
        assert_eq!(f.throughput_per_day, Some(1.0));
        assert_eq!(
            f.utilisation,
            Some(2.0),
            "twice as much arriving as leaving"
        );
    }

    // -- cumulative flow --------------------------------------------------

    #[test]
    fn cumulative_flow_tracks_the_board_over_time() {
        // One task created day 0, started day 2, finished day 4.
        let history = TaskHistory {
            created_ms: at(0),
            transitions: vec![
                tr(None, "todo", 0),
                tr(Some("todo"), "in_progress", 2),
                tr(Some("in_progress"), "done", 4),
            ],
        };
        let samples = cumulative_flow(&[history], at(0), at(5), DAY_MS);
        assert_eq!(samples.len(), 6, "days 0..=5 inclusive");
        let count = |day: usize, status: &str| samples[day].counts[status];
        assert_eq!(count(0, "todo"), 1);
        assert_eq!(count(1, "todo"), 1);
        assert_eq!(count(2, "in_progress"), 1);
        assert_eq!(count(3, "in_progress"), 1);
        assert_eq!(count(4, "done"), 1);
        assert_eq!(count(5, "done"), 1, "it stays done");
        assert_eq!(samples[3].work_in_progress, 1);
        assert_eq!(samples[4].work_in_progress, 0);
        assert_eq!(samples[4].done, 1);
        // Every band is present at every sample, including at zero, so a
        // stacked chart never has to guess whether a gap means zero.
        for sample in &samples {
            for status in BOARD_ORDER.iter().chain(std::iter::once(&BLOCKED_STATUS)) {
                assert!(sample.counts.contains_key(*status), "missing {status}");
            }
            let banded: usize = sample.counts.values().sum();
            assert_eq!(banded, sample.total, "bands sum to the total");
        }
    }

    #[test]
    fn a_task_does_not_appear_before_it_existed() {
        let history = TaskHistory {
            created_ms: at(3),
            transitions: vec![tr(None, "todo", 3)],
        };
        let samples = cumulative_flow(&[history], at(0), at(5), DAY_MS);
        assert_eq!(samples[0].total, 0, "not created yet");
        assert_eq!(samples[2].total, 0);
        assert_eq!(samples[3].total, 1, "created on day 3");
        assert_eq!(samples[5].total, 1);
    }

    #[test]
    fn a_task_with_no_transitions_reads_as_backlog_not_as_absent() {
        // A backfilled board can hold a task whose only transition is
        // later than an early sample. It must not vanish from the chart
        // and reappear — it was somewhere, and `todo` is the
        // pessimistic guess.
        let history = TaskHistory {
            created_ms: at(0),
            transitions: vec![tr(None, "in_progress", 4)],
        };
        let samples = cumulative_flow(&[history], at(0), at(5), DAY_MS);
        assert_eq!(samples[0].counts[BACKLOG_STATUS], 1);
        assert_eq!(samples[3].counts[BACKLOG_STATUS], 1);
        assert_eq!(samples[4].counts["in_progress"], 1);
        assert!(samples.iter().all(|s| s.total == 1), "never disappears");
    }

    #[test]
    fn cumulative_flow_is_bounded_and_ends_on_the_window() {
        // A step that does not divide the window still lands its last
        // sample exactly on `to_ms`, so the chart ends at "now".
        let history = TaskHistory {
            created_ms: at(0),
            transitions: vec![tr(None, "todo", 0)],
        };
        let samples = cumulative_flow(std::slice::from_ref(&history), at(0), at(10), 3 * DAY_MS);
        assert_eq!(samples.last().expect("last").at_ms, at(10));

        // Degenerate inputs yield nothing rather than looping forever.
        assert!(cumulative_flow(std::slice::from_ref(&history), at(0), at(5), 0).is_empty());
        assert!(cumulative_flow(std::slice::from_ref(&history), at(0), at(5), -1).is_empty());
        assert!(cumulative_flow(std::slice::from_ref(&history), at(5), at(0), DAY_MS).is_empty());
        assert!(cumulative_flow(&[], at(0), at(5), DAY_MS).len() == 6);

        // A very fine step over a wide window is capped, not unbounded.
        let capped = cumulative_flow(&[history], at(0), at(100_000), 1);
        assert_eq!(capped.len(), MAX_FLOW_SAMPLES);
    }

    #[test]
    fn status_at_answers_the_three_cases() {
        let history = TaskHistory {
            created_ms: at(1),
            transitions: vec![tr(None, "todo", 1), tr(Some("todo"), "in_progress", 3)],
        };
        assert_eq!(status_at(&history, at(0)), None, "before creation");
        assert_eq!(status_at(&history, at(1)), Some("todo"));
        assert_eq!(status_at(&history, at(2)), Some("todo"), "between moves");
        assert_eq!(
            status_at(&history, at(3)),
            Some("in_progress"),
            "on the move"
        );
        assert_eq!(status_at(&history, at(9)), Some("in_progress"), "after");
    }

    // -- cross-plan rollup (§15 TBA-9) -------------------------------------

    /// A uuid from a small integer, so tree tests read legibly.
    fn node(n: u8) -> uuid::Uuid {
        uuid::Uuid::from_bytes([n; 16])
    }

    /// Build a parent → children map from `(parent, child)` pairs.
    fn tree(pairs: &[(u8, u8)]) -> BTreeMap<uuid::Uuid, Vec<uuid::Uuid>> {
        let mut map: BTreeMap<uuid::Uuid, Vec<uuid::Uuid>> = BTreeMap::new();
        for (parent, child) in pairs {
            map.entry(node(*parent)).or_default().push(node(*child));
        }
        map
    }

    #[test]
    fn the_walk_covers_the_tree_breadth_first() {
        // 1 ─┬─ 2 ─── 4
        //    └─ 3
        let children = tree(&[(1, 2), (1, 3), (2, 4)]);
        let walk = walk_descendants(&children, node(1), 500, 32);
        assert_eq!(
            walk.nodes,
            vec![
                RollupNode {
                    pid: node(1),
                    depth: 0
                },
                RollupNode {
                    pid: node(2),
                    depth: 1
                },
                RollupNode {
                    pid: node(3),
                    depth: 1
                },
                RollupNode {
                    pid: node(4),
                    depth: 2
                },
            ]
        );
        assert!(!walk.truncated);
        assert_eq!(walk.revisits, 0);
    }

    #[test]
    fn a_leaf_rolls_up_to_itself() {
        let walk = walk_descendants(&BTreeMap::new(), node(1), 500, 32);
        assert_eq!(
            walk.nodes,
            vec![RollupNode {
                pid: node(1),
                depth: 0
            }]
        );
        assert!(!walk.truncated);
    }

    #[test]
    fn a_cycle_terminates_and_is_reported() {
        // 1 → 2 → 3 → 1. The write path refuses this, but a rollup that
        // trusts that is one direct UPDATE away from hanging the
        // service — and it must say what it found rather than absorbing
        // it silently.
        let children = tree(&[(1, 2), (2, 3), (3, 1)]);
        let walk = walk_descendants(&children, node(1), 500, 32);
        assert_eq!(walk.nodes.len(), 3, "each plan is visited once");
        assert_eq!(walk.revisits, 1, "the back-edge is counted");
        assert!(!walk.truncated, "a cycle is not a cap firing");
    }

    #[test]
    fn a_diamond_counts_the_second_path_as_a_revisit() {
        // 1 ─┬─ 2 ─┐
        //    └─ 3 ─┴─ 4   — 4 is reached twice, counted once.
        let children = tree(&[(1, 2), (1, 3), (2, 4), (3, 4)]);
        let walk = walk_descendants(&children, node(1), 500, 32);
        assert_eq!(walk.nodes.len(), 4, "no double counting");
        assert_eq!(walk.revisits, 1);
    }

    #[test]
    fn the_node_cap_fires_and_says_so() {
        let children = tree(&[(1, 2), (1, 3), (1, 4), (1, 5)]);
        let walk = walk_descendants(&children, node(1), 3, 32);
        assert_eq!(walk.nodes.len(), 3);
        assert!(
            walk.truncated,
            "a cap that fires silently reads as full coverage"
        );
    }

    #[test]
    fn the_depth_cap_fires_only_when_there_was_more_to_walk() {
        // Depth 1 with grandchildren present ⇒ truncated.
        let deep = tree(&[(1, 2), (2, 3)]);
        let walk = walk_descendants(&deep, node(1), 500, 1);
        assert_eq!(walk.nodes.len(), 2);
        assert!(walk.truncated);

        // Depth 1 with nothing below the children ⇒ complete.
        let shallow = tree(&[(1, 2), (1, 3)]);
        let walk = walk_descendants(&shallow, node(1), 500, 1);
        assert_eq!(walk.nodes.len(), 3);
        assert!(!walk.truncated, "the tree ran out, no cap fired");
    }

    #[test]
    fn a_deep_chain_stops_at_the_documented_depth() {
        let pairs: Vec<(u8, u8)> = (1..60).map(|i| (i, i + 1)).collect();
        let walk = walk_descendants(&tree(&pairs), node(1), MAX_ROLLUP_NODES, MAX_ROLLUP_DEPTH);
        assert_eq!(
            walk.nodes.len(),
            MAX_ROLLUP_DEPTH + 1,
            "root plus the depth"
        );
        assert!(walk.truncated);
        assert!(
            walk.nodes.iter().all(|n| n.depth <= MAX_ROLLUP_DEPTH),
            "no node past the cap"
        );
    }

    #[test]
    fn a_self_parent_does_not_loop() {
        let walk = walk_descendants(&tree(&[(1, 1)]), node(1), 500, 32);
        assert_eq!(walk.nodes.len(), 1);
        assert_eq!(walk.revisits, 1);
    }

    // -- Monte-Carlo forecasting (§15 TBA-11) ------------------------------

    #[test]
    fn throughput_history_buckets_completions_into_whole_periods() {
        let week = 7 * DAY_MS;
        // Three finished in week 0, one in week 1, none in week 2.
        let completed = vec![at(0), at(1), at(2), at(8)];
        let history = throughput_history(&completed, at(0), at(21), week);
        assert_eq!(history, vec![3, 1, 0]);
    }

    #[test]
    fn throughput_history_ignores_a_partial_trailing_period() {
        let week = 7 * DAY_MS;
        // Ten days of window is one whole week plus a stub. Counting the
        // stub as a period would understate throughput.
        let history = throughput_history(&[at(1), at(8)], at(0), at(10), week);
        assert_eq!(history, vec![1], "only whole periods count");
    }

    #[test]
    fn throughput_history_drops_anything_outside_the_window() {
        let week = 7 * DAY_MS;
        let history = throughput_history(&[at(-5), at(3), at(99)], at(0), at(14), week);
        assert_eq!(history, vec![1, 0]);
        assert!(throughput_history(&[at(1)], at(0), at(14), 0).is_empty());
        assert!(throughput_history(&[at(1)], at(14), at(0), week).is_empty());
    }

    #[test]
    fn a_batch_forecast_is_deterministic() {
        // A forecast that changes every time you reload it is not one
        // anybody will act on.
        let history = vec![3, 5, 2, 4, 6, 3, 5, 4];
        let first = forecast_batch(&history, 20, 2000, 7, 42);
        let repeat = forecast_batch(&history, 20, 2000, 7, 42);
        assert_eq!(first.p85_days, repeat.p85_days);
        assert_eq!(first.p50_days, repeat.p50_days);
        // A different seed may differ, but must stay in the same region.
        let reseeded = forecast_batch(&history, 20, 2000, 7, 7);
        let (left, right) = (
            first.p85_days.expect("seeded"),
            reseeded.p85_days.expect("reseeded"),
        );
        assert!((left - right).abs() <= 14.0, "{left} vs {right}");
    }

    #[test]
    fn a_batch_forecast_is_ordered_and_scales_with_the_batch() {
        let history = vec![3, 5, 2, 4, 6, 3, 5, 4];
        let f = forecast_batch(&history, 40, 4000, 7, 1);
        let (p50, p85, p95) = (
            f.p50_days.expect("p50"),
            f.p85_days.expect("p85"),
            f.p95_days.expect("p95"),
        );
        assert!(p50 <= p85 && p85 <= p95, "{p50} {p85} {p95}");
        // Roughly 4 items a week, so 40 items is about ten weeks.
        assert!((49.0..=105.0).contains(&p50), "p50 was {p50} days");
        // A bigger batch cannot finish sooner.
        let bigger = forecast_batch(&history, 80, 4000, 7, 1);
        assert!(bigger.p85_days.expect("p85") >= p85);
        assert_eq!(f.trials_hit_ceiling, 0);
    }

    #[test]
    fn a_batch_forecast_refuses_rather_than_guessing() {
        // Too little history.
        let thin = forecast_batch(&[3, 4, 5], 10, 1000, 7, 1);
        assert_eq!(thin.p85_days, None);
        assert!(thin.reason.is_some_and(|r| r.contains("noise")));

        // A history of zeroes: the honest answer is `never`, and the
        // per-trial ceiling turns what would be an infinite loop into a
        // stated refusal.
        let dead = forecast_batch(&[0; 8], 10, 1000, 7, 1);
        assert_eq!(dead.p85_days, None);
        assert!(dead.reason.is_some_and(|r| r.contains("never")));

        // Zero items is zero days, not a refusal.
        let none = forecast_batch(&[3, 4, 5, 2, 3, 4], 0, 100, 7, 1);
        assert_eq!(none.p85_days, Some(0.0));
        assert_eq!(none.reason, None);
    }

    #[test]
    fn a_mostly_dead_history_reports_the_trials_that_never_finished() {
        // One good week in eight. Some trials will not cover the batch
        // inside the ceiling, and the percentiles are then floors — so
        // the count is reported rather than silently rolled in.
        let history = vec![0, 0, 0, 0, 0, 0, 0, 1];
        let f = forecast_batch(&history, 5000, 200, 7, 1);
        assert!(
            f.trials_hit_ceiling > 0,
            "the ceiling must be visible, not silent"
        );
    }

    #[test]
    fn an_items_forecast_puts_the_conservative_figure_at_the_low_end() {
        // The direction reverses from `forecast_batch`: 85% confidence
        // of at-least-N is the 15th percentile. Quoting the 85th here
        // would promise the best case while sounding careful.
        let history = vec![1, 3, 5, 2, 8, 4, 6, 2];
        let f = forecast_items(&history, 4, 4000, 7, 3);
        let least = f.at_least_items.expect("at_least");
        let median = f.median_items.expect("median");
        let most = f.at_most_items.expect("at_most");
        assert!(least <= median && median <= most, "{least} {median} {most}");
        assert!(f.note.contains("15th percentile"));
        assert_eq!(f.days, 28, "four weeks");
    }

    #[test]
    fn an_items_forecast_refuses_rather_than_guessing() {
        let thin = forecast_items(&[3, 4, 5], 4, 1000, 7, 1);
        assert_eq!(thin.median_items, None);
        assert!(thin.reason.is_some_and(|r| r.contains("noise")));

        let history = vec![1, 3, 5, 2, 8, 4, 6, 2];
        assert!(forecast_items(&history, 0, 100, 7, 1).reason.is_some());
        assert!(
            forecast_items(&history, MAX_PERIODS_PER_TRIAL + 1, 100, 7, 1)
                .reason
                .is_some()
        );
    }

    #[test]
    fn trials_are_clamped_rather_than_trusted() {
        let history = vec![3, 4, 5, 2, 3, 4];
        assert_eq!(forecast_batch(&history, 5, 0, 7, 1).trials, 1);
        assert_eq!(
            forecast_batch(&history, 5, usize::MAX, 7, 1).trials,
            MAX_TRIALS
        );
        assert_eq!(forecast_items(&history, 2, 0, 7, 1).trials, 1);
    }

    #[test]
    fn a_zero_seed_still_produces_variety() {
        // xorshift emits only zeroes from a zero state, which would
        // make every trial identical and every percentile the same.
        let history = vec![1, 9, 2, 8, 3, 7];
        let f = forecast_items(&history, 5, 2000, 7, 0);
        assert!(
            f.at_least_items.expect("least") < f.at_most_items.expect("most"),
            "a zero seed must not collapse the distribution"
        );
    }

    // -- flow gauges (§15 TBA-10) -----------------------------------------

    /// A plan whose board holds `n` finished tasks.
    fn sample_of(pid: &str, n: usize) -> PlanFlowSample {
        let analyses: Vec<TaskAnalysis> = (0..n).map(|_| finished_task(1, 2, false)).collect();
        let cycles: Vec<i64> = analyses.iter().filter_map(|a| a.cycle_time_ms).collect();
        PlanFlowSample {
            plan_pid: pid.to_string(),
            analysis: plan(&analyses),
            sle: service_level_expectation(&cycles, 0.85, None),
            columns_over_limit: 0,
        }
    }

    #[test]
    fn small_boards_are_suppressed_not_exported() {
        // A flow efficiency over two tasks describes two people's week,
        // and §12.4 refuses per-person measurement; a public
        // `/metrics.prom` label that reaches it by arithmetic is the
        // same thing through a side door.
        let samples = vec![
            sample_of("big", 20),
            sample_of("tiny", 2),
            sample_of("empty", 0),
        ];
        let set = flow_metric_rows(&samples, 50, 5);
        assert_eq!(set.rows.len(), 1);
        assert_eq!(set.rows[0].plan_pid, "big");
        assert_eq!(set.suppressed_plans, 1);
        assert!(!set.rows.iter().any(|row| row.plan_pid == "tiny"));
        // An empty board is neither exported nor counted as suppressed:
        // there is nothing to withhold.
        assert!(!set.rows.iter().any(|row| row.plan_pid == "empty"));
    }

    #[test]
    fn the_series_cap_keeps_the_largest_boards_and_says_what_it_dropped() {
        let samples: Vec<PlanFlowSample> = (1..=8)
            .map(|i| sample_of(&format!("p{i:02}"), i * 5))
            .collect();
        let set = flow_metric_rows(&samples, 3, 5);
        assert_eq!(set.rows.len(), 3);
        assert_eq!(
            set.rows.iter().map(|r| r.tasks).collect::<Vec<_>>(),
            vec![40, 35, 30],
            "largest board first"
        );
        assert_eq!(set.dropped_plans, 5, "the cap is never silent");
    }

    #[test]
    fn the_p85_gauge_inherits_the_sle_refusal_to_forecast_from_noise() {
        // Eight finished tasks is below MIN_SLE_SAMPLE, so the
        // expectation is null — and the gauge must stay null rather
        // than re-deciding the question with a number from noise.
        let thin = sample_of("thin", 8);
        assert!(thin.sle.within_days.is_none(), "precondition");
        let set = flow_metric_rows(std::slice::from_ref(&thin), 50, 5);
        assert_eq!(set.rows[0].cycle_time_p85_days, None);

        let thick = sample_of("thick", 20);
        let set = flow_metric_rows(std::slice::from_ref(&thick), 50, 5);
        assert!(set.rows[0].cycle_time_p85_days.is_some());
    }

    #[test]
    fn selection_is_stable_across_passes() {
        // A dashboard must not reshuffle between scrapes.
        let samples: Vec<PlanFlowSample> = ["c", "a", "b"]
            .iter()
            .map(|pid| sample_of(pid, 9))
            .collect();
        let first = flow_metric_rows(&samples, 2, 5);
        let mut shuffled = samples.clone();
        shuffled.reverse();
        let second = flow_metric_rows(&shuffled, 2, 5);
        assert_eq!(first.rows, second.rows);
        assert_eq!(
            first
                .rows
                .iter()
                .map(|r| r.plan_pid.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "b"]
        );
    }

    #[test]
    fn an_empty_estate_exports_nothing_rather_than_zeroes() {
        let set = flow_metric_rows(&[], 50, 5);
        assert!(set.rows.is_empty());
        assert_eq!(set.suppressed_plans, 0);
        assert_eq!(set.dropped_plans, 0);
    }

    // -- property-style sweep ---------------------------------------------

    #[test]
    fn invariants_hold_over_a_generated_sweep() {
        let mut state: u64 = 0x2545_F491_4F6C_DD1D;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let statuses = ["todo", "in_progress", "in_review", "blocked", "done"];
        for _ in 0..500 {
            let count = usize::try_from(next() % 15).unwrap_or(0);
            let mut day = 0i64;
            let mut previous: Option<String> = None;
            let mut transitions = Vec::new();
            let mut done_day: Option<i64> = None;
            for _ in 0..count {
                day += i64::try_from(next() % 5).unwrap_or(0);
                let to = statuses[usize::try_from(next() % 5).unwrap_or(0)];
                transitions.push(tr(previous.as_deref(), to, day));
                previous = Some(to.to_string());
                if to == FINISHED_STATUS && done_day.is_none() {
                    done_day = Some(day);
                }
            }
            let as_of = at(day + i64::try_from(next() % 10).unwrap_or(0));
            let clock = TaskClock {
                created_ms: at(0),
                done_ms: done_day.map(at),
            };
            let a = analyze(&transitions, clock, &classes(), as_of);

            let cycle = a.cycle_time_ms.unwrap_or(0);
            let status_total: i64 = a.by_status.iter().map(|s| s.ms).sum();
            let category_total: i64 = a.by_category.iter().map(|c| c.ms).sum();
            assert_eq!(status_total, a.lead_time_ms, "statuses partition lead time");
            assert_eq!(
                category_total, a.lead_time_ms,
                "categories partition it too"
            );
            assert!(a.work_time_ms <= a.process_time_ms);
            assert!(
                a.process_time_ms <= cycle,
                "process {} > cycle {cycle}",
                a.process_time_ms
            );
            assert!(a.lead_time_ms >= 0);
            assert!(a.by_status.iter().all(|s| s.ms >= 0));
            if let Some(v) = a.flow_efficiency.value {
                assert!((0.0..=1.0).contains(&v), "efficiency out of range: {v}");
            }
            // A finished item's cycle time never exceeds its lead time.
            if a.finished {
                assert!(
                    cycle <= a.lead_time_ms,
                    "cycle {cycle} > lead {}",
                    a.lead_time_ms
                );
            }
        }
    }
}
