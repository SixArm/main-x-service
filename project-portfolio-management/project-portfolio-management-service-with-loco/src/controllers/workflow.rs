//! **Custom workflows** — the configuration surface over
//! [`crate::workflow`] (entity spec §9.2b / FR-26).
//!
//! A workflow declares the state vocabulary for a plan's tasks or
//! issues. **Every state must declare a category** (`todo` / `active` /
//! `waiting` / `done`), because the board, the burndown, the timeline
//! and every time-based-analysis figure compute from what a state
//! *means*. A state without one is refused here, and the schema refuses
//! it too — a rule this load-bearing does not live in a handler alone.
//!
//! **Resolution order**, used by the task path: the plan's own workflow
//! if it has one, else the deployment default, else the built-in
//! vocabulary. A plan with nothing configured therefore behaves exactly
//! as it did before this feature existed.

use axum::http::{HeaderMap, StatusCode};
use loco_rs::controller::ErrorDetail;
use loco_rs::prelude::*;
use sea_orm::QueryOrder;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::MaybeAuthUser;
use crate::models::_entities::{plans, tasks, workflow_states, workflow_transitions, workflows};
use crate::models::audit_logs::Model as AuditModel;
use crate::workflow as rules;

fn unprocessable(message: &str) -> Error {
    Error::CustomError(
        StatusCode::UNPROCESSABLE_ENTITY,
        ErrorDetail::new("unprocessable", message),
    )
}

fn db_err(e: sea_orm::DbErr) -> Error {
    Error::Model(ModelError::from(e))
}

/// Rebuild the pure definition from stored rows.
async fn definition_of(ctx: &AppContext, workflow_pid: Uuid) -> Result<rules::WorkflowDef> {
    let states = workflow_states::Entity::find()
        .filter(workflow_states::Column::WorkflowPid.eq(workflow_pid))
        .order_by_asc(workflow_states::Column::Position)
        .all(&ctx.db)
        .await
        .map_err(db_err)?;
    let transitions = workflow_transitions::Entity::find()
        .filter(workflow_transitions::Column::WorkflowPid.eq(workflow_pid))
        .all(&ctx.db)
        .await
        .map_err(db_err)?;

    Ok(rules::WorkflowDef {
        states: states
            .iter()
            .filter_map(|row| {
                // A stored row whose category no longer parses is
                // dropped rather than defaulted. The schema CHECK makes
                // this unreachable today; if it ever happens, losing the
                // state is visible (moves to it are refused) where
                // guessing a category would be silent.
                rules::Category::parse(&row.category).map(|category| rules::StateDef {
                    key: row.state_key.clone(),
                    label: row.label.clone(),
                    category,
                    wip_limit: row.wip_limit.and_then(|w| u32::try_from(w).ok()),
                    is_initial: row.is_initial,
                    is_terminal: row.is_terminal,
                })
            })
            .collect(),
        transitions: transitions
            .iter()
            .map(|row| rules::TransitionDef {
                from: row.from_key.clone(),
                to: row.to_key.clone(),
            })
            .collect(),
    })
}

