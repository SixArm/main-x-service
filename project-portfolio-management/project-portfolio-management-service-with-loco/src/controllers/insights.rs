//! Executive insight areas — read-only derived views for the three
//! C-suite personas, composed entirely from data the service already
//! stores (no new tables):
//!
//! - **CEO** `/api/executive/*` — per-portfolio health briefing (RAG
//!   rollup), the decision log (gate reviews, scenario commits,
//!   decided proposals, merges), and benefits realization.
//! - **CFO** `/api/financials/*` — budget variance (by collection,
//!   portfolio, and category) and per-currency exposure. Minor units;
//!   per-currency lines never merge; **no FX conversion**.
//! - **CTO** `/api/technology/*` — dependency risk (fan-out /
//!   cross-portfolio / red-predecessor edges) and the tag-encoded
//!   technology radar (`tech:<name>[:<ring>]`).
//!
//! Every response carries `as_of` and is ETag-conditional (the tag
//! excludes `as_of`), mirroring `/api/at-a-glance`. All endpoints are
//! GETs, so under the blanket ABAC guard they derive as `read`.

use axum::http::HeaderMap;
use loco_rs::prelude::*;
use serde::Deserialize;
use uuid::Uuid;

use super::visibility::{is_finished, schedule_item};
use crate::insights;
use crate::models::_entities::{
    benefits, budget_lines, gate_reviews, merge_records, milestones, objective_links, proposals,
    risks, scenarios, work_items,
};
use crate::models::visibility as vis_models;
use crate::visibility as rules;

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

/// The RAG colour of every live item, keyed by pid — the same
/// derivation `/api/at-a-glance` uses (materialised risks, budget
/// overrun, missed target, exposure, schedule violations).
pub(crate) fn rag_by_pid(
    items: &[work_items::Model],
    risk_rows: &[risks::Model],
    budget_rows: &[budget_lines::Model],
    dependency_rows: &[crate::models::_entities::work_item_dependencies::Model],
    today: chrono::NaiveDate,
) -> std::collections::BTreeMap<Uuid, &'static str> {
    let facts: Vec<rules::ScheduleItem> = items.iter().map(schedule_item).collect();
    let edges: Vec<rules::ScheduleEdge> = dependency_rows
        .iter()
        .map(|e| rules::ScheduleEdge {
            pid: e.pid,
            predecessor: e.predecessor_pid,
            successor: e.successor_pid,
            lag_days: e.lag_days,
        })
        .collect();
    let violated: std::collections::HashSet<Uuid> = rules::violations(&facts, &edges)
        .iter()
        .map(|v| v.successor)
        .collect();
    items
        .iter()
        .map(|item| {
            let member_risks: Vec<_> =
                risk_rows.iter().filter(|r| r.work_item_pid == item.pid).collect();
            let materialised = member_risks.iter().filter(|r| r.status == "materialised").count();
            let max_exposure = member_risks
                .iter()
                .filter(|r| matches!(r.status.as_str(), "open" | "mitigating"))
                .map(|r| r.probability * r.impact)
                .max()
                .unwrap_or(0);
            let member_budgets: Vec<_> = budget_rows
                .iter()
                .filter(|b| b.work_item_pid == item.pid)
                .map(|b| insights::MoneyLine {
                    currency: b.currency.clone(),
                    planned_minor: b.planned_minor,
                    actual_minor: b.actual_minor,
                })
                .collect();
            let overrun = insights::variance_by_currency(&member_budgets)
                .iter()
                .any(|row| row.overrun);
            let target = item
                .data
                .get("target_date")
                .and_then(|v| v.as_str())
                .and_then(|s| rules::parse_flex_date(s, true));
            let colour = rules::rag(
                is_finished(item),
                target,
                today,
                materialised,
                max_exposure,
                overrun,
                violated.contains(&item.pid),
            );
            (item.pid, colour)
        })
        .collect()
}

/// `{pid, name, kind}` reference for a work item.
pub(crate) fn item_ref(item: &work_items::Model) -> serde_json::Value {
    serde_json::json!({ "pid": item.pid, "name": item.name, "kind": item.kind })
}

/// The live risks carrying `category` (exposure-sort left to callers).
pub(crate) fn category_risks<'r>(
    risk_rows: &'r [risks::Model],
    category: &str,
) -> Vec<&'r risks::Model> {
    risk_rows
        .iter()
        .filter(|r| r.category.as_deref() == Some(category))
        .collect()
}

