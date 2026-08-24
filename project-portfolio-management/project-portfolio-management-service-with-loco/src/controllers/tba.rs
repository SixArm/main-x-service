//! **Time-based analysis** (TBA) — the read surface over [`crate::tba`].
//!
//! There is deliberately **no recording endpoint**: transitions are
//! written by the existing task-create and board-move calls, so the
//! measurement is a by-product of the work rather than another thing to
//! keep up to date. A method that asks engineers to log hours gets
//! logged hours, not true ones.
//!
//! Every figure is derived on read — there is no stored
//! `flow_efficiency` column, so appending a transition corrects the
//! analysis rather than leaving a stale number behind.
//!
//! See `spec/time-based-analysis.md` §10. Nothing here is per-person
//! (§12.4): handoff counts describe an item's journey, and there is no
//! per-assignee cycle time by design.

use loco_rs::prelude::*;
use sea_orm::{QueryOrder, QuerySelect};
use uuid::Uuid;

use crate::models::_entities::{plans, task_transitions, tasks};
use crate::tba;

/// Plan reads are capped so an unbounded board cannot become an
/// unbounded query (spec §11).
const MAX_TASKS: u64 = 1000;

/// Plans scanned to build the containment map for a rollup.
const MAX_PLANS_SCANNED: u64 = 1000;

/// Per-task transition cap, for the same reason.
const MAX_TRANSITIONS: u64 = 5000;

/// The flow-analysis window default (spec §9).
const DEFAULT_WINDOW_DAYS: i64 = 90;

/// The service-level-expectation percentile default (spec §7.3).
const DEFAULT_SLE_PERCENTILE: f64 = 0.85;

/// `422` with a reason.
fn refuse(reason: &str) -> Error {
    Error::CustomError(
        axum::http::StatusCode::UNPROCESSABLE_ENTITY,
        loco_rs::controller::ErrorDetail::new("unprocessable_entity", reason),
    )
}

