//! Oversight areas — read-only derived views for the governance
//! personas, composed from data the service already stores:
//!
//! - **Board** `/api/board/*` — the period-scoped board pack, the
//!   investment-decisions view, explicit estate snapshots, and the
//!   stored trend series (never re-derived history).
//! - **Auditor** `/api/auditor/*` — the estate-wide audit-trail
//!   explorer, segregation-of-duties findings, and the period-scoped
//!   evidence pack (JSON or CSV).
//! - **Compliance** `/api/compliance/*` — the compliance risk register
//!   and derived conformance findings (each with its rule disclosed).
//! - **CRO** `/api/risk/heatmap` — probability×impact matrix, posture
//!   per portfolio, concentration, review hygiene, and the declared
//!   risk appetite (absent ⇒ said so, never invented).
//! - **CISO** `/api/security/register` — security risks plus the
//!   disclosed no-security-risk-at-late-stage heuristic.
//! - **Regulator** `/api/regulator/extract` — deliberately coarse
//!   aggregates; honours the ABAC `mask` obligation by dropping
//!   portfolio names.
//!
//! Every GET is ETag-conditional with `as_of`, like the insight areas.
//! Persona gating is deployment configuration on the existing ABAC
//! guard (e.g. `dept=board` allow rules) — not new code here.

use std::fmt::Write as _;

use axum::http::HeaderMap;
use loco_rs::prelude::*;
use sea_orm::{QueryOrder, QuerySelect};
use serde::Deserialize;
use uuid::Uuid;

use super::insights::{category_risks, decision_entries, item_ref, rag_by_pid};
use authentication_verifier::Action;

use crate::auth::{MaybeAuthUser, authorize_record};
use crate::insights;
use crate::models::_entities::{
    audit_logs, benefits, budget_lines, gate_reviews, insight_snapshots, merge_records, milestones,
    plans, proposals, risks, scenarios,
};
use crate::models::visibility as vis_models;

/// Load all live rows of one entity (soft-deleted excluded).
macro_rules! live {
    ($module:ident, $db:expr) => {
        $module::Entity::find()
            .filter($module::Column::DeletedAt.is_null())
            .all($db)
            .await
            .map_err(|e| Error::Model(ModelError::from(e)))?
    };
}

/// A `?from=&to=` window (dates or RFC 3339 instants); defaults to the
/// trailing 90 days.
#[derive(Debug, Deserialize)]
struct WindowQuery {
    from: Option<String>,
    to: Option<String>,
}

/// Parse one boundary: `YYYY-MM-DD` (midnight UTC) or RFC 3339.
fn parse_instant(raw: &str, end_of_day: bool) -> Option<chrono::DateTime<chrono::Utc>> {
    if let Ok(instant) = raw.parse::<chrono::DateTime<chrono::Utc>>() {
        return Some(instant);
    }
    let date = raw.parse::<chrono::NaiveDate>().ok()?;
    let time = if end_of_day {
        chrono::NaiveTime::from_hms_opt(23, 59, 59)?
    } else {
        chrono::NaiveTime::MIN
    };
    Some(date.and_time(time).and_utc())
}

/// Resolve the window (default: trailing 90 days ending now).
fn window(query: &WindowQuery) -> (chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>) {
    let now = chrono::Utc::now();
    let to = query
        .to
        .as_deref()
        .and_then(|raw| parse_instant(raw, true))
        .unwrap_or(now);
    let from = query
        .from
        .as_deref()
        .and_then(|raw| parse_instant(raw, false))
        .unwrap_or(to - chrono::Days::new(90));
    (from, to)
}

/// Money lines for one set of budget rows.
fn money_lines(rows: &[&budget_lines::Model]) -> Vec<insights::MoneyLine> {
    rows.iter()
        .map(|b| insights::MoneyLine {
            currency: b.currency.clone(),
            planned_minor: b.planned_minor,
            actual_minor: b.actual_minor,
        })
        .collect()
}

// ─── Board ──────────────────────────────────────────────────────────────────

