//! Subject rights & retention (CRM-R20 / CRM-D14): the subject-access
//! export, erasure-as-anonymisation, and the retention report + sweep.
//! Erasure and the sweep are destructive-classified POSTs (`/erase`,
//! `/sweep` — `access=admin` under enforcement, via
//! [`crate::auth::DESTRUCTIVE_POST_SUFFIXES`]). CRM covers its own
//! store only: what the upstream person/organization/worker services
//! hold is the deployment's coordination duty, stated in the payloads.

use loco_rs::prelude::*;
use sea_orm::{ActiveValue, ConnectionTrait};
use uuid::Uuid;

use super::{record_rejection, unprocessable};
use crate::auth::{self, MaybeAuthUser};
use crate::models::_entities::{
    activities, consent_events, contacts, deals, leads, nurture_enrollments, tickets,
};
use crate::models::audit_logs::Model as Audit;
use crate::models::records;
use crate::rules::privacy as rules;

/// The tombstone URN an erased contact's `person_ref` becomes: a
/// syntactically valid `EntityRef` that resolves to no one.
const TOMBSTONE_PERSON: &str = "person:00000000-0000-0000-0000-000000000000";

/// The scrub placeholder for erased free text and names.
const ERASED: &str = "[erased]";

/// Ticket statuses that count as a **live** engagement, blocking
/// erasure (`rules::lifecycle::TICKET`: only these two are non-terminal).
const OPEN_TICKET_STATUSES: [&str; 2] = ["open", "pending"];

/// Rows keyed by one contact in `module` (helper macro: filter on the
/// given column, return the models as JSON — no `deleted_at` filter,
/// so a soft-deleted row still appears in the subject's own export).
macro_rules! rows_for {
    ($db:expr, $module:ident, $column:ident, $pid:expr) => {
        serde_json::json!(
            $module::Entity::find()
                .filter($module::Column::$column.eq($pid))
                .all($db)
                .await?
        )
    };
}

/// `GET /api/contacts/{pid}/subject-access` — everything CRM holds
/// keyed to this contact, in one audited JSON document. Exclusions
/// are named, not hidden: campaign-level counters are aggregate (no
/// per-recipient send log exists to attribute to one contact), and
/// account/organization data is a separate subject.
#[debug_handler]
async fn subject_access(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    let contact = records::find_contact(&ctx.db, records::parse_pid(&pid)?).await?;
    let obligations = auth::authorize_record(
        &caller,
        authentication_verifier::Action::Read,
        &auth::contact_resource_attrs(&contact),
    )
    .map_err(record_rejection)?;
    // A masked read of a *full export* would be a contradiction — the
    // export exists to disclose everything. Refuse rather than leak:
    // subject access is for the subject (`$sub`) and unmasked-read
    // personas (sales manager/admin/svc), never the masked fallback.
    if obligations.iter().any(|o| o == "mask") {
        return Err(record_rejection((
            axum::http::StatusCode::FORBIDDEN,
            "subject access requires an unmasked read (the export discloses \
             deal amounts and consent history; masked callers cannot receive it)"
                .to_string(),
        )));
    }
    let db = &ctx.db;
    let cpid = contact.pid;
    let export = serde_json::json!({
        "as_of": chrono::Utc::now(),
        "contact": contact,
        "consent_history": rows_for!(db, consent_events, ContactPid, cpid),
        "activities": rows_for!(db, activities, SubjectPid, cpid),
        "leads": rows_for!(db, leads, ContactPid, cpid),
        "deals_as_primary_contact": rows_for!(db, deals, PrimaryContactPid, cpid),
        "tickets": rows_for!(db, tickets, ContactPid, cpid),
        "nurture_enrollments": rows_for!(db, nurture_enrollments, ContactPid, cpid),
        "exclusions": [
            "campaign recipient/delivery/open/click counters: aggregate simulated counters, \
             not a per-recipient log — CRM stores no row attributing one send to one contact",
            "the contact's account (organization-level relationship data): a separate \
             subject; request its own export if the account itself is a data subject",
            "upstream person/organization/worker records: held by the identity services, \
             not CRM — coordinate subject access there too",
        ],
    });
    Audit::record(
        &ctx.db,
        "contact",
        cpid,
        "subject_access_exported",
        caller.actor(),
        None,
    )
    .await?;
    format::json(export)
}

