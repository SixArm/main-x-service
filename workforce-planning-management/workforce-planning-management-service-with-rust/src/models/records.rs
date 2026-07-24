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
    applications, benchmarks, benefit_enrollments, benefit_plans, candidates, employees,
    feedback_entries, goals, interviews, leave_entitlements, leave_requests, onboarding_items,
    payroll_runs, payslips, requisitions, review_cycles, reviews, shift_assignments, shifts,
    succession_candidates, succession_plans, time_entries, training_enrollments,
};

impl ActiveModelBehavior for employees::ActiveModel {}
impl ActiveModelBehavior for requisitions::ActiveModel {}
impl ActiveModelBehavior for candidates::ActiveModel {}
impl ActiveModelBehavior for applications::ActiveModel {}
impl ActiveModelBehavior for interviews::ActiveModel {}
impl ActiveModelBehavior for onboarding_items::ActiveModel {}
impl ActiveModelBehavior for time_entries::ActiveModel {}
impl ActiveModelBehavior for leave_entitlements::ActiveModel {}
impl ActiveModelBehavior for leave_requests::ActiveModel {}
impl ActiveModelBehavior for shifts::ActiveModel {}
impl ActiveModelBehavior for shift_assignments::ActiveModel {}
impl ActiveModelBehavior for benefit_plans::ActiveModel {}
impl ActiveModelBehavior for benefit_enrollments::ActiveModel {}
impl ActiveModelBehavior for review_cycles::ActiveModel {}
impl ActiveModelBehavior for reviews::ActiveModel {}
impl ActiveModelBehavior for goals::ActiveModel {}
impl ActiveModelBehavior for feedback_entries::ActiveModel {}
impl ActiveModelBehavior for training_enrollments::ActiveModel {}
impl ActiveModelBehavior for succession_plans::ActiveModel {}
impl ActiveModelBehavior for succession_candidates::ActiveModel {}
impl ActiveModelBehavior for payroll_runs::ActiveModel {}
impl ActiveModelBehavior for payslips::ActiveModel {}
impl ActiveModelBehavior for benchmarks::ActiveModel {}

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

find_active_by_pid!(find_employee, employees);
find_active_by_pid!(find_requisition, requisitions);
find_active_by_pid!(find_candidate, candidates);
find_active_by_pid!(find_application, applications);
find_active_by_pid!(find_onboarding_item, onboarding_items);
find_active_by_pid!(find_time_entry, time_entries);
find_active_by_pid!(find_leave_request, leave_requests);
find_active_by_pid!(find_shift, shifts);
find_active_by_pid!(find_shift_assignment, shift_assignments);
find_active_by_pid!(find_benefit_plan, benefit_plans);
find_active_by_pid!(find_benefit_enrollment, benefit_enrollments);
find_active_by_pid!(find_review_cycle, review_cycles);
find_active_by_pid!(find_review, reviews);
find_active_by_pid!(find_training_enrollment, training_enrollments);
find_active_by_pid!(find_succession_plan, succession_plans);
find_active_by_pid!(find_payroll_run, payroll_runs);
