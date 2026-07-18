//! Visibility model helpers (PPM Phase B): `ActiveModelBehavior`
//! impls and finders for dependencies / milestones / allocations /
//! report definitions.

use loco_rs::prelude::*;
use sea_orm::{ConnectionTrait, QueryOrder};
use uuid::Uuid;

use super::_entities::{allocations, milestones, report_definitions, work_item_dependencies};

impl ActiveModelBehavior for work_item_dependencies::ActiveModel {}
impl ActiveModelBehavior for milestones::ActiveModel {}
impl ActiveModelBehavior for allocations::ActiveModel {}
impl ActiveModelBehavior for report_definitions::ActiveModel {}

/// Every dependency edge (the cycle check and the schedule view both
/// want the full set — edge volume is operator-scale).
///
/// # Errors
///
/// When the query fails.
pub async fn all_dependencies<C: ConnectionTrait>(
    db: &C,
) -> Result<Vec<work_item_dependencies::Model>> {
    work_item_dependencies::Entity::find()
        .order_by_asc(work_item_dependencies::Column::Id)
        .all(db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))
}

/// One dependency by pid.
///
/// # Errors
///
/// [`Error::NotFound`] when absent.
pub async fn find_dependency<C: ConnectionTrait>(
    db: &C,
    pid: Uuid,
) -> Result<work_item_dependencies::Model> {
    work_item_dependencies::Entity::find()
        .filter(work_item_dependencies::Column::Pid.eq(pid))
        .one(db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?
        .ok_or(Error::NotFound)
}

/// Generate a `find_<x>` finder over an active (not soft-deleted) row.
macro_rules! find_active {
    ($fn_name:ident, $module:ident) => {
        /// Find the active row by public id.
        ///
        /// # Errors
        ///
        /// [`Error::NotFound`] when absent or soft-deleted.
        pub async fn $fn_name<C: ConnectionTrait>(db: &C, pid: Uuid) -> Result<$module::Model> {
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

find_active!(find_milestone, milestones);
find_active!(find_allocation, allocations);
find_active!(find_report, report_definitions);

/// A work item's active milestones, due-date order.
///
/// # Errors
///
/// When the query fails.
pub async fn milestones_for(
    db: &DatabaseConnection,
    work_item_pid: Uuid,
) -> Result<Vec<milestones::Model>> {
    milestones::Entity::find()
        .filter(milestones::Column::WorkItemPid.eq(work_item_pid))
        .filter(milestones::Column::DeletedAt.is_null())
        .order_by_asc(milestones::Column::Due)
        .all(db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))
}

/// A work item's active allocations, oldest first.
///
/// # Errors
///
/// When the query fails.
pub async fn allocations_for(
    db: &DatabaseConnection,
    work_item_pid: Uuid,
) -> Result<Vec<allocations::Model>> {
    allocations::Entity::find()
        .filter(allocations::Column::WorkItemPid.eq(work_item_pid))
        .filter(allocations::Column::DeletedAt.is_null())
        .order_by_asc(allocations::Column::Id)
        .all(db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))
}

/// Every active allocation (the capacity rollup input).
///
/// # Errors
///
/// When the query fails.
pub async fn all_allocations(db: &DatabaseConnection) -> Result<Vec<allocations::Model>> {
    allocations::Entity::find()
        .filter(allocations::Column::DeletedAt.is_null())
        .all(db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))
}
