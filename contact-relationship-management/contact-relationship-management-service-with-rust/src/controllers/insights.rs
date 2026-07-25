//! Operational insight views — read-only derivations over recorded
//! facts, mirroring the family's honesty rules (per-currency never
//! merged, ratios carry their inputs, derivations disclosed in-band,
//! `as_of` + `ETag` via the dashboards helpers):
//!
//! - **Sales** — stale-deal aging (from `deal_stage_changed` audits),
//!   follow-up calendar + overdue list (activity `due_on`/`done`),
//!   pipeline-hygiene findings (rule-disclosed), the period executive
//!   pack, and the stored forecast-trend series (no interpolation).
//! - **Support** — the SLA breach register + per-assignee workload.
//! - **DPO** — consent coverage + withdrawals + per-source counts
//!   (from `consent_events`) and duplicate-contact hygiene (CRM-local
//!   rows sharing one `person_ref`; identity dedup stays upstream).

use axum::http::HeaderMap;
use loco_rs::prelude::*;
use sea_orm::{PaginatorTrait, QueryOrder};
use serde::Deserialize;
use uuid::Uuid;

use super::dashboards::conditional;
use crate::models::_entities::{
    accounts, activities, audit_logs, consent_events, contacts, deals, forecast_snapshots, leads,
    memberships, partnerships, pipeline_stages, tickets,
};
use crate::models::records;
use crate::rules::engagement as engagement_rules;

/// Load all live rows of one entity (soft-deleted excluded).
macro_rules! live {
    ($module:ident, $db:expr) => {
        $module::Entity::find()
            .filter($module::Column::DeletedAt.is_null())
            .all($db)
            .await?
    };
}

/// A `?days=` recency threshold (default `default`, clamped 1–365).
fn days_or(query_days: Option<u32>, default: u32) -> i64 {
    i64::from(query_days.unwrap_or(default).clamp(1, 365))
}

/// Query carrying an optional `days` threshold.
#[derive(Debug, Deserialize)]
struct DaysQuery {
    days: Option<u32>,
}

/// `GET /api/insights/stale-deals?days=` — open deals by days in their
/// current stage, oldest first. Stage entry = the newest
/// `deal_stage_changed` audit for the deal, else the deal's creation
/// (never moved) — the derivation is served.
#[debug_handler]
async fn stale_deals(
    axum::extract::Query(query): axum::extract::Query<DaysQuery>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let threshold = days_or(query.days, 14);
    let now = chrono::Utc::now();
    let open: Vec<deals::Model> = deals::Entity::find()
        .filter(deals::Column::DeletedAt.is_null())
        .filter(deals::Column::ClosedAt.is_null())
        .all(&ctx.db)
        .await?;
    let stage_names: std::collections::BTreeMap<Uuid, String> = live!(pipeline_stages, &ctx.db)
        .into_iter()
        .map(|stage| (stage.pid, stage.name))
        .collect();
    let moves = audit_logs::Entity::find()
        .filter(audit_logs::Column::Entity.eq("deal"))
        .filter(audit_logs::Column::Action.eq("deal_stage_changed"))
        .all(&ctx.db)
        .await?;
    let mut last_move: std::collections::BTreeMap<Uuid, chrono::DateTime<chrono::Utc>> =
        std::collections::BTreeMap::new();
    for row in &moves {
        let at = row.created_at.to_utc();
        let entry = last_move.entry(row.entity_pid).or_insert(at);
        if at > *entry {
            *entry = at;
        }
    }
    let mut rows: Vec<(i64, serde_json::Value)> = open
        .iter()
        .map(|deal| {
            let entered = last_move
                .get(&deal.pid)
                .copied()
                .unwrap_or_else(|| deal.created_at.to_utc());
            let days_in_stage = (now - entered).num_days();
            (
                days_in_stage,
                serde_json::json!({
                    "pid": deal.pid,
                    "name": deal.name,
                    "stage": stage_names.get(&deal.stage_pid),
                    "owner_ref": deal.owner_ref,
                    "amount_minor": deal.amount_minor,
                    "currency": deal.currency,
                    "days_in_stage": days_in_stage,
                    "stale": days_in_stage > threshold,
                }),
            )
        })
        .collect();
    rows.sort_by_key(|(days, _)| std::cmp::Reverse(*days));
    let stale_count = rows.iter().filter(|(days, _)| *days > threshold).count();
    Ok(conditional(
        &headers,
        serde_json::json!({
            "derivation": "stage entry = newest deal_stage_changed audit, else deal creation \
                           (never moved); stale = days_in_stage over the threshold",
            "threshold_days": threshold,
            "open_deals": rows.len(),
            "stale_deals": stale_count,
            "deals": rows.into_iter().map(|(_, v)| v).collect::<Vec<_>>(),
        }),
    ))
}

/// Query for the follow-ups view: optional activity-kind filter (the
/// renewals convention: log renewals as due-dated `task` activities
/// and filter here).
#[derive(Debug, Deserialize)]
struct FollowupsQuery {
    kind: Option<String>,
}