/// `GET /api/board/pack?from=&to=` — the period board pack: decisions
/// taken, benefits realized, milestones completed, and tranches
/// released **in the window**, plus the current portfolio health
/// summary (as-of-now; trends come from the stored snapshots).
#[debug_handler]
async fn board_pack(
    axum::extract::Query(query): axum::extract::Query<WindowQuery>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let (from, to) = window(&query);
    let items = live!(plans, &ctx.db);
    let risk_rows = live!(risks, &ctx.db);
    let budget_rows = live!(budget_lines, &ctx.db);
    let milestone_rows = live!(milestones, &ctx.db);
    let benefit_rows = live!(benefits, &ctx.db);
    let dependency_rows = vis_models::all_dependencies(&ctx.db).await?;
    let today = chrono::Utc::now().date_naive();
    let rag = rag_by_pid(&items, &risk_rows, &budget_rows, &dependency_rows, today);

    // Current portfolio health summary (worst member colour per bucket).
    let mut status_counts =
        std::collections::BTreeMap::from([("red", 0usize), ("amber", 0), ("green", 0)]);
    for portfolio in items
        .iter()
        .filter(|i| i.kind.as_deref() == Some("Portfolio"))
    {
        let mut worst = "green";
        for member in items
            .iter()
            .filter(|i| i.pid == portfolio.pid || i.parent_pid == Some(portfolio.pid))
        {
            let colour = rag[&member.pid];
            if colour == "red" || (colour == "amber" && worst == "green") {
                worst = colour;
            }
        }
        *status_counts.get_mut(worst).expect("fixed keys") += 1;
    }

    // Decisions in the window (shared builder, filtered).
    let decisions: Vec<serde_json::Value> = decision_entries(&ctx)
        .await?
        .into_iter()
        .filter(|(at, _)| *at >= from && *at <= to)
        .map(|(_, entry)| entry)
        .collect();

    // Benefits realized in the window: `benefit_realized` audit rows,
    // amounts from their snapshots, currency joined from the benefit.
    let currency_of: std::collections::BTreeMap<Uuid, Option<String>> = benefit_rows
        .iter()
        .map(|b| (b.pid, b.currency.clone()))
        .collect();
    let realized_rows = audit_logs::Entity::find()
        .filter(audit_logs::Column::Action.eq("benefit_realized"))
        .filter(audit_logs::Column::CreatedAt.gte(from))
        .filter(audit_logs::Column::CreatedAt.lte(to))
        .all(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;
    let mut realized_per_currency: std::collections::BTreeMap<String, i64> =
        std::collections::BTreeMap::new();
    let mut realized_unattributed = 0usize;
    for row in &realized_rows {
        let amount = row
            .snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.get("amount_minor"))
            .and_then(serde_json::Value::as_i64);
        match (amount, currency_of.get(&row.entity_pid).cloned().flatten()) {
            (Some(amount), Some(currency)) => {
                let entry = realized_per_currency.entry(currency).or_default();
                *entry = entry.saturating_add(amount);
            }
            _ => realized_unattributed += 1,
        }
    }

    let milestones_completed = milestone_rows
        .iter()
        .filter(|m| {
            m.done_at
                .is_some_and(|done| done.to_utc() >= from && done.to_utc() <= to)
        })
        .count();
    let released: Vec<&budget_lines::Model> = budget_rows
        .iter()
        .filter(|b| {
            b.released_at
                .is_some_and(|at| at.to_utc() >= from && at.to_utc() <= to)
        })
        .collect();
    let released_money = insights::variance_by_currency(&money_lines(&released));

    let body = serde_json::json!({
        "as_of": chrono::Utc::now(),
        "window": { "from": from, "to": to },
        "health_now": {
            "portfolios": status_counts,
            "note": "as-of-now; historic trends come from /api/board/trends (stored snapshots)",
        },
        "decisions": decisions,
        "benefits_realized": {
            "events": realized_rows.len(),
            "per_currency_minor": realized_per_currency,
            "unattributed_events": realized_unattributed,
        },
        "milestones_completed": milestones_completed,
        "tranches_released": {
            "count": released.len(),
            "per_currency": released_money,
        },
    });
    conditional(&headers, &body)
}

/// `GET /api/board/investments` — money-moving decisions, newest
/// first: scenario commits (with their caps), tranche releases (with
/// the passed gate), and approved / promoted proposals (with the
/// requested amount). Cap 100.
#[debug_handler]
async fn board_investments(State(ctx): State<AppContext>, headers: HeaderMap) -> Result<Response> {
    let items = live!(plans, &ctx.db);
    let by_pid: std::collections::BTreeMap<Uuid, &plans::Model> =
        items.iter().map(|i| (i.pid, i)).collect();
    let mut entries: Vec<(chrono::DateTime<chrono::Utc>, serde_json::Value)> = Vec::new();
    for scenario in live!(scenarios, &ctx.db) {
        if let Some(at) = scenario.committed_at {
            let at = at.to_utc();
            entries.push((
                at,
                serde_json::json!({
                    "kind": "scenario_commit", "at": at,
                    "name": scenario.name,
                    "budget_cap_minor": scenario.budget_cap_minor,
                    "currency": scenario.currency,
                }),
            ));
        }
    }
    for line in live!(budget_lines, &ctx.db) {
        if let Some(at) = line.released_at {
            let at = at.to_utc();
            entries.push((
                at,
                serde_json::json!({
                    "kind": "tranche_release", "at": at,
                    "description": line.description,
                    "gate": line.gate,
                    "planned_minor": line.planned_minor,
                    "currency": line.currency,
                    "item": by_pid.get(&line.plan_pid).map(|i| item_ref(i)),
                }),
            ));
        }
    }
    for proposal in live!(proposals, &ctx.db) {
        if matches!(proposal.status.as_str(), "approved" | "promoted") {
            let at = proposal.updated_at.to_utc();
            entries.push((
                at,
                serde_json::json!({
                    "kind": "proposal", "at": at,
                    "title": proposal.title,
                    "decision": proposal.status,
                    "requested_minor": proposal.requested_minor,
                    "currency": proposal.currency,
                }),
            ));
        }
    }
    entries.sort_by_key(|entry| std::cmp::Reverse(entry.0));
    let investments: Vec<serde_json::Value> =
        entries.into_iter().take(100).map(|(_, v)| v).collect();
    let body = serde_json::json!({
        "as_of": chrono::Utc::now(),
        "investments": investments,
    });
    conditional(&headers, &body)
}