/// `GET /api/executive/health` — the CEO portfolio-health briefing:
/// one row per portfolio (plus an `unassigned` bucket) with a derived
/// RAG rollup, overdue milestones, escalated risks, open-risk
/// exposure, overrun currencies, and member counts. The bucket status
/// is the worst member colour. Derivation matches `/api/at-a-glance`.
#[debug_handler]
async fn executive_health(State(ctx): State<AppContext>, headers: HeaderMap) -> Result<Response> {
    let today = chrono::Utc::now().date_naive();
    let items = live!(work_items, &ctx.db);
    let risk_rows = live!(risks, &ctx.db);
    let budget_rows = live!(budget_lines, &ctx.db);
    let milestone_rows = live!(milestones, &ctx.db);
    let dependency_rows = vis_models::all_dependencies(&ctx.db).await?;
    let rag = rag_by_pid(&items, &risk_rows, &budget_rows, &dependency_rows, today);

    // Buckets: each portfolio item (itself + the items under it), plus
    // an `unassigned` bucket for parentless non-portfolio items.
    let mut buckets: Vec<(Option<&work_items::Model>, Vec<&work_items::Model>)> = items
        .iter()
        .filter(|i| i.kind == "Portfolio")
        .map(|p| {
            let mut members: Vec<&work_items::Model> =
                items.iter().filter(|i| i.portfolio_pid == Some(p.pid)).collect();
            members.push(p);
            (Some(p), members)
        })
        .collect();
    let unassigned: Vec<&work_items::Model> = items
        .iter()
        .filter(|i| i.kind != "Portfolio" && i.portfolio_pid.is_none())
        .collect();
    if !unassigned.is_empty() {
        buckets.push((None, unassigned));
    }

    let mut portfolios = Vec::new();
    for (portfolio, members) in &buckets {
        let member_pids: std::collections::HashSet<Uuid> = members.iter().map(|m| m.pid).collect();
        let mut rag_counts =
            std::collections::BTreeMap::from([("red", 0usize), ("amber", 0), ("green", 0)]);
        for member in members {
            *rag_counts.get_mut(rag[&member.pid]).expect("fixed keys") += 1;
        }
        let status = if rag_counts["red"] > 0 {
            "red"
        } else if rag_counts["amber"] > 0 {
            "amber"
        } else {
            "green"
        };
        let overdue_milestones = milestone_rows
            .iter()
            .filter(|m| member_pids.contains(&m.work_item_pid) && !m.done && m.due < today)
            .count();
        let escalated_risks = risk_rows
            .iter()
            .filter(|r| member_pids.contains(&r.work_item_pid) && r.escalated_at.is_some())
            .count();
        let open_risk_exposure: i32 = risk_rows
            .iter()
            .filter(|r| {
                member_pids.contains(&r.work_item_pid)
                    && matches!(r.status.as_str(), "open" | "mitigating")
            })
            .map(|r| r.probability * r.impact)
            .sum();
        let lines: Vec<insights::MoneyLine> = budget_rows
            .iter()
            .filter(|b| member_pids.contains(&b.work_item_pid))
            .map(|b| insights::MoneyLine {
                currency: b.currency.clone(),
                planned_minor: b.planned_minor,
                actual_minor: b.actual_minor,
            })
            .collect();
        let overrun_currencies: Vec<String> = insights::variance_by_currency(&lines)
            .into_iter()
            .filter(|row| row.overrun)
            .map(|row| row.currency)
            .collect();
        // Staleness: days since the newest member update (audit-visible
        // activity would be finer; member `updated_at` is the honest
        // floor the row store carries).
        let stale_days = members
            .iter()
            .map(|m| (chrono::Utc::now() - m.updated_at.to_utc()).num_days())
            .min()
            .unwrap_or(0);
        portfolios.push(serde_json::json!({
            "portfolio": portfolio.map(item_ref),
            "status": status,
            "rag": rag_counts,
            "members": members.len(),
            "overdue_milestones": overdue_milestones,
            "escalated_risks": escalated_risks,
            "open_risk_exposure": open_risk_exposure,
            "overrun_currencies": overrun_currencies,
            "days_since_last_update": stale_days,
        }));
    }
    let body = serde_json::json!({
        "as_of": chrono::Utc::now(),
        "derivation": "status = worst member RAG (at-a-glance rules); \
                       staleness = days since newest member update",
        "portfolios": portfolios,
    });
    conditional(&headers, &body)
}