/// `GET /api/insights/followups?kind=` — activities carrying a
/// `due_on` and not `done`: the overdue list (oldest first, with age)
/// and the next 30 days, plus per-recorder counts (`actor_ref` is the
/// recorder — said, not assumed to be an assignee).
#[debug_handler]
async fn followups(
    axum::extract::Query(query): axum::extract::Query<FollowupsQuery>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let today = chrono::Utc::now().date_naive();
    let horizon = today + chrono::Days::new(30);
    let mut find = activities::Entity::find()
        .filter(activities::Column::DeletedAt.is_null())
        .filter(activities::Column::Done.eq(false))
        .filter(activities::Column::DueOn.is_not_null());
    if let Some(kind) = query.kind.as_deref() {
        find = find.filter(activities::Column::Kind.eq(kind));
    }
    let rows: Vec<activities::Model> = find.all(&ctx.db).await?;
    let view = |activity: &activities::Model| {
        serde_json::json!({
            "pid": activity.pid,
            "kind": activity.kind,
            "summary": activity.summary,
            "subject_kind": activity.subject_kind,
            "subject_pid": activity.subject_pid,
            "actor_ref": activity.actor_ref,
            "due_on": activity.due_on,
            "overdue_days": activity
                .due_on
                .filter(|due| *due < today)
                .map(|due| (today - due).num_days()),
        })
    };
    let mut overdue: Vec<&activities::Model> = rows
        .iter()
        .filter(|a| a.due_on.is_some_and(|due| due < today))
        .collect();
    overdue.sort_by_key(|a| a.due_on);
    let mut upcoming: Vec<&activities::Model> = rows
        .iter()
        .filter(|a| a.due_on.is_some_and(|due| due >= today && due <= horizon))
        .collect();
    upcoming.sort_by_key(|a| a.due_on);
    let mut per_recorder: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    for activity in &rows {
        let key = activity
            .actor_ref
            .clone()
            .unwrap_or_else(|| "unattributed".to_string());
        *per_recorder.entry(key).or_default() += 1;
    }
    Ok(conditional(
        &headers,
        serde_json::json!({
            "note": "open follow-ups only (due_on set, not done); actor_ref is the \
                     recording actor, not necessarily an assignee",
            "overdue": overdue.iter().map(|a| view(a)).collect::<Vec<_>>(),
            "upcoming_30d": upcoming.iter().map(|a| view(a)).collect::<Vec<_>>(),
            "open_by_recorder": per_recorder,
        }),
    ))
}

/// `GET /api/insights/pipeline-hygiene?days=` — rule-disclosed
/// findings over the open pipeline: no invented health score, just
/// what each rule matched.
#[debug_handler]
async fn pipeline_hygiene(
    axum::extract::Query(query): axum::extract::Query<DaysQuery>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let threshold = days_or(query.days, 14);
    let now = chrono::Utc::now();
    let today = now.date_naive();
    let open: Vec<deals::Model> = deals::Entity::find()
        .filter(deals::Column::DeletedAt.is_null())
        .filter(deals::Column::ClosedAt.is_null())
        .all(&ctx.db)
        .await?;
    let lead_rows: Vec<leads::Model> = live!(leads, &ctx.db);
    let activity_rows: Vec<activities::Model> = live!(activities, &ctx.db);
    let last_touch = |kind: &str, pid: Uuid| -> Option<chrono::DateTime<chrono::Utc>> {
        activity_rows
            .iter()
            .filter(|a| a.subject_kind == kind && a.subject_pid == pid)
            .map(|a| a.occurred_at.to_utc())
            .max()
    };
    let mut findings: Vec<serde_json::Value> = Vec::new();
    for deal in &open {
        let deal_ref = serde_json::json!({ "pid": deal.pid, "name": deal.name });
        if deal.amount_minor == 0 {
            findings.push(serde_json::json!({
                "rule": "open_deal_without_amount",
                "detail": "an open deal carries no amount (forecast blind spot)",
                "deal": deal_ref,
            }));
        }
        if deal.expected_close_on.is_none() {
            findings.push(serde_json::json!({
                "rule": "open_deal_without_expected_close",
                "detail": "an open deal has no expected close date",
                "deal": deal_ref,
            }));
        } else if deal.expected_close_on.is_some_and(|d| d < today) {
            findings.push(serde_json::json!({
                "rule": "open_deal_past_expected_close",
                "detail": "an open deal is past its expected close date",
                "deal": deal_ref,
                "expected_close_on": deal.expected_close_on,
            }));
        }
        let untouched = match last_touch("deal", deal.pid) {
            Some(at) => (now - at).num_days() > threshold,
            None => true,
        };
        if untouched {
            findings.push(serde_json::json!({
                "rule": "open_deal_without_recent_activity",
                "detail": format!("no recorded activity in {threshold} days"),
                "deal": deal_ref,
            }));
        }
    }
    for lead in lead_rows
        .iter()
        .filter(|l| matches!(l.status.as_str(), "new" | "contacted"))
    {
        let untouched = match last_touch("lead", lead.pid) {
            Some(at) => (now - at).num_days() > threshold,
            None => (now - lead.created_at.to_utc()).num_days() > threshold,
        };
        if untouched {
            findings.push(serde_json::json!({
                "rule": "unworked_lead",
                "detail": format!("a {} lead with no activity in {threshold} days", lead.status),
                "lead_pid": lead.pid,
                "score": lead.score,
            }));
        }
    }
    Ok(conditional(
        &headers,
        serde_json::json!({
            "threshold_days": threshold,
            "findings": findings,
        }),
    ))
}

/// A `?from=&to=` window (dates or RFC 3339; default trailing 30 days).
#[derive(Debug, Deserialize)]
struct WindowQuery {
    from: Option<String>,
    to: Option<String>,
}

/// Parse one window boundary.
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