/// `POST /api/board/snapshots` — capture one estate snapshot now
/// (portfolio counts, open exposure, per-currency money). The trend
/// series only ever contains what was actually captured.
#[debug_handler]
async fn take_snapshot(State(ctx): State<AppContext>, caller: MaybeAuthUser) -> Result<Response> {
    let row = crate::snapshots::capture(&ctx.db)
        .await
        .map_err(Error::Model)?;
    crate::models::audit_logs::Model::record(
        &ctx.db,
        Uuid::new_v4(),
        "snapshot_taken",
        caller.actor(),
        Some(serde_json::json!({ "snapshot_id": row.id })),
    )
    .await
    .ok();
    format::json(row)
}

/// Query for the trend series: optional result cap.
#[derive(Debug, Deserialize)]
struct TrendsQuery {
    /// Maximum snapshots returned (default 100, cap 500), oldest first.
    limit: Option<u64>,
}

/// `GET /api/board/trends` — the stored estate-snapshot series, oldest
/// first. No interpolation, no re-derived history.
#[debug_handler]
async fn board_trends(
    axum::extract::Query(query): axum::extract::Query<TrendsQuery>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let limit = query.limit.unwrap_or(100).min(500);
    let rows = insight_snapshots::Entity::find()
        .filter(insight_snapshots::Column::Kind.eq("estate"))
        .order_by_asc(insight_snapshots::Column::TakenAt)
        .limit(limit)
        .all(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;
    let series: Vec<serde_json::Value> = rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "taken_at": row.taken_at.to_utc(),
                "body": row.body,
            })
        })
        .collect();
    let body = serde_json::json!({
        "as_of": chrono::Utc::now(),
        "note": "stored snapshots only (POST /api/board/snapshots or the env-gated ticker); no interpolated history",
        "series": series,
    });
    conditional(&headers, &body)
}

// ─── Auditor ────────────────────────────────────────────────────────────────

/// Query for the audit-trail explorer.
#[derive(Debug, Deserialize)]
struct TrailQuery {
    actor: Option<String>,
    action: Option<String>,
    /// Entity pid filter.
    entity: Option<String>,
    from: Option<String>,
    to: Option<String>,
    /// Maximum rows (default 100, cap 500), newest first.
    limit: Option<u64>,
}

