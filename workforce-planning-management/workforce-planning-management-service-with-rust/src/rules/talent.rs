//! Pure rules for **talent strategy** (WPM-R21–R24): development
//! plans (upskilling and reskilling), talent pipelines, early-career
//! programmes (apprenticeships / internships / graduate schemes), and
//! the succession + workforce-intelligence derivations.
//!
//! DB-free and clock-free (dates are always supplied), exhaustively
//! unit-tested; the controllers only wire these.
//!
//! Every rollup here reports its **terms** — a count with its
//! denominator, or `None` when there is nothing to divide — so no
//! derived figure can be mistaken for a richer measurement than it is.
//! Nothing is interpolated and nothing is rounded up.

use std::collections::BTreeMap;

// ─── Development plans (upskilling / reskilling) ─────────────────────────────

/// The two kinds of development plan.
///
/// - `upskill` — deepen the skills of the employee's **current** role.
/// - `reskill` — build the skills for a **different** role, so the
///   plan must name its target (see [`target_matches_kind`]).
pub const DEVELOPMENT_PLAN_KINDS: &[&str] = &["upskill", "reskill"];

/// Development-plan lifecycle statuses.
pub const DEVELOPMENT_PLAN_STATUSES: &[&str] = &["draft", "active", "completed", "cancelled"];

/// How a plan item is to be learned.
pub const DEVELOPMENT_METHODS: &[&str] = &[
    "course",
    "mentorship",
    "on_the_job",
    "apprenticeship",
    "internship",
    "self_study",
];

/// Per-item statuses. `achieved` is the only one that counts as
/// progress.
pub const DEVELOPMENT_ITEM_STATUSES: &[&str] = &["planned", "in_progress", "achieved", "abandoned"];

/// Whether a plan's target role is consistent with its kind: a
/// `reskill` plan must name a target job title or department (that is
/// what makes it a reskill), and an `upskill` plan must not (the target
/// is the current role).
///
/// # Errors
///
/// A human-readable refusal the controller turns into a `422`.
pub fn target_matches_kind(
    kind: &str,
    target_job_title: Option<&str>,
    target_department: Option<&str>,
) -> Result<(), String> {
    let has_target = target_job_title.is_some_and(|t| !t.trim().is_empty())
        || target_department.is_some_and(|d| !d.trim().is_empty());
    match (kind, has_target) {
        ("reskill", false) => Err(
            "a reskill plan must name the target role (target_job_title or target_department)"
                .to_string(),
        ),
        ("upskill", true) => Err(
            "an upskill plan deepens the current role; use kind `reskill` to name a target role"
                .to_string(),
        ),
        _ => Ok(()),
    }
}

/// The development-plan lifecycle: `draft → active → completed`, with
/// `cancelled` reachable from `draft` or `active`. Terminal states do
/// not transition.
///
/// # Errors
///
/// A human-readable refusal naming the legal moves.
pub fn plan_transition(current: &str, to: &str) -> Result<(), String> {
    if !DEVELOPMENT_PLAN_STATUSES.contains(&to) {
        return Err(format!(
            "unknown status `{to}` (statuses: {DEVELOPMENT_PLAN_STATUSES:?})"
        ));
    }
    let ok = matches!(
        (current, to),
        ("draft", "active" | "cancelled") | ("active", "completed" | "cancelled")
    );
    if ok {
        Ok(())
    } else {
        Err(format!(
            "illegal transition `{current}` → `{to}` (draft→active→completed, or cancel an open plan)"
        ))
    }
}

/// The declared proficiency scale, shared with
/// [`crate::rules::learning`] (1–5).
#[must_use]
pub fn valid_level(level: i32) -> bool {
    (1..=5).contains(&level)
}

/// Whether a plan item's step is coherent: both levels on the scale and
/// the target **above** the current level. A step that does not raise
/// the level is not development.
///
/// # Errors
///
/// A human-readable refusal.
pub fn valid_step(current_level: i32, target_level: i32) -> Result<(), String> {
    if !valid_level(current_level) || !valid_level(target_level) {
        return Err("levels must be between 1 and 5".to_string());
    }
    if target_level <= current_level {
        return Err(format!(
            "target_level ({target_level}) must be above current_level ({current_level})"
        ));
    }
    Ok(())
}

