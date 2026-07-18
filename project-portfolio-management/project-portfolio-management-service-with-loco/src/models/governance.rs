//! Governance model helpers (PPM Phase A): `ActiveModelBehavior`
//! impls and pid finders for the proposals / gate-reviews / risks /
//! budget-lines sub-resources.

use loco_rs::prelude::*;
use sea_orm::{ConnectionTrait, QueryOrder};
use uuid::Uuid;

use super::_entities::{budget_lines, gate_reviews, proposals, risks};

impl ActiveModelBehavior for proposals::ActiveModel {}
impl ActiveModelBehavior for gate_reviews::ActiveModel {}
impl ActiveModelBehavior for risks::ActiveModel {}
impl ActiveModelBehavior for budget_lines::ActiveModel {}

/// Parse a path `pid`, mapping failure to `404` (an unparsable pid
/// can never name a record; loco 0.16 does not map
/// `ModelError::EntityNotFound`, so `Error::NotFound` directly).
///
/// # Errors
///
/// [`Error::NotFound`] when `pid` is not a UUID.
pub fn parse_pid(pid: &str) -> Result<Uuid> {
    Uuid::parse_str(pid).map_err(|_| Error::NotFound)
}

/// Generate a `find_<x>` finder over an active (not soft-deleted) row.
macro_rules! find_active {
    ($fn_name:ident, $module:ident) => {
        /// Find the active row by public id.
        ///
        /// # Errors
        ///
        /// [`Error::NotFound`] when absent or soft-deleted.
        pub async fn $fn_name<C: ConnectionTrait>(
            db: &C,
            pid: Uuid,
        ) -> Result<$module::Model> {
            let row = $module::Entity::find()
                .filter($module::Column::Pid.eq(pid))
                .filter($module::Column::DeletedAt.is_null())
                .one(db)
                .await
                .map_err(|e| Error::Model(ModelError::from(e)))?;
            row.ok_or(Error::NotFound)
        }
    };
}

find_active!(find_proposal, proposals);
find_active!(find_risk, risks);
find_active!(find_budget_line, budget_lines);

/// A work item's gate reviews, oldest first.
///
/// # Errors
///
/// When the query fails.
pub async fn gate_reviews_for(
    db: &DatabaseConnection,
    work_item_pid: Uuid,
) -> Result<Vec<gate_reviews::Model>> {
    let rows = gate_reviews::Entity::find()
        .filter(gate_reviews::Column::WorkItemPid.eq(work_item_pid))
        .order_by_asc(gate_reviews::Column::Id)
        .all(db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;
    Ok(rows)
}

/// A work item's active risks, oldest first.
///
/// # Errors
///
/// When the query fails.
pub async fn risks_for(db: &DatabaseConnection, work_item_pid: Uuid) -> Result<Vec<risks::Model>> {
    let rows = risks::Entity::find()
        .filter(risks::Column::WorkItemPid.eq(work_item_pid))
        .filter(risks::Column::DeletedAt.is_null())
        .order_by_asc(risks::Column::Id)
        .all(db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;
    Ok(rows)
}

/// A work item's active budget lines, oldest first.
///
/// # Errors
///
/// When the query fails.
pub async fn budget_lines_for(
    db: &DatabaseConnection,
    work_item_pid: Uuid,
) -> Result<Vec<budget_lines::Model>> {
    let rows = budget_lines::Entity::find()
        .filter(budget_lines::Column::WorkItemPid.eq(work_item_pid))
        .filter(budget_lines::Column::DeletedAt.is_null())
        .order_by_asc(budget_lines::Column::Id)
        .all(db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;
    Ok(rows)
}
