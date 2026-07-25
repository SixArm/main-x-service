//! In-app notifications (WPM-R31 / WPM-D23) — the employee's own
//! list (unread first) and the mark-read action. Reference-only rows;
//! the write side lives in the handlers that make the change.

use loco_rs::prelude::*;
use sea_orm::{ActiveValue, QueryOrder};

use super::record_rejection;
use crate::auth::{self, MaybeAuthUser};
use crate::models::_entities::notifications;
use crate::models::records;

/// `GET /api/employees/{pid}/notifications` — the employee's
/// notifications, unread first, newest within each. `$sub`-owned.
#[debug_handler]
async fn list_notifications(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    let employee = records::find_employee(&ctx.db, records::parse_pid(&pid)?).await?;
    auth::authorize_record(
        &caller,
        authentication_verifier::Action::Read,
        &auth::employee_resource_attrs(&employee),
    )
    .map_err(record_rejection)?;
    let mut rows = notifications::Entity::find()
        .filter(notifications::Column::EmployeePid.eq(employee.pid))
        .order_by_desc(notifications::Column::Id)
        .all(&ctx.db)
        .await?;
    rows.sort_by_key(|n| n.read_at.is_some());
    format::json(rows)
}

/// `POST /api/notifications/{pid}/read` — mark one notification read.
/// Owner-only: the record-level check runs against the notification's
/// employee, so nobody clears another person's bell.
#[debug_handler]
async fn mark_read(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    let notification = notifications::Entity::find()
        .filter(notifications::Column::Pid.eq(records::parse_pid(&pid)?))
        .one(&ctx.db)
        .await?
        .ok_or(Error::NotFound)?;
    let owner = records::find_employee(&ctx.db, notification.employee_pid).await?;
    auth::authorize_record(
        &caller,
        authentication_verifier::Action::Write,
        &auth::employee_resource_attrs(&owner),
    )
    .map_err(record_rejection)?;
    let mut active: notifications::ActiveModel = notification.into();
    active.read_at = ActiveValue::set(Some(chrono::Utc::now().into()));
    let updated = active.update(&ctx.db).await?;
    format::json(updated)
}

/// The notification routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/employees/{pid}/notifications", get(list_notifications))
        .add("/notifications/{pid}/read", post(mark_read))
}