/// The workflow in force for a plan and resource kind.
///
/// The plan's own, else the deployment default, else the built-in.
/// **Never `None`**: there is always a vocabulary, which is what keeps
/// every existing board working with nothing configured.
///
/// # Errors
///
/// Propagates a database error while resolving the workflow.
pub async fn in_force(
    ctx: &AppContext,
    plan_pid: Uuid,
    applies_to: &str,
) -> Result<rules::WorkflowDef> {
    let own = workflows::Entity::find()
        .filter(workflows::Column::PlanPid.eq(plan_pid))
        .filter(workflows::Column::AppliesTo.eq(applies_to))
        .filter(workflows::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await
        .map_err(db_err)?;
    if let Some(workflow) = own {
        return definition_of(ctx, workflow.pid).await;
    }

    let fallback = workflows::Entity::find()
        .filter(workflows::Column::PlanPid.is_null())
        .filter(workflows::Column::AppliesTo.eq(applies_to))
        .filter(workflows::Column::IsDefault.eq(true))
        .filter(workflows::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await
        .map_err(db_err)?;
    if let Some(workflow) = fallback {
        return definition_of(ctx, workflow.pid).await;
    }

    Ok(if applies_to == "issue" {
        rules::built_in_issue()
    } else {
        rules::built_in_task()
    })
}

/// `POST /api/workflows` body.
#[derive(Debug, Deserialize)]
struct WorkflowPayload {
    name: String,
    applies_to: String,
    #[serde(default)]
    plan_pid: Option<Uuid>,
    #[serde(default)]
    is_default: bool,
    states: Vec<StatePayload>,
    #[serde(default)]
    transitions: Vec<TransitionPayload>,
}

#[derive(Debug, Deserialize)]
struct StatePayload {
    key: String,
    label: String,
    category: String,
    #[serde(default)]
    wip_limit: Option<u32>,
    #[serde(default)]
    is_initial: bool,
    #[serde(default)]
    is_terminal: bool,
}

#[derive(Debug, Deserialize)]
struct TransitionPayload {
    from: String,
    to: String,
}

/// Write the workflow and its states and transitions.
///
/// Split from the handler so the validation half stays readable; the
/// ordering matters — the parent row first, so a state can never
/// reference a workflow that does not exist.
async fn persist(
    ctx: &AppContext,
    payload: &WorkflowPayload,
    def: &rules::WorkflowDef,
) -> Result<workflows::Model> {
    let workflow = workflows::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        plan_pid: ActiveValue::set(payload.plan_pid),
        name: ActiveValue::set(payload.name.trim().to_string()),
        applies_to: ActiveValue::set(payload.applies_to.clone()),
        // A plan-scoped workflow is never the deployment default:
        // "default" means the fallback when a plan has none.
        is_default: ActiveValue::set(payload.is_default && payload.plan_pid.is_none()),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await
    .map_err(db_err)?;

    for (position, state) in def.states.iter().enumerate() {
        workflow_states::ActiveModel {
            pid: ActiveValue::set(Uuid::new_v4()),
            workflow_pid: ActiveValue::set(workflow.pid),
            state_key: ActiveValue::set(state.key.clone()),
            label: ActiveValue::set(state.label.clone()),
            category: ActiveValue::set(state.category.token().to_string()),
            wip_limit: ActiveValue::set(state.wip_limit.and_then(|w| i32::try_from(w).ok())),
            is_initial: ActiveValue::set(state.is_initial),
            is_terminal: ActiveValue::set(state.is_terminal),
            position: ActiveValue::set(i32::try_from(position).unwrap_or(0)),
            ..Default::default()
        }
        .insert(&ctx.db)
        .await
        .map_err(db_err)?;
    }
    for transition in &def.transitions {
        workflow_transitions::ActiveModel {
            pid: ActiveValue::set(Uuid::new_v4()),
            workflow_pid: ActiveValue::set(workflow.pid),
            from_key: ActiveValue::set(transition.from.clone()),
            to_key: ActiveValue::set(transition.to.clone()),
            ..Default::default()
        }
        .insert(&ctx.db)
        .await
        .map_err(db_err)?;
    }
    Ok(workflow)
}

/// `POST /api/workflows` — register a workflow.
#[debug_handler]
async fn create(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<WorkflowPayload>,
) -> Result<Response> {
    if !["task", "issue"].contains(&payload.applies_to.as_str()) {
        return Err(unprocessable("applies_to must be task or issue"));
    }
    if payload.name.trim().is_empty() || payload.name.len() > crate::validation::MAX_TEXT_LEN {
        return Err(unprocessable("name is required (and capped)"));
    }
    if let Some(plan_pid) = payload.plan_pid {
        let exists = plans::Entity::find()
            .filter(plans::Column::Pid.eq(plan_pid))
            .filter(plans::Column::DeletedAt.is_null())
            .one(&ctx.db)
            .await
            .map_err(db_err)?;
        if exists.is_none() {
            return Err(Error::NotFound);
        }
    }

    // An unrecognised category is refused **by name**, so an operator
    // sees which state is wrong rather than that something is.
    let mut states = Vec::with_capacity(payload.states.len());
    for state in &payload.states {
        let Some(category) = rules::Category::parse(&state.category) else {
            return Err(unprocessable(&format!(
                "state `{}` declares category `{}`, which is not one of todo, active, \
                 waiting, done. Every state must declare one: the board, the burndown \
                 and every flow figure are computed from what a state means, not its name.",
                state.key, state.category
            )));
        };
        states.push(rules::StateDef {
            key: state.key.trim().to_string(),
            label: state.label.trim().to_string(),
            category,
            wip_limit: state.wip_limit,
            is_initial: state.is_initial,
            is_terminal: state.is_terminal,
        });
    }

    let def = rules::WorkflowDef {
        states,
        transitions: payload
            .transitions
            .iter()
            .map(|t| rules::TransitionDef {
                from: t.from.trim().to_string(),
                to: t.to.trim().to_string(),
            })
            .collect(),
    };
    if let Err(problems) = rules::validate(&def) {
        return Err(unprocessable(&format!(
            "workflow is not usable: {}",
            serde_json::to_string(&problems).unwrap_or_default()
        )));
    }

    let workflow = persist(&ctx, &payload, &def).await?;

    AuditModel::record(
        &ctx.db,
        payload.plan_pid.unwrap_or(workflow.pid),
        "workflow_registered",
        caller.actor(),
        None,
    )
    .await
    .ok();
    format::json(serde_json::json!({ "pid": workflow.pid.to_string() }))
}

/// `GET /api/workflows` — every registered workflow.
#[debug_handler]
async fn list(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = workflows::Entity::find()
        .filter(workflows::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await
        .map_err(db_err)?;
    format::json(serde_json::json!(rows))
}

/// `DELETE /api/workflows/{pid}` — withdraw a workflow.
///
/// Refused while any live task sits in a state only this workflow
/// declares: withdrawing it would leave that work in a state no
/// vocabulary explains, which is exactly the uncategorised-state
/// problem the category column exists to prevent.
#[debug_handler]
async fn remove(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
) -> Result<Response> {
    let workflow_pid = Uuid::parse_str(&pid).map_err(|_| Error::NotFound)?;
    let workflow = workflows::Entity::find()
        .filter(workflows::Column::Pid.eq(workflow_pid))
        .filter(workflows::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await
        .map_err(db_err)?
        .ok_or(Error::NotFound)?;

    if let Some(plan_pid) = workflow.plan_pid {
        let def = definition_of(&ctx, workflow.pid).await?;
        let built_in = if workflow.applies_to == "issue" {
            rules::built_in_issue()
        } else {
            rules::built_in_task()
        };
        let live = tasks::Entity::find()
            .filter(tasks::Column::PlanPid.eq(plan_pid))
            .filter(tasks::Column::DeletedAt.is_null())
            .all(&ctx.db)
            .await
            .map_err(db_err)?;
        let stranded: Vec<&str> = live
            .iter()
            .map(|task| task.status.as_str())
            .filter(|status| {
                rules::category_of(&def, status).is_some()
                    && rules::category_of(&built_in, status).is_none()
            })
            .collect();
        if !stranded.is_empty() {
            return Err(unprocessable(&format!(
                "live work sits in {} state(s) only this workflow declares (e.g. `{}`); \
                 move that work first, or those items would be left in a state no \
                 vocabulary explains",
                stranded.len(),
                stranded.first().copied().unwrap_or_default()
            )));
        }
    }

    let plan_pid = workflow.plan_pid.unwrap_or(workflow.pid);
    let mut active: workflows::ActiveModel = workflow.into();
    active.deleted_at = ActiveValue::set(Some(chrono::Utc::now().into()));
    active.update(&ctx.db).await.map_err(db_err)?;
    AuditModel::record(
        &ctx.db,
        plan_pid,
        "workflow_withdrawn",
        caller.actor(),
        None,
    )
    .await
    .ok();
    format::empty_json()
}

/// `GET /api/plans/{pid}/workflow?applies_to=` query.
#[derive(Debug, Deserialize)]
struct InForceParams {
    #[serde(default)]
    applies_to: Option<String>,
}

/// `GET /api/plans/{pid}/workflow` — the vocabulary actually in force,
/// and where it came from.
#[debug_handler]
async fn plan_workflow(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    Query(params): Query<InForceParams>,
    headers: HeaderMap,
) -> Result<Response> {
    let plan_pid = Uuid::parse_str(&pid).map_err(|_| Error::NotFound)?;
    let plan = plans::Entity::find()
        .filter(plans::Column::Pid.eq(plan_pid))
        .filter(plans::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await
        .map_err(db_err)?
        .ok_or(Error::NotFound)?;

    let applies_to = params.applies_to.unwrap_or_else(|| "task".to_string());
    if !["task", "issue"].contains(&applies_to.as_str()) {
        return Err(unprocessable("applies_to must be task or issue"));
    }
    let def = in_force(&ctx, plan.pid, &applies_to).await?;

    let own = workflows::Entity::find()
        .filter(workflows::Column::PlanPid.eq(plan.pid))
        .filter(workflows::Column::AppliesTo.eq(applies_to.clone()))
        .filter(workflows::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await
        .map_err(db_err)?;

    let body = serde_json::json!({
        "plan_pid": plan.pid.to_string(),
        "applies_to": applies_to,
        // Naming the source matters: "why can I not move this card"
        // is answered by which vocabulary is in force, not by the
        // vocabulary alone.
        "source": if own.is_some() { "plan" } else { "built_in_or_default" },
        "constrained": !def.transitions.is_empty(),
        "workflow": def,
    });
    let etag = super::etag_of(&body);
    super::conditional_json(&headers, &etag, &body)
}

/// The workflow routes.
#[must_use]
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/workflows", post(create))
        .add("/workflows", get(list))
        .add("/workflows/{pid}", delete(remove))
        .add("/plans/{pid}/workflow", get(plan_workflow))
}