/// `GET /api/insights/executive?from=&to=` — the period sales pack:
/// deals won/lost in the window (per-currency won value, never
/// merged; lost reasons verbatim), new leads, tickets opened /
/// resolved (from status audits), activities logged, campaigns
/// started, and consent withdrawals.
#[debug_handler]
#[allow(clippy::too_many_lines)] // one pass over the period's facts
async fn executive(
    axum::extract::Query(query): axum::extract::Query<WindowQuery>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
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
        .unwrap_or(to - chrono::Days::new(30));
    let in_window = |at: chrono::DateTime<chrono::Utc>| at >= from && at <= to;

    let deal_rows: Vec<deals::Model> = live!(deals, &ctx.db);
    let mut won = 0usize;
    let mut lost = 0usize;
    let mut won_value: std::collections::BTreeMap<String, i64> = std::collections::BTreeMap::new();
    let mut lost_reasons: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    for deal in deal_rows
        .iter()
        .filter(|d| d.closed_at.is_some_and(|at| in_window(at.to_utc())))
    {
        if deal.won {
            won += 1;
            let entry = won_value.entry(deal.currency.clone()).or_default();
            *entry = entry.saturating_add(deal.amount_minor);
        } else {
            lost += 1;
            let reason = deal
                .lost_reason
                .clone()
                .unwrap_or_else(|| "(no reason recorded)".to_string());
            *lost_reasons.entry(reason).or_default() += 1;
        }
    }
    let new_leads = leads::Entity::find()
        .filter(leads::Column::DeletedAt.is_null())
        .filter(leads::Column::CreatedAt.gte(from))
        .filter(leads::Column::CreatedAt.lte(to))
        .count(&ctx.db)
        .await?;
    let tickets_opened = tickets::Entity::find()
        .filter(tickets::Column::DeletedAt.is_null())
        .filter(tickets::Column::OpenedAt.gte(from))
        .filter(tickets::Column::OpenedAt.lte(to))
        .count(&ctx.db)
        .await?;
    let audit_rows = audit_logs::Entity::find()
        .filter(audit_logs::Column::CreatedAt.gte(from))
        .filter(audit_logs::Column::CreatedAt.lte(to))
        .all(&ctx.db)
        .await?;
    let tickets_resolved = audit_rows
        .iter()
        .filter(|row| row.entity == "ticket" && row.action == "resolved")
        .count();
    let campaigns_started = audit_rows
        .iter()
        .filter(|row| row.entity == "campaign" && row.action == "running")
        .count();
    let activities_logged = activities::Entity::find()
        .filter(activities::Column::DeletedAt.is_null())
        .filter(activities::Column::OccurredAt.gte(from))
        .filter(activities::Column::OccurredAt.lte(to))
        .count(&ctx.db)
        .await?;
    let consent_withdrawals = consent_events::Entity::find()
        .filter(consent_events::Column::Action.eq("withdrawn"))
        .filter(consent_events::Column::OccurredAt.gte(from))
        .filter(consent_events::Column::OccurredAt.lte(to))
        .count(&ctx.db)
        .await?;
    Ok(conditional(
        &headers,
        serde_json::json!({
            "window": { "from": from, "to": to },
            "deals_won": won,
            "deals_lost": lost,
            "won_value_by_currency_minor": won_value,
            "lost_reasons": lost_reasons,
            "new_leads": new_leads,
            "tickets_opened": tickets_opened,
            "tickets_resolved": tickets_resolved,
            "campaigns_started": campaigns_started,
            "activities_logged": activities_logged,
            "consent_withdrawals": consent_withdrawals,
            "note": "per-currency won value is never merged or converted; \
                     tickets_resolved / campaigns_started derive from status audits",
        }),
    ))
}

/// `GET /api/insights/forecast-trends` — the stored forecast-snapshot
/// series, oldest first (cap 200). History is only ever what
/// `POST /api/forecast/snapshot` actually captured — no interpolation.
#[debug_handler]
async fn forecast_trends(State(ctx): State<AppContext>, headers: HeaderMap) -> Result<Response> {
    let rows: Vec<forecast_snapshots::Model> = forecast_snapshots::Entity::find()
        .filter(forecast_snapshots::Column::DeletedAt.is_null())
        .order_by_asc(forecast_snapshots::Column::TakenOn)
        .all(&ctx.db)
        .await?;
    let series: Vec<serde_json::Value> = rows
        .iter()
        .take(200)
        .map(|row| serde_json::json!({ "taken_on": row.taken_on, "totals": row.totals }))
        .collect();
    Ok(conditional(
        &headers,
        serde_json::json!({
            "note": "stored snapshots only (POST /api/forecast/snapshot); no interpolated history",
            "series": series,
        }),
    ))
}