/// Whether `contact_pid` currently has an open deal (naming it primary
/// contact, not soft-deleted, not yet closed).
async fn has_open_deal<C: ConnectionTrait>(db: &C, contact_pid: Uuid) -> Result<bool> {
    Ok(deals::Entity::find()
        .filter(deals::Column::PrimaryContactPid.eq(contact_pid))
        .filter(deals::Column::DeletedAt.is_null())
        .filter(deals::Column::ClosedAt.is_null())
        .one(db)
        .await?
        .is_some())
}

/// Whether `contact_pid` currently has an open (`open`/`pending`)
/// support ticket.
async fn has_open_ticket<C: ConnectionTrait>(db: &C, contact_pid: Uuid) -> Result<bool> {
    Ok(tickets::Entity::find()
        .filter(tickets::Column::ContactPid.eq(contact_pid))
        .filter(tickets::Column::DeletedAt.is_null())
        .filter(tickets::Column::Status.is_in(OPEN_TICKET_STATUSES))
        .one(db)
        .await?
        .is_some())
}

/// Whether `contact_pid` currently has an active nurture enrolment.
async fn has_active_nurture<C: ConnectionTrait>(db: &C, contact_pid: Uuid) -> Result<bool> {
    Ok(nurture_enrollments::Entity::find()
        .filter(nurture_enrollments::Column::ContactPid.eq(contact_pid))
        .filter(nurture_enrollments::Column::DeletedAt.is_null())
        .filter(nurture_enrollments::Column::Status.eq("active"))
        .one(db)
        .await?
        .is_some())
}

/// `POST /api/contacts/{pid}/erase` — anonymise (CRM-D14): scrub the
/// contact's identity fields to a tombstone and soft-delete the row,
/// scrub the personal free text linked to it (activity summaries
/// logged directly about the contact, the lead record's name/email),
/// and remove working-group roster entries naming it. Deals, tickets
/// and consent history remain — keyed to a pid that no longer
/// identifies anyone — under the same "financial/operational rows
/// remain" posture the family uses elsewhere (WPM keeps payroll rows;
/// CRM has no monetary field on Contact itself to null, so nothing
/// analogous to WPM's `salary_minor` needs clearing here — the
/// monetary data lives on Deal rows, which stay for revenue-reporting
/// continuity). Refused while a live engagement exists (CRM-R20).
/// Destructive-classified; audited with counts.
#[debug_handler]
async fn erase(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    let contact = records::find_contact(&ctx.db, records::parse_pid(&pid)?).await?;
    auth::authorize_record(
        &caller,
        authentication_verifier::Action::Destructive,
        &auth::contact_resource_attrs(&contact),
    )
    .map_err(record_rejection)?;
    let cpid = contact.pid;
    let (open_deal, open_ticket, active_nurture) = (
        has_open_deal(&ctx.db, cpid).await?,
        has_open_ticket(&ctx.db, cpid).await?,
        has_active_nurture(&ctx.db, cpid).await?,
    );
    if !rules::erasable(open_deal, open_ticket, active_nurture) {
        return Err(unprocessable(
            "erasure requires no open deal, no open ticket, and no active nurture \
             enrolment (a live engagement is the lawful basis for holding the data)",
        ));
    }
    let txn = ctx.db.begin().await?;
    // Identity fields scrubbed in place; the row soft-deleted.
    let mut scrubbed: contacts::ActiveModel = contact.into();
    scrubbed.display_name = ActiveValue::set(ERASED.to_string());
    scrubbed.person_ref = ActiveValue::set(TOMBSTONE_PERSON.to_string());
    scrubbed.job_title = ActiveValue::set(None);
    scrubbed.marketing_consent = ActiveValue::set("withdrawn".to_string());
    scrubbed.deleted_at = ActiveValue::set(Some(chrono::Utc::now().into()));
    scrubbed.update(&txn).await?;
    // Free text linked to this contact, and rows that are about it only.
    let statements = [
        format!(
            "UPDATE activities SET summary = '{ERASED}' WHERE subject_kind = 'contact' \
             AND subject_pid = '{cpid}' AND deleted_at IS NULL"
        ),
        format!(
            "UPDATE leads SET display_name = '{ERASED}', email = NULL \
             WHERE contact_pid = '{cpid}' AND deleted_at IS NULL"
        ),
        format!("DELETE FROM working_group_members WHERE contact_pid = '{cpid}'"),
    ];
    let mut affected = Vec::new();
    for statement in &statements {
        let result = txn.execute_unprepared(statement).await?;
        affected.push(result.rows_affected());
    }
    Audit::record(
        &txn,
        "contact",
        cpid,
        "erased",
        caller.actor(),
        Some(serde_json::json!({
            "activity_summaries_scrubbed": affected[0],
            "leads_scrubbed": affected[1],
            "working_group_memberships_deleted": affected[2],
        })),
    )
    .await?;
    txn.commit().await?;
    format::json(serde_json::json!({
        "erased": cpid,
        "note": "anonymised, not deleted: deals/tickets/consent history remain \
                 under CRM's own retention posture, keyed to a pid that no \
                 longer identifies anyone; coordinate erasure with the \
                 upstream identity services",
    }))
}

