//! Read-only graph controllers (spec §9.1). The world-facing API is
//! read-only; all state changes arrive via the apply seam
//! ([`crate::events`]). Every graph response carries an `as_of`
//! watermark (spec §6 FR-17).

use std::collections::{HashMap, HashSet};

use axum::extract::{ConnectInfo, Query};
use axum::http::HeaderMap;
use axum::http::header::USER_AGENT;
use chrono::{DateTime, FixedOffset, Utc};
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use uuid::Uuid;

use entity_ref::{EdgeKind, EntityRef};

use crate::auth::{self, MaybeAuthUser};
use crate::envelope;
use crate::events::entity_from_topic;
use crate::graph::{self, EdgeStatus};
use crate::models::audit_log::{self, AuditContext};
use crate::models::{consumer_offsets, edges};
use crate::probe;

/// The distinct, parseable endpoint refs of a set of edge rows — the
/// candidates for lazy verify-on-read.
fn endpoints_of(rows: &[edges::Model]) -> Vec<EntityRef> {
    rows.iter()
        .flat_map(|m| {
            [
                m.from_ref.parse::<EntityRef>().ok(),
                m.to_ref.parse::<EntityRef>().ok(),
            ]
        })
        .flatten()
        .collect()
}

/// Build the governance-audit context for a request: the caller `sub`
/// (when a valid token was presented), the source address, and the
/// `User-Agent` header.
///
/// The address comes from `X-Forwarded-For`'s **first** hop when the
/// header is present, else the transport peer (`ConnectInfo`). That
/// order matters: this service normally sits behind a proxy, so the
/// peer address would otherwise be the proxy's every time — an audit
/// trail recording one address for every caller is worse than none,
/// because it looks like evidence. A forwarded header is client-supplied
/// and therefore only as trustworthy as the proxy that set it; the trail
/// records what was presented, which is the honest thing for it to hold.
fn audit_context<'a>(
    caller: &'a MaybeAuthUser,
    headers: &'a HeaderMap,
    peer: Option<&'a str>,
) -> AuditContext<'a> {
    let forwarded = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(str::trim)
        .filter(|v| !v.is_empty());
    AuditContext {
        actor: caller.claims().map(|c| c.sub.as_str()),
        user_ip: forwarded.or(peer),
        user_agent: headers.get(USER_AGENT).and_then(|v| v.to_str().ok()),
    }
}

/// Audit every governed (`subject_of`) edge **surfaced** in a read
/// response (design §10). Called on the post-concealment rows, so it
/// only fires when a governed edge was actually returned to the caller —
/// a concealed read audits nothing.
async fn audit_surfaced_edges(
    db: &DatabaseConnection,
    ctx: &AuditContext<'_>,
    action: &str,
    rows: &[edges::Model],
) -> ModelResult<()> {
    for row in rows {
        if EdgeKind::from_token(&row.kind).is_some_and(auth::is_governed) {
            audit_log::Model::record(db, ctx, action, &row.kind, &row.from_ref, &row.to_ref)
                .await?;
        }
    }
    Ok(())
}

/// The maximum `neighbors` traversal depth in v1 (spec §9.2 / §16).
pub const MAX_DEPTH: u32 = 2;

/// Query parameters for `GET /api/neighbors/{ref}`.
#[derive(Debug, Deserialize)]
struct NeighborsParams {
    /// Optional edge-kind filter (wire token).
    kind: Option<String>,
    /// Optional direction: `out` | `in` | `both` (default `both`).
    direction: Option<String>,
    /// Optional traversal depth (default 1; capped at [`MAX_DEPTH`]).
    depth: Option<u32>,
}

/// Query parameters for `GET /api/edges`.
#[derive(Debug, Deserialize)]
struct EdgesParams {
    /// Filter by `from_ref` URN.
    from: Option<String>,
    /// Filter by `to_ref` URN.
    to: Option<String>,
    /// Filter by edge-kind wire token.
    kind: Option<String>,
    /// Filter by status wire token (`unverified`|`verified`|`dangling`).
    status: Option<String>,
}

/// `data` payload for `GET /api/neighbors/{ref}`.
#[derive(Debug, Serialize)]
struct NeighborsData {
    /// The queried ref (URN).
    r#ref: String,
    /// The incident edges.
    edges: Vec<edges::Model>,
    /// The read-model freshness watermark.
    as_of: Option<DateTime<FixedOffset>>,
}

/// `data` payload for `GET /api/edges`.
#[derive(Debug, Serialize)]
struct EdgesData {
    /// The filtered edges.
    edges: Vec<edges::Model>,
    /// The read-model freshness watermark.
    as_of: Option<DateTime<FixedOffset>>,
}

/// One affiliation edge in a single-view response.
#[derive(Debug, Serialize)]
struct AffiliationDto {
    /// The `from` endpoint URN.
    from: String,
    /// The `to` endpoint URN.
    to: String,
    /// The edge-kind wire token.
    kind: String,
}