/// `GET /api/insights/sla` — the breach register (open/pending tickets
/// past a due stamp, most-overdue first) and the per-assignee
/// workload (open counts, due-within-4h at-risk counts — the window
/// is disclosed).
#[debug_handler]
async fn sla_register(State(ctx): State<AppContext>, headers: HeaderMap) -> Result<Response> {
    let now = chrono::Utc::now();
    let at_risk_horizon = now + chrono::Duration::hours(4);
    let open: Vec<tickets::Model> = tickets::Entity::find()
        .filter(tickets::Column::DeletedAt.is_null())
        .filter(tickets::Column::Status.is_in(["open", "pending"]))
        .all(&ctx.db)
        .await?;
    let mut breaches: Vec<(i64, serde_json::Value)> = Vec::new();
    let mut workload: std::collections::BTreeMap<String, (usize, usize, usize)> =
        std::collections::BTreeMap::new();
    for ticket in &open {
        let key = ticket
            .assignee_ref
            .clone()
            .unwrap_or_else(|| "unassigned".to_string());
        let entry = workload.entry(key).or_default();
        entry.0 += 1;
        let mut worst_overdue: Option<(&str, i64)> = None;
        for (which, due) in [
            ("first_response", ticket.first_response_due_at),
            ("resolution", ticket.resolution_due_at),
        ] {
            let Some(due) = due else { continue };
            let due = due.to_utc();
            if due < now {
                let hours = (now - due).num_hours();
                if worst_overdue.is_none_or(|(_, worst)| hours > worst) {
                    worst_overdue = Some((which, hours));
                }
            } else if due <= at_risk_horizon {
                entry.2 += 1;
            }
        }
        if let Some((which, hours)) = worst_overdue {
            entry.1 += 1;
            breaches.push((
                hours,
                serde_json::json!({
                    "pid": ticket.pid,
                    "title": ticket.title,
                    "priority": ticket.priority,
                    "status": ticket.status,
                    "assignee_ref": ticket.assignee_ref,
                    "breached": which,
                    "overdue_hours": hours,
                }),
            ));
        }
    }
    breaches.sort_by_key(|(hours, _)| std::cmp::Reverse(*hours));
    let workload_view: Vec<serde_json::Value> = workload
        .iter()
        .map(|(assignee, (open, breached, at_risk))| {
            serde_json::json!({
                "assignee_ref": assignee,
                "open": open,
                "breached": breached,
                "at_risk_4h": at_risk,
            })
        })
        .collect();
    Ok(conditional(
        &headers,
        serde_json::json!({
            "derivation": "breach = an open/pending ticket past a due stamp (worst deadline \
                           reported); at_risk = a deadline within the disclosed 4h window",
            "breaches": breaches.into_iter().map(|(_, v)| v).collect::<Vec<_>>(),
            "workload": workload_view,
        }),
    ))
}

/// `GET /api/insights/dpo?days=` — the data-protection view: consent
/// coverage (current per-contact state, counted verbatim),
/// withdrawals in the window, per-source consent-event counts, and
/// duplicate-contact hygiene (live contacts sharing one `person_ref`
/// — CRM-local hygiene; identity dedup stays upstream).
#[debug_handler]
async fn dpo(
    axum::extract::Query(query): axum::extract::Query<DaysQuery>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let window_days = days_or(query.days, 30);
    let now = chrono::Utc::now();
    let since = now - chrono::Days::new(u64::try_from(window_days).unwrap_or(30));
    let contact_rows: Vec<contacts::Model> = live!(contacts, &ctx.db);
    let mut coverage: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for contact in &contact_rows {
        *coverage
            .entry(contact.marketing_consent.as_str())
            .or_default() += 1;
    }
    let event_rows = consent_events::Entity::find().all(&ctx.db).await?;
    let withdrawals_in_window = event_rows
        .iter()
        .filter(|event| event.action == "withdrawn" && event.occurred_at.to_utc() >= since)
        .count();
    let mut per_source: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for event in &event_rows {
        *per_source.entry(event.source.as_str()).or_default() += 1;
    }
    // Duplicate hygiene: live contacts sharing a person_ref.
    let mut by_person: std::collections::BTreeMap<&str, Vec<&contacts::Model>> =
        std::collections::BTreeMap::new();
    for contact in &contact_rows {
        by_person
            .entry(contact.person_ref.as_str())
            .or_default()
            .push(contact);
    }
    let duplicates: Vec<serde_json::Value> = by_person
        .iter()
        .filter(|(_, group)| group.len() > 1)
        .map(|(person_ref, group)| {
            serde_json::json!({
                "person_ref": person_ref,
                "contacts": group
                    .iter()
                    .map(|c| serde_json::json!({ "pid": c.pid, "display_name": c.display_name }))
                    .collect::<Vec<_>>(),
            })
        })
        .collect();
    Ok(conditional(
        &headers,
        serde_json::json!({
            "note": "coverage counts each contact's current marketing_consent verbatim; \
                     duplicates are CRM-local rows sharing one person_ref — identity \
                     dedup stays upstream in the person service",
            "contacts": contact_rows.len(),
            "consent_coverage": coverage,
            "window_days": window_days,
            "withdrawals_in_window": withdrawals_in_window,
            "consent_events_by_source": per_source,
            "duplicate_person_refs": duplicates,
        }),
    ))
}

