//! Cross-service entity-link endpoints — the **write side** of
//! `agents/share/cross-service-linking.md` (§4.1, §4.2) for the
//! care-pathway service.
//!
//! The v1 kind here is `continues_as` (§9): one subject's journey
//! passing from a care-pathway **instance** into the next episode —
//! another pathway instance (a transfer), an inpatient stay, or a case.
//! It is what lets time-based analysis follow a journey across a service
//! boundary instead of stopping at it.
//!
//! **The edge originates from an instance, not a template.** A journey
//! belongs to an enrolment; the template is a document that many
//! journeys share. Linking templates would assert that two *documents*
//! continue into one another, which means nothing.
//!
//! The write is **optimistic**: it records the assertion and emits a
//! `linked` event, never calling the target service. Verification is the
//! aggregator's concern (§5), which is the only party that sees both
//! ends.
//!
//! ## Governance
//!
//! `continues_as` is a **high**-sensitivity kind, for the same reason
//! `subject_of` is: "this patient's stroke pathway continued as that
//! inpatient stay" is clinical data about a named person. Three
//! consequences, all enforced here:
//!
//! - **Authorised at the read-the-journey level.** Creating, listing and
//!   withdrawing an edge are authorised against the *pathway template's*
//!   resource attributes (care setting, and the sensitive-setting flag
//!   that covers mental-health and palliative pathways) — so an edge on
//!   a mental-health journey is exactly as hard to touch as the journey
//!   itself.
//! - **Every write is audited** (`linked` / `unlinked`), in the same
//!   transaction as the edge under the durable transport.
//! - **The bulk reconciliation pull is a privileged read.** It surfaces
//!   every journey edge at once, which is a different disclosure from
//!   reading one; it is gated as [`Action::Destructive`], which the
//!   built-in default policy grants only to a machine peer (`svc=true`)
//!   or an admin.
//!
//! ### A denied request is a `404`, not a `403`
//!
//! On these endpoints a policy denial is reported as **not found**. A
//! `403` would answer the question the caller was not allowed to ask:
//! "this journey exists, and you may not see it" is itself a
//! disclosure, and on a mental-health or palliative pathway it is the
//! disclosure that matters. An empty list and a denied read are
//! deliberately indistinguishable.
//!
//! The cost is real and worth stating: a misconfigured operator sees
//! `404` where the truthful answer is "your policy denies this", which
//! is harder to debug. The **audit trail** carries the denial, so the
//! information is not lost — it is moved somewhere the caller cannot
//! read. That trade is right here and is **not** made on the pathway
//! record endpoints, which still return `403`: a care-pathway template
//! is a document, not a person, and knowing one exists discloses
//! nothing about anybody.

use std::collections::BTreeMap;

use authentication_verifier::Action;
use axum::http::StatusCode;
use entity_ref::{EdgeKind, EntityRef, EntityType};
use loco_rs::controller::ErrorDetail;
use loco_rs::prelude::*;
use sea_orm::ColumnTrait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::MaybeAuthUser;
use crate::journey::{self, Leg};
use crate::models::_entities::pathway_instances;
use crate::models::care_pathways::Model as PathwayModel;
use crate::models::entity_links::{Model as EntityLinkModel, NewEdge};
use crate::streaming;

/// Cap on one reconciliation pull, so a full replay cannot become an
/// unbounded response.
const MAX_BULK_EDGES: u64 = 5000;

/// Body of `POST /api/instances/{pid}/links`.
#[derive(Debug, Deserialize)]
struct LinkRequest {
    /// The edge kind token (§9), e.g. `continues_as`.
    kind: String,
    /// The far record's `EntityRef` URN, e.g. `patient_flow_stay:<uuid>`.
    to_ref: String,
    /// Optional role label.
    #[serde(default)]
    role: Option<String>,
    /// Optional confidence in `[0.0, 1.0]`.
    #[serde(default)]
    confidence: Option<f64>,
    /// Optional provenance override; defaults to `operator`.
    #[serde(default)]
    provenance: Option<String>,
    /// When the continuation began (`YYYY-MM-DD`).
    #[serde(default)]
    valid_from: Option<chrono::NaiveDate>,
    /// When it ended.
    #[serde(default)]
    valid_to: Option<chrono::NaiveDate>,
}