/// `data` payload for `GET /api/single-view/{ref}`.
#[derive(Debug, Serialize)]
struct SingleViewData {
    /// The unified identity set (URNs).
    identity_refs: Vec<EntityRef>,
    /// The affiliations incident to the identity set.
    affiliations: Vec<AffiliationDto>,
    /// The read-model freshness watermark.
    as_of: Option<DateTime<FixedOffset>>,
}

/// One topic's freshness in the freshness response.
#[derive(Debug, Serialize)]
struct TopicFreshness {
    /// The entity token behind the topic.
    entity: String,
    /// The `occurred_at` of the last consumed event.
    last_occurred_at: DateTime<FixedOffset>,
    /// Seconds between now and `last_occurred_at`.
    lag_seconds: i64,
}

/// `data` payload for `GET /api/health/freshness`.
#[derive(Debug, Serialize)]
struct FreshnessData {
    /// Per-topic freshness.
    topics: Vec<TopicFreshness>,
    /// The read-model freshness watermark.
    as_of: Option<DateTime<FixedOffset>>,
}

/// Bounded breadth-first collection of edges incident to `start`, up to
/// `depth` hops (spec §6 FR-7/13). Edges dedupe on `edge_id`.
async fn collect_neighbors(
    db: &DatabaseConnection,
    start: EntityRef,
    kind: Option<EdgeKind>,
    direction: edges::Direction,
    depth: u32,
) -> ModelResult<Vec<edges::Model>> {
    let mut seen: HashMap<Uuid, edges::Model> = HashMap::new();
    let mut visited: HashSet<String> = HashSet::new();
    let mut frontier: Vec<EntityRef> = vec![start];

    for _ in 0..depth {
        let mut next: Vec<EntityRef> = Vec::new();
        for node in std::mem::take(&mut frontier) {
            if !visited.insert(node.to_string()) {
                continue;
            }
            let rows = edges::Model::neighbors(db, &node, kind, direction).await?;
            for row in rows {
                if let Ok(fr) = row.from_ref.parse::<EntityRef>()
                    && !visited.contains(&fr.to_string())
                {
                    next.push(fr);
                }
                if let Ok(tr) = row.to_ref.parse::<EntityRef>()
                    && !visited.contains(&tr.to_string())
                {
                    next.push(tr);
                }
                seen.entry(row.edge_id).or_insert(row);
            }
        }
        frontier = next;
        if frontier.is_empty() {
            break;
        }
    }

    let mut out: Vec<edges::Model> = seen.into_values().collect();
    out.sort_by_key(|e| e.edge_id);
    Ok(out)
}

/// `GET /api/neighbors/{ref}` — edges incident to `{ref}` (spec §6 FR-13).
#[debug_handler]
async fn neighbors(
    Path(ref_urn): Path<String>,
    Query(params): Query<NeighborsParams>,
    caller: MaybeAuthUser,
    headers: HeaderMap,
    State(ctx): State<AppContext>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
) -> Result<Response> {
    let Ok(start) = ref_urn.parse::<EntityRef>() else {
        return envelope::bad_request(&format!("invalid entity ref: {ref_urn:?}"));
    };
    let kind = match params.kind.as_deref() {
        None => None,
        Some(k) => match EdgeKind::from_token(k) {
            Some(x) => Some(x),
            None => return envelope::bad_request(&format!("unknown edge kind: {k:?}")),
        },
    };
    let Some(direction) = edges::Direction::parse(params.direction.as_deref()) else {
        return envelope::bad_request("direction must be one of: out | in | both");
    };
    let depth = params.depth.unwrap_or(1);
    if depth > MAX_DEPTH {
        return envelope::bad_request(&format!("depth exceeds cap of {MAX_DEPTH}"));
    }

    let mut rows = collect_neighbors(&ctx.db, start, kind, direction, depth).await?;
    // Lazy verify-on-read (T-10): resolve any unknown endpoint presence and
    // recompute status, then re-read so the response reflects it.
    if probe::enabled()
        && probe::verify_unknown(&ctx.db, &probe::HttpPresenceProbe, &endpoints_of(&rows)).await?
            > 0
    {
        rows = collect_neighbors(&ctx.db, start, kind, direction, depth).await?;
    }
    // Governance: conceal high-sensitivity case↔person edges from callers
    // without case-read authorisation (design §10).
    let may = auth::may_see_governed(caller.claims());
    let rows = auth::conceal_governed(rows, may, |m| EdgeKind::from_token(&m.kind));
    let peer = peer.ip().to_string();
    let audit = audit_context(&caller, &headers, Some(peer.as_str()));
    audit_surfaced_edges(&ctx.db, &audit, "read_edge", &rows).await?;
    let as_of = consumer_offsets::Model::watermark(&ctx.db).await?;
    envelope::ok(NeighborsData {
        r#ref: start.to_string(),
        edges: rows,
        as_of,
    })
}