/// `GET /api/auditor/trail` — the estate-wide audit explorer: filter
/// by actor / action / entity pid / window; rows newest first plus
/// integrity stats over the filtered set (per-day counts, actorless
/// count, distinct actors).
#[debug_handler]
async fn auditor_trail(
    axum::extract::Query(query): axum::extract::Query<TrailQuery>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let limit = query.limit.unwrap_or(100).min(500);
    let mut find = audit_logs::Entity::find();
    if let Some(actor) = query.actor.as_deref() {
        find = find.filter(audit_logs::Column::Actor.eq(actor));
    }
    if let Some(action) = query.action.as_deref() {
        find = find.filter(audit_logs::Column::Action.eq(action));
    }
    if let Some(entity) = query.entity.as_deref() {
        let pid = Uuid::parse_str(entity).map_err(|_| Error::NotFound)?;
        find = find.filter(audit_logs::Column::EntityPid.eq(pid));
    }
    if let Some(from) = query
        .from
        .as_deref()
        .and_then(|raw| parse_instant(raw, false))
    {
        find = find.filter(audit_logs::Column::CreatedAt.gte(from));
    }
    if let Some(to) = query.to.as_deref().and_then(|raw| parse_instant(raw, true)) {
        find = find.filter(audit_logs::Column::CreatedAt.lte(to));
    }
    let rows = find
        .order_by_desc(audit_logs::Column::CreatedAt)
        .limit(limit)
        .all(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;

    let mut per_day: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut actors: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    let mut actorless = 0usize;
    for row in &rows {
        *per_day
            .entry(row.created_at.to_utc().format("%Y-%m-%d").to_string())
            .or_default() += 1;
        match row.actor.as_deref() {
            Some(actor) => {
                actors.insert(actor);
            }
            None => actorless += 1,
        }
    }
    let body = serde_json::json!({
        "as_of": chrono::Utc::now(),
        "returned": rows.len(),
        "stats": {
            "per_day": per_day,
            "distinct_actors": actors.len(),
            "actorless": actorless,
        },
        "rows": rows,
    });
    conditional(&headers, &body)
}

/// `GET /api/auditor/findings` — segregation-of-duties and hygiene
/// findings, each carrying its rule. All derived from recorded audit
/// actors (one identifier space — bearer subjects), never from
/// cross-space comparisons.
#[debug_handler]
async fn auditor_findings(State(ctx): State<AppContext>, headers: HeaderMap) -> Result<Response> {
    let items = live!(plans, &ctx.db);
    let by_pid: std::collections::BTreeMap<Uuid, &plans::Model> =
        items.iter().map(|i| (i.pid, i)).collect();
    let audit_rows = audit_logs::Entity::find()
        .all(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;
    let line_item: std::collections::BTreeMap<Uuid, Uuid> = live!(budget_lines, &ctx.db)
        .iter()
        .map(|line| (line.pid, line.plan_pid))
        .collect();
    let mut findings: Vec<serde_json::Value> = Vec::new();

    // Rule 1: the same actor recorded a gate review AND a tranche
    // release on the same plan.
    let gate_actors: std::collections::BTreeSet<(Uuid, &str)> = audit_rows
        .iter()
        .filter(|row| row.action == "gate_reviewed")
        .filter_map(|row| row.actor.as_deref().map(|actor| (row.entity_pid, actor)))
        .collect();
    for row in audit_rows
        .iter()
        .filter(|row| row.action == "budget_line_released")
    {
        let (Some(actor), Some(item_pid)) = (row.actor.as_deref(), line_item.get(&row.entity_pid))
        else {
            continue;
        };
        if gate_actors.contains(&(*item_pid, actor)) {
            findings.push(serde_json::json!({
                "rule": "gate_and_release_same_actor",
                "detail": "the actor who recorded a gate review also released a funding tranche on the same item",
                "actor": actor,
                "item": by_pid.get(item_pid).map(|i| item_ref(i)),
            }));
        }
    }

    // Rule 2: the same actor created AND decided the same proposal.
    let created_by: std::collections::BTreeMap<Uuid, &str> = audit_rows
        .iter()
        .filter(|row| row.action == "proposal_created")
        .filter_map(|row| row.actor.as_deref().map(|actor| (row.entity_pid, actor)))
        .collect();
    for row in audit_rows.iter().filter(|row| {
        matches!(
            row.action.as_str(),
            "proposal_approved" | "proposal_rejected" | "proposal_promoted"
        )
    }) {
        if let (Some(actor), Some(creator)) =
            (row.actor.as_deref(), created_by.get(&row.entity_pid))
            && actor == *creator
        {
            findings.push(serde_json::json!({
                "rule": "propose_and_decide_same_actor",
                "detail": format!("the proposal's creator also recorded `{}`", row.action),
                "actor": actor,
                "proposal_pid": row.entity_pid,
            }));
        }
    }

    // Rule 3: merges recorded with no reason.
    for merge in merge_records::Entity::find()
        .all(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?
    {
        if merge.reason.as_deref().is_none_or(|r| r.trim().is_empty()) {
            findings.push(serde_json::json!({
                "rule": "merge_without_reason",
                "detail": "a record merge was performed with no recorded reason",
                "main_pid": merge.main_pid,
                "duplicate_pid": merge.duplicate_pid,
                "actor": merge.actor,
            }));
        }
    }

    // Rule 4: actor-less mutations (submitted without a verified token).
    let actorless = audit_rows.iter().filter(|row| row.actor.is_none()).count();

    let body = serde_json::json!({
        "as_of": chrono::Utc::now(),
        "note": "rules compare recorded audit actors (bearer subjects) only — \
                 never identities from other spaces (e.g. approver_ref URNs)",
        "findings": findings,
        "actorless_actions": actorless,
    });
    conditional(&headers, &body)
}

/// Query for the evidence pack: window + format.
#[derive(Debug, Deserialize)]
struct EvidenceQuery {
    from: Option<String>,
    to: Option<String>,
    /// `json` (default) or `csv` (flat audit rows).
    format: Option<String>,
}

/// `GET /api/auditor/evidence-pack?from=&to=&format=json|csv` — the
/// period's decisions plus its audit rows (cap 2000, oldest first) in
/// one bundle; `csv` flattens the audit rows for spreadsheet review.
#[debug_handler]
async fn evidence_pack(
    axum::extract::Query(query): axum::extract::Query<EvidenceQuery>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let win = WindowQuery {
        from: query.from.clone(),
        to: query.to.clone(),
    };
    let (from, to) = window(&win);
    let audit_rows = audit_logs::Entity::find()
        .filter(audit_logs::Column::CreatedAt.gte(from))
        .filter(audit_logs::Column::CreatedAt.lte(to))
        .order_by_asc(audit_logs::Column::CreatedAt)
        .limit(2000)
        .all(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;
    if query.format.as_deref() == Some("csv") {
        let mut csv = String::from("created_at,actor,action,entity_pid\n");
        for row in &audit_rows {
            let _ = writeln!(
                csv,
                "{},{},{},{}",
                row.created_at.to_utc().to_rfc3339(),
                crate::visibility::csv_field(row.actor.as_deref().unwrap_or("")),
                crate::visibility::csv_field(&row.action),
                row.entity_pid,
            );
        }
        return axum::response::Response::builder()
            .header("content-type", "text/csv; charset=utf-8")
            .body(axum::body::Body::from(csv))
            .map_err(|e| Error::Any(e.into()));
    }
    let decisions: Vec<serde_json::Value> = decision_entries(&ctx)
        .await?
        .into_iter()
        .filter(|(at, _)| *at >= from && *at <= to)
        .map(|(_, entry)| entry)
        .collect();
    format::json(serde_json::json!({
        "window": { "from": from, "to": to },
        "audit_rows": audit_rows,
        "decisions": decisions,
        "note": "audit rows capped at 2000 oldest-first; narrow the window for completeness",
    }))
}

// ─── Compliance ─────────────────────────────────────────────────────────────

/// `GET /api/compliance/register` — risks categorised `compliance`,
/// exposure-sorted, mirroring the technical-debt register.
#[debug_handler]
async fn compliance_register(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let body = risk_register_body(&ctx, "compliance").await?;
    conditional(&headers, &body)
}

/// Query for conformance findings: the gate-recency threshold.
#[derive(Debug, Deserialize)]
struct ConformanceQuery {
    /// Days an unfinished, target-dated item may go without a gate
    /// review before it is a finding (default 90, cap 365).
    review_days: Option<u32>,
}

/// `GET /api/compliance/findings?review_days=` — derived conformance
/// checks, each with its rule disclosed. A findings list — no
/// invented compliance score.
#[debug_handler]
async fn compliance_findings(
    axum::extract::Query(query): axum::extract::Query<ConformanceQuery>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let review_days = i64::from(query.review_days.unwrap_or(90).clamp(1, 365));
    let now = chrono::Utc::now();
    let today = now.date_naive();
    let items = live!(plans, &ctx.db);
    let risk_rows = live!(risks, &ctx.db);
    let review_rows = gate_reviews::Entity::find()
        .all(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;
    let mut findings: Vec<serde_json::Value> = Vec::new();

    // Rule 1: unfinished item past its target date with no gate review
    // in the last `review_days` days.
    let last_review: std::collections::BTreeMap<Uuid, chrono::DateTime<chrono::Utc>> = review_rows
        .iter()
        .fold(std::collections::BTreeMap::new(), |mut acc, review| {
            let at = review.decided_at.to_utc();
            let entry = acc.entry(review.plan_pid).or_insert(at);
            if at > *entry {
                *entry = at;
            }
            acc
        });
    for item in &items {
        let finished = super::visibility::is_finished(item);
        let target = item
            .data
            .get("target_date")
            .and_then(|v| v.as_str())
            .and_then(|s| crate::visibility::parse_flex_date(s, true));
        if !finished && target.is_some_and(|t| t < today) {
            let stale = last_review
                .get(&item.pid)
                .is_none_or(|at| (now - *at).num_days() > review_days);
            if stale {
                findings.push(serde_json::json!({
                    "rule": "overdue_item_without_recent_gate_review",
                    "detail": format!("past target with no gate review in {review_days} days"),
                    "item": item_ref(item),
                }));
            }
        }
    }

    // Rule 2: escalated risks with no owner.
    for risk in risk_rows
        .iter()
        .filter(|r| r.escalated_at.is_some() && r.owner_ref.is_none())
    {
        findings.push(serde_json::json!({
            "rule": "escalated_risk_without_owner",
            "detail": "an escalated risk has no recorded owner",
            "risk_pid": risk.pid,
            "title": risk.title,
        }));
    }

    // Rule 3: open/mitigating risks past their review date.
    for risk in risk_rows.iter().filter(|r| {
        matches!(r.status.as_str(), "open" | "mitigating")
            && r.review_date.is_some_and(|d| d < today)
    }) {
        findings.push(serde_json::json!({
            "rule": "risk_past_review_date",
            "detail": "an open risk is past its scheduled review date",
            "risk_pid": risk.pid,
            "title": risk.title,
            "review_date": risk.review_date,
        }));
    }

    // Rule 4: approving gate decisions recorded without an approver.
    for review in review_rows.iter().filter(|r| {
        matches!(r.decision.as_str(), "approved" | "approved_with_conditions")
            && r.approver_ref.is_none()
    }) {
        findings.push(serde_json::json!({
            "rule": "gate_approval_without_approver",
            "detail": "an approving gate decision carries no approver_ref",
            "review_pid": review.pid,
            "gate": review.gate,
        }));
    }

    let body = serde_json::json!({
        "as_of": now,
        "review_days": review_days,
        "findings": findings,
    });
    conditional(&headers, &body)
}

// ─── CRO ────────────────────────────────────────────────────────────────────

/// `GET /api/risk/heatmap` — the CRO view: probability×impact matrix
/// over open risks, top exposures, per-portfolio posture,
/// concentration (disclosed 25% threshold), review hygiene, and the
/// declared risk appetite (or an honest "not configured").
#[debug_handler]
#[allow(clippy::too_many_lines)] // one pass: matrix + posture + hygiene + appetite
async fn risk_heatmap(State(ctx): State<AppContext>, headers: HeaderMap) -> Result<Response> {
    let today = chrono::Utc::now().date_naive();
    let items = live!(plans, &ctx.db);
    let risk_rows = live!(risks, &ctx.db);
    let by_pid: std::collections::BTreeMap<Uuid, &plans::Model> =
        items.iter().map(|i| (i.pid, i)).collect();
    let open: Vec<&risks::Model> = risk_rows
        .iter()
        .filter(|r| matches!(r.status.as_str(), "open" | "mitigating"))
        .collect();

    let cells = insights::heatmap_cells(
        &open
            .iter()
            .map(|r| (r.probability, r.impact))
            .collect::<Vec<_>>(),
    );
    let mut top: Vec<&&risks::Model> = open.iter().collect();
    top.sort_by_key(|r| std::cmp::Reverse(r.probability * r.impact));
    let top_risks: Vec<serde_json::Value> = top
        .iter()
        .take(10)
        .map(|risk| {
            serde_json::json!({
                "pid": risk.pid, "title": risk.title,
                "exposure": risk.probability * risk.impact,
                "category": risk.category.as_deref().unwrap_or("delivery"),
                "item": by_pid.get(&risk.plan_pid).map(|i| item_ref(i)),
            })
        })
        .collect();

    // Posture per portfolio bucket.
    let bucket_of = |pid: &Uuid| -> Option<Uuid> {
        by_pid.get(pid).and_then(|item| {
            if item.kind.as_deref() == Some("Portfolio") {
                Some(item.pid)
            } else {
                item.parent_pid
            }
        })
    };
    let mut posture: std::collections::BTreeMap<Option<Uuid>, (i32, usize, usize)> =
        std::collections::BTreeMap::new();
    for risk in &risk_rows {
        let bucket = bucket_of(&risk.plan_pid);
        let entry = posture.entry(bucket).or_default();
        if matches!(risk.status.as_str(), "open" | "mitigating") {
            entry.0 += risk.probability * risk.impact;
        }
        if risk.escalated_at.is_some() {
            entry.1 += 1;
        }
        if risk.status == "materialised" {
            entry.2 += 1;
        }
    }
    let posture: Vec<serde_json::Value> = posture
        .iter()
        .map(|(bucket, (exposure, escalated, materialised))| {
            serde_json::json!({
                "portfolio": bucket.and_then(|pid| by_pid.get(&pid)).map(|i| item_ref(i)),
                "open_exposure": exposure,
                "escalated": escalated,
                "materialised": materialised,
            })
        })
        .collect();

    // Concentration: risks carrying >= 25% of estate open exposure.
    let estate_exposure: i32 = open.iter().map(|r| r.probability * r.impact).sum();
    let concentration: Vec<serde_json::Value> = open
        .iter()
        .filter(|r| estate_exposure > 0 && (r.probability * r.impact) * 4 >= estate_exposure)
        .map(|risk| {
            serde_json::json!({
                "pid": risk.pid, "title": risk.title,
                "exposure": risk.probability * risk.impact,
                "share_threshold": "25% of estate open exposure",
            })
        })
        .collect();

    // Hygiene: open risks past their review date.
    let overdue_reviews: Vec<serde_json::Value> = open
        .iter()
        .filter(|r| r.review_date.is_some_and(|d| d < today))
        .take(10)
        .map(|risk| {
            serde_json::json!({
                "pid": risk.pid, "title": risk.title, "review_date": risk.review_date,
            })
        })
        .collect();

    // Declared appetite (or an honest absence).
    let appetite = insights::parse_appetite(
        std::env::var("PROJECT_PORTFOLIO_MANAGEMENT_RISK_APPETITE")
            .ok()
            .as_deref(),
    );
    let breaches: Vec<serde_json::Value> = match &appetite {
        None => Vec::new(),
        Some(appetite) => {
            let mut breaches = Vec::new();
            if let Some(max) = appetite.max_open_exposure
                && estate_exposure > max
            {
                breaches.push(serde_json::json!({
                    "rule": "estate_open_exposure_above_appetite",
                    "exposure": estate_exposure, "max": max,
                }));
            }
            if let Some(max) = appetite.max_item_exposure {
                for risk in open.iter().filter(|r| r.probability * r.impact > max) {
                    breaches.push(serde_json::json!({
                        "rule": "risk_exposure_above_appetite",
                        "pid": risk.pid, "title": risk.title,
                        "exposure": risk.probability * risk.impact, "max": max,
                    }));
                }
            }
            breaches
        }
    };

    let body = serde_json::json!({
        "as_of": chrono::Utc::now(),
        "open_risks": open.len(),
        "estate_open_exposure": estate_exposure,
        "cells": cells,
        "top_risks": top_risks,
        "posture": posture,
        "concentration": concentration,
        "overdue_reviews": overdue_reviews,
        "appetite": appetite,
        "appetite_note": if appetite.is_some() {
            "declared via PROJECT_PORTFOLIO_MANAGEMENT_RISK_APPETITE"
        } else {
            "no risk appetite configured; no thresholds invented"
        },
        "breaches": breaches,
    });
    conditional(&headers, &body)
}

// ─── CISO ───────────────────────────────────────────────────────────────────

/// `GET /api/security/register` — risks categorised `security`, plus
/// the disclosed heuristic: items at `g3_delivery` or later with no
/// security-category risk ever recorded (a proxy for "no security
/// review happened", stated as such).
#[debug_handler]
async fn security_register(State(ctx): State<AppContext>, headers: HeaderMap) -> Result<Response> {
    let mut body = risk_register_body(&ctx, "security").await?;
    let items = live!(plans, &ctx.db);
    let risk_rows = live!(risks, &ctx.db);
    let with_security: std::collections::HashSet<Uuid> = risk_rows
        .iter()
        .filter(|r| r.category.as_deref() == Some("security"))
        .map(|r| r.plan_pid)
        .collect();
    let late = ["g3_delivery", "g4_launch", "g5_benefits"];
    let unreviewed: Vec<serde_json::Value> = items
        .iter()
        .filter(|item| {
            item.stage
                .as_deref()
                .is_some_and(|stage| late.contains(&stage))
                && !with_security.contains(&item.pid)
        })
        .map(|item| serde_json::json!({ "item": item_ref(item), "stage": item.stage }))
        .collect();
    if let Some(map) = body.as_object_mut() {
        map.insert(
            "unreviewed_at_late_stage".to_string(),
            serde_json::json!({
                "heuristic": "items at g3_delivery+ with no security-category risk ever recorded — a proxy, not proof of a missing review",
                "items": unreviewed,
            }),
        );
    }
    conditional(&headers, &body)
}

// ─── Regulator ──────────────────────────────────────────────────────────────

/// `GET /api/regulator/extract` — deliberately coarse aggregates per
/// portfolio: stage, member counts, gate-decision counts, per-currency
/// spend totals, and benefit claims. No person references, no
/// item-level budgets — coarseness is the design. Under enforcement,
/// an ABAC `mask` obligation additionally drops portfolio names.
#[debug_handler]
async fn regulator_extract(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    headers: HeaderMap,
) -> Result<Response> {
    // Record-level pass: read with no resource attrs; a policy may
    // attach the `mask` obligation (e.g. for dept=regulator tokens).
    let obligations = authorize_record(&caller, Action::Read, &std::collections::BTreeMap::new())
        .map_err(|(status, reason)| {
        Error::CustomError(
            status,
            loco_rs::controller::ErrorDetail::new("forbidden", reason),
        )
    })?;
    let masked = obligations.iter().any(|o| o == "mask");

    let items = live!(plans, &ctx.db);
    let budget_rows = live!(budget_lines, &ctx.db);
    let benefit_rows = live!(benefits, &ctx.db);
    let review_rows = gate_reviews::Entity::find()
        .all(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;

    let mut portfolios = Vec::new();
    for portfolio in items
        .iter()
        .filter(|i| i.kind.as_deref() == Some("Portfolio"))
    {
        let member_pids: std::collections::HashSet<Uuid> = items
            .iter()
            .filter(|i| i.pid == portfolio.pid || i.parent_pid == Some(portfolio.pid))
            .map(|i| i.pid)
            .collect();
        let mut members: std::collections::BTreeMap<&str, usize> =
            std::collections::BTreeMap::new();
        for item in items.iter().filter(|i| member_pids.contains(&i.pid)) {
            *members
                .entry(item.kind.as_deref().unwrap_or("unspecified"))
                .or_default() += 1;
        }
        let mut gates: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
        for review in review_rows
            .iter()
            .filter(|r| member_pids.contains(&r.plan_pid))
        {
            *gates.entry(review.decision.as_str()).or_default() += 1;
        }
        let lines: Vec<&budget_lines::Model> = budget_rows
            .iter()
            .filter(|b| member_pids.contains(&b.plan_pid))
            .collect();
        let spend = insights::variance_by_currency(&money_lines(&lines))
            .into_iter()
            .map(|row| {
                serde_json::json!({
                    "currency": row.currency,
                    "planned_minor": row.planned_minor,
                    "actual_minor": row.actual_minor,
                })
            })
            .collect::<Vec<_>>();
        let mut benefits_per_currency: std::collections::BTreeMap<&str, (i64, i64)> =
            std::collections::BTreeMap::new();
        for benefit in benefit_rows
            .iter()
            .filter(|b| member_pids.contains(&b.plan_pid))
        {
            if let Some(currency) = benefit.currency.as_deref() {
                let entry = benefits_per_currency.entry(currency).or_default();
                entry.0 = entry.0.saturating_add(benefit.target_minor.unwrap_or(0));
                entry.1 = entry.1.saturating_add(benefit.realized_minor);
            }
        }
        let benefits_view: Vec<serde_json::Value> = benefits_per_currency
            .iter()
            .map(|(currency, (target, realized))| {
                serde_json::json!({
                    "currency": currency,
                    "target_minor": target,
                    "realized_minor": realized,
                })
            })
            .collect();
        portfolios.push(serde_json::json!({
            "pid": portfolio.pid,
            "name": if masked { serde_json::Value::Null } else { serde_json::json!(portfolio.name) },
            "stage": portfolio.stage,
            "members": members,
            "gate_decisions": gates,
            "spend": spend,
            "benefits": benefits_view,
        }));
    }
    let body = serde_json::json!({
        "as_of": chrono::Utc::now(),
        "masked": masked,
        "note": "deliberately coarse: portfolio-level aggregates only — no person references, no item-level budgets; `mask` obligation additionally withholds names",
        "portfolios": portfolios,
    });
    conditional(&headers, &body)
}

// ─── Shared ─────────────────────────────────────────────────────────────────

/// The category risk-register body (compliance / security), mirroring
/// the technical-debt register's shape.
async fn risk_register_body(ctx: &AppContext, category: &str) -> Result<serde_json::Value> {
    let items = live!(plans, &ctx.db);
    let risk_rows = live!(risks, &ctx.db);
    let by_pid: std::collections::BTreeMap<Uuid, &plans::Model> =
        items.iter().map(|i| (i.pid, i)).collect();
    let mut rows = category_risks(&risk_rows, category);
    rows.sort_by_key(|r| std::cmp::Reverse(r.probability * r.impact));
    let mut statuses: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for risk in &rows {
        *statuses.entry(risk.status.as_str()).or_default() += 1;
    }
    let open_exposure: i32 = rows
        .iter()
        .filter(|r| matches!(r.status.as_str(), "open" | "mitigating"))
        .map(|r| r.probability * r.impact)
        .sum();
    let register: Vec<serde_json::Value> = rows
        .iter()
        .map(|risk| {
            serde_json::json!({
                "pid": risk.pid,
                "title": risk.title,
                "status": risk.status,
                "exposure": risk.probability * risk.impact,
                "escalated": risk.escalated_at.is_some(),
                "owner_ref": risk.owner_ref,
                "item": by_pid.get(&risk.plan_pid).map(|i| item_ref(i)),
            })
        })
        .collect();
    Ok(serde_json::json!({
        "as_of": chrono::Utc::now(),
        "note": format!("risks with category `{category}`; exposure = probability x impact"),
        "open_exposure": open_exposure,
        "statuses": statuses,
        "register": register,
    }))
}

/// ETag-conditional JSON response; the tag excludes `as_of`.
fn conditional(headers: &HeaderMap, body: &serde_json::Value) -> Result<Response> {
    let mut fingerprint = body.clone();
    if let Some(map) = fingerprint.as_object_mut() {
        map.remove("as_of");
    }
    super::conditional_json(headers, &super::etag_of(&fingerprint), body)
}

/// The oversight routes (read-only GETs plus the snapshot POST).
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/board/pack", get(board_pack))
        .add("/board/investments", get(board_investments))
        .add("/board/snapshots", post(take_snapshot))
        .add("/board/trends", get(board_trends))
        .add("/auditor/trail", get(auditor_trail))
        .add("/auditor/findings", get(auditor_findings))
        .add("/auditor/evidence-pack", get(evidence_pack))
        .add("/compliance/register", get(compliance_register))
        .add("/compliance/findings", get(compliance_findings))
        .add("/risk/heatmap", get(risk_heatmap))
        .add("/security/register", get(security_register))
        .add("/regulator/extract", get(regulator_extract))
}