/// A stored edge as returned to an operator.
#[derive(Debug, Serialize)]
struct LinkView {
    id: String,
    /// This instance as an `EntityRef` URN.
    from_ref: String,
    kind: String,
    to_ref: String,
    role: Option<String>,
    confidence: Option<f64>,
    provenance: String,
    valid_from: Option<String>,
    valid_to: Option<String>,
}

impl LinkView {
    fn of(m: &EntityLinkModel) -> Self {
        Self {
            id: m.id.to_string(),
            from_ref: format!("care_pathway_instance:{}", m.from_pid),
            kind: m.kind.clone(),
            to_ref: m.to_ref.clone(),
            role: m.role.clone(),
            confidence: m.confidence,
            provenance: m.provenance.clone(),
            valid_from: m.valid_from.map(|d| d.to_string()),
            valid_to: m.valid_to.map(|d| d.to_string()),
        }
    }
}

/// The canonical §4.2 edge detail — the shape the `linked` / `unlinked`
/// events carry, so the aggregator deserializes a reconciliation pull
/// and an event stream with the same code.
#[derive(Debug, Serialize)]
struct EdgeDetail {
    edge_id: String,
    from_ref: String,
    to_ref: String,
    edge_kind: String,
    role: Option<String>,
    confidence: Option<f64>,
    provenance: String,
    valid_from: Option<String>,
    valid_to: Option<String>,
}

impl EdgeDetail {
    fn of(m: &EntityLinkModel) -> Self {
        Self {
            edge_id: m.id.to_string(),
            from_ref: format!("care_pathway_instance:{}", m.from_pid),
            to_ref: m.to_ref.clone(),
            edge_kind: m.kind.clone(),
            role: m.role.clone(),
            confidence: m.confidence,
            provenance: m.provenance.clone(),
            valid_from: m.valid_from.map(|d| d.to_string()),
            valid_to: m.valid_to.map(|d| d.to_string()),
        }
    }
}

/// Query params for the bulk-links endpoint.
#[derive(Debug, Deserialize)]
struct BulkParams {
    /// RFC3339 lower bound on `created_at` for an incremental pull;
    /// absent ⇒ a full replay.
    #[serde(default)]
    since: Option<String>,
}

/// Validate an incoming edge: `to_ref` must parse, `kind` must be known,
/// and the kind must permit `care_pathway_instance → <to type>` (§9).
///
/// For this service that admits exactly `continues_as` into another
/// pathway instance, an inpatient stay, or a case — and rejects
/// everything else: a `same_identity`, a `continues_as` pointing at a
/// person, a reversed direction, a malformed ref. Pure and DB-free, so
/// the accept/reject matrix is unit-tested without a database.
///
/// # Errors
///
/// A human-readable reason the handler surfaces as `422`.
pub fn validate_edge(
    kind: &str,
    to_ref: &str,
) -> std::result::Result<(EdgeKind, EntityRef), String> {
    let to = to_ref
        .parse::<EntityRef>()
        .map_err(|e| format!("invalid to_ref: {e}"))?;
    let edge_kind =
        EdgeKind::from_token(kind).ok_or_else(|| format!("unknown edge kind: {kind:?}"))?;
    if !edge_kind.permits(EntityType::CarePathwayInstance, to.entity_type) {
        return Err(format!(
            "edge kind `{edge_kind}` does not permit care_pathway_instance → {} \
             (this service originates only `continues_as` → care_pathway_instance \
             | patient_flow_stay | case)",
            to.entity_type
        ));
    }
    Ok((edge_kind, to))
}

/// `422` with a reason.
fn unprocessable(reason: &str) -> Error {
    Error::CustomError(
        StatusCode::UNPROCESSABLE_ENTITY,
        ErrorDetail::new("validation", reason),
    )
}

/// Map a record-level authorization rejection to its HTTP shape on the
/// **link** endpoints, where a denial must not disclose existence.
///
/// A `403` ("forbidden") is folded into `404` ("not found") so that a
/// denied read is indistinguishable from a journey with no edges — see
/// the module docs for why, and for why the pathway record endpoints
/// deliberately do not do this. A `401` is left alone: "you sent no
/// credential" tells the caller nothing about what exists, and turning
/// it into a `404` would only make an unauthenticated client retry
/// forever against a URL it should be authenticating to.
fn record_rejection((status, reason): (StatusCode, String)) -> Error {
    if status == StatusCode::FORBIDDEN {
        return Error::NotFound;
    }
    Error::CustomError(status, ErrorDetail::new("unauthorized", &reason))
}

