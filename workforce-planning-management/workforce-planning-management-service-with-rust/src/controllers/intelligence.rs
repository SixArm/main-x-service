//! **Workforce intelligence** (WPM-R24) — the read-only analytical
//! layer over everything WPM records: headcount and shape, capability
//! and capability gaps, succession bench strength, and the talent
//! pipeline / early-career funnel.
//!
//! Four principles hold across every view here, because an analytics
//! surface is exactly where numbers stop being checkable:
//!
//! 1. **Every rate carries its terms.** A ratio is always
//!    `{numerator, denominator, value}` ([`crate::rules::talent::ratio`]),
//!    and is `null` — never `0` — when the denominator is zero.
//! 2. **Nothing is imputed.** A missing declaration is reported as
//!    missing (`not_assessed`, `undeclared`), never as a zero that
//!    reads like a measurement.
//! 3. **Every payload names its derivation** in a `derivation` field,
//!    so a consumer cannot mistake a proxy for the thing itself.
//! 4. **No individual's sensitive data appears.** These are aggregate
//!    counts: no salary, no assessment score, no review content. The
//!    one place an individual is named is the succession
//!    single-point-of-failure list, which names *roles* and their
//!    incumbent employee pid — the same information the succession
//!    endpoints already return under their own audit.
//!
//! Aggregates are computed in-process from the entity rows rather than
//! in SQL: the datasets are small (one organisation's workforce), and
//! the derivations stay in reviewable Rust next to their tests.

use loco_rs::prelude::*;
use sea_orm::EntityTrait;
use serde::Deserialize;
use std::collections::BTreeMap;
use uuid::Uuid;

use crate::models::_entities::{
    assessment_instruments, assessments, development_plans, early_career_programs,
    employee_skills, employees, pipeline_members, program_placements, skills, succession_candidates,
    succession_plans, talent_pipelines,
};
use crate::rules::assessment as assessment_rules;
use crate::rules::talent as rules;

/// Render a `(numerator, denominator, value)` triple as the family's
/// terms-carrying ratio object, or `null` when there is nothing to
/// divide.
fn ratio_json(terms: Option<(usize, usize, f64)>) -> serde_json::Value {
    terms.map_or(serde_json::Value::Null, |(numerator, denominator, value)| {
        serde_json::json!({ "numerator": numerator, "denominator": denominator, "value": value })
    })
}

/// Query accepted by every intelligence view.
#[derive(Debug, Deserialize)]
struct AsOfQuery {
    /// The date the view is computed against (default today).
    as_of: Option<chrono::NaiveDate>,
}

/// `GET /api/workforce-intelligence/overview?as_of=` — the shape of the
/// workforce: headcount by department, status, and employment type;
/// total FTE; tenure distribution; and manager spans of control.
///
/// FTE is summed from the declared `fte_percent`, and reported as
/// hundredths so no float rounds a headcount.
#[debug_handler]
async fn overview(
    axum::extract::Query(query): axum::extract::Query<AsOfQuery>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let as_of = query.as_of.unwrap_or_else(|| chrono::Utc::now().date_naive());
    let staff = live_employees(&ctx).await?;

    let mut by_department: BTreeMap<String, usize> = BTreeMap::new();
    let mut by_status: BTreeMap<String, usize> = BTreeMap::new();
    let mut by_employment_type: BTreeMap<String, usize> = BTreeMap::new();
    let mut by_tenure: BTreeMap<&str, usize> = BTreeMap::new();
    let mut fte_percent_total: i64 = 0;
    let mut span_of_control: BTreeMap<Uuid, usize> = BTreeMap::new();

    for employee in &staff {
        *by_department.entry(employee.department.clone()).or_default() += 1;
        *by_status.entry(employee.status.clone()).or_default() += 1;
        *by_employment_type
            .entry(employee.employment_type.clone())
            .or_default() += 1;
        let months = rules::months_of_service(employee.hired_on, as_of);
        *by_tenure.entry(rules::tenure_bucket(months)).or_default() += 1;
        fte_percent_total += i64::from(employee.fte_percent);
        if let Some(manager) = employee.manager_pid {
            *span_of_control.entry(manager).or_default() += 1;
        }
    }

    let name_of: BTreeMap<Uuid, &str> = staff
        .iter()
        .map(|e| (e.pid, e.display_name.as_str()))
        .collect();
    let mut spans: Vec<serde_json::Value> = span_of_control
        .iter()
        .map(|(manager, reports)| {
            serde_json::json!({
                "manager_pid": manager,
                "display_name": name_of.get(manager),
                "direct_reports": reports,
            })
        })
        .collect();
    spans.sort_by_key(|s| std::cmp::Reverse(s["direct_reports"].as_u64().unwrap_or(0)));

    format::json(serde_json::json!({
        "as_of": as_of,
        "derivation": "headcount counts live (not soft-deleted) employee records; FTE is the \
                       sum of declared fte_percent (in hundredths of a person); tenure is whole \
                       completed months since hired_on",
        "headcount": staff.len(),
        "fte_percent_total": fte_percent_total,
        "by_department": by_department,
        "by_status": by_status,
        "by_employment_type": by_employment_type,
        "by_tenure": by_tenure,
        "spans_of_control": spans,
    }))
}