/// `GET /api/insights/cadence?days=` — relationship-cadence aging:
/// contacts and accounts by days since their last recorded touch
/// (account touch = a direct account activity **or** one on any of
/// its contacts — disclosed), plus no-next-touch coverage (rows with
/// no open `due_on` activity).
#[debug_handler]
#[allow(clippy::too_many_lines)] // one pass over contacts + accounts
async fn cadence(
    axum::extract::Query(query): axum::extract::Query<DaysQuery>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let threshold = days_or(query.days, 30);
    let now = chrono::Utc::now();
    let contact_rows: Vec<contacts::Model> = live!(contacts, &ctx.db);
    let account_rows: Vec<accounts::Model> = live!(accounts, &ctx.db);
    let activity_rows: Vec<activities::Model> = live!(activities, &ctx.db);

    let mut last_contact_touch: std::collections::BTreeMap<Uuid, chrono::DateTime<chrono::Utc>> =
        std::collections::BTreeMap::new();
    let mut last_account_touch: std::collections::BTreeMap<Uuid, chrono::DateTime<chrono::Utc>> =
        std::collections::BTreeMap::new();
    let mut has_next_contact: std::collections::HashSet<Uuid> = std::collections::HashSet::new();
    let account_of: std::collections::BTreeMap<Uuid, Option<Uuid>> = contact_rows
        .iter()
        .map(|c| (c.pid, c.account_pid))
        .collect();
    for activity in &activity_rows {
        let at = activity.occurred_at.to_utc();
        let bump = |map: &mut std::collections::BTreeMap<Uuid, chrono::DateTime<chrono::Utc>>,
                    pid: Uuid| {
            let entry = map.entry(pid).or_insert(at);
            if at > *entry {
                *entry = at;
            }
        };
        match activity.subject_kind.as_str() {
            "contact" => {
                bump(&mut last_contact_touch, activity.subject_pid);
                if let Some(Some(account_pid)) = account_of.get(&activity.subject_pid) {
                    bump(&mut last_account_touch, *account_pid);
                }
                if !activity.done && activity.due_on.is_some() {
                    has_next_contact.insert(activity.subject_pid);
                }
            }
            "account" => bump(&mut last_account_touch, activity.subject_pid),
            _ => {}
        }
    }
    let mut untouched_contacts: Vec<(i64, serde_json::Value)> = contact_rows
        .iter()
        .map(|contact| {
            let days = last_contact_touch.get(&contact.pid).map_or_else(
                || (now - contact.created_at.to_utc()).num_days(),
                |at| (now - *at).num_days(),
            );
            (
                days,
                serde_json::json!({
                    "pid": contact.pid,
                    "display_name": contact.display_name,
                    "stakeholder_role": contact.stakeholder_role,
                    "days_since_touch": days,
                    "has_next_touch": has_next_contact.contains(&contact.pid),
                }),
            )
        })
        .filter(|(days, _)| *days > threshold)
        .collect();
    untouched_contacts.sort_by_key(|(days, _)| std::cmp::Reverse(*days));
    let mut untouched_accounts: Vec<(i64, serde_json::Value)> = account_rows
        .iter()
        .map(|account| {
            let days = last_account_touch.get(&account.pid).map_or_else(
                || (now - account.created_at.to_utc()).num_days(),
                |at| (now - *at).num_days(),
            );
            (
                days,
                serde_json::json!({
                    "pid": account.pid,
                    "display_name": account.display_name,
                    "stakeholder_role": account.stakeholder_role,
                    "days_since_touch": days,
                }),
            )
        })
        .filter(|(days, _)| *days > threshold)
        .collect();
    untouched_accounts.sort_by_key(|(days, _)| std::cmp::Reverse(*days));
    let no_next_touch = contact_rows
        .iter()
        .filter(|c| !has_next_contact.contains(&c.pid))
        .count();
    Ok(conditional(
        &headers,
        serde_json::json!({
            "derivation": "touch = a recorded activity (account touch includes its \
                           contacts' activities); never-touched rows age from creation; \
                           no-next-touch = no open due-dated activity",
            "threshold_days": threshold,
            "untouched_contacts": untouched_contacts.into_iter().map(|(_, v)| v).collect::<Vec<_>>(),
            "untouched_accounts": untouched_accounts.into_iter().map(|(_, v)| v).collect::<Vec<_>>(),
            "contacts_without_next_touch": no_next_touch,
        }),
    ))
}

/// `GET /api/insights/engagement?days=` — the engagement workload:
/// touches per recorder per month and the per-kind mix over the
/// window, plus recorded-sentiment counts (recorded only — absent
/// sentiment is counted as unrecorded, never guessed).
#[debug_handler]
async fn engagement(
    axum::extract::Query(query): axum::extract::Query<DaysQuery>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let window = days_or(query.days, 90);
    let now = chrono::Utc::now();
    let since = now - chrono::Days::new(u64::try_from(window).unwrap_or(90));
    let rows: Vec<activities::Model> = live!(activities, &ctx.db);
    let mut per_recorder_month: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    let mut per_kind: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut sentiment: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    let mut total = 0usize;
    for activity in &rows {
        let at = activity.occurred_at.to_utc();
        if at < since || at > now {
            continue;
        }
        total += 1;
        let recorder = activity
            .actor_ref
            .clone()
            .unwrap_or_else(|| "unattributed".to_string());
        *per_recorder_month
            .entry(format!("{} {}", recorder, at.format("%Y-%m")))
            .or_default() += 1;
        *per_kind.entry(activity.kind.clone()).or_default() += 1;
        let key = activity
            .sentiment
            .clone()
            .unwrap_or_else(|| "unrecorded".to_string());
        *sentiment.entry(key).or_default() += 1;
    }
    Ok(conditional(
        &headers,
        serde_json::json!({
            "window_days": window,
            "touches": total,
            "per_recorder_month": per_recorder_month,
            "per_kind": per_kind,
            "sentiment": sentiment,
            "note": "sentiment counts recorded declarations only; unrecorded stays unrecorded",
        }),
    ))
}