/// Find one live instance, or `404`.
async fn find_instance(ctx: &AppContext, raw: &str) -> Result<pathway_instances::Model> {
    let pid = Uuid::parse_str(raw).map_err(|_| Error::NotFound)?;
    pathway_instances::Entity::find()
        .filter(pathway_instances::Column::Pid.eq(pid))
        .filter(pathway_instances::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await?
        .ok_or(Error::NotFound)
}

/// Authorise an action on a journey's edges at the **read-the-journey**
/// level: the instance's own pathway template supplies the resource
/// attributes, so a mental-health journey's edges inherit that
/// pathway's protection. A no-op when enforcement is off.
///
/// Returns the template, which the caller needs for the event name.
async fn authorize_journey(
    ctx: &AppContext,
    caller: &MaybeAuthUser,
    action: Action,
    instance: &pathway_instances::Model,
) -> Result<PathwayModel> {
    let template = PathwayModel::find_by_pid(&ctx.db, &instance.pathway_pid.to_string())
        .await
        .map_err(|_| Error::NotFound)?;
    let attrs = crate::auth::care_pathway_resource_attrs(&template.to_pathway()?);
    crate::auth::authorize_record(caller, action, &attrs).map_err(record_rejection)?;
    Ok(template)
}

/// `POST /api/instances/{pid}/links` — assert an outbound journey edge.
#[debug_handler]
async fn create_link(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(req): Json<LinkRequest>,
) -> Result<Response> {
    let instance = find_instance(&ctx, &pid).await?;
    let template = authorize_journey(&ctx, &caller, Action::Write, &instance).await?;
    let (edge_kind, to) =
        validate_edge(&req.kind, &req.to_ref).map_err(|reason: String| unprocessable(&reason))?;
    // A journey cannot continue as itself; the edge would be a cycle of
    // length one and would make a stitched timeline non-terminating.
    if to.to_string() == format!("care_pathway_instance:{}", instance.pid) {
        return Err(unprocessable("a journey cannot continue as itself"));
    }
    let provenance = req
        .provenance
        .filter(|p| !p.trim().is_empty())
        .unwrap_or_else(|| "operator".to_string());
    let edge = NewEdge {
        from_pid: instance.pid,
        kind: edge_kind.as_str().to_string(),
        // Store the canonical URN form, normalised by `EntityRef`.
        to_ref: to.to_string(),
        role: req.role,
        confidence: req.confidence,
        provenance,
        valid_from: req.valid_from,
        valid_to: req.valid_to,
    };
    let link = streaming::link_and_emit(&ctx.db, &edge, &template.name, caller.actor()).await?;
    format::json(LinkView::of(&link))
}

/// `GET /api/instances/{pid}/links` — this journey's active outbound
/// edges.
#[debug_handler]
async fn list_links(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
) -> Result<Response> {
    let instance = find_instance(&ctx, &pid).await?;
    authorize_journey(&ctx, &caller, Action::Read, &instance).await?;
    let rows = EntityLinkModel::list_active(&ctx.db, instance.pid).await?;
    format::json(rows.iter().map(LinkView::of).collect::<Vec<_>>())
}

/// `DELETE /api/instances/{pid}/links/{id}` — withdraw an edge.
#[debug_handler]
async fn delete_link(
    Path((pid, id)): Path<(String, String)>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
) -> Result<Response> {
    let instance = find_instance(&ctx, &pid).await?;
    let template = authorize_journey(&ctx, &caller, Action::Delete, &instance).await?;
    let Ok(edge_id) = Uuid::parse_str(&id) else {
        return Err(unprocessable("invalid link id"));
    };
    let link = EntityLinkModel::find_active(&ctx.db, instance.pid, edge_id)
        .await
        .map_err(super::model_not_found)?;
    streaming::unlink_and_emit(&ctx.db, link, &template.name, caller.actor()).await?;
    format::empty_json()
}

/// Authorise the cross-journey **bulk** governed read.
///
/// Unlike the per-instance endpoints there is no single journey to key
/// on, and a dump of every `continues_as` edge is a materially different
/// disclosure from reading one: it maps which patients moved between
/// which services. Treated as [`Action::Destructive`], which the
/// built-in default policy grants only to a machine peer (`svc=true`) or
/// an admin — a deployment grants a dedicated reconcile identity by
/// policy. A no-op when enforcement is off.
fn authorize_bulk(caller: &MaybeAuthUser) -> Result<()> {
    let resource: BTreeMap<String, Vec<String>> =
        BTreeMap::from([("governed".to_string(), vec!["continues_as".to_string()])]);
    crate::auth::authorize_record(caller, Action::Destructive, &resource)
        .map_err(record_rejection)?;
    Ok(())
}

/// `GET /api/instances/links[?since=]` — every active outbound edge, in
/// the canonical §4.2 shape, for the aggregator's reconciliation (§8).
#[debug_handler]
async fn bulk_links(
    Query(params): Query<BulkParams>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
) -> Result<Response> {
    authorize_bulk(&caller)?;
    let since = match params.since.as_deref() {
        None => None,
        Some(s) => Some(
            chrono::DateTime::parse_from_rfc3339(s)
                .map_err(|e| unprocessable(&format!("invalid `since` (want RFC3339): {e}")))?,
        ),
    };
    let rows = EntityLinkModel::all_active(&ctx.db, since, MAX_BULK_EDGES).await?;
    let edges: Vec<EdgeDetail> = rows.iter().map(EdgeDetail::of).collect();
    format::json(serde_json::json!({
        "note": "every active outbound journey edge, oldest first, capped at \
                 5000. `since` bounds an incremental pull.",
        "capped": rows.len() as u64 >= MAX_BULK_EDGES,
        "edges": edges,
    }))
}

/// Resolve one **local** leg — a pathway instance in this service — by
/// reading its own clock and segments. No HTTP, no configuration: the
/// most common journey, a transfer between pathways, stitches with
/// nothing wired up at all.
async fn local_leg(ctx: &AppContext, pid: Uuid, hop: usize, as_of_ms: i64) -> Leg {
    let entity_ref = journey::instance_ref(pid);
    let Ok(instance) = find_instance(ctx, &pid.to_string()).await else {
        return Leg {
            entity_ref,
            hop,
            status: journey::LegStatus::UnavailableOrDenied,
            detail: journey::LegStatus::UnavailableOrDenied.detail(),
            lead_time_ms: None,
            value_time_ms: None,
            clock_start_ms: None,
            clock_stop_ms: None,
        };
    };
    match super::tba::analyze_instance(ctx, &instance, as_of_ms).await {
        Ok(analysis) => Leg {
            entity_ref,
            hop,
            status: journey::LegStatus::Resolved,
            detail: journey::LegStatus::Resolved.detail(),
            lead_time_ms: Some(analysis.lead_time_ms),
            value_time_ms: Some(analysis.value_time_ms),
            clock_start_ms: Some(analysis.clock.start_ms),
            clock_stop_ms: Some(analysis.clock.stop_ms),
        },
        Err(_) => Leg {
            entity_ref,
            hop,
            status: journey::LegStatus::Unreachable,
            detail: journey::LegStatus::Unreachable.detail(),
            lead_time_ms: None,
            value_time_ms: None,
            clock_start_ms: None,
            clock_stop_ms: None,
        },
    }
}

/// Resolve a **remote** leg by asking the far service for its timeline,
/// forwarding the caller's own credential.
///
/// The bearer is the *caller's*, never a service identity: with a peer
/// token this service would be a confused deputy, handing a caller a
/// timeline the far service would have refused them. When the caller
/// presented no bearer, none is forwarded — preserving the default-off
/// posture instead of silently escalating.
async fn remote_leg(target: &EntityRef, hop: usize, bearer: Option<&str>) -> Leg {
    let entity_ref = target.to_string();
    let blank = |status: journey::LegStatus| Leg {
        entity_ref: entity_ref.clone(),
        hop,
        status,
        detail: status.detail(),
        lead_time_ms: None,
        value_time_ms: None,
        clock_start_ms: None,
        clock_stop_ms: None,
    };
    let Some(template) = journey::url_template_for(target.entity_type) else {
        return blank(journey::LegStatus::NotConfigured);
    };
    let url = journey::leg_url(&template, target.id);
    let mut request = journey::leg_client().get(&url);
    if let Some(bearer) = bearer {
        request = request.header(axum::http::header::AUTHORIZATION, bearer);
    }
    let Ok(response) = request.send().await else {
        return blank(journey::LegStatus::Unreachable);
    };
    let status = journey::classify_status(response.status().as_u16());
    if !status.is_resolved() {
        return blank(status);
    }
    let Ok(body) = response.json::<serde_json::Value>().await else {
        return blank(journey::LegStatus::Unreachable);
    };
    let Some(timeline) = journey::parse_leg(&body) else {
        // The peer answered, but not with the contract. Reported as
        // unreachable rather than guessed at.
        return blank(journey::LegStatus::Unreachable);
    };
    Leg {
        entity_ref,
        hop,
        status: journey::LegStatus::Resolved,
        detail: journey::LegStatus::Resolved.detail(),
        lead_time_ms: Some(timeline.lead_time_ms),
        value_time_ms: Some(timeline.value_time_ms),
        clock_start_ms: Some(timeline.clock_start_ms),
        clock_stop_ms: Some(timeline.clock_stop_ms),
    }
}

/// `GET /api/instances/{pid}/journey` — the whole journey, stitched
/// across every `continues_as` link.
///
/// Walks the chain breadth-first (it may branch: the kind is M:N),
/// resolving local legs from this database and remote legs from the far
/// service, then combines them. The combined figures are **withheld
/// unless every leg resolved** — see [`crate::journey`] for why a
/// partial stitched total is a wrong number rather than an imprecise
/// one.
#[debug_handler]
async fn journey_view(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    headers: axum::http::HeaderMap,
    caller: MaybeAuthUser,
) -> Result<Response> {
    let now = chrono::Utc::now();
    let as_of_ms = now.timestamp_millis();
    let instance = find_instance(&ctx, &pid).await?;
    authorize_journey(&ctx, &caller, Action::Read, &instance).await?;

    // The caller's own credential, forwarded verbatim to each peer.
    let bearer = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(ToString::to_string);

    let mut seen: std::collections::BTreeSet<String> =
        std::collections::BTreeSet::from([journey::instance_ref(instance.pid)]);
    let mut legs = vec![local_leg(&ctx, instance.pid, 0, as_of_ms).await];
    // Frontier of local instances whose outbound edges still need
    // following. A remote leg is a terminus here: this service cannot
    // read another's links, and asking it to would be the aggregator's
    // job, not a fetch.
    let mut frontier = vec![instance.pid];
    let mut hop = 1usize;
    let mut truncated = false;

    while !frontier.is_empty() && hop <= journey::MAX_LEGS {
        let mut next_frontier = Vec::new();
        for from in std::mem::take(&mut frontier) {
            for edge in EntityLinkModel::list_active(&ctx.db, from).await? {
                if edge.kind != EdgeKind::ContinuesAs.as_str() {
                    continue;
                }
                let Ok(target) = edge.to_ref.parse::<EntityRef>() else {
                    continue;
                };
                let Some(target) = journey::next_hop(&mut seen, &target, hop) else {
                    truncated = true;
                    continue;
                };
                if target.entity_type == EntityType::CarePathwayInstance {
                    legs.push(local_leg(&ctx, target.id, hop, as_of_ms).await);
                    next_frontier.push(target.id);
                } else {
                    legs.push(remote_leg(&target, hop, bearer.as_deref()).await);
                }
            }
        }
        frontier = next_frontier;
        hop += 1;
    }
    if !frontier.is_empty() {
        truncated = true;
    }
    if truncated {
        legs.push(Leg {
            entity_ref: String::new(),
            hop,
            status: journey::LegStatus::Truncated,
            detail: journey::LegStatus::Truncated.detail(),
            lead_time_ms: None,
            value_time_ms: None,
            clock_start_ms: None,
            clock_stop_ms: None,
        });
    }

    let totals = journey::stitch(&legs);
    format::json(serde_json::json!({
        "as_of": now,
        "start": journey::instance_ref(instance.pid),
        "note": "the journey followed across `continues_as` links. Each leg is \
                 read under **your** credential, not this service's, so a leg \
                 you may not see is reported as unavailable rather than \
                 fetched on your behalf. The stitched span runs from the \
                 earliest clock start to the latest stop — not the sum of the \
                 legs, because the gap between two episodes is real waiting. \
                 Combined figures are withheld unless every leg resolved: a \
                 total missing a leg understates the journey by exactly the \
                 part nobody could see.",
        "totals": totals,
        "legs": legs,
    }))
}

/// The journey-link routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/instances")
        // The literal path is declared before the `{pid}` captures so it
        // is not swallowed by them.
        .add("/links", get(bulk_links))
        .add("/{pid}/links", post(create_link))
        .add("/{pid}/links", get(list_links))
        .add("/{pid}/links/{id}", delete(delete_link))
        .add("/{pid}/journey", get(journey_view))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uuid() -> String {
        "0c4f1e2a-0000-4000-8000-000000000000".to_string()
    }

    /// A policy denial must be indistinguishable from "no such
    /// journey". If this ever reverts to `403`, the endpoint starts
    /// answering the question the caller was not allowed to ask.
    #[test]
    fn a_denied_request_is_reported_as_not_found() {
        let denied = record_rejection((
            StatusCode::FORBIDDEN,
            "policy denied: sensitive_setting".to_string(),
        ));
        assert!(
            matches!(denied, Error::NotFound),
            "a denial must not disclose existence, got {denied:?}"
        );
        // The reason must not survive into the response either — it
        // names the attribute that denied, which is itself a hint.
        let rendered = format!("{denied:?}");
        assert!(
            !rendered.contains("sensitive_setting"),
            "the denial reason must not leak: {rendered}"
        );
    }

    /// A missing credential stays a `401`: it discloses nothing about
    /// what exists, and folding it into `404` would leave an
    /// unauthenticated client retrying a URL it should authenticate to.
    #[test]
    fn a_missing_credential_stays_unauthorized() {
        let unauthed =
            record_rejection((StatusCode::UNAUTHORIZED, "missing bearer token".to_string()));
        assert!(
            !matches!(unauthed, Error::NotFound),
            "401 must not be folded into 404"
        );
    }

    #[test]
    fn a_journey_edge_is_accepted_into_its_permitted_far_ends() {
        for target in ["care_pathway_instance", "patient_flow_stay", "case"] {
            let to = format!("{target}:{}", uuid());
            let (kind, parsed) =
                validate_edge("continues_as", &to).unwrap_or_else(|e| panic!("{target}: {e}"));
            assert_eq!(kind, EdgeKind::ContinuesAs);
            assert_eq!(parsed.to_string(), to);
        }
    }

    #[test]
    fn everything_else_is_refused_with_a_reason() {
        // A kind this service does not originate.
        let err = validate_edge("same_identity", &format!("worker:{}", uuid()))
            .expect_err("same_identity");
        assert!(err.contains("does not permit"), "{err}");

        // The right kind pointing somewhere it may not.
        let err = validate_edge("continues_as", &format!("person:{}", uuid())).expect_err("person");
        assert!(err.contains("continues_as"), "{err}");

        // An unknown kind, and a malformed or unknown-type ref.
        assert!(
            validate_edge("teleports_to", &format!("case:{}", uuid()))
                .expect_err("unknown kind")
                .contains("unknown edge kind")
        );
        assert!(
            validate_edge("continues_as", "not-a-ref")
                .expect_err("malformed")
                .contains("invalid to_ref")
        );
        assert!(
            validate_edge("continues_as", &format!("dragon:{}", uuid()))
                .expect_err("unknown type")
                .contains("invalid to_ref")
        );
    }

    #[test]
    fn the_far_end_is_normalised_to_its_canonical_urn() {
        // Uppercase hex in a UUID must not create a second edge for the
        // same target — the upsert key is the stored string.
        let upper = format!("case:{}", uuid().to_uppercase());
        let (_, parsed) = validate_edge("continues_as", &upper).expect("parses");
        assert_eq!(parsed.to_string(), format!("case:{}", uuid()));
    }
}