/// Find one live task of one plan, or `404`.
async fn find_task(ctx: &AppContext, plan_pid: Uuid, raw: &str) -> Result<tasks::Model> {
    let pid = Uuid::parse_str(raw).map_err(|_| Error::NotFound)?;
    tasks::Entity::find()
        .filter(tasks::Column::Pid.eq(pid))
        .filter(tasks::Column::PlanPid.eq(plan_pid))
        .filter(tasks::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?
        .ok_or(Error::NotFound)
}

/// The task's clock: created and (first) finished.
fn clock_of(task: &tasks::Model) -> tba::TaskClock {
    tba::TaskClock {
        created_ms: task.created_at.timestamp_millis(),
        done_ms: task.done_at.map(|at| at.timestamp_millis()),
    }
}

/// A `{pid, title, status}` reference for a task.
fn task_ref(task: &tasks::Model) -> serde_json::Value {
    serde_json::json!({
        "pid": task.pid,
        "title": task.title,
        "status": task.status,
        "assignee_ref": task.assignee_ref,
    })
}

/// Load one plan's live tasks (capped) plus every transition belonging
/// to them, in two bounded queries — no N+1 (spec §13).
pub(crate) async fn load_board(
    ctx: &AppContext,
    plan_pid: Uuid,
    sprint: Option<Uuid>,
) -> Result<(Vec<tasks::Model>, Vec<task_transitions::Model>)> {
    let mut query = tasks::Entity::find()
        .filter(tasks::Column::PlanPid.eq(plan_pid))
        .filter(tasks::Column::DeletedAt.is_null());
    if let Some(sprint) = sprint {
        query = query.filter(tasks::Column::SprintPid.eq(sprint));
    }
    let tasks_rows = query
        .order_by_asc(tasks::Column::Id)
        .limit(MAX_TASKS)
        .all(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;
    if tasks_rows.is_empty() {
        return Ok((tasks_rows, Vec::new()));
    }
    let pids: Vec<Uuid> = tasks_rows.iter().map(|t| t.pid).collect();
    let transitions = task_transitions::Entity::find()
        .filter(task_transitions::Column::TaskPid.is_in(pids))
        .order_by_asc(task_transitions::Column::At)
        .all(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;
    Ok((tasks_rows, transitions))
}

/// Analyse every task of a board, pairing each with its analysis.
pub(crate) fn analyze_board(
    tasks_rows: &[tasks::Model],
    transitions: &[task_transitions::Model],
    classes: &std::collections::BTreeMap<String, String>,
    as_of_ms: i64,
) -> Vec<(tasks::Model, tba::TaskAnalysis)> {
    let mut per_task: std::collections::HashMap<Uuid, Vec<tba::Transition>> =
        std::collections::HashMap::new();
    for row in transitions {
        per_task
            .entry(row.task_pid)
            .or_default()
            .push(tba::to_transition(row));
    }
    tasks_rows
        .iter()
        .map(|task| {
            let empty: Vec<tba::Transition> = Vec::new();
            let its = per_task.get(&task.pid).unwrap_or(&empty);
            let analysis = tba::analyze(its, clock_of(task), classes, as_of_ms);
            (task.clone(), analysis)
        })
        .collect()
}

/// The classification map, echoed on every response so a figure can
/// never be compared across two deployments without the difference
/// being visible (spec §5.3).
fn classes_note(classes: &std::collections::BTreeMap<String, String>) -> serde_json::Value {
    serde_json::json!({
        "classes": classes,
        "overridden": *classes != tba::default_classes(),
        "source": "PROJECT_PORTFOLIO_MANAGEMENT_FLOW_CLASSES, else the disclosed default",
    })
}

/// `GET /api/plans/{pid}/tasks/{t_pid}/transitions` — the append-only
/// log for one task.
///
/// Read-only by design: there is no edit or delete route, because an
/// editable flow log measures whatever the editor wanted (spec §5.1
/// invariant 2). Correcting history means moving the card, which is
/// itself recorded.
#[debug_handler]
async fn list_transitions(
    State(ctx): State<AppContext>,
    Path((pid, t_pid)): Path<(String, String)>,
) -> Result<Response> {
    let item = super::governance::find_item(&ctx, &pid).await?;
    let task = find_task(&ctx, item.pid, &t_pid).await?;
    let rows = task_transitions::Entity::find()
        .filter(task_transitions::Column::TaskPid.eq(task.pid))
        .order_by_asc(task_transitions::Column::At)
        .limit(MAX_TRANSITIONS)
        .all(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;
    format::json(serde_json::json!({
        "task": task_ref(&task),
        "note": "append-only; there is no edit or delete. `backfilled` marks a \
                 transition synthesised by the migration rather than observed",
        "transitions": rows,
    }))
}

/// `GET /api/plans/{pid}/tasks/{t_pid}/time-analysis` — the per-task
/// analysis (spec §6).
#[debug_handler]
async fn task_time_analysis(
    State(ctx): State<AppContext>,
    Path((pid, t_pid)): Path<(String, String)>,
) -> Result<Response> {
    let now = chrono::Utc::now();
    let item = super::governance::find_item(&ctx, &pid).await?;
    let task = find_task(&ctx, item.pid, &t_pid).await?;
    let rows = task_transitions::Entity::find()
        .filter(task_transitions::Column::TaskPid.eq(task.pid))
        .order_by_asc(task_transitions::Column::At)
        .limit(MAX_TRANSITIONS)
        .all(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;
    let transitions: Vec<tba::Transition> = rows.iter().map(tba::to_transition).collect();
    let classes = tba::classes_in_force();
    let analysis = tba::analyze(
        &transitions,
        clock_of(&task),
        &classes,
        now.timestamp_millis(),
    );
    format::json(serde_json::json!({
        "as_of": now,
        "task": task_ref(&task),
        "note": "cycle_time is what the team controls (first started → \
                 finished); lead_time is what the requester waits (created → \
                 finished). They are different numbers and the difference is \
                 the backlog dwell — quoting the first as delivery time is the \
                 commonest misreport in flow measurement. flow_efficiency is \
                 work over cycle time; by_status and by_category partition the \
                 lead time, so nothing is lost.",
        "classification": classes_note(&classes),
        "analysis": analysis,
    }))
}

/// Plan-level query parameters.
#[derive(Debug, serde::Deserialize)]
struct PlanQuery {
    /// The service-level-expectation percentile (default 0.85).
    #[serde(default)]
    sle_percentile: Option<f64>,
    /// A commitment to score against, in days.
    #[serde(default)]
    target_days: Option<f64>,
    /// Restrict to one sprint.
    #[serde(default)]
    sprint: Option<String>,
}

impl PlanQuery {
    /// The requested percentile, validated.
    fn percentile(&self) -> Result<f64> {
        match self.sle_percentile {
            None => Ok(DEFAULT_SLE_PERCENTILE),
            Some(p) if p.is_finite() && (0.0..=1.0).contains(&p) => Ok(p),
            Some(_) => Err(refuse("sle_percentile must be between 0 and 1")),
        }
    }

    /// The sprint filter, parsed.
    fn sprint(&self) -> Result<Option<Uuid>> {
        self.sprint
            .as_deref()
            .map(|raw| Uuid::parse_str(raw).map_err(|_| refuse("sprint must be a uuid")))
            .transpose()
    }
}

/// `GET /api/plans/{pid}/time-analysis` — the plan cohort (spec §7).
#[debug_handler]
async fn plan_time_analysis(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
    Query(query): Query<PlanQuery>,
) -> Result<Response> {
    let now = chrono::Utc::now();
    let percentile = query.percentile()?;
    let sprint = query.sprint()?;
    if query
        .target_days
        .is_some_and(|d| !d.is_finite() || d <= 0.0)
    {
        return Err(refuse("target_days must be a positive number"));
    }
    let item = super::governance::find_item(&ctx, &pid).await?;
    let (rows, transitions) = load_board(&ctx, item.pid, sprint).await?;
    let classes = tba::classes_in_force();
    let paired = analyze_board(&rows, &transitions, &classes, now.timestamp_millis());
    let analyses: Vec<tba::TaskAnalysis> = paired.iter().map(|(_, a)| a.clone()).collect();
    let summary = tba::plan(&analyses);

    let cycle_times: Vec<i64> = analyses
        .iter()
        .filter(|a| a.finished)
        .filter_map(|a| a.cycle_time_ms)
        .collect();
    let sle = tba::service_level_expectation(&cycle_times, percentile, query.target_days);

    format::json(serde_json::json!({
        "as_of": now,
        "plan": plan_ref(&item),
        "note": "cycle-time percentiles are nearest-rank, so every one is an \
                 observed item. The lead-time distribution is always reported \
                 beside it, so the flattering number cannot travel alone. \
                 Throughput is reported beside rolled_first_pass_yield for the \
                 same reason: throughput rising while yield falls is not going \
                 faster, it is shipping work back to yourself.",
        "classification": classes_note(&classes),
        "service_level_expectation": sle,
        "plan_analysis": summary,
    }))
}

/// `GET /api/plans/{pid}/constraints` — the ranked constraints
/// (spec §8).
#[debug_handler]
async fn plan_constraints(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
    Query(query): Query<PlanQuery>,
) -> Result<Response> {
    let now = chrono::Utc::now();
    let sprint = query.sprint()?;
    let item = super::governance::find_item(&ctx, &pid).await?;
    let (rows, transitions) = load_board(&ctx, item.pid, sprint).await?;
    let classes = tba::classes_in_force();
    let paired = analyze_board(&rows, &transitions, &classes, now.timestamp_millis());
    let analyses: Vec<tba::TaskAnalysis> = paired.iter().map(|(_, a)| a.clone()).collect();
    let summary = tba::plan(&analyses);
    let findings = tba::constraints(&analyses, &summary);
    format::json(serde_json::json!({
        "as_of": now,
        "plan": plan_ref(&item),
        "note": "findings ordered by recoverable time; each names the rule that \
                 produced it and the threshold that fired. Deliberately not a \
                 composite score, and deliberately never per-person.",
        "classification": classes_note(&classes),
        "tasks": summary.tasks,
        "findings": findings,
    }))
}

/// `GET /api/plans/{pid}/aging-wip` — open items ranked by age against
/// the service level expectation (spec §8).
///
/// The only view here about work that can still be helped: cycle time,
/// throughput and WIP are all history, while an item's age is a fact
/// about today.
#[debug_handler]
async fn aging_wip(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
    Query(query): Query<PlanQuery>,
) -> Result<Response> {
    let now = chrono::Utc::now();
    let percentile = query.percentile()?;
    let sprint = query.sprint()?;
    let item = super::governance::find_item(&ctx, &pid).await?;
    let (rows, transitions) = load_board(&ctx, item.pid, sprint).await?;
    let classes = tba::classes_in_force();
    let paired = analyze_board(&rows, &transitions, &classes, now.timestamp_millis());

    let cycle_times: Vec<i64> = paired
        .iter()
        .filter(|(_, a)| a.finished)
        .filter_map(|(_, a)| a.cycle_time_ms)
        .collect();
    let sle = tba::service_level_expectation(&cycle_times, percentile, None);

    let mut open: Vec<serde_json::Value> = paired
        .iter()
        .filter_map(|(task, analysis)| {
            analysis.age_ms.map(|age| {
                let scored = tba::aging(age, sle.within_ms);
                serde_json::json!({
                    "task": task_ref(task),
                    "status": task.status,
                    "aging": scored,
                    "blocked_time_ms": analysis.blocked_time_ms,
                    "rework_count": analysis.rework_count,
                })
            })
        })
        .collect();
    open.sort_by_key(|row| std::cmp::Reverse(row["aging"]["age_ms"].as_i64().unwrap_or(0)));

    format::json(serde_json::json!({
        "as_of": now,
        "plan": plan_ref(&item),
        "note": "open items ranked by age, scored against the plan's own \
                 service level expectation. Items with no expectation to \
                 compare against are still listed, with a null ratio, rather \
                 than dropped.",
        "classification": classes_note(&classes),
        "service_level_expectation": sle,
        "aging": open,
    }))
}

/// Flow query parameters.
#[derive(Debug, serde::Deserialize)]
struct FlowQuery {
    #[serde(default)]
    window_days: Option<i64>,
}

/// `GET /api/plans/{pid}/flow` — queueing-theory flow (spec §9).
#[debug_handler]
async fn plan_flow(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
    Query(query): Query<FlowQuery>,
) -> Result<Response> {
    let now = chrono::Utc::now();
    let window_days = query.window_days.unwrap_or(DEFAULT_WINDOW_DAYS);
    if window_days <= 0 || window_days > 3650 {
        return Err(refuse("window_days must be between 1 and 3650"));
    }
    let since = now - chrono::Duration::days(window_days);
    let item = super::governance::find_item(&ctx, &pid).await?;
    let (rows, transitions) = load_board(&ctx, item.pid, None).await?;
    let classes = tba::classes_in_force();
    let paired = analyze_board(&rows, &transitions, &classes, now.timestamp_millis());

    let arrivals = rows.iter().filter(|t| t.created_at >= since).count();
    let finished_in_window: Vec<&(tasks::Model, tba::TaskAnalysis)> = paired
        .iter()
        .filter(|(task, _)| task.done_at.is_some_and(|at| at >= since))
        .collect();
    let work_in_progress = paired
        .iter()
        .filter(|(_, a)| !a.finished && a.cycle_time_ms.is_some())
        .count();

    let mut observed: Vec<i64> = finished_in_window
        .iter()
        .filter_map(|(_, a)| a.cycle_time_ms)
        .collect();
    observed.sort_unstable();
    let observed_p50 = tba::percentile(&observed, 0.50);

    let analysis = tba::flow(
        window_days,
        arrivals,
        finished_in_window.len(),
        work_in_progress,
        observed_p50,
    );

    // WIP against the configured caps: the one lever Little's Law
    // identifies, since cycle time = WIP / throughput.
    let limits = crate::engineering::parse_wip_limits(
        std::env::var("PROJECT_PORTFOLIO_MANAGEMENT_WIP_LIMITS")
            .ok()
            .as_deref(),
    );
    let occupancy: Vec<serde_json::Value> = crate::engineering::TASK_STATUSES
        .iter()
        .map(|status| {
            let count = rows.iter().filter(|t| t.status == *status).count();
            serde_json::json!({
                "status": status,
                "count": count,
                "limit": limits.as_ref().and_then(|l| l.get(*status)),
                "over_limit": limits
                    .as_ref()
                    .and_then(|l| l.get(*status))
                    .is_some_and(|cap| count > *cap),
            })
        })
        .collect();

    format::json(serde_json::json!({
        "as_of": now,
        "plan": plan_ref(&item),
        "window_since": since,
        "note": "Little's Law κ = λτ used as a consistency check on observed \
                 figures, not as a forecast — it assumes arrivals and \
                 departures balance over the window, so a short window on a \
                 volatile board should not be quoted. Column occupancy is \
                 reported here rather than in a separate capacity view because \
                 utilisation near 1 and long queues are the same observation: \
                 lowering a WIP cap shortens cycle time without anyone working \
                 faster.",
        "flow": analysis,
        "columns": occupancy,
    }))
}

/// Cumulative-flow query parameters.
#[derive(Debug, serde::Deserialize)]
struct CumulativeFlowQuery {
    /// How far back to sample (default 60).
    #[serde(default)]
    days: Option<i64>,
}

/// `GET /api/plans/{pid}/cumulative-flow` — the board's composition
/// sampled daily across a window (spec §10.2).
///
/// Served here rather than assembled in the browser because it needs
/// every task's whole history at once: an API that shipped the log to
/// the client to re-derive this would be sending far more data to
/// compute what the server already indexes.
#[debug_handler]
async fn cumulative_flow(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
    Query(query): Query<CumulativeFlowQuery>,
) -> Result<Response> {
    let now = chrono::Utc::now();
    let days = query.days.unwrap_or(60);
    if days <= 0 || days > 365 {
        return Err(refuse("days must be between 1 and 365"));
    }
    let item = super::governance::find_item(&ctx, &pid).await?;
    let (rows, transitions) = load_board(&ctx, item.pid, None).await?;

    let mut per_task: std::collections::HashMap<Uuid, Vec<tba::Transition>> =
        std::collections::HashMap::new();
    for row in &transitions {
        per_task
            .entry(row.task_pid)
            .or_default()
            .push(tba::to_transition(row));
    }
    let histories: Vec<tba::TaskHistory> = rows
        .iter()
        .map(|task| tba::TaskHistory {
            created_ms: task.created_at.timestamp_millis(),
            transitions: per_task.get(&task.pid).cloned().unwrap_or_default(),
        })
        .collect();

    let to_ms = now.timestamp_millis();
    let from_ms = to_ms.saturating_sub(days.saturating_mul(tba::DAY_MS));
    let samples = tba::cumulative_flow(&histories, from_ms, to_ms, tba::DAY_MS);

    format::json(serde_json::json!({
        "as_of": now,
        "plan": plan_ref(&item),
        "days": days,
        "note": "the board's composition sampled daily. Every status band is                  present at every sample, including at zero, so a stacked chart                  never has to decide whether a missing band means zero. A task                  does not appear before it was created, and one whose history                  predates its first recorded transition reads as `todo` rather                  than vanishing and reappearing. The vertical gap between the                  total and the done band is work in progress; its width is                  approximately the cycle time, which is Little's Law read off                  the chart.",
        "classification": classes_note(&tba::classes_in_force()),
        "samples": samples,
    }))
}

/// Rollup query parameters.
#[derive(Debug, serde::Deserialize)]
struct RollupQuery {
    /// Cap the containment depth walked (default and maximum 32).
    #[serde(default)]
    depth: Option<usize>,
}

/// `GET /api/plans/{pid}/rollup` — flow across a plan and everything it
/// contains (spec §15 TBA-9).
///
/// **The aggregate is the union of the descendants' tasks, not the
/// average of their ratios** — the same reasoning as §7.2: an average
/// of ratios weights a five-task plan equally with a five-hundred-task
/// one. The per-plan table is returned alongside, and for a portfolio
/// it is usually the more useful half: a rollup mixes boards whose
/// teams mean different things by `in_progress`, so *which child is
/// different* is a firmer finding than the combined number.
#[debug_handler]
async fn rollup(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
    Query(query): Query<RollupQuery>,
) -> Result<Response> {
    let now = chrono::Utc::now();
    let as_of_ms = now.timestamp_millis();
    let depth = query.depth.unwrap_or(tba::MAX_ROLLUP_DEPTH);
    if depth == 0 || depth > tba::MAX_ROLLUP_DEPTH {
        return Err(refuse(&format!(
            "depth must be between 1 and {}",
            tba::MAX_ROLLUP_DEPTH
        )));
    }
    let root = super::governance::find_item(&ctx, &pid).await?;

    // One query for the whole containment map, so the walk costs no
    // queries at all — a query per level is the N+1 this avoids.
    let all = plans::Entity::find()
        .filter(plans::Column::DeletedAt.is_null())
        .limit(MAX_PLANS_SCANNED)
        .all(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;
    let mut children: std::collections::BTreeMap<Uuid, Vec<Uuid>> =
        std::collections::BTreeMap::new();
    for plan in &all {
        if let Some(parent) = plan.parent_pid {
            children.entry(parent).or_default().push(plan.pid);
        }
    }
    let names: std::collections::HashMap<Uuid, &str> = all
        .iter()
        .map(|plan| (plan.pid, plan.name.as_str()))
        .collect();

    let walk = tba::walk_descendants(&children, root.pid, tba::MAX_ROLLUP_NODES, depth);
    let classes = tba::classes_in_force();

    // Per plan: its own board only. The union is assembled from the
    // same analyses, so the two halves cannot disagree.
    let mut per_plan: Vec<serde_json::Value> = Vec::with_capacity(walk.nodes.len());
    let mut union: Vec<tba::TaskAnalysis> = Vec::new();
    for node in &walk.nodes {
        let (rows, transitions) = load_board(&ctx, node.pid, None).await?;
        let paired = analyze_board(&rows, &transitions, &classes, as_of_ms);
        let analyses: Vec<tba::TaskAnalysis> = paired.into_iter().map(|(_, a)| a).collect();
        let summary = tba::plan(&analyses);
        let cycle_times: Vec<i64> = analyses
            .iter()
            .filter(|a| a.finished)
            .filter_map(|a| a.cycle_time_ms)
            .collect();
        let sle = tba::service_level_expectation(&cycle_times, DEFAULT_SLE_PERCENTILE, None);
        per_plan.push(serde_json::json!({
            "plan": {
                "pid": node.pid,
                "name": names.get(&node.pid).copied().unwrap_or_default(),
            },
            "depth": node.depth,
            "tasks": summary.tasks,
            "finished": summary.finished,
            "work_in_progress": summary.work_in_progress,
            "flow_efficiency": summary.aggregate_flow_efficiency,
            "rolled_first_pass_yield": summary.rolled_first_pass_yield,
            "service_level_expectation": sle,
        }));
        union.extend(analyses);
    }

    let combined = tba::plan(&union);
    let union_cycle_times: Vec<i64> = union
        .iter()
        .filter(|a| a.finished)
        .filter_map(|a| a.cycle_time_ms)
        .collect();
    let combined_sle =
        tba::service_level_expectation(&union_cycle_times, DEFAULT_SLE_PERCENTILE, None);

    format::json(serde_json::json!({
        "as_of": now,
        "root": plan_ref(&root),
        "note": "the combined figures are the union of every task under this                  plan, not an average of the children's ratios — averaging                  would weight a five-task plan equally with a five-hundred-task                  one. For a portfolio the per-plan table is usually the more                  useful half: a rollup mixes boards whose teams mean different                  things by `in_progress`, so which child differs is a firmer                  finding than the combined number.",
        "classification": classes_note(&classes),
        "tree": {
            "plans": walk.nodes.len(),
            "max_depth": walk.nodes.iter().map(|n| n.depth).max().unwrap_or(0),
            "depth_limit": depth,
            "truncated": walk.truncated,
            "truncation_note": walk.truncated.then_some(
                "a depth or node cap stopped the walk: these figures cover part                  of the tree, not all of it"
            ),
            "revisits": walk.revisits,
            "revisit_note": (walk.revisits > 0).then_some(
                "a plan was reached by more than one path. Containment should be                  a tree — a non-zero count means the data holds a cycle or a                  shared child that the write path should have refused. Each                  plan is still counted once."
            ),
        },
        "combined": combined,
        "combined_service_level_expectation": combined_sle,
        "by_plan": per_plan,
    }))
}

/// Forecast query parameters.
#[derive(Debug, serde::Deserialize)]
struct ForecastQuery {
    /// How many items to forecast a completion date for.
    #[serde(default)]
    items: Option<usize>,
    /// How many periods ahead to forecast a delivered count for.
    #[serde(default)]
    periods: Option<usize>,
    /// Periods of history to sample (default 12).
    #[serde(default)]
    history_periods: Option<usize>,
    /// Days per period (default 7).
    #[serde(default)]
    period_days: Option<i64>,
    /// Simulation trials (default 10 000, capped).
    #[serde(default)]
    trials: Option<usize>,
    /// Seed, so a caller can vary the draw deliberately.
    #[serde(default)]
    seed: Option<u64>,
}

/// `GET /api/plans/{pid}/forecast` — Monte-Carlo delivery forecasting
/// (spec §15 TBA-11).
///
/// Answers both directions at once, because quoting one without the
/// other is how a forecast gets misread: *how long will N items take*
/// and *how many will land in N periods*. Both sample the plan's own
/// **throughput** history — not its cycle-time distribution, which
/// answers a question about one item and would be roughly a factor of
/// the team's parallelism too pessimistic for a batch.
#[debug_handler]
async fn forecast(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
    Query(query): Query<ForecastQuery>,
) -> Result<Response> {
    let now = chrono::Utc::now();
    let period_days = query.period_days.unwrap_or(tba::DEFAULT_PERIOD_DAYS);
    if period_days <= 0 || period_days > 90 {
        return Err(refuse("period_days must be between 1 and 90"));
    }
    let history_periods = query.history_periods.unwrap_or(12);
    if history_periods == 0 || history_periods > 260 {
        return Err(refuse("history_periods must be between 1 and 260"));
    }
    let trials = query.trials.unwrap_or(tba::DEFAULT_TRIALS);
    if trials > tba::MAX_TRIALS {
        return Err(refuse(&format!(
            "trials must be at most {}",
            tba::MAX_TRIALS
        )));
    }

    let item = super::governance::find_item(&ctx, &pid).await?;
    let (rows, transitions) = load_board(&ctx, item.pid, None).await?;
    let classes = tba::classes_in_force();
    let paired = analyze_board(&rows, &transitions, &classes, now.timestamp_millis());

    // Completion instants, from the tasks' own `done_at` stamps.
    let completed_ms: Vec<i64> = rows
        .iter()
        .filter_map(|task| task.done_at.map(|at| at.timestamp_millis()))
        .collect();
    let period_ms = period_days.saturating_mul(tba::DAY_MS);
    let to_ms = now.timestamp_millis();
    let from_ms = to_ms.saturating_sub(
        i64::try_from(history_periods)
            .unwrap_or(0)
            .saturating_mul(period_ms),
    );
    let history = tba::throughput_history(&completed_ms, from_ms, to_ms, period_ms);

    // What is left to do, so the default question is the useful one.
    let open = paired.iter().filter(|(_, a)| !a.finished).count();
    let items = query.items.unwrap_or(open);
    let periods = query.periods.unwrap_or(4);
    let seed = query.seed.unwrap_or(0x5EED_5EED_5EED_5EED);

    format::json(serde_json::json!({
        "as_of": now,
        "plan": plan_ref(&item),
        "note": "both forecasts sample the plan's own throughput history — how                  many items it actually finished per period — rather than its                  cycle-time distribution. Cycle time answers a question about                  one item (that is the service level expectation); using it for                  a batch assumes items are worked one at a time, which for a                  team running several in parallel is pessimistic by roughly                  that factor. Sampling is with replacement, and the seed is                  fixed unless you pass one, so the same question gives the same                  answer.",
        "throughput_history": history,
        "open_items": open,
        "batch": tba::forecast_batch(&history, items, trials, period_days, seed),
        "horizon": tba::forecast_items(&history, periods, trials, period_days, seed),
    }))
}

/// `GET /api/flow-classes` — the classification map in force and the
/// vocabularies behind it (spec §5.3).
#[debug_handler]
async fn flow_classes() -> Result<Response> {
    let classes = tba::classes_in_force();
    format::json(serde_json::json!({
        "note": "the status → VSM category map every figure is computed with. \
                 `todo` is `inventory` waste rather than merely not-started: \
                 work bought and not yet used, aging while it waits. An \
                 unclassified status falls back to unnecessary_non_value_adding \
                 so that adding a board column cannot silently improve the flow \
                 efficiency. Override with \
                 PROJECT_PORTFOLIO_MANAGEMENT_FLOW_CLASSES; an unparsable or \
                 unknown-category override falls back whole rather than \
                 half-applying.",
        "classification": classes_note(&classes),
        "default": tba::default_classes(),
        "categories": tba::CATEGORIES,
        "board_order": tba::BOARD_ORDER,
        "finished_status": tba::FINISHED_STATUS,
        "backlog_status": tba::BACKLOG_STATUS,
        "blocked_status": tba::BLOCKED_STATUS,
        "minimum_sle_sample": tba::MIN_SLE_SAMPLE,
    }))
}

/// A `{pid, name}` reference for a plan.
fn plan_ref(item: &plans::Model) -> serde_json::Value {
    serde_json::json!({ "pid": item.pid, "name": item.name })
}

/// The time-based-analysis routes (all read-only GETs; transitions are
/// written by the existing task endpoints — spec §10.1).
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/flow-classes", get(flow_classes))
        .add(
            "/plans/{pid}/tasks/{t_pid}/transitions",
            get(list_transitions),
        )
        .add(
            "/plans/{pid}/tasks/{t_pid}/time-analysis",
            get(task_time_analysis),
        )
        .add("/plans/{pid}/time-analysis", get(plan_time_analysis))
        .add("/plans/{pid}/constraints", get(plan_constraints))
        .add("/plans/{pid}/aging-wip", get(aging_wip))
        .add("/plans/{pid}/flow", get(plan_flow))
        .add("/plans/{pid}/cumulative-flow", get(cumulative_flow))
        .add("/plans/{pid}/forecast", get(forecast))
        .add("/plans/{pid}/rollup", get(rollup))
}