/// Plan progress as `(achieved, total)` over item statuses. Only
/// `achieved` counts; `abandoned` items stay in the denominator, so
/// abandoning work never flatters the ratio.
#[must_use]
pub fn plan_progress(item_statuses: &[String]) -> (usize, usize) {
    let achieved = item_statuses
        .iter()
        .filter(|s| s.as_str() == "achieved")
        .count();
    (achieved, item_statuses.len())
}

/// Progress **verified against declared proficiency**: an item counts
/// only when the employee's declared level for that skill has actually
/// reached the item's target. Returns `(verified, total)`.
///
/// This is the honest counterpart of [`plan_progress`]: marking an item
/// `achieved` is a claim, reaching the target proficiency is evidence.
#[must_use]
pub fn verified_progress(
    targets: &[(uuid::Uuid, i32)],
    declared: &BTreeMap<uuid::Uuid, i32>,
) -> (usize, usize) {
    let verified = targets
        .iter()
        .filter(|(skill, target)| declared.get(skill).is_some_and(|level| level >= target))
        .count();
    (verified, targets.len())
}

// ─── Talent pipelines ────────────────────────────────────────────────────────

/// What a pipeline is being grown for.
pub const PIPELINE_PURPOSES: &[&str] =
    &["succession", "hiring", "early_careers", "internal_mobility"];

/// Pipeline member stages, in progression order.
pub const PIPELINE_STAGES: &[&str] = &[
    "identified",
    "assessing",
    "developing",
    "ready",
    "placed",
    "exited",
];

/// Who can be in a pipeline.
pub const PIPELINE_SUBJECTS: &[&str] = &["candidate", "employee"];

/// Readiness ratings, shared with succession
/// ([`crate::rules::tokens::READINESS`]).
pub const READINESS: &[&str] = &["ready_now", "ready_1y", "ready_2y"];

/// The pipeline stage machine: forward through
/// `identified → assessing → developing → ready → placed`, with
/// `exited` reachable from any non-terminal stage and a step **back**
/// from `ready` to `developing` allowed (readiness can regress — a
/// pipeline that can only move forward lies).
///
/// # Errors
///
/// A human-readable refusal naming the legal moves.
pub fn pipeline_transition(current: &str, to: &str) -> Result<(), String> {
    if !PIPELINE_STAGES.contains(&to) {
        return Err(format!(
            "unknown stage `{to}` (stages: {PIPELINE_STAGES:?})"
        ));
    }
    let ok = matches!(
        (current, to),
        ("identified", "assessing" | "developing" | "exited")
            | ("assessing", "developing" | "ready" | "exited")
            | ("developing", "ready" | "exited")
            | ("ready", "placed" | "developing" | "exited")
    );
    if ok {
        Ok(())
    } else {
        Err(format!(
            "illegal stage move `{current}` → `{to}` \
             (identified→assessing→developing→ready→placed; a ready member may return to \
             developing; exit an open one)"
        ))
    }
}

/// Pipeline health from its members' stages: how many are in each
/// stage, how many are `ready` (the only stage that answers "could we
/// fill the role today?"), and the live total (`placed` and `exited`
/// members have left the pipeline).
#[must_use]
pub fn pipeline_health(stages: &[String]) -> PipelineHealth {
    let mut by_stage: BTreeMap<String, usize> = BTreeMap::new();
    for stage in stages {
        *by_stage.entry(stage.clone()).or_default() += 1;
    }
    let ready = by_stage.get("ready").copied().unwrap_or(0);
    let live = stages
        .iter()
        .filter(|s| !matches!(s.as_str(), "placed" | "exited"))
        .count();
    PipelineHealth {
        by_stage,
        ready,
        live,
        placed: stages.iter().filter(|s| s.as_str() == "placed").count(),
    }
}

/// The derived shape [`pipeline_health`] returns.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct PipelineHealth {
    /// Member count per stage.
    pub by_stage: BTreeMap<String, usize>,
    /// Members at the `ready` stage.
    pub ready: usize,
    /// Members still in the pipeline (not `placed` or `exited`).
    pub live: usize,
    /// Members placed out of the pipeline.
    pub placed: usize,
}

