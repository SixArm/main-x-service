//! Shared record helpers: the per-entity `find_*` finders the
//! controllers call (active = not soft-deleted). The SQL is in each
//! table's module.
//!
//! The finders return the **axa** `Result` with `Error::NotFound` on a
//! missing row, so every `GET /…/{pid}` contract is an honest 404.

use axa::prelude::*;
use axa::sqlx::Db;
use uuid::Uuid;

use super::{bays, bed_requests, beds, sites, stays, wards};

/// Parse a path `pid` string to a [`Uuid`], mapping failure to `404`
/// (an unparsable pid can never name a record).
///
/// # Errors
///
/// [`Error::NotFound`] when `pid` is not a UUID.
pub fn parse_pid(pid: &str) -> Result<Uuid> {
    Uuid::parse_str(pid).map_err(|_| Error::NotFound)
}

/// Generate the per-entity `find_*` finder: the row with this `pid`
/// whose `deleted_at` is null, else [`Error::NotFound`].
macro_rules! find_active_by_pid {
    ($fn_name:ident, $module:ident) => {
        /// Find the active (not soft-deleted) row by public id.
        ///
        /// # Errors
        ///
        /// [`Error::NotFound`] when absent or soft-deleted; any query
        /// error otherwise.
        pub async fn $fn_name(db: impl Into<Db<'_>>, pid: Uuid) -> Result<$module::Model> {
            Ok($module::Model::find_active(db, pid).await?)
        }
    };
}

find_active_by_pid!(find_site, sites);
find_active_by_pid!(find_ward, wards);
find_active_by_pid!(find_bay, bays);
find_active_by_pid!(find_bed, beds);
find_active_by_pid!(find_stay, stays);
find_active_by_pid!(find_bed_request, bed_requests);