/// `GET /api/insights/funnel?pipeline=` — stage-entry counts and
/// step conversion through one pipeline, derived from recorded stage
/// audits (`to_stage`): entered(first stage) = deals created into the
/// pipeline; entered(later) = distinct deals whose audits reached that
/// stage. Ratios carry numerator and denominator.
#[debug_handler]
async fn funnel(
    axum::extract::Query(query): axum::extract::Query<FunnelQuery>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let pipeline = records::find_pipeline(&ctx.db, records::parse_pid(&query.pipeline)?).await?;
    let mut stages: Vec<pipeline_stages::Model> = live!(pipeline_stages, &ctx.db)
        .into_iter()
        .filter(|stage| stage.pipeline_pid == pipeline.pid)
        .collect();
    stages.sort_by_key(|stage| stage.position);
    let pipeline_deals: Vec<deals::Model> = deals::Entity::find()
        .filter(deals::Column::DeletedAt.is_null())
        .filter(deals::Column::PipelinePid.eq(pipeline.pid))
        .all(&ctx.db)
        .await?;
    let deal_pids: std::collections::HashSet<Uuid> = pipeline_deals.iter().map(|d| d.pid).collect();
    let audit_rows = audit_logs::Entity::find()
        .filter(audit_logs::Column::Entity.eq("deal"))
        .filter(audit_logs::Column::Action.is_in(["deal_stage_changed", "deal_won", "deal_lost"]))
        .all(&ctx.db)
        .await?;
    // Distinct deals that ever entered each stage, from to_stage.
    let mut entered: std::collections::BTreeMap<Uuid, std::collections::HashSet<Uuid>> =
        std::collections::BTreeMap::new();
    for row in &audit_rows {
        if !deal_pids.contains(&row.entity_pid) {
            continue;
        }
        let Some(to_stage) = row
            .snapshot
            .as_ref()
            .and_then(|snap| snap.get("to_stage"))
            .and_then(|v| v.as_str())
            .and_then(|raw| Uuid::parse_str(raw).ok())
        else {
            continue;
        };
        entered.entry(to_stage).or_default().insert(row.entity_pid);
    }
    let mut previous: Option<usize> = None;
    let mut rows: Vec<serde_json::Value> = Vec::new();
    for (index, stage) in stages.iter().enumerate() {
        let count = if index == 0 {
            pipeline_deals.len()
        } else {
            entered
                .get(&stage.pid)
                .map_or(0, std::collections::HashSet::len)
        };
        let conversion = previous.map(|prev| {
            #[allow(clippy::cast_precision_loss)] // display ratio, not money math
            let value = if prev == 0 {
                serde_json::Value::Null
            } else {
                serde_json::json!(count as f64 / prev as f64)
            };
            serde_json::json!({
                "numerator": count,
                "denominator": prev,
                "value": value,
            })
        });
        rows.push(serde_json::json!({
            "stage": stage.name,
            "position": stage.position,
            "is_won": stage.is_won,
            "is_lost": stage.is_lost,
            "entered": count,
            "conversion_from_previous": conversion,
        }));
        if !stage.is_lost {
            previous = Some(count);
        }
    }
    Ok(conditional(
        &headers,
        serde_json::json!({
            "pipeline": { "pid": pipeline.pid, "name": pipeline.name },
            "derivation": "entered(first stage) = deals created into the pipeline; \
                           entered(later stages) = distinct deals with a recorded \
                           to_stage audit; conversion skips lost stages in the chain",
            "stages": rows,
        }),
    ))
}

/// Query for the funnel: the pipeline pid.
#[derive(Debug, Deserialize)]
struct FunnelQuery {
    pipeline: String,
}

/// `GET /api/insights/members?days=` — confederation member-account
/// health: contacts, last touch, open follow-ups, open tickets, and
/// membership state per account, with the silent list (no touch in
/// the window).
#[debug_handler]
#[allow(clippy::too_many_lines)] // one pass over the account estate
async fn members(
    axum::extract::Query(query): axum::extract::Query<DaysQuery>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let threshold = days_or(query.days, 30);
    let now = chrono::Utc::now();
    let account_rows: Vec<accounts::Model> = live!(accounts, &ctx.db);
    let contact_rows: Vec<contacts::Model> = live!(contacts, &ctx.db);
    let activity_rows: Vec<activities::Model> = live!(activities, &ctx.db);
    let ticket_rows: Vec<tickets::Model> = tickets::Entity::find()
        .filter(tickets::Column::DeletedAt.is_null())
        .filter(tickets::Column::Status.is_in(["open", "pending"]))
        .all(&ctx.db)
        .await?;
    let membership_rows: Vec<memberships::Model> = live!(memberships, &ctx.db);
    let membership_of: std::collections::BTreeMap<Uuid, &memberships::Model> =
        membership_rows.iter().map(|m| (m.account_pid, m)).collect();
    let account_of: std::collections::BTreeMap<Uuid, Option<Uuid>> = contact_rows
        .iter()
        .map(|c| (c.pid, c.account_pid))
        .collect();
    let mut last_touch: std::collections::BTreeMap<Uuid, chrono::DateTime<chrono::Utc>> =
        std::collections::BTreeMap::new();
    let mut open_followups: std::collections::BTreeMap<Uuid, usize> =
        std::collections::BTreeMap::new();
    for activity in &activity_rows {
        let at = activity.occurred_at.to_utc();
        let account_pid = match activity.subject_kind.as_str() {
            "account" => Some(activity.subject_pid),
            "contact" => account_of.get(&activity.subject_pid).copied().flatten(),
            _ => None,
        };
        let Some(account_pid) = account_pid else {
            continue;
        };
        let entry = last_touch.entry(account_pid).or_insert(at);
        if at > *entry {
            *entry = at;
        }
        if !activity.done && activity.due_on.is_some() {
            *open_followups.entry(account_pid).or_default() += 1;
        }
    }
    let mut open_tickets: std::collections::BTreeMap<Uuid, usize> =
        std::collections::BTreeMap::new();
    for ticket in &ticket_rows {
        if let Some(account_pid) = ticket.account_pid {
            *open_tickets.entry(account_pid).or_default() += 1;
        }
    }
    let mut rows: Vec<serde_json::Value> = Vec::new();
    let mut silent = 0usize;
    for account in &account_rows {
        let contacts_count = contact_rows
            .iter()
            .filter(|c| c.account_pid == Some(account.pid))
            .count();
        let days = last_touch.get(&account.pid).map_or_else(
            || (now - account.created_at.to_utc()).num_days(),
            |at| (now - *at).num_days(),
        );
        let is_silent = days > threshold;
        if is_silent {
            silent += 1;
        }
        let membership = membership_of.get(&account.pid).map(|m| {
            serde_json::json!({ "status": m.status, "joined_on": m.joined_on,
                                "renewal_on": m.renewal_on })
        });
        rows.push(serde_json::json!({
            "pid": account.pid,
            "display_name": account.display_name,
            "tier": account.tier,
            "stakeholder_role": account.stakeholder_role,
            "membership": membership,
            "contacts": contacts_count,
            "days_since_touch": days,
            "silent": is_silent,
            "open_followups": open_followups.get(&account.pid).copied().unwrap_or(0),
            "open_tickets": open_tickets.get(&account.pid).copied().unwrap_or(0),
        }));
    }
    rows.sort_by_key(|row| std::cmp::Reverse(row["days_since_touch"].as_i64().unwrap_or(0)));
    Ok(conditional(
        &headers,
        serde_json::json!({
            "derivation": "account touch includes its contacts' activities; \
                           never-touched accounts age from creation",
            "threshold_days": threshold,
            "accounts": rows,
            "silent_accounts": silent,
        }),
    ))
}