/// Every recorded decision as `(instant, entry)` pairs: gate-review
/// verdicts, scenario commits, decided proposals, and merges. Shared
/// by the executive decision log, the board pack, and the evidence
/// pack.
pub(crate) async fn decision_entries(
    ctx: &AppContext,
) -> Result<Vec<(chrono::DateTime<chrono::Utc>, serde_json::Value)>> {
    let items = live!(work_items, &ctx.db);
    let names: std::collections::BTreeMap<Uuid, &str> =
        items.iter().map(|i| (i.pid, i.name.as_str())).collect();
    let mut entries: Vec<(chrono::DateTime<chrono::Utc>, serde_json::Value)> = Vec::new();

    for review in gate_reviews::Entity::find()
        .all(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?
    {
        let at = review.decided_at.to_utc();
        entries.push((at, serde_json::json!({
            "kind": "gate_review",
            "at": at,
            "decision": review.decision,
            "gate": review.gate,
            "subject": { "pid": review.work_item_pid,
                         "name": names.get(&review.work_item_pid) },
            "actor": review.approver_ref,
            "conditions": review.conditions,
        })));
    }
    for scenario in live!(scenarios, &ctx.db) {
        if let Some(committed_at) = scenario.committed_at {
            let at = committed_at.to_utc();
            entries.push((at, serde_json::json!({
                "kind": "scenario_commit",
                "at": at,
                "decision": "committed",
                "subject": { "pid": scenario.pid, "name": scenario.name },
            })));
        }
    }
    for proposal in live!(proposals, &ctx.db) {
        if matches!(proposal.status.as_str(), "approved" | "rejected" | "promoted") {
            let at = proposal.updated_at.to_utc();
            entries.push((at, serde_json::json!({
                "kind": "proposal",
                "at": at,
                "decision": proposal.status,
                "subject": { "pid": proposal.pid, "name": proposal.title },
                "sponsor": proposal.sponsor_ref,
            })));
        }
    }
    for merge in merge_records::Entity::find()
        .all(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?
    {
        let at = merge.created_at.to_utc();
        entries.push((at, serde_json::json!({
            "kind": "merge",
            "at": at,
            "decision": "merged",
            "subject": { "pid": merge.main_pid, "name": names.get(&merge.main_pid) },
            "merged_from": merge.duplicate_pid,
            "actor": merge.actor,
            "reason": merge.reason,
        })));
    }
    Ok(entries)
}

/// Query for the decision log: optional result cap.
#[derive(Debug, Deserialize)]
struct DecisionsQuery {
    /// Maximum entries (default 50, cap 200).
    limit: Option<usize>,
}

/// `GET /api/executive/decisions` — the chronological decision log:
/// gate-review verdicts, scenario commits, decided proposals
/// (approved / rejected / promoted), and record merges, newest first.
/// Every entry is already stored by its own subsystem; this is a
/// projection, not a new record type.
#[debug_handler]
async fn executive_decisions(
    axum::extract::Query(query): axum::extract::Query<DecisionsQuery>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let limit = query.limit.unwrap_or(50).min(200);
    let mut entries = decision_entries(&ctx).await?;
    entries.sort_by_key(|entry| std::cmp::Reverse(entry.0));
    let total = entries.len();
    let decisions: Vec<serde_json::Value> =
        entries.into_iter().take(limit).map(|(_, v)| v).collect();
    let body = serde_json::json!({
        "as_of": chrono::Utc::now(),
        "total": total,
        "returned": decisions.len(),
        "decisions": decisions,
    });
    conditional(&headers, &body)
}

/// `GET /api/executive/benefits` — benefits realization per portfolio
/// bucket: per-currency target vs realized (with a ratio only where a
/// positive target exists) plus non-financial benefit counts by
/// status. Currencies never merge.
#[debug_handler]
async fn executive_benefits(State(ctx): State<AppContext>, headers: HeaderMap) -> Result<Response> {
    let items = live!(work_items, &ctx.db);
    let benefit_rows = live!(benefits, &ctx.db);
    let portfolio_of: std::collections::BTreeMap<Uuid, Option<Uuid>> = items
        .iter()
        .map(|i| {
            let bucket = if i.kind == "Portfolio" { Some(i.pid) } else { i.portfolio_pid };
            (i.pid, bucket)
        })
        .collect();
    let portfolio_items: std::collections::BTreeMap<Uuid, &work_items::Model> =
        items.iter().filter(|i| i.kind == "Portfolio").map(|i| (i.pid, i)).collect();

    let mut buckets: std::collections::BTreeMap<Option<Uuid>, Vec<&benefits::Model>> =
        std::collections::BTreeMap::new();
    for benefit in &benefit_rows {
        let bucket = portfolio_of.get(&benefit.work_item_pid).copied().flatten();
        buckets.entry(bucket).or_default().push(benefit);
    }

    let mut rows = Vec::new();
    for (bucket, members) in &buckets {
        // Financial lines: rows carrying a currency; target may still
        // be absent (accumulate-realize), so ratios stay honest.
        let mut per_currency: std::collections::BTreeMap<&str, (i64, i64)> =
            std::collections::BTreeMap::new();
        let mut statuses: std::collections::BTreeMap<&str, usize> =
            std::collections::BTreeMap::new();
        let mut non_financial = 0usize;
        for benefit in members {
            *statuses.entry(benefit.status.as_str()).or_default() += 1;
            if let Some(currency) = benefit.currency.as_deref() {
                let entry = per_currency.entry(currency).or_default();
                entry.0 = entry.0.saturating_add(benefit.target_minor.unwrap_or(0));
                entry.1 = entry.1.saturating_add(benefit.realized_minor);
            } else {
                non_financial += 1;
            }
        }
        let financial: Vec<serde_json::Value> = per_currency
            .iter()
            .map(|(currency, (target, realized))| {
                serde_json::json!({
                    "currency": currency,
                    "target_minor": target,
                    "realized_minor": realized,
                    "realization_ratio": insights::realization_ratio(*target, *realized),
                })
            })
            .collect();
        rows.push(serde_json::json!({
            "portfolio": bucket.and_then(|pid| portfolio_items.get(&pid)).map(|p| item_ref(p)),
            "benefits": members.len(),
            "non_financial": non_financial,
            "statuses": statuses,
            "financial": financial,
        }));
    }
    let body = serde_json::json!({
        "as_of": chrono::Utc::now(),
        "note": "per-currency lines are never merged and never converted; \
                 realization_ratio is null when no positive target exists",
        "portfolios": rows,
    });
    conditional(&headers, &body)
}

/// `GET /api/financials/variance` — committed vs actual vs remaining,
/// in minor units, per currency — rolled up three ways: by owning
/// collection, by portfolio bucket, and by budget-line category.
#[debug_handler]
async fn financial_variance(State(ctx): State<AppContext>, headers: HeaderMap) -> Result<Response> {
    let items = live!(work_items, &ctx.db);
    let budget_rows = live!(budget_lines, &ctx.db);
    let kind_of: std::collections::BTreeMap<Uuid, &str> =
        items.iter().map(|i| (i.pid, i.kind.as_str())).collect();
    let bucket_of: std::collections::BTreeMap<Uuid, Option<Uuid>> = items
        .iter()
        .map(|i| {
            let bucket = if i.kind == "Portfolio" { Some(i.pid) } else { i.portfolio_pid };
            (i.pid, bucket)
        })
        .collect();
    let portfolio_items: std::collections::BTreeMap<Uuid, &work_items::Model> =
        items.iter().filter(|i| i.kind == "Portfolio").map(|i| (i.pid, i)).collect();

    let line_of = |b: &budget_lines::Model| insights::MoneyLine {
        currency: b.currency.clone(),
        planned_minor: b.planned_minor,
        actual_minor: b.actual_minor,
    };
    let group = |key_of: &dyn Fn(&budget_lines::Model) -> String| -> Vec<(String, Vec<insights::VarianceRow>)> {
        let mut per: std::collections::BTreeMap<String, Vec<insights::MoneyLine>> =
            std::collections::BTreeMap::new();
        for row in &budget_rows {
            per.entry(key_of(row)).or_default().push(line_of(row));
        }
        per.into_iter()
            .map(|(key, lines)| (key, insights::variance_by_currency(&lines)))
            .collect()
    };

    let by_collection = group(&|b| {
        kind_of.get(&b.work_item_pid).copied().unwrap_or("unknown").to_string()
    });
    let by_category = group(&|b| b.category.clone());
    let mut by_portfolio = Vec::new();
    {
        let mut per: std::collections::BTreeMap<Option<Uuid>, Vec<insights::MoneyLine>> =
            std::collections::BTreeMap::new();
        for row in &budget_rows {
            let bucket = bucket_of.get(&row.work_item_pid).copied().flatten();
            per.entry(bucket).or_default().push(line_of(row));
        }
        for (bucket, lines) in per {
            by_portfolio.push(serde_json::json!({
                "portfolio": bucket.and_then(|pid| portfolio_items.get(&pid)).map(|p| item_ref(p)),
                "variance": insights::variance_by_currency(&lines),
            }));
        }
    }
    let body = serde_json::json!({
        "as_of": chrono::Utc::now(),
        "note": "minor units; one row per currency; remaining = planned - actual; no FX conversion",
        "by_collection": by_collection.into_iter().map(|(k, v)| serde_json::json!({"collection": k, "variance": v})).collect::<Vec<_>>(),
        "by_category": by_category.into_iter().map(|(k, v)| serde_json::json!({"category": k, "variance": v})).collect::<Vec<_>>(),
        "by_portfolio": by_portfolio,
    });
    conditional(&headers, &body)
}

/// `GET /api/financials/exposure` — the whole estate's per-currency
/// totals (planned / actual / remaining, line and item counts).
/// Deliberately **no** cross-currency total: converting would invent a
/// number the store does not hold.
#[debug_handler]
async fn financial_exposure(State(ctx): State<AppContext>, headers: HeaderMap) -> Result<Response> {
    let budget_rows = live!(budget_lines, &ctx.db);
    let lines: Vec<insights::MoneyLine> = budget_rows
        .iter()
        .map(|b| insights::MoneyLine {
            currency: b.currency.clone(),
            planned_minor: b.planned_minor,
            actual_minor: b.actual_minor,
        })
        .collect();
    let per_currency = insights::variance_by_currency(&lines);
    let currencies: Vec<serde_json::Value> = per_currency
        .into_iter()
        .map(|row| {
            let mut item_pids: Vec<Uuid> = budget_rows
                .iter()
                .filter(|b| b.currency == row.currency)
                .map(|b| b.work_item_pid)
                .collect();
            item_pids.sort_unstable();
            item_pids.dedup();
            let held_minor: i64 = budget_rows
                .iter()
                .filter(|b| {
                    b.currency == row.currency && b.gate.is_some() && b.released_at.is_none()
                })
                .map(|b| b.planned_minor)
                .sum();
            serde_json::json!({
                "currency": row.currency,
                "planned_minor": row.planned_minor,
                "actual_minor": row.actual_minor,
                "remaining_minor": row.remaining_minor,
                "held_minor": held_minor,
                "overrun": row.overrun,
                "line_count": row.line_count,
                "work_items": item_pids.len(),
            })
        })
        .collect();
    let body = serde_json::json!({
        "as_of": chrono::Utc::now(),
        "note": "currencies are reported separately and never converted or merged; \
                 held_minor = planned on stage-gated tranches not yet released",
        "currencies": currencies,
    });
    conditional(&headers, &body)
}

/// `GET /api/technology/dependency-risk` — the CTO dependency lens:
/// most-depended-on items (fan-out; single-point-of-failure
/// candidates), edges crossing portfolio boundaries, and edges whose
/// predecessor is currently RAG-red (delivery risk imported through
/// the graph).
#[debug_handler]
async fn technology_dependency_risk(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let today = chrono::Utc::now().date_naive();
    let items = live!(work_items, &ctx.db);
    let risk_rows = live!(risks, &ctx.db);
    let budget_rows = live!(budget_lines, &ctx.db);
    let dependency_rows = vis_models::all_dependencies(&ctx.db).await?;
    let rag = rag_by_pid(&items, &risk_rows, &budget_rows, &dependency_rows, today);
    let by_pid: std::collections::BTreeMap<Uuid, &work_items::Model> =
        items.iter().map(|i| (i.pid, i)).collect();

    let edge_pairs: Vec<(Uuid, Uuid)> = dependency_rows
        .iter()
        .map(|e| (e.predecessor_pid, e.successor_pid))
        .collect();
    let fan_out: Vec<serde_json::Value> = insights::fan_out(&edge_pairs)
        .into_iter()
        .take(10)
        .filter_map(|(pid, dependents)| {
            by_pid.get(&pid).map(|item| {
                serde_json::json!({
                    "item": item_ref(item),
                    "dependents": dependents,
                    "rag": rag.get(&pid),
                })
            })
        })
        .collect();
    let cross_portfolio: Vec<serde_json::Value> = dependency_rows
        .iter()
        .filter_map(|edge| {
            let p = by_pid.get(&edge.predecessor_pid)?;
            let s = by_pid.get(&edge.successor_pid)?;
            let pb = if p.kind == "Portfolio" { Some(p.pid) } else { p.portfolio_pid };
            let sb = if s.kind == "Portfolio" { Some(s.pid) } else { s.portfolio_pid };
            (pb != sb).then(|| {
                serde_json::json!({
                    "edge": edge.pid,
                    "predecessor": item_ref(p),
                    "successor": item_ref(s),
                })
            })
        })
        .collect();
    let red_predecessors: Vec<serde_json::Value> = dependency_rows
        .iter()
        .filter_map(|edge| {
            let p = by_pid.get(&edge.predecessor_pid)?;
            let s = by_pid.get(&edge.successor_pid)?;
            (rag.get(&p.pid) == Some(&"red")).then(|| {
                serde_json::json!({
                    "edge": edge.pid,
                    "predecessor": item_ref(p),
                    "successor": item_ref(s),
                })
            })
        })
        .collect();
    let body = serde_json::json!({
        "as_of": chrono::Utc::now(),
        "edges": dependency_rows.len(),
        "top_fan_out": fan_out,
        "cross_portfolio": cross_portfolio,
        "red_predecessor_edges": red_predecessors,
    });
    conditional(&headers, &body)
}

/// `GET /api/technology/radar` — the tag-encoded technology radar.
/// Convention (documented in the response): a work item tagged
/// `tech:<name>` uses that technology; `tech:<name>:<ring>` also casts
/// a ring vote (`assess` / `trial` / `adopt` / `hold`); a technology's
/// ring is the majority vote, ties breaking toward the more cautious
/// ring, `unclassified` with no votes.
/// Per-technology radar accumulator (rings voted, per-kind counts,
/// capped item refs).
struct Tech<'a> {
    rings: Vec<String>,
    per_kind: std::collections::BTreeMap<&'a str, usize>,
    items: Vec<serde_json::Value>,
}

#[debug_handler]
async fn technology_radar(State(ctx): State<AppContext>, headers: HeaderMap) -> Result<Response> {
    let items = live!(work_items, &ctx.db);
    let mut techs: std::collections::BTreeMap<String, Tech<'_>> =
        std::collections::BTreeMap::new();
    for item in &items {
        let tags = item
            .data
            .get("tags")
            .and_then(|v| v.as_array())
            .map_or(&[][..], Vec::as_slice);
        let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for tag in tags {
            let Some(tag) = tag.as_str() else { continue };
            let Some((name, ring)) = insights::parse_tech_tag(tag) else { continue };
            let tech = techs.entry(name.clone()).or_insert_with(|| Tech {
                rings: Vec::new(),
                per_kind: std::collections::BTreeMap::new(),
                items: Vec::new(),
            });
            if let Some(ring) = ring {
                tech.rings.push(ring);
            }
            // Count each item once per technology even if tagged twice.
            if seen.insert(name) {
                *tech.per_kind.entry(item.kind.as_str()).or_default() += 1;
                if tech.items.len() < 20 {
                    tech.items.push(item_ref(item));
                }
            }
        }
    }
    let technologies: Vec<serde_json::Value> = techs
        .iter()
        .map(|(name, tech)| {
            serde_json::json!({
                "technology": name,
                "ring": insights::ring_consensus(&tech.rings),
                "ring_votes": tech.rings.len(),
                "per_collection": tech.per_kind,
                "items": tech.items,
            })
        })
        .collect();
    let body = serde_json::json!({
        "as_of": chrono::Utc::now(),
        "convention": "tag `tech:<name>` marks usage; `tech:<name>:<ring>` \
                       (assess|trial|adopt|hold) also casts a ring vote; \
                       majority wins, ties break toward the cautious ring",
        "technologies": technologies,
    });
    conditional(&headers, &body)
}

/// `GET /api/executive/alignment` — strategic-alignment coverage: per
/// collection, how many live items carry at least one OKR mapping, and
/// the unaligned items with their per-currency planned spend (the
/// CEO's "unaligned spend" list). Ordering of the unaligned list is
/// each item's largest single-currency planned amount (a disclosed
/// heuristic — cross-currency amounts are never summed).
#[debug_handler]
async fn executive_alignment(State(ctx): State<AppContext>, headers: HeaderMap) -> Result<Response> {
    let items = live!(work_items, &ctx.db);
    let budget_rows = live!(budget_lines, &ctx.db);
    let link_rows = objective_links::Entity::find()
        .all(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;
    let aligned: std::collections::HashSet<Uuid> =
        link_rows.iter().map(|l| l.work_item_pid).collect();

    let mut per_collection: std::collections::BTreeMap<&str, (usize, usize)> =
        std::collections::BTreeMap::new();
    for item in &items {
        let entry = per_collection.entry(item.kind.as_str()).or_default();
        entry.0 += 1;
        if aligned.contains(&item.pid) {
            entry.1 += 1;
        }
    }

    // Unaligned spend per currency + the ranked unaligned list.
    let unaligned_lines: Vec<insights::MoneyLine> = budget_rows
        .iter()
        .filter(|b| !aligned.contains(&b.work_item_pid))
        .map(|b| insights::MoneyLine {
            currency: b.currency.clone(),
            planned_minor: b.planned_minor,
            actual_minor: b.actual_minor,
        })
        .collect();
    let unaligned_spend = insights::variance_by_currency(&unaligned_lines);
    let mut unaligned_items: Vec<(i64, serde_json::Value)> = items
        .iter()
        .filter(|i| !aligned.contains(&i.pid))
        .map(|item| {
            let lines: Vec<insights::MoneyLine> = budget_rows
                .iter()
                .filter(|b| b.work_item_pid == item.pid)
                .map(|b| insights::MoneyLine {
                    currency: b.currency.clone(),
                    planned_minor: b.planned_minor,
                    actual_minor: b.actual_minor,
                })
                .collect();
            let per_currency = insights::variance_by_currency(&lines);
            let rank = per_currency.iter().map(|r| r.planned_minor).max().unwrap_or(0);
            (rank, serde_json::json!({
                "item": item_ref(item),
                "planned": per_currency
                    .iter()
                    .map(|r| serde_json::json!({
                        "currency": r.currency, "planned_minor": r.planned_minor,
                    }))
                    .collect::<Vec<_>>(),
            }))
        })
        .collect();
    unaligned_items.sort_by_key(|(rank, _)| std::cmp::Reverse(*rank));
    let unaligned_items: Vec<serde_json::Value> =
        unaligned_items.into_iter().take(25).map(|(_, v)| v).collect();

    let body = serde_json::json!({
        "as_of": chrono::Utc::now(),
        "derivation": "aligned = has at least one OKR mapping; unaligned list \
                       ranked by largest single-currency planned amount \
                       (currencies never summed)",
        "by_collection": per_collection
            .iter()
            .map(|(kind, (total, aligned_count))| serde_json::json!({
                "collection": kind, "total": total,
                "aligned": aligned_count, "unaligned": total - aligned_count,
            }))
            .collect::<Vec<_>>(),
        "unaligned_spend": unaligned_spend,
        "unaligned_items": unaligned_items,
    });
    conditional(&headers, &body)
}

/// `GET /api/technology/debt` — the technical-debt register: risks
/// categorised `tech_debt`, exposure-sorted, with item refs and
/// status counts. Rides the risk lifecycle (raise / mitigate /
/// escalate / close) — an uncategorised risk reads as `delivery` and
/// stays off this view.
#[debug_handler]
async fn technology_debt(State(ctx): State<AppContext>, headers: HeaderMap) -> Result<Response> {
    let items = live!(work_items, &ctx.db);
    let risk_rows = live!(risks, &ctx.db);
    let by_pid: std::collections::BTreeMap<Uuid, &work_items::Model> =
        items.iter().map(|i| (i.pid, i)).collect();
    let mut debt: Vec<&risks::Model> = category_risks(&risk_rows, "tech_debt");
    debt.sort_by_key(|r| std::cmp::Reverse(r.probability * r.impact));
    let mut statuses: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for risk in &debt {
        *statuses.entry(risk.status.as_str()).or_default() += 1;
    }
    let open_exposure: i32 = debt
        .iter()
        .filter(|r| matches!(r.status.as_str(), "open" | "mitigating"))
        .map(|r| r.probability * r.impact)
        .sum();
    let register: Vec<serde_json::Value> = debt
        .iter()
        .map(|risk| {
            serde_json::json!({
                "pid": risk.pid,
                "title": risk.title,
                "status": risk.status,
                "exposure": risk.probability * risk.impact,
                "escalated": risk.escalated_at.is_some(),
                "owner_ref": risk.owner_ref,
                "item": by_pid.get(&risk.work_item_pid).map(|i| item_ref(i)),
            })
        })
        .collect();
    let body = serde_json::json!({
        "as_of": chrono::Utc::now(),
        "note": "risks with category `tech_debt`; exposure = probability x impact",
        "open_exposure": open_exposure,
        "statuses": statuses,
        "register": register,
    });
    conditional(&headers, &body)
}

/// Query for the flow metrics: the trailing window in months.
#[derive(Debug, Deserialize)]
struct FlowQuery {
    /// Trailing months to report (default 6, cap 24).
    months: Option<u32>,
}

/// `GET /api/technology/flow` — delivery-flow metrics from milestone
/// completion stamps: throughput per month and the median
/// creation-to-completion lead time. Milestones completed before the
/// completion stamp existed are counted separately and never timed —
/// an honest gap, not a guess.
#[debug_handler]
async fn technology_flow(
    axum::extract::Query(query): axum::extract::Query<FlowQuery>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let months = i64::from(query.months.unwrap_or(6).clamp(1, 24));
    let now = chrono::Utc::now();
    let window_start = now - chrono::Days::new(u64::try_from(months * 31).unwrap_or(186));
    let milestone_rows = live!(milestones, &ctx.db);

    let mut per_month: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    let mut lead_days: Vec<i64> = Vec::new();
    let mut undated_completions = 0usize;
    for milestone in milestone_rows.iter().filter(|m| m.done) {
        match milestone.done_at {
            Some(done_at) => {
                let done_at = done_at.to_utc();
                if done_at >= window_start {
                    *per_month.entry(done_at.format("%Y-%m").to_string()).or_default() += 1;
                    lead_days.push((done_at - milestone.created_at.to_utc()).num_days());
                }
            }
            None => undated_completions += 1,
        }
    }
    lead_days.sort_unstable();
    let median_lead_days = if lead_days.is_empty() {
        None
    } else {
        Some(lead_days[lead_days.len() / 2])
    };
    let body = serde_json::json!({
        "as_of": now,
        "window_months": months,
        "derivation": "throughput = milestones completed per month (by done_at); \
                       lead_days = done_at - created_at; completions predating \
                       the done_at stamp are counted in undated_completions and \
                       never timed",
        "throughput_by_month": per_month,
        "timed_completions": lead_days.len(),
        "median_lead_days": median_lead_days,
        "undated_completions": undated_completions,
    });
    conditional(&headers, &body)
}

/// ETag-conditional JSON response; the tag excludes `as_of` so a
/// byte-identical estate revalidates with `304`.
fn conditional(headers: &HeaderMap, body: &serde_json::Value) -> Result<Response> {
    let mut fingerprint = body.clone();
    if let Some(map) = fingerprint.as_object_mut() {
        map.remove("as_of");
    }
    super::conditional_json(headers, &super::etag_of(&fingerprint), body)
}

/// The executive-insight routes (all read-only GETs).
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/executive/health", get(executive_health))
        .add("/executive/decisions", get(executive_decisions))
        .add("/executive/benefits", get(executive_benefits))
        .add("/executive/alignment", get(executive_alignment))
        .add("/financials/variance", get(financial_variance))
        .add("/financials/exposure", get(financial_exposure))
        .add("/technology/dependency-risk", get(technology_dependency_risk))
        .add("/technology/radar", get(technology_radar))
        .add("/technology/debt", get(technology_debt))
        .add("/technology/flow", get(technology_flow))
}