// ─── Early-career programmes ─────────────────────────────────────────────────

/// Programme kinds.
pub const PROGRAM_KINDS: &[&str] = &["apprenticeship", "internship", "graduate"];

/// Placement lifecycle statuses.
pub const PLACEMENT_STATUSES: &[&str] = &["offered", "active", "completed", "withdrawn"];

/// What became of a placement.
pub const PLACEMENT_OUTCOMES: &[&str] = &["pending", "converted", "not_converted", "withdrawn"];

/// The placement lifecycle: `offered → active → completed`, with
/// `withdrawn` reachable from `offered` or `active`.
///
/// # Errors
///
/// A human-readable refusal naming the legal moves.
pub fn placement_transition(current: &str, to: &str) -> Result<(), String> {
    if !PLACEMENT_STATUSES.contains(&to) {
        return Err(format!(
            "unknown status `{to}` (statuses: {PLACEMENT_STATUSES:?})"
        ));
    }
    let ok = matches!(
        (current, to),
        ("offered", "active" | "withdrawn") | ("active", "completed" | "withdrawn")
    );
    if ok {
        Ok(())
    } else {
        Err(format!(
            "illegal transition `{current}` → `{to}` \
             (offered→active→completed, or withdraw an open placement)"
        ))
    }
}

/// Whether a placement may be **completed**: an apprenticeship must
/// have met its programme's minimum off-the-job training hours, because
/// completing one that has not is a false record of a regulated
/// programme. Other programme kinds have no hours requirement unless
/// their programme declares one.
///
/// # Errors
///
/// A refusal naming the hours recorded and the hours required.
pub fn may_complete_placement(
    kind: &str,
    off_the_job_hours: i32,
    min_off_the_job_hours: Option<i32>,
) -> Result<(), String> {
    let Some(required) = min_off_the_job_hours else {
        return Ok(());
    };
    if off_the_job_hours >= required {
        return Ok(());
    }
    Err(format!(
        "the {kind} requires {required} off-the-job training hours; {off_the_job_hours} recorded"
    ))
}

/// Conversion rate as `(converted, completed, ratio)` — how many
/// completed placements led to a `converted` outcome.
///
/// The denominator is **completed** placements only: a placement still
/// running has not had the chance to convert, and counting it would
/// understate the rate. `None` when nothing has completed yet — never a
/// zero that reads like a failure.
#[must_use]
pub fn conversion_rate(outcomes_of_completed: &[String]) -> Option<(usize, usize, f64)> {
    let completed = outcomes_of_completed.len();
    if completed == 0 {
        return None;
    }
    let converted = outcomes_of_completed
        .iter()
        .filter(|o| o.as_str() == "converted")
        .count();
    #[allow(clippy::cast_precision_loss)] // display ratio; the terms are returned alongside
    let ratio = converted as f64 / completed as f64;
    Some((converted, completed, ratio))
}

// ─── Succession + workforce intelligence ─────────────────────────────────────

/// Risk that the incumbent of a critical role leaves.
pub const RISK_OF_LOSS: &[&str] = &["low", "medium", "high"];

/// Bench coverage for one succession plan, from its candidates'
/// readiness ratings:
///
/// - `covered_now` — at least one `ready_now` successor.
/// - `covered_soon` — none now, but at least one `ready_1y`.
/// - `developing` — only `ready_2y` successors.
/// - `uncovered` — no successors at all.
///
/// Deliberately conservative: a role is only "covered now" if someone
/// could actually step in today.
#[must_use]
pub fn bench_coverage(readiness: &[String]) -> &'static str {
    if readiness.iter().any(|r| r == "ready_now") {
        "covered_now"
    } else if readiness.iter().any(|r| r == "ready_1y") {
        "covered_soon"
    } else if readiness.is_empty() {
        "uncovered"
    } else {
        "developing"
    }
}

/// Whether a succession plan is a **single point of failure**: a
/// critical role (`criticality >= 4`) whose bench is not covered now.
/// A high risk of loss makes it one at a lower criticality too, because
/// the exposure is the product of the two.
#[must_use]
pub fn is_single_point_of_failure(
    criticality: i32,
    coverage: &str,
    risk_of_loss: Option<&str>,
) -> bool {
    if coverage == "covered_now" {
        return false;
    }
    criticality >= 4 || (risk_of_loss == Some("high") && criticality >= 3)
}