/// `GET /api/insights/consent-by-account?days=` — the DPO consent
/// rollup per member account: each account's contacts' current
/// consent states (verbatim) and withdrawals in the window.
#[debug_handler]
async fn consent_by_account(
    axum::extract::Query(query): axum::extract::Query<DaysQuery>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let window = days_or(query.days, 30);
    let now = chrono::Utc::now();
    let since = now - chrono::Days::new(u64::try_from(window).unwrap_or(30));
    let account_rows: Vec<accounts::Model> = live!(accounts, &ctx.db);
    let contact_rows: Vec<contacts::Model> = live!(contacts, &ctx.db);
    let event_rows = consent_events::Entity::find().all(&ctx.db).await?;
    let account_of: std::collections::BTreeMap<Uuid, Option<Uuid>> = contact_rows
        .iter()
        .map(|c| (c.pid, c.account_pid))
        .collect();
    let mut withdrawals: std::collections::BTreeMap<Uuid, usize> =
        std::collections::BTreeMap::new();
    for event in &event_rows {
        if event.action == "withdrawn"
            && event.occurred_at.to_utc() >= since
            && let Some(Some(account_pid)) = account_of.get(&event.contact_pid)
        {
            *withdrawals.entry(*account_pid).or_default() += 1;
        }
    }
    let rows: Vec<serde_json::Value> = account_rows
        .iter()
        .map(|account| {
            let mut coverage: std::collections::BTreeMap<&str, usize> =
                std::collections::BTreeMap::new();
            for contact in contact_rows
                .iter()
                .filter(|c| c.account_pid == Some(account.pid))
            {
                *coverage
                    .entry(contact.marketing_consent.as_str())
                    .or_default() += 1;
            }
            serde_json::json!({
                "pid": account.pid,
                "display_name": account.display_name,
                "consent_coverage": coverage,
                "withdrawals_in_window": withdrawals.get(&account.pid).copied().unwrap_or(0),
            })
        })
        .collect();
    Ok(conditional(
        &headers,
        serde_json::json!({
            "window_days": window,
            "note": "coverage counts each contact's current marketing_consent verbatim; \
                     contacts without an account are in the estate-wide /insights/dpo view",
            "accounts": rows,
        }),
    ))
}

