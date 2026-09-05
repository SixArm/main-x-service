//! Content types (CMS-R2): the operator-declared field schemas, and
//! the compatibility gate that stands between an edit and the content
//! already stored under it.
//!
//! The gate is the point of this module. `PUT` classifies the edit with
//! the pure core (`rules::schema::classify`) and:
//!
//! - **additive** — applied; `schema_version` bumps only if the fields
//!   actually changed.
//! - **tightening** — applied, and the response says so, so an operator
//!   knows to expect `needs_migration` findings (CMS-R21).
//! - **breaking** — **refused** unless the request carries
//!   `confirm_breaking: true` *and* a reason; the refusal lists exactly
//!   which changes are breaking and why.
//!
//! Every outcome is audited with its classification, so "who broke the
//! Article type, and did they mean to?" has an answer.

use loco_rs::prelude::*;
use sea_orm::{QueryOrder, QuerySelect, TransactionTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{authz_error, conflict, ensure_valid, unprocessable};
use crate::auth::{self, Action, MaybeAuthUser};
use crate::models::_entities::{content_types, templates};
use crate::models::audit_logs::Model as Audit;
use crate::models::records;
use crate::rules::schema::{self, Classification, Compatibility, FieldSpec};
use crate::streaming;
use crate::validation::Problems;

/// `POST` content-type body.
#[derive(Debug, Deserialize)]
struct CreatePayload {
    key: String,
    name: String,
    #[serde(default)]
    description: Option<String>,
    fields: Vec<FieldSpec>,
    #[serde(default = "default_true")]
    routable: bool,
    #[serde(default)]
    template_key: Option<String>,
}

/// `PUT` content-type body: the create shape plus the breaking-change
/// confirmation.
#[derive(Debug, Deserialize)]
struct UpdatePayload {
    #[serde(flatten)]
    base: CreatePayload,
    /// Explicit acknowledgement that a `breaking` edit is intended.
    #[serde(default)]
    confirm_breaking: bool,
    /// Why — required for a breaking edit, recorded in the audit row.
    #[serde(default)]
    reason: Option<String>,
}

/// `POST .../compatibility` body — classify a proposed field set
/// without writing anything.
#[derive(Debug, Deserialize)]
struct CompatibilityPayload {
    fields: Vec<FieldSpec>,
}

/// A `{pid}` reference response.
#[derive(Debug, Serialize)]
struct PidRef {
    pid: String,
}

/// The response to an applied edit: what changed, and how severe it was.
#[derive(Debug, Serialize)]
struct AppliedView {
    pid: String,
    schema_version: i32,
    compatibility: Classification,
}

const fn default_true() -> bool {
    true
}

/// Validate the declared shape: identifiers, then the field schema in
/// the pure core.
fn validate_type(payload: &CreatePayload) -> Vec<String> {
    let mut problems = Problems::new();
    problems.require_key("key", &payload.key);
    problems.require_text("name", &payload.name);
    problems.cap_opt("description", payload.description.as_deref());
    if let Some(template_key) = &payload.template_key {
        problems.require_key("template_key", template_key);
    }
    let mut problems = problems.into_vec();
    problems.extend(schema::validate_fields(&payload.fields));
    problems
}

/// Read a stored `fields` column back into field specs. A column that
/// no longer deserializes is a bug we want visible, not silently
/// treated as "no fields" — which would classify every edit as purely
/// additive and wave a breaking change through.
fn stored_fields(row: &content_types::Model) -> Result<Vec<FieldSpec>> {
    serde_json::from_value(row.fields.clone()).map_err(|e| {
        unprocessable(&format!(
            "stored field schema for {:?} is unreadable: {e}",
            row.key
        ))
    })
}

/// The template named by a content type must exist on the same site —
/// a dangling `template_key` would leave a channel with no region
/// contract to lay out.
async fn ensure_template_exists(
    db: &DatabaseConnection,
    site_pid: Uuid,
    template_key: Option<&str>,
) -> Result<()> {
    let Some(key) = template_key else {
        return Ok(());
    };
    let found = templates::Entity::find()
        .filter(templates::Column::SitePid.eq(site_pid))
        .filter(templates::Column::Key.eq(key))
        .filter(templates::Column::DeletedAt.is_null())
        .one(db)
        .await?;
    if found.is_none() {
        return Err(unprocessable(&format!(
            "template_key {key:?} does not name a template on this site"
        )));
    }
    Ok(())
}

/// The live content type with this key on this site, if any.
async fn find_by_key(
    db: &DatabaseConnection,
    site_pid: Uuid,
    key: &str,
) -> Result<Option<content_types::Model>> {
    let row = content_types::Entity::find()
        .filter(content_types::Column::SitePid.eq(site_pid))
        .filter(content_types::Column::Key.eq(key))
        .filter(content_types::Column::DeletedAt.is_null())
        .one(db)
        .await?;
    Ok(row)
}

/// `POST /api/sites/{pid}/content-types` — declare a content type.
#[debug_handler]
async fn create(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
    caller: MaybeAuthUser,
    Json(payload): Json<CreatePayload>,
) -> Result<Response> {
    let site = records::find_site(&ctx.db, records::parse_pid(&pid)?).await?;
    ensure_valid(&validate_type(&payload))?;
    ensure_template_exists(&ctx.db, site.pid, payload.template_key.as_deref()).await?;
    if find_by_key(&ctx.db, site.pid, &payload.key)
        .await?
        .is_some()
    {
        return Err(conflict(&format!(
            "content type key {:?} is already in use on site {}",
            payload.key, site.key
        )));
    }
    let txn = ctx.db.begin().await?;
    let row = content_types::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        site_pid: ActiveValue::set(site.pid),
        key: ActiveValue::set(payload.key.clone()),
        name: ActiveValue::set(payload.name.clone()),
        description: ActiveValue::set(payload.description.clone()),
        fields: ActiveValue::set(serde_json::json!(payload.fields)),
        routable: ActiveValue::set(payload.routable),
        template_key: ActiveValue::set(payload.template_key.clone()),
        schema_version: ActiveValue::set(1),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(
        &txn,
        "content_type",
        row.pid,
        "created",
        caller.actor(),
        Some(serde_json::json!({
            "site": site.key,
            "key": row.key,
            "schema_version": row.schema_version,
        })),
    )
    .await?;
    streaming::emit_on(
        &txn,
        "content_type",
        "content_type_changed",
        &row.pid.to_string(),
        &row.name,
        caller.actor(),
        Some(serde_json::json!({ "change": "created", "schema_version": row.schema_version })),
    )
    .await?;
    txn.commit().await?;
    crate::metrics::Metrics::global()
        .content_type_created_total
        .inc();
    format::json(PidRef {
        pid: row.pid.to_string(),
    })
}

/// `GET /api/sites/{pid}/content-types` — the site's live types.
#[debug_handler]
async fn list(State(ctx): State<AppContext>, Path(pid): Path<String>) -> Result<Response> {
    let site = records::find_site(&ctx.db, records::parse_pid(&pid)?).await?;
    let rows = content_types::Entity::find()
        .filter(content_types::Column::SitePid.eq(site.pid))
        .filter(content_types::Column::DeletedAt.is_null())
        .order_by_asc(content_types::Column::Id)
        .limit(1000)
        .all(&ctx.db)
        .await?;
    format::json(rows)
}

/// `GET /api/content-types/{pid}`.
///
/// # Errors
///
/// `403` when the record-level ABAC policy denies reading this content type.
#[debug_handler]
async fn show(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
    caller: MaybeAuthUser,
) -> Result<Response> {
    let row = records::find_content_type(&ctx.db, records::parse_pid(&pid)?).await?;
    auth::authorize_record(
        &caller,
        Action::Read,
        &auth::content_type_resource_attrs(&row),
    )
    .map_err(authz_error)?;
    format::json(row)
}

/// `POST /api/content-types/{pid}/compatibility` — classify a proposed
/// field set **without writing**. The dry run exists so an operator can
/// find out what an edit would do before doing it; the same classifier
/// then gates the write, so the preview cannot disagree with the gate.
#[debug_handler]
async fn compatibility(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
    Json(payload): Json<CompatibilityPayload>,
) -> Result<Response> {
    let row = records::find_content_type(&ctx.db, records::parse_pid(&pid)?).await?;
    ensure_valid(&schema::validate_fields(&payload.fields))?;
    let classification = schema::classify(&stored_fields(&row)?, &payload.fields);
    format::json(classification)
}

/// `PUT /api/content-types/{pid}` — edit the declaration, gated by the
/// compatibility classifier (see the module docs).
#[debug_handler]
async fn update(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
    caller: MaybeAuthUser,
    Json(payload): Json<UpdatePayload>,
) -> Result<Response> {
    let row = records::find_content_type(&ctx.db, records::parse_pid(&pid)?).await?;
    auth::authorize_record(
        &caller,
        Action::Write,
        &auth::content_type_resource_attrs(&row),
    )
    .map_err(authz_error)?;
    ensure_valid(&validate_type(&payload.base))?;
    ensure_template_exists(&ctx.db, row.site_pid, payload.base.template_key.as_deref()).await?;
    if payload.base.key != row.key
        && let Some(other) = find_by_key(&ctx.db, row.site_pid, &payload.base.key).await?
        && other.pid != row.pid
    {
        return Err(conflict(&format!(
            "content type key {:?} is already in use on this site",
            payload.base.key
        )));
    }

    let old_fields = stored_fields(&row)?;
    let classification = schema::classify(&old_fields, &payload.base.fields);
    if classification.requires_confirmation() {
        let breaking: Vec<String> = classification
            .changes
            .iter()
            .filter(|c| c.level == Compatibility::Breaking)
            .map(|c| format!("{}: {}", c.field, c.detail))
            .collect();
        if !payload.confirm_breaking {
            return Err(unprocessable(&format!(
                "this edit is breaking and needs confirm_breaking with a reason — {}",
                breaking.join("; ")
            )));
        }
        if payload.reason.as_ref().is_none_or(|r| r.trim().is_empty()) {
            return Err(unprocessable(
                "a breaking content-type edit requires a reason",
            ));
        }
    }

    let fields_changed = old_fields != payload.base.fields;
    let schema_version = if fields_changed {
        row.schema_version.saturating_add(1)
    } else {
        row.schema_version
    };
    let txn = ctx.db.begin().await?;
    let type_pid = row.pid;
    let mut active: content_types::ActiveModel = row.into();
    active.key = ActiveValue::set(payload.base.key.clone());
    active.name = ActiveValue::set(payload.base.name.clone());
    active.description = ActiveValue::set(payload.base.description.clone());
    active.fields = ActiveValue::set(serde_json::json!(payload.base.fields));
    active.routable = ActiveValue::set(payload.base.routable);
    active.template_key = ActiveValue::set(payload.base.template_key.clone());
    active.schema_version = ActiveValue::set(schema_version);
    let updated = active.update(&txn).await?;
    Audit::record(
        &txn,
        "content_type",
        type_pid,
        "schema_changed",
        caller.actor(),
        Some(serde_json::json!({
            "key": updated.key,
            "schema_version": updated.schema_version,
            "compatibility": classification.level.as_str(),
            "changes": classification.changes,
            "confirmed_breaking": payload.confirm_breaking,
            "reason": payload.reason,
        })),
    )
    .await?;
    streaming::emit_on(
        &txn,
        "content_type",
        "content_type_changed",
        &type_pid.to_string(),
        &updated.name,
        caller.actor(),
        Some(serde_json::json!({
            "compatibility": classification.level.as_str(),
            "schema_version": updated.schema_version,
        })),
    )
    .await?;
    txn.commit().await?;
    if classification.level == Compatibility::Breaking {
        crate::metrics::Metrics::global()
            .content_type_breaking_change_total
            .inc();
    }
    format::json(AppliedView {
        pid: type_pid.to_string(),
        schema_version: updated.schema_version,
        compatibility: classification,
    })
}

/// `DELETE /api/content-types/{pid}` — soft-delete.
///
/// Entries do not exist yet (CMS-T5), so there is nothing to refuse
/// against; when they land, this gains the same delete-refusal as
/// assets and templates (CMS-D8) — recorded here so the omission is a
/// noted gap rather than a silent one.
#[debug_handler]
async fn remove(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
    caller: MaybeAuthUser,
) -> Result<Response> {
    let row = records::find_content_type(&ctx.db, records::parse_pid(&pid)?).await?;
    auth::authorize_record(
        &caller,
        Action::Delete,
        &auth::content_type_resource_attrs(&row),
    )
    .map_err(authz_error)?;
    let txn = ctx.db.begin().await?;
    let type_pid = row.pid;
    let name = row.name.clone();
    let mut active: content_types::ActiveModel = row.into();
    active.deleted_at = ActiveValue::set(Some(chrono::Utc::now().into()));
    active.update(&txn).await?;
    Audit::record(
        &txn,
        "content_type",
        type_pid,
        "deleted",
        caller.actor(),
        None,
    )
    .await?;
    streaming::emit_on(
        &txn,
        "content_type",
        "deleted",
        &type_pid.to_string(),
        &name,
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::empty_json()
}

/// The content-type routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/sites/{pid}/content-types", post(create).get(list))
        .add("/content-types/{pid}", get(show).put(update).delete(remove))
        .add("/content-types/{pid}/compatibility", post(compatibility))
}