/// `GET /api/workforce-intelligence/capability` — what the workforce
/// can do and where it cannot: declared skill coverage, declared gaps
/// (proficiency below a declared target), the development plans in
/// flight against those gaps, and assessment coverage.
///
/// "Coverage" is deliberately narrow: it counts **declarations**, not
/// ability. A skill nobody has declared is reported as undeclared, not
/// as absent.
#[debug_handler]
#[allow(clippy::too_many_lines)] // one pass over skills, plans, and sittings
async fn capability(State(ctx): State<AppContext>) -> Result<Response> {
    let staff = live_employees(&ctx).await?;
    let headcount = staff.len();
    let department_of: BTreeMap<Uuid, &str> = staff
        .iter()
        .map(|e| (e.pid, e.department.as_str()))
        .collect();

    let skill_rows = skills::Entity::find()
        .filter(skills::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await?;
    let skill_name: BTreeMap<Uuid, &str> = skill_rows.iter().map(|s| (s.pid, s.name.as_str())).collect();
    let declared = employee_skills::Entity::find()
        .filter(employee_skills::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await?;

    // skill → (holders, at-or-above-target, below-target)
    let mut per_skill: BTreeMap<Uuid, (usize, usize, usize)> = BTreeMap::new();
    let mut people_with_a_gap: std::collections::HashSet<Uuid> = std::collections::HashSet::new();
    for row in &declared {
        if !department_of.contains_key(&row.employee_pid) {
            continue; // a declaration from a departed employee
        }
        let entry = per_skill.entry(row.skill_pid).or_default();
        entry.0 += 1;
        match row.target {
            Some(target) if row.proficiency < target => {
                entry.2 += 1;
                people_with_a_gap.insert(row.employee_pid);
            }
            Some(_) => entry.1 += 1,
            None => {}
        }
    }

    let coverage: Vec<serde_json::Value> = skill_rows
        .iter()
        .map(|skill| {
            let (holders, at_target, below_target) =
                per_skill.get(&skill.pid).copied().unwrap_or((0, 0, 0));
            serde_json::json!({
                "skill": skill.name,
                "category": skill.category,
                "declared_by": holders,
                "coverage": ratio_json(rules::ratio(holders, headcount)),
                "at_or_above_target": at_target,
                "below_target": below_target,
                "undeclared": holders == 0,
            })
        })
        .collect();

    // Development plans in flight, by kind.
    let plans = development_plans::Entity::find()
        .filter(development_plans::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await?;
    let mut plans_by_kind: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    for plan in &plans {
        *plans_by_kind
            .entry(plan.kind.clone())
            .or_default()
            .entry(plan.status.clone())
            .or_default() += 1;
    }
    let people_on_active_plans: std::collections::HashSet<Uuid> = plans
        .iter()
        .filter(|p| p.status == "active")
        .map(|p| p.employee_pid)
        .collect();
    let gap_covered = people_with_a_gap
        .iter()
        .filter(|pid| people_on_active_plans.contains(pid))
        .count();

    // Assessment coverage: employees with at least one completed sitting.
    let sittings = assessments::Entity::find()
        .filter(assessments::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await?;
    let instruments = assessment_instruments::Entity::find().all(&ctx.db).await?;
    let category_of: BTreeMap<Uuid, &str> = instruments
        .iter()
        .map(|i| (i.pid, i.category.as_str()))
        .collect();
    let mut assessed_by_category: BTreeMap<&str, std::collections::HashSet<Uuid>> =
        BTreeMap::new();
    for sitting in &sittings {
        if sitting.subject_kind != "employee" || sitting.status != "completed" {
            continue;
        }
        if let Some(category) = category_of.get(&sitting.instrument_pid) {
            assessed_by_category
                .entry(category)
                .or_default()
                .insert(sitting.subject_pid);
        }
    }
    let assessment_coverage: Vec<serde_json::Value> = assessment_rules::ASSESSMENT_CATEGORIES
        .iter()
        .map(|category| {
            let assessed = assessed_by_category
                .get(category)
                .map_or(0, std::collections::HashSet::len);
            serde_json::json!({
                "category": category,
                "employees_assessed": assessed,
                "coverage": ratio_json(rules::ratio(assessed, headcount)),
            })
        })
        .collect();

    format::json(serde_json::json!({
        "derivation": "coverage counts DECLARED skills over live headcount — a skill nobody \
                       declared is `undeclared`, not absent; a gap is a declared proficiency \
                       below a declared target; assessment coverage counts employees with at \
                       least one COMPLETED sitting in that category",
        "headcount": headcount,
        "skills": coverage,
        "people_with_a_declared_gap": people_with_a_gap.len(),
        "gaps_with_an_active_plan": ratio_json(rules::ratio(gap_covered, people_with_a_gap.len())),
        "development_plans_by_kind": plans_by_kind,
        "assessment_coverage": assessment_coverage,
        "skill_catalog_size": skill_name.len(),
    }))
}

/// `GET /api/workforce-intelligence/succession` — bench strength across
/// the succession plans, and the **single points of failure**: critical
/// roles with nobody ready now (a high risk of loss lowers the
/// criticality threshold, since exposure is criticality × risk).
#[debug_handler]
async fn succession(State(ctx): State<AppContext>) -> Result<Response> {
    let plans = succession_plans::Entity::find()
        .filter(succession_plans::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await?;
    let all_candidates = succession_candidates::Entity::find()
        .filter(succession_candidates::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await?;

    let mut by_coverage: BTreeMap<&str, usize> = BTreeMap::new();
    let mut per_plan = Vec::with_capacity(plans.len());
    let mut single_points = Vec::new();
    let mut covered_now = 0usize;

    for plan in &plans {
        let readiness: Vec<String> = all_candidates
            .iter()
            .filter(|c| c.plan_pid == plan.pid)
            .map(|c| c.readiness.clone())
            .collect();
        let coverage = rules::bench_coverage(&readiness);
        *by_coverage.entry(coverage).or_default() += 1;
        if coverage == "covered_now" {
            covered_now += 1;
        }
        let mut by_readiness: BTreeMap<String, usize> = BTreeMap::new();
        for rating in &readiness {
            *by_readiness.entry(rating.clone()).or_default() += 1;
        }
        let spof = rules::is_single_point_of_failure(
            plan.criticality,
            coverage,
            plan.risk_of_loss.as_deref(),
        );
        if spof {
            single_points.push(serde_json::json!({
                "plan_pid": plan.pid,
                "role_title": plan.role_title,
                "department": plan.department,
                "criticality": plan.criticality,
                "risk_of_loss": plan.risk_of_loss,
                "incumbent_pid": plan.incumbent_pid,
                "coverage": coverage,
                "vacancy_expected_on": plan.vacancy_expected_on,
            }));
        }
        per_plan.push(serde_json::json!({
            "plan_pid": plan.pid,
            "role_title": plan.role_title,
            "department": plan.department,
            "criticality": plan.criticality,
            "risk_of_loss": plan.risk_of_loss,
            "bench": by_readiness,
            "bench_size": readiness.len(),
            "coverage": coverage,
            "single_point_of_failure": spof,
        }));
    }

    format::json(serde_json::json!({
        "derivation": "coverage is conservative — `covered_now` requires a ready_now successor; \
                       a single point of failure is an uncovered role with criticality ≥ 4, or \
                       ≥ 3 when the incumbent's risk_of_loss is high",
        "plans": plans.len(),
        "covered_now": ratio_json(rules::ratio(covered_now, plans.len())),
        "by_coverage": by_coverage,
        "single_points_of_failure": single_points,
        "per_plan": per_plan,
    }))
}

/// `GET /api/workforce-intelligence/pipelines` — the talent funnel:
/// each pipeline's health, plus the early-career programmes' placement
/// counts and conversion rates.
#[debug_handler]
async fn pipelines(State(ctx): State<AppContext>) -> Result<Response> {
    let pipelines = talent_pipelines::Entity::find()
        .filter(talent_pipelines::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await?;
    let members = pipeline_members::Entity::find()
        .filter(pipeline_members::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await?;

    let mut by_purpose: BTreeMap<String, usize> = BTreeMap::new();
    let mut per_pipeline = Vec::with_capacity(pipelines.len());
    let mut ready_total = 0usize;
    let mut live_total = 0usize;
    for pipeline in &pipelines {
        *by_purpose.entry(pipeline.purpose.clone()).or_default() += 1;
        let stages: Vec<String> = members
            .iter()
            .filter(|m| m.pipeline_pid == pipeline.pid)
            .map(|m| m.stage.clone())
            .collect();
        let health = rules::pipeline_health(&stages);
        ready_total += health.ready;
        live_total += health.live;
        per_pipeline.push(serde_json::json!({
            "pid": pipeline.pid,
            "name": pipeline.name,
            "purpose": pipeline.purpose,
            "target_job_title": pipeline.target_job_title,
            "target_department": pipeline.target_department,
            "health": health,
        }));
    }

    let programs = early_career_programs::Entity::find()
        .filter(early_career_programs::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await?;
    let placements = program_placements::Entity::find()
        .filter(program_placements::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await?;

    let mut per_kind: BTreeMap<&str, (usize, Vec<String>, usize)> = BTreeMap::new();
    for program in &programs {
        let mine: Vec<&program_placements::Model> = placements
            .iter()
            .filter(|p| p.program_pid == program.pid)
            .collect();
        let entry = per_kind.entry(program.kind.as_str()).or_default();
        entry.0 += 1;
        entry.1.extend(
            mine.iter()
                .filter(|p| p.status == "completed")
                .map(|p| p.outcome.clone()),
        );
        entry.2 += mine.iter().filter(|p| p.status == "active").count();
    }
    let early_careers: Vec<serde_json::Value> = per_kind
        .iter()
        .map(|(kind, (programs, completed_outcomes, active))| {
            serde_json::json!({
                "kind": kind,
                "programs": programs,
                "active_placements": active,
                "completed_placements": completed_outcomes.len(),
                "conversion_rate": ratio_json(rules::conversion_rate(completed_outcomes)),
            })
        })
        .collect();

    format::json(serde_json::json!({
        "derivation": "a pipeline's live pool excludes placed and exited members; the \
                       conversion rate divides converted outcomes by COMPLETED placements only \
                       (a running placement has not had the chance to convert)",
        "pipelines": per_pipeline,
        "by_purpose": by_purpose,
        "ready_across_pipelines": ready_total,
        "live_across_pipelines": live_total,
        "ready_share": ratio_json(rules::ratio(ready_total, live_total)),
        "early_careers": early_careers,
    }))
}

/// Every live (not soft-deleted) employee record.
async fn live_employees(ctx: &AppContext) -> Result<Vec<employees::Model>> {
    let rows = employees::Entity::find()
        .filter(employees::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await?;
    Ok(rows)
}

/// The workforce-intelligence routes (read-only).
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/workforce-intelligence")
        .add("/overview", get(overview))
        .add("/capability", get(capability))
        .add("/succession", get(succession))
        .add("/pipelines", get(pipelines))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A ratio is rendered with its terms, and a zero denominator is
    /// `null` rather than a misleading `0`.
    #[test]
    fn ratios_render_their_terms_or_null() {
        let value = ratio_json(rules::ratio(3, 4));
        assert_eq!(value["numerator"], 3);
        assert_eq!(value["denominator"], 4);
        assert!((value["value"].as_f64().expect("f64") - 0.75).abs() < f64::EPSILON);

        assert!(
            ratio_json(rules::ratio(0, 0)).is_null(),
            "nothing to divide ⇒ null, never 0"
        );
        let real_zero = ratio_json(rules::ratio(0, 7));
        assert_eq!(real_zero["numerator"], 0);
        assert_eq!(real_zero["denominator"], 7, "0 of 7 is a measurement");
    }
}
