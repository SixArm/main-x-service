//! Record-level access to the **instance layer** (spec §13 T-15h).
//!
//! An enrolment names a person and carries their scores, so it is as
//! sensitive as the pathway it sits on. The decision is taken against the
//! instance's own pathway template, exactly as the journey-link endpoints
//! take it, and every instance, cohort and time-analysis handler goes
//! through here so the rule is written once.
//!
//! A denial is `404`, never `403`: "an enrolment exists here and you may
//! not see it" is itself a disclosure on a mental-health pathway. A `401`
//! is left alone, because "you sent no credential" discloses nothing.
//!
//! Every function is a no-op when `CARE_PATHWAY_REQUIRE_AUTH` is off, and
//! does not touch the database then, so default behaviour is unchanged.

use std::collections::HashMap;

use authentication_verifier::Action;
use axa::error::ErrorDetail;
use axa::prelude::*;
use axum::http::StatusCode;
use uuid::Uuid;

use crate::auth::MaybeAuthUser;
use crate::instruments;
use crate::models::care_pathways::Model as PathwayModel;
use crate::models::{instance_measures, pathway_instances};

/// What a caller may see of one pathway's enrolments.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Access {
    /// A `mask` obligation applies: withhold `subject_ref` and every
    /// measure that is, or cannot be shown not to be, mental-health.
    pub masked: bool,
}

/// Turn a record-level rejection into the HTTP error: `403` becomes `404`.
fn rejection((status, reason): (StatusCode, String)) -> Error {
    if status == StatusCode::FORBIDDEN {
        return Error::NotFound;
    }
    Error::CustomError(status, ErrorDetail::new("unauthorized", &reason))
}

/// The access `caller` has to the enrolments on `template`, or `404`.
fn decide(caller: &MaybeAuthUser, action: Action, template: &PathwayModel) -> Result<Access> {
    let pathway = template.to_pathway()?;
    let attrs = crate::auth::care_pathway_resource_attrs(&pathway);
    let obligations = crate::auth::authorize_record(caller, action, &attrs).map_err(rejection)?;
    Ok(Access {
        masked: obligations.iter().any(|o| o == "mask"),
    })
}

/// Access to the enrolments on the pathway `pathway_pid`. A pathway that
/// is gone is `404`, which is also the safe answer under enforcement.
///
/// # Errors
///
/// `404` on a denial or an unknown pathway; `401` without a credential.
pub async fn pathway_access(
    ctx: &AppContext,
    caller: &MaybeAuthUser,
    action: Action,
    pathway_pid: Uuid,
) -> Result<Access> {
    if !crate::auth::require_auth() {
        return Ok(Access::default());
    }
    let template = PathwayModel::find_by_pid(&ctx.db, &pathway_pid.to_string())
        .await
        .map_err(|_| Error::NotFound)?;
    decide(caller, action, &template)
}

/// Access to one instance, through its pathway template.
///
/// # Errors
///
/// As [`pathway_access`].
pub async fn instance_access(
    ctx: &AppContext,
    caller: &MaybeAuthUser,
    action: Action,
    instance: &pathway_instances::Model,
) -> Result<Access> {
    pathway_access(ctx, caller, action, instance.pathway_pid).await
}

/// Per-request memo of the decision for each pathway, for a handler that
/// reads instances across many pathways and must leave a denied one out
/// rather than fail.
pub struct Visibility<'a> {
    ctx: &'a AppContext,
    caller: &'a MaybeAuthUser,
    seen: HashMap<Uuid, Option<Access>>,
}

impl<'a> Visibility<'a> {
    /// A fresh memo for one request.
    #[must_use]
    pub fn new(ctx: &'a AppContext, caller: &'a MaybeAuthUser) -> Self {
        Self {
            ctx,
            caller,
            seen: HashMap::new(),
        }
    }

    /// The read access to `pathway_pid`, or `None` when it is denied or
    /// unknown. A `401` is returned as an error, not hidden.
    ///
    /// # Errors
    ///
    /// Only the `401` of a missing credential.
    pub async fn read(&mut self, pathway_pid: Uuid) -> Result<Option<Access>> {
        if let Some(found) = self.seen.get(&pathway_pid) {
            return Ok(*found);
        }
        let found = match pathway_access(self.ctx, self.caller, Action::Read, pathway_pid).await {
            Ok(access) => Some(access),
            Err(Error::NotFound) => None,
            Err(other) => return Err(other),
        };
        self.seen.insert(pathway_pid, found);
        Ok(found)
    }
}

/// Whether a measure's content must be withheld under a `mask`
/// obligation: a mental-health instrument, or a free-named measure, whose
/// content cannot be shown not to be one.
#[must_use]
pub fn withheld_when_masked(measure: &instance_measures::Model) -> bool {
    measure
        .instrument
        .as_deref()
        .and_then(instruments::find)
        .is_none_or(|i| i.category == instruments::Category::MentalHealth)
}