/// A ratio reported with its terms: `(numerator, denominator, value)`,
/// or `None` when the denominator is zero. The workforce-intelligence
/// views use this everywhere so no rate is ever shown without the
/// counts behind it.
#[must_use]
pub fn ratio(numerator: usize, denominator: usize) -> Option<(usize, usize, f64)> {
    if denominator == 0 {
        return None;
    }
    #[allow(clippy::cast_precision_loss)] // display ratio; the terms are returned alongside
    let value = numerator as f64 / denominator as f64;
    Some((numerator, denominator, value))
}

/// Tenure bucket for a whole number of completed months of service —
/// the standard workforce-intelligence bands.
#[must_use]
pub fn tenure_bucket(months: i64) -> &'static str {
    match months {
        i64::MIN..=-1 => "not_started",
        0..=11 => "under_1y",
        12..=35 => "1_to_3y",
        36..=59 => "3_to_5y",
        60..=119 => "5_to_10y",
        _ => "over_10y",
    }
}

/// Whole months of service between `hired_on` and `as_of` (negative
/// when the hire date is in the future, so a future start is visible
/// rather than silently zero).
#[must_use]
pub fn months_of_service(hired_on: chrono::NaiveDate, as_of: chrono::NaiveDate) -> i64 {
    use chrono::Datelike;
    let years = i64::from(as_of.year()) - i64::from(hired_on.year());
    let months = years * 12 + i64::from(as_of.month()) - i64::from(hired_on.month());
    if as_of.day() < hired_on.day() {
        months - 1
    } else {
        months
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn day(y: i32, m: u32, d: u32) -> chrono::NaiveDate {
        chrono::NaiveDate::from_ymd_opt(y, m, d).expect("valid date")
    }

    /// Upskilling deepens the current role; reskilling must name a
    /// target. Each refuses the other's shape.
    #[test]
    fn plan_kind_and_target_agree() {
        assert!(target_matches_kind("upskill", None, None).is_ok());
        assert!(target_matches_kind("reskill", Some("Data Engineer"), None).is_ok());
        assert!(target_matches_kind("reskill", None, Some("Analytics")).is_ok());

        let no_target = target_matches_kind("reskill", None, None).expect_err("needs a target");
        assert!(no_target.contains("target"));
        let stray = target_matches_kind("upskill", Some("Data Engineer"), None)
            .expect_err("upskill has no target role");
        assert!(stray.contains("reskill"));
        // A blank string is not a target.
        assert!(target_matches_kind("reskill", Some("   "), None).is_err());
    }

    /// The plan lifecycle: forward only, cancel from an open state,
    /// terminal states stuck.
    #[test]
    fn plan_lifecycle() {
        assert!(plan_transition("draft", "active").is_ok());
        assert!(plan_transition("active", "completed").is_ok());
        assert!(plan_transition("draft", "cancelled").is_ok());
        assert!(plan_transition("active", "cancelled").is_ok());
        assert!(
            plan_transition("draft", "completed").is_err(),
            "must activate first"
        );
        assert!(plan_transition("completed", "active").is_err(), "terminal");
        assert!(plan_transition("cancelled", "draft").is_err(), "terminal");
        assert!(plan_transition("draft", "sideways").is_err(), "unknown");
        for status in DEVELOPMENT_PLAN_STATUSES {
            assert!(
                plan_transition(status, status).is_err(),
                "{status} → {status}"
            );
        }
    }

    /// A development step must raise the level, and stay on the 1–5
    /// scale.
    #[test]
    fn steps_must_raise_the_level() {
        assert!(valid_step(2, 4).is_ok());
        assert!(valid_step(3, 3).is_err(), "not a step");
        assert!(valid_step(4, 2).is_err(), "backwards");
        assert!(valid_step(0, 3).is_err(), "off the scale");
        assert!(valid_step(3, 6).is_err(), "off the scale");
    }

    /// Progress counts only achieved items, and abandoning work does
    /// not shrink the denominator.
    #[test]
    fn progress_counts_achievements_honestly() {
        let statuses: Vec<String> = ["achieved", "in_progress", "abandoned", "planned"]
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(plan_progress(&statuses), (1, 4));
        assert_eq!(plan_progress(&[]), (0, 0));
    }

    /// Verified progress needs the declared proficiency to have reached
    /// the target — a claim alone does not count.
    #[test]
    fn verified_progress_needs_evidence() {
        let rust = Uuid::new_v4();
        let sql = Uuid::new_v4();
        let targets = vec![(rust, 4), (sql, 3)];

        let mut declared = BTreeMap::new();
        declared.insert(rust, 4); // reached
        declared.insert(sql, 2); // not yet
        assert_eq!(verified_progress(&targets, &declared), (1, 2));

        // Exceeding the target still counts; an undeclared skill does not.
        declared.insert(rust, 5);
        declared.remove(&sql);
        assert_eq!(verified_progress(&targets, &declared), (1, 2));
        assert_eq!(verified_progress(&targets, &BTreeMap::new()), (0, 2));
    }

    /// The pipeline stage machine, including the deliberate regression
    /// from `ready` back to `developing`.
    #[test]
    fn pipeline_stage_machine() {
        assert!(pipeline_transition("identified", "assessing").is_ok());
        assert!(pipeline_transition("assessing", "developing").is_ok());
        assert!(pipeline_transition("developing", "ready").is_ok());
        assert!(pipeline_transition("ready", "placed").is_ok());
        assert!(
            pipeline_transition("ready", "developing").is_ok(),
            "readiness can regress; the pipeline must be able to say so"
        );
        assert!(pipeline_transition("identified", "exited").is_ok());
        assert!(
            pipeline_transition("identified", "placed").is_err(),
            "no skipping to placed"
        );
        assert!(pipeline_transition("placed", "ready").is_err(), "terminal");
        assert!(
            pipeline_transition("exited", "identified").is_err(),
            "terminal"
        );
        assert!(
            pipeline_transition("ready", "elsewhere").is_err(),
            "unknown"
        );
    }

    /// Pipeline health separates the live pool from those who have left
    /// it, and counts the ready.
    #[test]
    fn pipeline_health_counts_the_live_pool() {
        let stages: Vec<String> = [
            "identified",
            "developing",
            "ready",
            "ready",
            "placed",
            "exited",
        ]
        .iter()
        .map(ToString::to_string)
        .collect();
        let health = pipeline_health(&stages);
        assert_eq!(health.ready, 2);
        assert_eq!(health.placed, 1);
        assert_eq!(health.live, 4, "placed and exited have left the pipeline");
        assert_eq!(health.by_stage.get("ready"), Some(&2));
        assert_eq!(health.by_stage.get("assessing"), None);

        let empty = pipeline_health(&[]);
        assert_eq!((empty.ready, empty.live, empty.placed), (0, 0, 0));
    }

    /// The placement lifecycle.
    #[test]
    fn placement_lifecycle() {
        assert!(placement_transition("offered", "active").is_ok());
        assert!(placement_transition("active", "completed").is_ok());
        assert!(placement_transition("offered", "withdrawn").is_ok());
        assert!(placement_transition("active", "withdrawn").is_ok());
        assert!(
            placement_transition("offered", "completed").is_err(),
            "must start first"
        );
        assert!(
            placement_transition("completed", "active").is_err(),
            "terminal"
        );
        assert!(
            placement_transition("withdrawn", "offered").is_err(),
            "terminal"
        );
    }

    /// An apprenticeship cannot be completed below its off-the-job
    /// hours minimum; a programme with no minimum is unconstrained.
    #[test]
    fn apprenticeship_completion_needs_its_hours() {
        assert!(may_complete_placement("apprenticeship", 400, Some(400)).is_ok());
        assert!(may_complete_placement("apprenticeship", 401, Some(400)).is_ok());
        let short = may_complete_placement("apprenticeship", 399, Some(400))
            .expect_err("below the minimum");
        assert!(short.contains("400") && short.contains("399"));
        assert!(
            may_complete_placement("internship", 0, None).is_ok(),
            "no declared minimum ⇒ no hours gate"
        );
        assert!(
            may_complete_placement("internship", 5, Some(10)).is_err(),
            "a declared minimum applies to any kind"
        );
    }

    /// Conversion rate divides by completed placements only, and is
    /// `None` before anything completes.
    #[test]
    fn conversion_rate_divides_by_completed_only() {
        assert_eq!(
            conversion_rate(&[]),
            None,
            "nothing completed ⇒ no rate, not 0%"
        );
        let outcomes: Vec<String> = ["converted", "not_converted", "converted", "withdrawn"]
            .iter()
            .map(ToString::to_string)
            .collect();
        let (converted, completed, ratio) = conversion_rate(&outcomes).expect("four completed");
        assert_eq!((converted, completed), (2, 4));
        assert!((ratio - 0.5).abs() < f64::EPSILON);
    }

    /// Bench coverage is conservative: only a `ready_now` successor
    /// covers a role today.
    #[test]
    fn bench_coverage_is_conservative() {
        let of =
            |ratings: &[&str]| -> Vec<String> { ratings.iter().map(ToString::to_string).collect() };
        assert_eq!(
            bench_coverage(&of(&["ready_2y", "ready_now"])),
            "covered_now"
        );
        assert_eq!(
            bench_coverage(&of(&["ready_1y", "ready_2y"])),
            "covered_soon"
        );
        assert_eq!(bench_coverage(&of(&["ready_2y"])), "developing");
        assert_eq!(bench_coverage(&[]), "uncovered");
    }

    /// A single point of failure is an uncovered critical role — and a
    /// high-risk incumbent lowers the criticality threshold.
    #[test]
    fn single_points_of_failure() {
        assert!(is_single_point_of_failure(5, "uncovered", None));
        assert!(is_single_point_of_failure(4, "developing", Some("low")));
        assert!(
            !is_single_point_of_failure(5, "covered_now", Some("high")),
            "a ready successor means it is not a single point of failure"
        );
        assert!(
            !is_single_point_of_failure(3, "uncovered", Some("low")),
            "a non-critical role with a settled incumbent is not one"
        );
        assert!(
            is_single_point_of_failure(3, "uncovered", Some("high")),
            "a likely departure makes a mid-criticality role one"
        );
    }

    /// Ratios carry their terms and refuse to divide by zero.
    #[test]
    fn ratios_report_their_terms() {
        assert_eq!(ratio(0, 0), None);
        let (n, d, value) = ratio(3, 4).expect("non-zero denominator");
        assert_eq!((n, d), (3, 4));
        assert!((value - 0.75).abs() < f64::EPSILON);
        let (n, d, value) = ratio(0, 5).expect("a real zero");
        assert_eq!((n, d), (0, 5));
        assert!(
            value.abs() < f64::EPSILON,
            "0 of 5 is a fact, not an absence"
        );
    }

    /// Tenure: whole months of service, bucketed; a not-yet-started
    /// hire is visible rather than folded into the first bucket.
    #[test]
    fn tenure_is_whole_months() {
        let as_of = day(2026, 7, 23);
        assert_eq!(months_of_service(day(2026, 7, 23), as_of), 0);
        assert_eq!(
            months_of_service(day(2026, 6, 24), as_of),
            0,
            "not a full month"
        );
        assert_eq!(months_of_service(day(2026, 6, 23), as_of), 1);
        assert_eq!(months_of_service(day(2025, 7, 23), as_of), 12);
        assert_eq!(
            months_of_service(day(2026, 8, 1), as_of),
            -1,
            "future start"
        );

        assert_eq!(tenure_bucket(-1), "not_started");
        assert_eq!(tenure_bucket(0), "under_1y");
        assert_eq!(tenure_bucket(11), "under_1y");
        assert_eq!(tenure_bucket(12), "1_to_3y");
        assert_eq!(tenure_bucket(35), "1_to_3y");
        assert_eq!(tenure_bucket(36), "3_to_5y");
        assert_eq!(tenure_bucket(59), "3_to_5y");
        assert_eq!(tenure_bucket(60), "5_to_10y");
        assert_eq!(tenure_bucket(119), "5_to_10y");
        assert_eq!(tenure_bucket(120), "over_10y");
    }
}