/// `GET /api/insights/stakeholders` — the declared-stakeholder
/// register: per-role lists with cadence and consent per row, and the
/// power–interest grid over contacts with **both** scores declared
/// (undeclared counted, never guessed into a cell).
#[debug_handler]
async fn stakeholders(State(ctx): State<AppContext>, headers: HeaderMap) -> Result<Response> {
    let now = chrono::Utc::now();
    let contact_rows: Vec<contacts::Model> = live!(contacts, &ctx.db);
    let account_rows: Vec<accounts::Model> = live!(accounts, &ctx.db);
    let activity_rows: Vec<activities::Model> = live!(activities, &ctx.db);
    let mut last_touch: std::collections::BTreeMap<Uuid, chrono::DateTime<chrono::Utc>> =
        std::collections::BTreeMap::new();
    for activity in activity_rows.iter().filter(|a| a.subject_kind == "contact") {
        let at = activity.occurred_at.to_utc();
        let entry = last_touch.entry(activity.subject_pid).or_insert(at);
        if at > *entry {
            *entry = at;
        }
    }
    let mut by_role: std::collections::BTreeMap<&str, Vec<serde_json::Value>> =
        engagement_rules::STAKEHOLDER_ROLES
            .iter()
            .map(|r| (*r, Vec::new()))
            .collect();
    let mut grid: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut undeclared_contacts = 0usize;
    let mut ungridded = 0usize;
    for contact in &contact_rows {
        match contact.stakeholder_role.as_deref() {
            Some(role) => {
                let days = last_touch.get(&contact.pid).map_or_else(
                    || (now - contact.created_at.to_utc()).num_days(),
                    |at| (now - *at).num_days(),
                );
                if let Some(bucket) = by_role.get_mut(role) {
                    bucket.push(serde_json::json!({
                        "pid": contact.pid,
                        "display_name": contact.display_name,
                        "marketing_consent": contact.marketing_consent,
                        "influence": contact.influence,
                        "interest": contact.interest,
                        "days_since_touch": days,
                    }));
                }
                match (contact.influence, contact.interest) {
                    (Some(influence), Some(interest)) => {
                        *grid.entry(format!("p{influence}i{interest}")).or_default() += 1;
                    }
                    _ => ungridded += 1,
                }
            }
            None => undeclared_contacts += 1,
        }
    }
    let account_roles: Vec<serde_json::Value> = account_rows
        .iter()
        .filter_map(|account| {
            account.stakeholder_role.as_deref().map(|role| {
                serde_json::json!({
                    "pid": account.pid, "display_name": account.display_name, "role": role,
                })
            })
        })
        .collect();
    Ok(conditional(
        &headers,
        serde_json::json!({
            "note": "roles and grid scores are declared, never inferred; the grid \
                     covers stakeholders with both scores (key p<influence>i<interest>)",
            "by_role": by_role,
            "grid": grid,
            "stakeholders_without_grid_scores": ungridded,
            "undeclared_contacts": undeclared_contacts,
            "account_roles": account_roles,
        }),
    ))
}

/// `GET /api/insights/partnerships` — the innovation-partnership
/// register: per-kind and per-stage counts plus the live records with
/// their accounts.
#[debug_handler]
async fn partnerships_register(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let rows: Vec<partnerships::Model> = live!(partnerships, &ctx.db);
    let account_rows: Vec<accounts::Model> = live!(accounts, &ctx.db);
    let names: std::collections::BTreeMap<Uuid, &str> = account_rows
        .iter()
        .map(|a| (a.pid, a.display_name.as_str()))
        .collect();
    let mut by_kind: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    let mut by_stage: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for row in &rows {
        *by_kind.entry(row.kind.as_str()).or_default() += 1;
        *by_stage.entry(row.stage.as_str()).or_default() += 1;
    }
    let register: Vec<serde_json::Value> = rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "pid": row.pid,
                "account_pid": row.account_pid,
                "account": names.get(&row.account_pid),
                "kind": row.kind,
                "stage": row.stage,
                "summary": row.summary,
                "started_on": row.started_on,
            })
        })
        .collect();
    Ok(conditional(
        &headers,
        serde_json::json!({
            "by_kind": by_kind,
            "by_stage": by_stage,
            "register": register,
        }),
    ))
}

/// `GET /api/insights/memberships?days=` — membership renewals due
/// within the window plus the lapsed list.
#[debug_handler]
async fn memberships_view(
    axum::extract::Query(query): axum::extract::Query<DaysQuery>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let window = days_or(query.days, 90);
    let today = chrono::Utc::now().date_naive();
    let horizon = today + chrono::Days::new(u64::try_from(window).unwrap_or(90));
    let rows: Vec<memberships::Model> = live!(memberships, &ctx.db);
    let account_rows: Vec<accounts::Model> = live!(accounts, &ctx.db);
    let names: std::collections::BTreeMap<Uuid, &str> = account_rows
        .iter()
        .map(|a| (a.pid, a.display_name.as_str()))
        .collect();
    let view = |m: &memberships::Model| {
        serde_json::json!({
            "pid": m.pid,
            "account_pid": m.account_pid,
            "account": names.get(&m.account_pid),
            "status": m.status,
            "joined_on": m.joined_on,
            "renewal_on": m.renewal_on,
        })
    };
    let mut renewals_due: Vec<&memberships::Model> = rows
        .iter()
        .filter(|m| m.status == "active" && m.renewal_on.is_some_and(|d| d <= horizon))
        .collect();
    renewals_due.sort_by_key(|m| m.renewal_on);
    let lapsed: Vec<serde_json::Value> = rows
        .iter()
        .filter(|m| m.status == "lapsed")
        .map(view)
        .collect();
    Ok(conditional(
        &headers,
        serde_json::json!({
            "window_days": window,
            "memberships": rows.len(),
            "renewals_due": renewals_due.iter().map(|m| view(m)).collect::<Vec<_>>(),
            "lapsed": lapsed,
        }),
    ))
}

/// The insight routes (all read-only GETs).
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/insights/stale-deals", get(stale_deals))
        .add("/insights/followups", get(followups))
        .add("/insights/pipeline-hygiene", get(pipeline_hygiene))
        .add("/insights/executive", get(executive))
        .add("/insights/forecast-trends", get(forecast_trends))
        .add("/insights/sla", get(sla_register))
        .add("/insights/dpo", get(dpo))
        .add("/insights/cadence", get(cadence))
        .add("/insights/engagement", get(engagement))
        .add("/insights/funnel", get(funnel))
        .add("/insights/members", get(members))
        .add("/insights/consent-by-account", get(consent_by_account))
        .add("/insights/stakeholders", get(stakeholders))
        .add("/insights/partnerships", get(partnerships_register))
        .add("/insights/memberships", get(memberships_view))
}
