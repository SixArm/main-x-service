//! Shared record helpers: `ActiveModelBehavior` impls for every domain
//! entity plus the per-entity `find_active_by_pid` finders (active =
//! not soft-deleted).
//!
//! The finders return the **loco** `Result` with `Error::NotFound` on
//! a missing row (not `ModelError::EntityNotFound`): loco 0.16's
//! `IntoResponse` catch-all turns an unmapped model error into a 500,
//! so mapping here keeps every `GET /…/{pid}` contract an honest 404.

use loco_rs::prelude::*;
use sea_orm::ConnectionTrait;
use uuid::Uuid;

use super::_entities::{
    bays, bed_requests, beds, infection_flags, red_green_days, sites, stays, transfers, wards,
};

impl ActiveModelBehavior for sites::ActiveModel {}
impl ActiveModelBehavior for wards::ActiveModel {}
impl ActiveModelBehavior for bays::ActiveModel {}
impl ActiveModelBehavior for beds::ActiveModel {}
impl ActiveModelBehavior for stays::ActiveModel {}
impl ActiveModelBehavior for transfers::ActiveModel {}
impl ActiveModelBehavior for bed_requests::ActiveModel {}
impl ActiveModelBehavior for red_green_days::ActiveModel {}
impl ActiveModelBehavior for infection_flags::ActiveModel {}

/// Parse a path `pid` string to a [`Uuid`], mapping failure to `404`
/// (an unparsable pid can never name a record).
///
/// # Errors
///
/// [`Error::NotFound`] when `pid` is not a UUID.
pub fn parse_pid(pid: &str) -> Result<Uuid> {
    Uuid::parse_str(pid).map_err(|_| Error::NotFound)
}

/// Generate the per-entity `find_active_by_pid` finder: the row with
/// this `pid` whose `deleted_at` is null, else `EntityNotFound`.
macro_rules! find_active_by_pid {
    ($fn_name:ident, $module:ident) => {
        /// Find the active (not soft-deleted) row by public id.
        ///
        /// # Errors
        ///
        /// [`Error::NotFound`] when absent or soft-deleted; any query
        /// error otherwise.
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

find_active_by_pid!(find_site, sites);
find_active_by_pid!(find_ward, wards);
find_active_by_pid!(find_bay, bays);
find_active_by_pid!(find_bed, beds);
find_active_by_pid!(find_stay, stays);
find_active_by_pid!(find_bed_request, bed_requests);
