//! Shared record helpers: `ActiveModelBehavior` impls for every domain
//! entity plus the per-entity `find_*` finders (active = not
//! soft-deleted).
//!
//! The finders return the **loco** `Result` with `Error::NotFound` on
//! a missing row (not `ModelError::EntityNotFound`): loco 0.16's
//! `IntoResponse` catch-all turns an unmapped model error into a 500,
//! so mapping here keeps every `GET /…/{pid}` contract an honest 404
//! (family lesson, pinned in the request tests).

use loco_rs::prelude::*;
use sea_orm::ConnectionTrait;
use uuid::Uuid;

use super::_entities::{
    accounts, activities, articles, campaigns, consent_events, contacts, deals, forecast_snapshots,
    leads, nurture_enrollments, nurture_sequences, nurture_steps, pipeline_stages, pipelines,
    segments, sla_policies, tickets,
};

impl ActiveModelBehavior for contacts::ActiveModel {}
impl ActiveModelBehavior for accounts::ActiveModel {}
impl ActiveModelBehavior for activities::ActiveModel {}
impl ActiveModelBehavior for consent_events::ActiveModel {}
impl ActiveModelBehavior for leads::ActiveModel {}
impl ActiveModelBehavior for pipelines::ActiveModel {}
impl ActiveModelBehavior for pipeline_stages::ActiveModel {}
impl ActiveModelBehavior for deals::ActiveModel {}
impl ActiveModelBehavior for forecast_snapshots::ActiveModel {}
impl ActiveModelBehavior for segments::ActiveModel {}
impl ActiveModelBehavior for campaigns::ActiveModel {}
impl ActiveModelBehavior for nurture_sequences::ActiveModel {}
impl ActiveModelBehavior for nurture_steps::ActiveModel {}
impl ActiveModelBehavior for nurture_enrollments::ActiveModel {}
impl ActiveModelBehavior for sla_policies::ActiveModel {}
impl ActiveModelBehavior for tickets::ActiveModel {}
impl ActiveModelBehavior for articles::ActiveModel {}

/// Parse a path `pid` string to a [`Uuid`], mapping failure to `404`.
///
/// # Errors
///
/// [`Error::NotFound`] when `pid` is not a UUID.
pub fn parse_pid(pid: &str) -> Result<Uuid> {
    Uuid::parse_str(pid).map_err(|_| Error::NotFound)
}

/// Generate the per-entity `find_*` finder: the row with this `pid`
/// whose `deleted_at` is null, else `Error::NotFound`.
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

find_active_by_pid!(find_contact, contacts);
find_active_by_pid!(find_account, accounts);
find_active_by_pid!(find_lead, leads);
find_active_by_pid!(find_pipeline, pipelines);
find_active_by_pid!(find_stage, pipeline_stages);
find_active_by_pid!(find_deal, deals);
find_active_by_pid!(find_segment, segments);
find_active_by_pid!(find_campaign, campaigns);
find_active_by_pid!(find_sequence, nurture_sequences);
find_active_by_pid!(find_enrollment, nurture_enrollments);
find_active_by_pid!(find_ticket, tickets);
find_active_by_pid!(find_article, articles);