/// `GET /api/edges` — filtered edge list (spec §6 FR-14).
#[debug_handler]
async fn edges_list(
    Query(params): Query<EdgesParams>,
    caller: MaybeAuthUser,
    headers: HeaderMap,
    State(ctx): State<AppContext>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
) -> Result<Response> {
    let from = match parse_opt_ref(params.from.as_deref()) {
        Ok(v) => v,
        Err(msg) => return envelope::bad_request(&msg),
    };
    let to = match parse_opt_ref(params.to.as_deref()) {
        Ok(v) => v,
        Err(msg) => return envelope::bad_request(&msg),
    };
    let kind = match params.kind.as_deref() {
        None => None,
        Some(k) => match EdgeKind::from_token(k) {
            Some(x) => Some(x),
            None => return envelope::bad_request(&format!("unknown edge kind: {k:?}")),
        },
    };
    let status = match params.status.as_deref() {
        None => None,
        Some(s) => match EdgeStatus::from_token(s) {
            Some(x) => Some(x),
            None => return envelope::bad_request(&format!("unknown status: {s:?}")),
        },
    };

    let mut rows = edges::Model::filter(&ctx.db, from.as_ref(), to.as_ref(), kind, status).await?;
    // Lazy verify-on-read (T-10): resolve unknown endpoint presence, then
    // re-filter so the response reflects any status change.
    if probe::enabled()
        && probe::verify_unknown(&ctx.db, &probe::HttpPresenceProbe, &endpoints_of(&rows)).await?
            > 0
    {
        rows = edges::Model::filter(&ctx.db, from.as_ref(), to.as_ref(), kind, status).await?;
    }
    // Governance: a caller without case-read authorisation cannot see (or
    // learn of) subject_of edges — even a direct `?kind=subject_of` query
    // returns an empty list rather than revealing them (design §10).
    let may = auth::may_see_governed(caller.claims());
    let rows = auth::conceal_governed(rows, may, |m| EdgeKind::from_token(&m.kind));
    let peer = peer.ip().to_string();
    let audit = audit_context(&caller, &headers, Some(peer.as_str()));
    audit_surfaced_edges(&ctx.db, &audit, "read_edge", &rows).await?;
    let as_of = consumer_offsets::Model::watermark(&ctx.db).await?;
    envelope::ok(EdgesData { edges: rows, as_of })
}

/// `GET /api/single-view/{ref}` — golden-record walk (spec §6 FR-15).
#[debug_handler]
async fn single_view(
    Path(ref_urn): Path<String>,
    caller: MaybeAuthUser,
    headers: HeaderMap,
    State(ctx): State<AppContext>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
) -> Result<Response> {
    let Ok(start) = ref_urn.parse::<EntityRef>() else {
        return envelope::bad_request(&format!("invalid entity ref: {ref_urn:?}"));
    };
    let views = edges::Model::all_views(&ctx.db).await?;
    let view = graph::single_view(start, &views);
    // Governance: drop any subject_of affiliation the caller may not see,
    // so `single-view` never surfaces a concealed case↔person link (§10).
    let may = auth::may_see_governed(caller.claims());
    let affs = auth::conceal_governed(view.affiliations, may, |e| Some(e.kind));
    // Audit each governed affiliation actually surfaced (design §10).
    let peer = peer.ip().to_string();
    let audit = audit_context(&caller, &headers, Some(peer.as_str()));
    for e in &affs {
        if auth::is_governed(e.kind) {
            audit_log::Model::record(
                &ctx.db,
                &audit,
                "read_single_view",
                e.kind.as_str(),
                &e.from_ref.to_string(),
                &e.to_ref.to_string(),
            )
            .await?;
        }
    }
    let affiliations = affs
        .iter()
        .map(|e| AffiliationDto {
            from: e.from_ref.to_string(),
            to: e.to_ref.to_string(),
            kind: e.kind.as_str().to_string(),
        })
        .collect();
    let as_of = consumer_offsets::Model::watermark(&ctx.db).await?;
    envelope::ok(SingleViewData {
        identity_refs: view.identity_refs,
        affiliations,
        as_of,
    })
}

/// `GET /api/health/freshness` — per-topic consumer lag (spec §6 FR-16).
#[debug_handler]
async fn freshness(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = consumer_offsets::Model::freshness(&ctx.db).await?;
    let now = Utc::now().fixed_offset();
    let topics: Vec<TopicFreshness> = rows
        .iter()
        .map(|(topic, occurred)| TopicFreshness {
            entity: entity_from_topic(topic).to_string(),
            last_occurred_at: *occurred,
            lag_seconds: (now - *occurred).num_seconds(),
        })
        .collect();
    let as_of = rows.iter().map(|(_, t)| *t).max();
    envelope::ok(FreshnessData { topics, as_of })
}

/// Parse an optional `EntityRef` filter, returning a `400` message on a
/// malformed URN.
fn parse_opt_ref(raw: Option<&str>) -> std::result::Result<Option<EntityRef>, String> {
    match raw {
        None | Some("") => Ok(None),
        Some(s) => s
            .parse::<EntityRef>()
            .map(Some)
            .map_err(|e| format!("invalid entity ref {s:?}: {e}")),
    }
}

/// The read-only graph routes, mounted under `/api` (spec §9.1).
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/neighbors/{ref}", get(neighbors))
        .add("/edges", get(edges_list))
        .add("/single-view/{ref}", get(single_view))
        .add("/health/freshness", get(freshness))
}