/// The retention horizon in days (`CRM_RETENTION_DAYS`, default 365,
/// floor 30 — CRM-D14).
fn horizon_days() -> i64 {
    rules::retention_days(std::env::var("CRM_RETENTION_DAYS").ok().as_deref())
}

/// `GET /api/retention` — the report: per table, soft-deleted rows
/// older than the horizon, plus a count of contacts whose marketing
/// consent has stood `withdrawn` since before the horizon. That count
/// is **informational only** — unlike WPM's expired-consent
/// candidates (which the sweep scrubs directly, because a candidate
/// has no employment-style lawful-basis gate), a CRM contact carries
/// the same `/erase` gate (CRM-R20) regardless of consent state, so
/// the sweep never bulk-anonymises contacts; an operator reviews this
/// count and erases individually. Read-only; the sweep is the
/// separate destructive POST.
#[debug_handler]
async fn retention_report(State(ctx): State<AppContext>) -> Result<Response> {
    let days = horizon_days();
    let mut tables = serde_json::Map::new();
    for table in rules::SOFT_DELETED_TABLES {
        let count = ctx
            .db
            .query_one_raw(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                format!(
                    "SELECT COUNT(*) AS n FROM {table} \
                     WHERE deleted_at < now() - interval '{days} days'"
                ),
            ))
            .await?
            .and_then(|row| row.try_get::<i64>("", "n").ok())
            .unwrap_or(0);
        if count > 0 {
            tables.insert((*table).to_string(), serde_json::json!(count));
        }
    }
    let withdrawn_consent_past_horizon = contacts::Entity::find()
        .filter(contacts::Column::DeletedAt.is_null())
        .filter(contacts::Column::MarketingConsent.eq("withdrawn"))
        .filter(
            contacts::Column::ConsentChangedAt
                .lt(chrono::Utc::now() - chrono::Duration::days(days)),
        )
        .all(&ctx.db)
        .await?
        .len();
    format::json(serde_json::json!({
        "as_of": chrono::Utc::now(),
        "horizon_days": days,
        "soft_deleted_past_horizon": tables,
        "withdrawn_consent_past_horizon": withdrawn_consent_past_horizon,
        "derivation": "soft-deleted rows older than the horizon are hard-deleted by the \
                       sweep; contacts with marketing_consent = withdrawn since before the \
                       horizon are reported (not auto-scrubbed — each still needs the \
                       /erase gate, CRM-R20); the horizon floors at 30 days (CRM-D14)",
    }))
}

/// `POST /api/retention/sweep` — hard-delete soft-deleted rows past
/// the horizon across every soft-deleting table. Destructive-classified;
/// audited with counts.
#[debug_handler]
async fn retention_sweep(State(ctx): State<AppContext>, caller: MaybeAuthUser) -> Result<Response> {
    let days = horizon_days();
    let txn = ctx.db.begin().await?;
    let mut deleted = serde_json::Map::new();
    let mut total: u64 = 0;
    for table in rules::SOFT_DELETED_TABLES {
        let result = txn
            .execute_unprepared(&format!(
                "DELETE FROM {table} WHERE deleted_at < now() - interval '{days} days'"
            ))
            .await?;
        if result.rows_affected() > 0 {
            deleted.insert(
                (*table).to_string(),
                serde_json::json!(result.rows_affected()),
            );
            total += result.rows_affected();
        }
    }
    Audit::record(
        &txn,
        "retention",
        Uuid::nil(),
        "retention_swept",
        caller.actor(),
        Some(serde_json::json!({
            "horizon_days": days,
            "rows_deleted": total,
        })),
    )
    .await?;
    txn.commit().await?;
    format::json(serde_json::json!({
        "horizon_days": days,
        "deleted": deleted,
        "rows_deleted": total,
    }))
}

/// The privacy routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/contacts/{pid}/subject-access", get(subject_access))
        .add("/contacts/{pid}/erase", post(erase))
        .add("/retention", get(retention_report))
        .add("/retention/sweep", post(retention_sweep))
}
