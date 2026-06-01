//! Axum handlers for server-rendered HTML pages and HTMX partials.
//!
//! These render Tera templates from `assets/views/` using [`WebState`].

use axum::{
    extract::{Path, Query, State},
    http::{StatusCode, Uri},
    response::{Html, IntoResponse, Response},
};
use serde::Deserialize;

use super::views::WebState;

/// `GET /` — the home page.
pub async fn home(State(state): State<WebState>) -> Response {
    render(&state, "home.html.tera", state.context()).into_response()
}

/// `GET /settings` — per-browser preferences page.
///
/// Scaffold: pure client-side scaffold; preferences live in
/// `localStorage["lily-settings"]` and are read/written by inline
/// Alpine code in the template. The page wires the headless
/// `switch-button` (role="switch", aria-checked toggle) for boolean
/// settings and `select`/`option` for enumerated ones.
pub async fn settings(State(state): State<WebState>) -> Response {
    render(&state, "settings.html.tera", state.context()).into_response()
}

/// `GET /tokens` — personal API tokens with reveal/hide/copy semantics.
///
/// Scaffold: seeds 4 tokens (one expired, one never-used, one admin-
/// scoped, one with multiple scopes). Production should read from
/// `TokenRepository::list_for_user(user_id)` and wire revoke to
/// `DELETE /api/tokens/{id}`. Token *creation* is intentionally
/// client-side in the scaffold so the secret never round-trips through
/// the server response logs; production should `POST /api/tokens` and
/// return the generated secret exactly once in the response body.
pub async fn tokens(State(state): State<WebState>) -> Response {
    let mut ctx = state.context();
    let tokens = serde_json::json!([
        {
            "id": "tok-001",
            "label": "ETL pipeline (production)",
            "scopes": ["read", "match"],
            "preview_full": "tok_a1b2c3d4e5f6g7h8i9j0klmnopqrstuv",
            "last4": "stuv",
            "created_at": "2026-02-14T09:18:00Z",
            "last_used_at": "2026-05-28T16:44:21Z",
            "expires_at": "2027-02-14T09:18:00Z"
        },
        {
            "id": "tok-002",
            "label": "Warehouse sync",
            "scopes": ["read", "write"],
            "preview_full": "tok_z9y8x7w6v5u4t3s2r1q0ponmlkjihgfe",
            "last4": "hgfe",
            "created_at": "2026-04-02T11:00:00Z",
            "last_used_at": "2026-05-26T03:12:09Z",
            "expires_at": "2026-08-02T11:00:00Z"
        },
        {
            "id": "tok-003",
            "label": "Local dev CLI",
            "scopes": ["read", "write", "match", "merge"],
            "preview_full": "tok_local_dev_token_for_jph_only_xx",
            "last4": "_xx",
            "created_at": "2026-05-20T20:15:33Z",
            "last_used_at": serde_json::Value::Null,
            "expires_at": "2026-06-20T20:15:33Z"
        },
        {
            "id": "tok-004",
            "label": "Admin break-glass",
            "scopes": ["read", "write", "match", "merge", "admin"],
            "preview_full": "tok_admin_break_glass_keep_in_vault",
            "last4": "ault",
            "created_at": "2025-12-01T08:00:00Z",
            "last_used_at": "2026-05-15T22:18:44Z",
            "expires_at": serde_json::Value::Null
        }
    ]);
    ctx.insert("tokens", &tokens);
    render(&state, "tokens.html.tera", ctx).into_response()
}

/// Fallback handler for unmatched routes — renders the 404 template.
///
/// Mounted via `Router::fallback_service(...)`. Receives the unmatched
/// `Uri` so the template can show the user what they asked for.
pub async fn not_found(State(state): State<WebState>, uri: Uri) -> Response {
    let mut ctx = state.context();
    ctx.insert("requested_path", &uri.path());
    let body = match state.render("not_found.html.tera", &ctx) {
        Ok(html) => html,
        Err(e) => format!(
            "<h1>404 Not found</h1><p>{}</p><p>Template error: <code>{}</code></p>",
            uri.path(),
            html_escape(&e.to_string())
        ),
    };
    (StatusCode::NOT_FOUND, Html(body)).into_response()
}

/// `GET /dev/500` — deliberately produce a 500 response.
///
/// Useful for verifying the error page renders correctly end-to-end.
/// Real 500s flow through the same `error_page()` helper from inside
/// `make_error_response()` once the web tier is plumbed into `AppState`.
pub async fn dev_error(State(state): State<WebState>) -> Response {
    error_page(
        &state,
        StatusCode::INTERNAL_SERVER_ERROR,
        Some("Synthetic error from /dev/500 for verifying the error page."),
        Some("Triggered by the dev_error handler. Wire this into a real fault path before relying on it in production."),
    )
}

/// Shared helper for rendering the styled error template at any status code.
///
/// Centralizing this here means the web router can call
/// `error_page(&state, StatusCode::INTERNAL_SERVER_ERROR, Some(msg), None)`
/// from any handler that wants a styled 5xx response.
pub fn error_page(
    state: &WebState,
    status: StatusCode,
    message: Option<&str>,
    detail: Option<&str>,
) -> Response {
    let mut ctx = state.context();
    ctx.insert("status_code", &status.as_u16());
    ctx.insert(
        "status_text",
        status.canonical_reason().unwrap_or("Server error"),
    );
    if let Some(m) = message {
        ctx.insert("message", m);
    }
    if let Some(d) = detail {
        ctx.insert("detail", d);
    }
    ctx.insert("request_id", "req-scaffold-0001");
    ctx.insert("at", "2026-05-25T07:14:08Z");
    let body = match state.render("error.html.tera", &ctx) {
        Ok(html) => html,
        Err(_) => format!(
            "<h1>{} {}</h1>",
            status.as_u16(),
            status.canonical_reason().unwrap_or("Server error")
        ),
    };
    (status, Html(body)).into_response()
}

#[derive(Debug, Deserialize)]
pub struct MetricsQuery {
    #[serde(default = "default_metrics_range")]
    pub range: String,
}

fn default_metrics_range() -> String {
    "24h".to_string()
}

/// `GET /palette` — full NHS-token color customizer.
///
/// Scaffold: lists every United Kingdom National Health Service England design-token grouped by category
/// (Blues / Neutrals / Greens / Highlights / Focus). For each token,
/// renders a swatch `color-picker` (radio group) plus a custom-hex
/// `color-input` and a Clear button. Overrides are stored in
/// `localStorage["lily-palette"]` as a flat `{ "nhs-blue": "#…", … }`
/// map and applied as CSS custom properties on `<html>` so any
/// component that reads `var(--nhs-*)` recolors instantly. The live
/// preview panel on the right shows buttons, badges, alerts, RAG
/// indicators, stars, and a sparkline so the effect is visible at a
/// glance.
pub async fn palette(State(state): State<WebState>) -> Response {
    let mut ctx = state.context();

    let groups = vec![
        token_group(
            "NHS Blues",
            &[
                ("nhs-dark-blue", "#003087"),
                ("nhs-blue", "#005eb8"),
                ("nhs-bright-blue", "#0072ce"),
                ("nhs-light-blue", "#41b6e6"),
                ("nhs-aqua-blue", "#00a9ce"),
            ],
        ),
        token_group(
            "NHS Neutrals",
            &[
                ("nhs-black", "#231f20"),
                ("nhs-dark-grey", "#425563"),
                ("nhs-mid-grey", "#768692"),
                ("nhs-pale-grey", "#e8edee"),
                ("nhs-white", "#ffffff"),
            ],
        ),
        token_group(
            "NHS Support Greens",
            &[
                ("nhs-dark-green", "#006747"),
                ("nhs-green", "#009639"),
                ("nhs-light-green", "#78be20"),
                ("nhs-aqua-green", "#00a499"),
            ],
        ),
        token_group(
            "NHS Highlights",
            &[
                ("nhs-purple", "#330072"),
                ("nhs-dark-pink", "#7c2855"),
                ("nhs-pink", "#ae2573"),
                ("nhs-dark-red", "#8a1538"),
                ("nhs-red", "#da291c"),
                ("nhs-orange", "#ed8b00"),
                ("nhs-warm-yellow", "#ffb81c"),
                ("nhs-yellow", "#fae100"),
            ],
        ),
        token_group(
            "Focus state",
            &[
                ("nhs-focus-color", "#ffeb3b"),
                ("nhs-focus-text-color", "#231f20"),
            ],
        ),
    ];

    let mut all_tokens: Vec<serde_json::Value> = Vec::new();
    for g in &groups {
        if let Some(arr) = g["tokens"].as_array() {
            for t in arr {
                all_tokens.push(t.clone());
            }
        }
    }

    // 12-color swatch palette used in the per-token radio group; covers
    // the NHS spectrum plus a few near-neutrals so most realistic
    // overrides have a clickable preset.
    let swatch_palette = vec![
        "#005eb8", "#003087", "#0072ce", "#41b6e6", "#00a9ce", "#009639",
        "#006747", "#330072", "#ae2573", "#da291c", "#ed8b00", "#ffb81c",
    ];

    ctx.insert("groups", &groups);
    ctx.insert("tokens", &all_tokens);
    ctx.insert("swatch_palette", &swatch_palette);
    render(&state, "palette.html.tera", ctx).into_response()
}

fn token_group(label: &str, tokens: &[(&str, &str)]) -> serde_json::Value {
    let tokens: Vec<serde_json::Value> = tokens
        .iter()
        .map(|(name, default)| {
            serde_json::json!({
                "name": name,
                "default": default,
            })
        })
        .collect();
    serde_json::json!({
        "label": label,
        "tokens": tokens,
    })
}

/// `GET /notifications` — persistent notification center.
///
/// Scaffold: seeds five notifications spanning all four severities
/// (success / info / warning / error). Alpine state in the template
/// reads `localStorage["lily-notifications"]` on `x-init` so the user
/// can mark-read or dismiss between sessions; the seeded payload is
/// the initial state on first visit. Production should swap this for
/// an SSE / WebSocket stream from the event publisher (`PersonCreated`,
/// `PersonMerged`, `PersonDeleted`, etc.).
pub async fn notifications(State(state): State<WebState>) -> Response {
    let mut ctx = state.context();
    let singular = state.app.entity_singular;
    let plural = state.app.entity_plural;
    let notifications = vec![
        notification_entry(
            "n-001",
            "info",
            true,
            "Bulk import complete",
            format!(
                "Imported 247 {plural} from bulk-2026-05-27.csv. 3 duplicates auto-merged; \
                 2 queued for review."
            ),
            format!("/{plural}/review-queue"),
            "2026-05-28T08:14:23Z",
        ),
        notification_entry(
            "n-002",
            "warning",
            true,
            "Fluvio producer lag rising",
            "Event-stream backlog reached 12 messages; auto-recovered after 47 s.".to_string(),
            "/health".to_string(),
            "2026-05-27T06:42:11Z",
        ),
        notification_entry(
            "n-003",
            "success",
            false,
            "Merge completed",
            format!(
                "Confirmed-duplicate merge of {singular} aaaa-7 → aaaa-1 \
                 (score 0.99, tax-ID exact match)."
            ),
            format!("/{plural}/00000000-0000-0000-0000-000000000001/audit"),
            "2026-05-26T15:31:48Z",
        ),
        notification_entry(
            "n-004",
            "error",
            true,
            "Validation failure (HTTP 422)",
            format!(
                "Create-{singular} request rejected: birth_date in the future (2030-01-15)."
            ),
            "/audit?action=Created".to_string(),
            "2026-05-26T11:08:09Z",
        ),
        notification_entry(
            "n-005",
            "info",
            false,
            "Nightly normalization run",
            "82 addresses re-capitalized; 14 phone numbers re-normalized to E.164.".to_string(),
            "/audit".to_string(),
            "2026-05-26T02:00:00Z",
        ),
    ];

    let filters = vec![
        serde_json::json!({ "value": "all",     "label": "All" }),
        serde_json::json!({ "value": "unread",  "label": "Unread" }),
        serde_json::json!({ "value": "success", "label": "Success" }),
        serde_json::json!({ "value": "info",    "label": "Info" }),
        serde_json::json!({ "value": "warning", "label": "Warning" }),
        serde_json::json!({ "value": "error",   "label": "Error" }),
    ];

    ctx.insert("notifications", &notifications);
    ctx.insert("filters", &filters);
    render(&state, "notifications.html.tera", ctx).into_response()
}

#[allow(clippy::too_many_arguments)]
fn notification_entry(
    id: &str,
    severity: &str,
    unread: bool,
    title: &str,
    body: String,
    href: String,
    at: &str,
) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "severity": severity,
        "unread": unread,
        "title": title,
        "body": body,
        "href": href,
        "at": at,
    })
}

/// `GET /docs` — API documentation landing.
///
/// Scaffold: groups the existing REST + FHIR endpoint inventory into
/// collapsible accordion groups (Health / CRUD / Search / Matching /
/// Privacy / Audit / FHIR). The interactive try-it experience is
/// delegated to the OpenAPI/Swagger UI at `/swagger-ui`; this page is
/// a navigable reference. Method `badge`s are colored by verb
/// (GET=info, POST=success, PUT=warning, DELETE=error).
pub async fn docs(State(state): State<WebState>) -> Response {
    let mut ctx = state.context();
    let plural = state.app.entity_plural;
    let groups = vec![
        endpoint_group(
            "Health",
            "info",
            vec![("GET", "/api/health".to_string(), "Liveness + dependency status")],
        ),
        endpoint_group(
            "CRUD",
            "success",
            vec![
                ("POST", format!("/api/{plural}"), "Create record (with real-time duplicate detection)"),
                ("GET", format!("/api/{plural}/:id"), "Get record by ID"),
                ("PUT", format!("/api/{plural}/:id"), "Update record"),
                ("DELETE", format!("/api/{plural}/:id"), "Soft delete record"),
            ],
        ),
        endpoint_group(
            "Search",
            "info",
            vec![(
                "GET",
                format!("/api/{plural}/search"),
                "Full-text search (?q=, ?fuzzy=, ?phonetic=, ?mask_sensitive=, ?limit=, ?offset=)",
            )],
        ),
        endpoint_group(
            "Matching & deduplication",
            "warning",
            vec![
                ("POST", format!("/api/{plural}/match"), "Match record against existing"),
                ("POST", format!("/api/{plural}/check-duplicates"), "Check for duplicates without creating"),
                ("POST", format!("/api/{plural}/merge"), "Merge two records"),
                ("POST", format!("/api/{plural}/deduplicate"), "Batch deduplication scan"),
            ],
        ),
        endpoint_group(
            "Privacy",
            "warning",
            vec![
                ("GET", format!("/api/{plural}/:id/export"), "GDPR Article 15 data export"),
                ("GET", format!("/api/{plural}/:id/masked"), "Masked record view"),
            ],
        ),
        endpoint_group(
            "Audit",
            "info",
            vec![
                ("GET", format!("/api/{plural}/:id/audit"), "Per-record audit history"),
                ("GET", "/api/audit/recent".to_string(), "Recent system-wide audit activity"),
                ("GET", "/api/audit/user".to_string(), "Per-user audit log"),
            ],
        ),
        endpoint_group(
            "FHIR R5",
            "success",
            vec![
                ("GET", "/fhir/Person/:id".to_string(), "Get FHIR Person"),
                ("POST", "/fhir/Person".to_string(), "Create FHIR Person"),
                ("PUT", "/fhir/Person/:id".to_string(), "Update FHIR Person"),
                ("DELETE", "/fhir/Person/:id".to_string(), "Delete FHIR Person"),
                ("GET", "/fhir/Person".to_string(), "Search FHIR Persons (?name=, ?family=, ?given=, ?identifier=, ?birthdate=, ?gender=, ?_count=)"),
            ],
        ),
    ];
    ctx.insert("groups", &groups);
    render(&state, "docs.html.tera", ctx).into_response()
}

fn endpoint_group(
    label: &str,
    badge_type: &str,
    eps: Vec<(&str, String, &str)>,
) -> serde_json::Value {
    let endpoints: Vec<serde_json::Value> = eps
        .iter()
        .map(|(method, path, purpose)| {
            serde_json::json!({
                "method": method,
                "path": path,
                "purpose": purpose,
            })
        })
        .collect();
    serde_json::json!({
        "label": label,
        "badge_type": badge_type,
        "count": endpoints.len(),
        "endpoints": endpoints,
    })
}

/// `GET /tour` — guided onboarding tour.
///
/// Scaffold: seeds 8 stops linking to the main pages (index, record
/// detail, edit, review queue, system audit, health, settings, and
/// the keyboard-shortcut overlay). Steps are interactive — clicking
/// "Mark done" persists a per-step key in `localStorage["lily-tour-steps"]`
/// and the `tour-list-item` shows a `success` badge once complete.
/// The `<progress>` bar in the header tracks completion ratio.
pub async fn tour(State(state): State<WebState>) -> Response {
    let mut ctx = state.context();
    let plural = state.app.entity_plural;
    let singular = state.app.entity_singular;

    let steps = vec![
        tour_step(
            "index",
            "Browse the index",
            None,
            format!(
                "The <strong>/{plural}</strong> page is a paginated table of all \
                 records with status badges and per-page bulk-select. Use the \
                 checkboxes for bulk delete or bulk merge."
            ),
            format!("/{plural}"),
            Some("Open index"),
        ),
        tour_step(
            "detail",
            "Open a record detail page",
            None,
            format!(
                "Each {singular} has a detail page with breadcrumb, summary list, \
                 and an action-bar exposing Edit, Audit log, Export, FHIR, \
                 Consents, Links, and Delete."
            ),
            format!("/{plural}/00000000-0000-0000-0000-000000000001"),
            Some("Open sample record"),
        ),
        tour_step(
            "edit",
            "Edit a record",
            None,
            format!(
                "The edit page wraps the {singular} in headless form components \
                 plus four tag-input editors: identifiers, addresses, contacts, \
                 and identity documents. Every input has hover-card help and \
                 inline validation."
            ),
            format!("/{plural}/00000000-0000-0000-0000-000000000001/edit"),
            Some("Open edit form"),
        ),
        tour_step(
            "review-queue",
            "Triage potential duplicates",
            None,
            format!(
                "The review queue shows pairs flagged by deduplication. Each row \
                 has a side-by-side compare link, an alert-dialog merge \
                 confirmation, and a one-click reject."
            ),
            format!("/{plural}/review-queue"),
            Some("Open review queue"),
        ),
        tour_step(
            "audit",
            "Trace activity",
            Some(vec!["g".to_string(), "a".to_string()]),
            "The system audit log shows every Created / Updated / Deleted / Merged / \
             Linked event across the crate, filterable by user and action."
                .to_string(),
            "/audit".to_string(),
            Some("Open system audit"),
        ),
        tour_step(
            "health",
            "Check service health",
            Some(vec!["g".to_string(), "e".to_string()]),
            "The health page renders RAG indicators per subsystem, a resource \
             utilization grid, and a recent-incident timeline. The metrics page \
             adds sparklines per endpoint."
                .to_string(),
            "/health".to_string(),
            Some("Open health"),
        ),
        tour_step(
            "settings",
            "Customize your view",
            None,
            "Per-browser settings: theme (NHS / dark / high-contrast), default \
             page size, mask-sensitive-by-default, and other toggles. Stored \
             in <code>localStorage</code>."
                .to_string(),
            "/settings".to_string(),
            Some("Open settings"),
        ),
        tour_step(
            "shortcuts",
            "Learn the keyboard shortcuts",
            Some(vec!["?".to_string()]),
            "Press <kbd class=\"kbd\">?</kbd> to open the shortcut overlay. \
             <kbd class=\"kbd\">/</kbd> focuses the search box on any page; \
             <kbd class=\"kbd\">g</kbd> + a navigation key jumps between \
             sections."
                .to_string(),
            String::new(),
            None,
        ),
    ];

    ctx.insert("steps", &steps);
    render(&state, "tour.html.tera", ctx).into_response()
}

fn tour_step(
    key: &str,
    title: &str,
    shortcut: Option<Vec<String>>,
    body: String,
    href: String,
    cta: Option<&str>,
) -> serde_json::Value {
    serde_json::json!({
        "key": key,
        "title": title,
        "shortcut": shortcut,
        "body": body,
        "href": href,
        "cta": cta,
    })
}

/// `GET /metrics` — performance dashboard.
///
/// Scaffold: seeds 4 system-overview cards (req/sec, p95 latency,
/// error rate, DB pool) with synthesized polyline data, an
/// endpoint-latency `data-table` with per-endpoint sparklines + RAG
/// health + p50/p95/p99 / req-sec, and a `summary-list` of 2xx/4xx/5xx
/// error-class shares. Each sparkline is inlined SVG so no JS chart
/// library is required. Wire through to OTLP metrics (latency
/// histograms, request counters, error counters) once the web tier
/// is plumbed into `AppState`.
pub async fn metrics(
    State(state): State<WebState>,
    Query(MetricsQuery { range }): Query<MetricsQuery>,
) -> Response {
    let mut ctx = state.context();
    let range_label = match range.as_str() {
        "1h" => "Last hour",
        "6h" => "Last 6 hours",
        "24h" => "Last 24 hours",
        "7d" => "Last 7 days",
        _ => "Last 24 hours",
    };

    let system_metrics = vec![
        system_metric(
            "Requests / sec",
            "req/s",
            "247",
            "180",
            "312",
            -8,
            &[260, 252, 244, 238, 245, 256, 268, 264, 251, 242, 247],
        ),
        system_metric(
            "Latency p95",
            "ms",
            "84",
            "62",
            "187",
            4,
            &[74, 78, 80, 82, 79, 78, 81, 84, 87, 83, 84],
        ),
        system_metric(
            "Error rate",
            "%",
            "0.42",
            "0.10",
            "1.2",
            -12,
            &[55, 48, 52, 60, 64, 58, 52, 49, 45, 43, 42],
        ),
        system_metric(
            "DB pool",
            "/10",
            "8",
            "3",
            "10",
            0,
            &[6, 7, 7, 8, 9, 8, 7, 8, 8, 8, 8],
        ),
    ];

    let endpoints = vec![
        endpoint_row("GET", "/api/health", "green", 3, 7, 18, 28.0, &[3, 4, 3, 5, 4, 3, 4, 3, 4]),
        endpoint_row(
            "GET",
            &format!("/api/{}/:id", state.app.entity_plural),
            "green",
            5, 14, 32, 84.0,
            &[5, 6, 7, 8, 7, 6, 7, 8, 7],
        ),
        endpoint_row(
            "POST",
            &format!("/api/{}/search", state.app.entity_plural),
            "amber",
            42, 187, 412, 32.5,
            &[60, 80, 100, 140, 160, 180, 175, 170, 187],
        ),
        endpoint_row(
            "POST",
            &format!("/api/{}/match", state.app.entity_plural),
            "amber",
            120, 380, 612, 5.3,
            &[200, 240, 280, 320, 360, 380, 365, 372, 380],
        ),
        endpoint_row(
            "POST",
            &format!("/api/{}/merge", state.app.entity_plural),
            "green",
            18, 41, 88, 0.4,
            &[20, 22, 25, 24, 28, 30, 35, 38, 41],
        ),
    ];

    let error_classes = vec![
        serde_json::json!({
            "class": "2xx", "description": "Success",
            "count": 412_984u32, "pct": "99.58"
        }),
        serde_json::json!({
            "class": "4xx", "description": "Client errors (404, 409, 422)",
            "count": 1_481u32, "pct": "0.36"
        }),
        serde_json::json!({
            "class": "5xx", "description": "Server errors (500, 503)",
            "count": 247u32, "pct": "0.06"
        }),
    ];

    ctx.insert("range", &range);
    ctx.insert("range_label", &range_label);
    ctx.insert("system_metrics", &system_metrics);
    ctx.insert("endpoints", &endpoints);
    ctx.insert("error_classes", &error_classes);
    render(&state, "metrics.html.tera", ctx).into_response()
}

/// Build a system-overview metric card with an inline-SVG polyline.
fn system_metric(
    label: &str,
    unit: &str,
    current: &str,
    min: &str,
    max: &str,
    delta_pct: i32,
    samples: &[u32],
) -> serde_json::Value {
    serde_json::json!({
        "label": label,
        "unit": unit,
        "current": current,
        "min": min,
        "max": max,
        "delta_pct": delta_pct,
        "points": sparkline_points(samples, 100.0, 30.0),
    })
}

/// Build an endpoint-row with an inline-SVG polyline.
fn endpoint_row(
    method: &str,
    path: &str,
    status: &str,
    p50_ms: u32,
    p95_ms: u32,
    p99_ms: u32,
    rps: f64,
    samples: &[u32],
) -> serde_json::Value {
    serde_json::json!({
        "method": method,
        "path": path,
        "status": status,
        "p50_ms": p50_ms,
        "p95_ms": p95_ms,
        "p99_ms": p99_ms,
        "rps": rps,
        "points": sparkline_points(samples, 100.0, 20.0),
    })
}

/// Project a series of samples onto a `width × height` SVG viewBox
/// (Y is inverted because SVG origin is top-left). Returns a
/// space-separated `x,y` list suitable for `<polyline points="…">`.
fn sparkline_points(samples: &[u32], width: f64, height: f64) -> String {
    if samples.is_empty() {
        return String::new();
    }
    let (mut lo, mut hi) = (samples[0], samples[0]);
    for &v in samples {
        if v < lo {
            lo = v;
        }
        if v > hi {
            hi = v;
        }
    }
    let span = (hi - lo).max(1) as f64;
    let dx = if samples.len() > 1 {
        width / (samples.len() - 1) as f64
    } else {
        0.0
    };
    samples
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let x = i as f64 * dx;
            let y = height - ((v as f64 - lo as f64) / span) * height;
            format!("{:.1},{:.1}", x, y)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// `GET /health` — human-readable system health page.
///
/// Scaffold: seeds overall + per-subsystem RAG status, latency, resource
/// utilization, and a short incident timeline so the headless
/// `red-amber-green-view` (with `data-status`-keyed CSS dots), `meter`
/// (latency), `progress` (utilization), `data-table`, `timeline-list`,
/// quality-keyed `badge`s, and `diff` two-column grid are visible
/// end-to-end. The "JSON endpoint" link points at the existing
/// `GET /api/health` REST endpoint; the "Refresh" button HTMX-reloads
/// this page.
pub async fn health(State(state): State<WebState>) -> Response {
    let mut ctx = state.context();

    let service = serde_json::json!({
        "name": state.app.app_display,
        "version": env!("CARGO_PKG_VERSION"),
        "uptime_human": "4d 7h 21m",
        "checked_at": "2026-05-25T07:14:08Z",
    });

    let overall = serde_json::json!({
        "status": "amber",
    });

    let subsystems = vec![
        serde_json::json!({
            "id": "postgresql",
            "label": "PostgreSQL",
            "status": "green",
            "latency_ms": 7,
            "detail": "Pool 8/10, replicas in sync",
        }),
        serde_json::json!({
            "id": "tantivy",
            "label": "Tantivy search index",
            "status": "green",
            "latency_ms": 12,
            "detail": "Index size 1.2 GB, last refresh 32 s ago",
        }),
        serde_json::json!({
            "id": "fluvio",
            "label": "Fluvio event stream",
            "status": "amber",
            "latency_ms": 180,
            "detail": "Producer lag rising (4 messages queued)",
        }),
        serde_json::json!({
            "id": "otlp",
            "label": "OpenTelemetry collector",
            "status": "green",
            "latency_ms": 18,
            "detail": "Exporting traces + metrics to OTLP gRPC",
        }),
        serde_json::json!({
            "id": "audit",
            "label": "Audit log writer",
            "status": "green",
            "latency_ms": 5,
            "detail": "Batched writes; backlog 0",
        }),
    ];

    let resources = vec![
        serde_json::json!({
            "label": "DB connections",
            "value": 8,
            "max": 10,
            "unit": "connections",
            "status": "amber",
        }),
        serde_json::json!({
            "label": "Memory",
            "value": 412,
            "max": 1024,
            "unit": "MB",
            "status": "green",
        }),
        serde_json::json!({
            "label": "Search index disk",
            "value": 1228,
            "max": 5120,
            "unit": "MB",
            "status": "green",
        }),
        serde_json::json!({
            "label": "Request rate",
            "value": 247,
            "max": 1000,
            "unit": "req/sec",
            "status": "green",
        }),
    ];

    let recent_incidents = vec![
        serde_json::json!({
            "severity": "degraded",
            "at": "2026-05-25T06:42:11Z",
            "summary": "Fluvio producer lag spiked to 12 messages; auto-recovered after 47 s.",
        }),
        serde_json::json!({
            "severity": "outage",
            "at": "2026-05-22T14:08:33Z",
            "summary": "Tantivy index rebuild blocked search for 3 m 12 s during a schema migration.",
        }),
    ];

    ctx.insert("service", &service);
    ctx.insert("overall", &overall);
    ctx.insert("subsystems", &subsystems);
    ctx.insert("resources", &resources);
    ctx.insert("recent_incidents", &recent_incidents);
    render(&state, "health.html.tera", ctx).into_response()
}

#[derive(Debug, Deserialize)]
pub struct IndexQuery {
    #[serde(default = "default_page")]
    pub page: u32,
}

fn default_page() -> u32 {
    1
}

/// `GET /{entity_plural}` — list page.
///
/// Scaffold: seeds three placeholder records (one inactive) so the
/// `data-table`, `badge` (success/error variants), and `pagination-nav`
/// headless components are visible end-to-end. Pagination metadata is
/// synthesized for two pages so the `pagination-nav` renders with
/// previous/next links. Wire through to the real repository once the
/// web tier is plumbed into `AppState`.
pub async fn index(
    State(state): State<WebState>,
    Query(IndexQuery { page }): Query<IndexQuery>,
) -> Response {
    let mut ctx = state.context();
    let records = vec![
        serde_json::json!({
            "id": "00000000-0000-0000-0000-000000000001",
            "label": format!("Sample {} 1", state.app.entity_singular),
            "subtitle": "Placeholder record",
            "active": true,
        }),
        serde_json::json!({
            "id": "00000000-0000-0000-0000-000000000002",
            "label": format!("Sample {} 2", state.app.entity_singular),
            "subtitle": "Placeholder record",
            "active": true,
        }),
        serde_json::json!({
            "id": "00000000-0000-0000-0000-000000000003",
            "label": format!("Sample {} 3", state.app.entity_singular),
            "subtitle": "Soft-deleted record",
            "active": false,
        }),
    ];
    let total_pages: u32 = 2;
    let pagination = serde_json::json!({
        "page": page,
        "total_pages": total_pages,
        "has_prev": page > 1,
        "has_next": page < total_pages,
        "pages": (1..=total_pages).collect::<Vec<u32>>(),
    });
    ctx.insert("records", &records);
    ctx.insert("pagination", &pagination);
    render(&state, "index.html.tera", ctx).into_response()
}

/// `GET /{entity_plural}/import` — bulk-import wizard.
///
/// Scaffold: renders the wizard chrome (breadcrumb, tab-bar step
/// indicator, file-upload drop zone, preview table, mapping form,
/// progress bar, result summary). All state lives in Alpine
/// (`step`, `fileName`, `rows`, `mapping`, `progressPercent`,
/// `result`); the CSV is parsed client-side and the "import" step
/// simulates progress with a 200 ms tick. Real implementation should
/// stream the CSV to `POST /api/{plural}/import` and consume an
/// `HX-Trigger: importProgress` stream for live progress updates.
pub async fn import(State(state): State<WebState>) -> Response {
    render(&state, "import.html.tera", state.context()).into_response()
}

/// `GET /{entity_plural}/{id}` — detail page.
///
/// Scaffold: seeds a placeholder record so the headless `summary-list`,
/// `breadcrumb-nav`, `action-bar`, and `alert-dialog` components are
/// verifiable end-to-end. Wire this through to the real repository
/// (e.g. `crate::db::PersonRepository::get_by_id`) once the web tier
/// is plumbed into `AppState`.
///
/// For healthcare-aware crates (patient, person), this also seeds a
/// `healthcare` context that drives the `medical-banner` overlay
/// (alerts + advice + NHS / SSN identifier views + `care-card`s).
pub async fn show(State(state): State<WebState>, Path(id): Path<String>) -> Response {
    let mut ctx = state.context();
    ctx.insert("record", &seed_record(&state, &id));
    if let Some(hc) = seed_healthcare(&state) {
        ctx.insert("healthcare", &hc);
    }
    render(&state, "show.html.tera", ctx).into_response()
}

/// `GET /{entity_plural}/{id}/edit` — edit page.
///
/// Scaffold: seeds the same placeholder record as [`show`] so the
/// headless `form`, `field`, `label`, `hint`, `text-input`, `select`,
/// `option`, `error-summary`, and `error-message` components are
/// verifiable end-to-end. The real implementation should fetch from
/// the repository and pre-populate `field_errors` / `errors` from the
/// `422` validation response when the PUT round-trip fails.
///
/// For healthcare-aware crates the same `healthcare` context drives a
/// `fieldset` of `united-kingdom-national-health-service-number-input`
/// + `united-states-social-security-number-input` inputs.
pub async fn edit(State(state): State<WebState>, Path(id): Path<String>) -> Response {
    let mut ctx = state.context();
    ctx.insert("record", &seed_record(&state, &id));
    ctx.insert("errors", &Vec::<serde_json::Value>::new());
    ctx.insert("field_errors", &serde_json::Map::<String, serde_json::Value>::new());
    let identifiers = vec![
        serde_json::json!({ "type": "MRN", "value": "M-001" }),
        serde_json::json!({ "type": "SSN", "value": "***-**-6789" }),
    ];
    ctx.insert("identifiers", &identifiers);
    let addresses = vec![serde_json::json!({
        "line1": "10 Downing St",
        "line2": "",
        "city": "London",
        "state": "Greater London",
        "postal_code": "SW1A 2AA",
        "country": "GB",
    })];
    ctx.insert("addresses", &addresses);
    let telecoms = vec![
        serde_json::json!({ "system": "phone", "value": "+44 20 7946 0958" }),
        serde_json::json!({
            "system": "email",
            "value": format!("sample.{}@example.test", state.app.entity_singular)
        }),
    ];
    ctx.insert("telecoms", &telecoms);
    let documents = vec![
        serde_json::json!({
            "type": "PASSPORT",
            "number": "X12345678",
            "issuing_country": "US",
            "issuing_authority": "US Dept of State",
            "issue_date": "2022-04-12",
            "expiry_date": "2032-04-11",
        }),
        serde_json::json!({
            "type": "DRIVERS_LICENSE",
            "number": "DL-099-887-665",
            "issuing_country": "GB",
            "issuing_authority": "DVLA",
            "issue_date": "2020-08-01",
            "expiry_date": "2026-07-31",
        }),
    ];
    ctx.insert("documents", &documents);
    if let Some(hc) = seed_healthcare(&state) {
        ctx.insert("healthcare", &hc);
        ctx.insert("emergency_contacts", &seed_emergency_contacts());
    } else {
        ctx.insert(
            "emergency_contacts",
            &Vec::<serde_json::Value>::new(),
        );
    }
    render(&state, "edit.html.tera", ctx).into_response()
}

/// Seed emergency-contact list for healthcare crates.
///
/// Returned only when `seed_healthcare` would also be non-`None`. The
/// edit page uses this to populate the `<fieldset class="fieldset"
/// aria-label="Emergency contacts">` block. In production this should
/// come from `crate::models::Person::emergency_contacts` (or the
/// patient equivalent).
fn seed_emergency_contacts() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({
            "name": "Jane Smith",
            "relationship": "spouse",
            "phone": "+44 20 7946 0199",
            "email": "jane.smith@example.test",
            "is_primary": true,
        }),
        serde_json::json!({
            "name": "Robert Smith",
            "relationship": "parent",
            "phone": "+44 20 7946 0233",
            "email": "",
            "is_primary": false,
        }),
    ]
}

#[derive(Debug, Deserialize)]
pub struct AuditQuery {
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default = "default_page")]
    pub page: u32,
}

/// `GET /{entity_plural}/{id}/export` — GDPR Article 15 data export view.
///
/// Scaffold: seeds the same placeholder record as [`show`] plus a
/// pretty-printed JSON payload, a compact one-line clipboard string,
/// the generated-at timestamp, byte-count, and the active consents
/// list (rendered as quality-keyed `badge`s). The page wires together
/// the headless `clipboard-copy-button`, `code-block`, `information-callout`,
/// and `summary-list` components.
///
/// The "Download JSON" link points at the existing REST endpoint
/// `GET /api/{plural}/{id}/export`, so the page is a thin UI wrapper
/// over a server response that is already authoritative.

/// `GET /{entity_plural}/{id}/consents` — consent management page.
///
/// Scaffold: seeds five consent records spanning every status
/// (Active / Revoked / Expired) so the headless `data-table`,
/// status-keyed `badge`s, per-row revoke `alert-dialog`, "grant new
/// consent" `drawer` with `form`/`field`/`select`/`text-input`, and
/// `information-callout` are visible end-to-end. Wire through to the
/// real `Consent` repository once the web tier is plumbed into
/// `AppState`.
pub async fn consents(State(state): State<WebState>, Path(id): Path<String>) -> Response {
    let mut ctx = state.context();
    let consents = vec![
        serde_json::json!({
            "id": "c-001",
            "consent_type": "DataProcessing",
            "status": "Active",
            "granted_date": "2025-09-12",
            "expiry_date": null,
            "method": "written",
            "purpose": "Routine clinical care and administration",
        }),
        serde_json::json!({
            "id": "c-002",
            "consent_type": "DataSharing",
            "status": "Active",
            "granted_date": "2025-09-12",
            "expiry_date": "2027-09-12",
            "method": "electronic",
            "purpose": "Share summary record with referring GP practice",
        }),
        serde_json::json!({
            "id": "c-003",
            "consent_type": "Research",
            "status": "Revoked",
            "granted_date": "2024-03-04",
            "expiry_date": null,
            "method": "written",
            "purpose": "Cardiology cohort study 2024-CC-08; revoked on patient request",
        }),
        serde_json::json!({
            "id": "c-004",
            "consent_type": "Marketing",
            "status": "Expired",
            "granted_date": "2023-01-15",
            "expiry_date": "2024-01-15",
            "method": "electronic",
            "purpose": "Newsletter and event invitations",
        }),
        serde_json::json!({
            "id": "c-005",
            "consent_type": "EmergencyAccess",
            "status": "Active",
            "granted_date": "2025-09-12",
            "expiry_date": null,
            "method": "implied",
            "purpose": "Break-glass access during emergency presentations",
        }),
    ];
    let stats = serde_json::json!({
        "active": 3,
        "revoked": 1,
        "expired": 1,
    });
    ctx.insert("record", &seed_record(&state, &id));
    ctx.insert("consents", &consents);
    ctx.insert("stats", &stats);
    render(&state, "consents.html.tera", ctx).into_response()
}

/// `GET /{entity_plural}/{id}/quality` — data-quality score view.
///
/// Scaffold: seeds an overall completeness percentage, a per-component
/// breakdown (Name / Birth date / Identifiers / Address / Contacts /
/// Documents / Consent) with star ratings, and a prioritized list of
/// improvement suggestions. Wire through to a real `data_quality`
/// scorer once the web tier is plumbed into `AppState`.
pub async fn quality(State(state): State<WebState>, Path(id): Path<String>) -> Response {
    let mut ctx = state.context();
    let plural = state.app.entity_plural;
    let singular = state.app.entity_singular;

    let overall_pct: u32 = 76;
    let overall = serde_json::json!({
        "score_pct": overall_pct,
        "stars": stars_for(overall_pct),
        "face": face_label(overall_pct),
        "face_emoji": face_emoji(overall_pct),
    });

    let components = vec![
        quality_component("Name", 100, "Family + given names present; all required fields populated"),
        quality_component("Birth date", 100, "ISO-8601 birth date present and within valid range"),
        quality_component("Identifiers", 80, "MRN + SSN present; NPI not yet on file"),
        quality_component("Address", 95, "Primary address present and validated; postal code resolves"),
        quality_component("Contacts", 60, "Phone present; email missing for primary contact"),
        quality_component("Documents", 50, "Passport on file; drivers licence expires in <12 months"),
        quality_component("Consent", 50, "DataProcessing consent active; Research consent revoked"),
    ];

    let suggestions = vec![
        serde_json::json!({
            "severity": "high",
            "title": "No email contact on file",
            "body": format!(
                "Add a primary email to this {singular}'s telecom list. Email enables \
                 GDPR-Article-15 export delivery, OTP login, and password resets."
            ),
            "action_label": "Add an email contact",
            "action_href": format!("/{plural}/{id}/edit"),
        }),
        serde_json::json!({
            "severity": "medium",
            "title": "Driver's licence expires within 12 months",
            "body": "Set a renewal reminder, or update the record after the licence is reissued.",
            "action_label": "Update document expiry",
            "action_href": format!("/{plural}/{id}/edit"),
        }),
        serde_json::json!({
            "severity": "medium",
            "title": "Research consent has been revoked",
            "body": format!(
                "If research participation is required, grant a fresh consent. Otherwise, \
                 leave revoked — this {singular}'s data must not appear in research cohorts."
            ),
            "action_label": "Open consents",
            "action_href": format!("/{plural}/{id}/consents"),
        }),
        serde_json::json!({
            "severity": "low",
            "title": "No NPI (National Provider Identifier) on file",
            "body": format!(
                "Most {plural} do not need an NPI. Skip if this {singular} is not a \
                 healthcare provider."
            ),
            "action_label": null,
            "action_href": null,
        }),
    ];

    ctx.insert("record", &seed_record(&state, &id));
    ctx.insert("overall", &overall);
    ctx.insert("components", &components);
    ctx.insert("suggestions", &suggestions);
    render(&state, "quality.html.tera", ctx).into_response()
}

/// Star count derived from a 0–100 score (5★ ≥ 90, 4★ ≥ 75, 3★ ≥ 55,
/// 2★ ≥ 35, 1★ otherwise).
fn stars_for(pct: u32) -> u32 {
    match pct {
        90..=u32::MAX => 5,
        75..=89 => 4,
        55..=74 => 3,
        35..=54 => 2,
        _ => 1,
    }
}

fn face_emoji(pct: u32) -> &'static str {
    match pct {
        90..=u32::MAX => "😀",
        75..=89 => "🙂",
        55..=74 => "😐",
        35..=54 => "☹️",
        _ => "😠",
    }
}

fn face_label(pct: u32) -> &'static str {
    match pct {
        90..=u32::MAX => "very satisfied",
        75..=89 => "satisfied",
        55..=74 => "neutral",
        35..=54 => "dissatisfied",
        _ => "very dissatisfied",
    }
}

fn quality_component(label: &str, score_pct: u32, note: &str) -> serde_json::Value {
    serde_json::json!({
        "label": label,
        "score_pct": score_pct,
        "stars": stars_for(score_pct),
        "note": note,
    })
}

/// `GET /{entity_plural}/map` — schematic world-grid view of records
/// with geo-coordinates.
///
/// Scaffold: projects 6 well-known lat/lon pairs onto a 1000×500 SVG
/// grid using an equirectangular projection. The result is a
/// clickable pin layout, not a real map. Production should swap the
/// grid SVG for a Leaflet + OSM tile layer (or equivalent), passing
/// the same `pins` array through.
pub async fn map(State(state): State<WebState>) -> Response {
    let mut ctx = state.context();
    let singular = state.app.entity_singular;

    let raw_pins = [
        ("London", "London, UK", 51.5074_f64, -0.1278_f64),
        ("New York", "New York, US", 40.7128, -74.0060),
        ("Tokyo", "Tokyo, JP", 35.6762, 139.6503),
        ("Sydney", "Sydney, AU", -33.8688, 151.2093),
        ("São Paulo", "São Paulo, BR", -23.5505, -46.6333),
        ("Cape Town", "Cape Town, ZA", -33.9249, 18.4241),
    ];

    let pins: Vec<serde_json::Value> = raw_pins
        .iter()
        .enumerate()
        .map(|(i, &(label, locality, lat, lon))| {
            // Equirectangular projection onto a 1000×500 viewBox:
            // x = (lon + 180) / 360 * 1000
            // y = (90 - lat) / 180 * 500
            let x = (lon + 180.0) / 360.0 * 1000.0;
            let y = (90.0 - lat) / 180.0 * 500.0;
            serde_json::json!({
                "id": format!("pin-{i}"),
                "label": label,
                "locality": locality,
                "lat": lat,
                "lon": lon,
                "x": format!("{:.1}", x),
                "y": format!("{:.1}", y),
                "record_id": format!("00000000-0000-0000-0000-{:012}", i + 1),
                "record_label": format!("Sample {singular} {} ({label})", i + 1),
            })
        })
        .collect();

    ctx.insert("pins", &pins);
    render(&state, "map.html.tera", ctx).into_response()
}

#[derive(Debug, Deserialize)]
pub struct CalendarQuery {
    #[serde(default)]
    pub month: Option<String>,
}

/// `GET /{entity_plural}/calendar?month=YYYY-MM` — browse records by
/// creation date.
///
/// Scaffold: builds a Sunday-anchored 7-column month grid for the
/// requested `?month=YYYY-MM` (default 2026-05); seeds 8 sample
/// records spread across the current month + 2 records in the
/// previous month so the prev/next navigation has something to show.
/// Wire through to `Repository::list_by_created_date_range(start,
/// end)` once the web tier is plumbed into `AppState`.
pub async fn calendar(
    State(state): State<WebState>,
    Query(CalendarQuery { month }): Query<CalendarQuery>,
) -> Response {
    let mut ctx = state.context();
    let singular = state.app.entity_singular;

    let (year, mo) = parse_month(month.as_deref()).unwrap_or((2026, 5));
    let month_label = format!("{} {}", month_name(mo), year);

    let first_dow = day_of_week(year, mo, 1);
    let days_in_month_n = days_in_month(year, mo);
    let (prev_year, prev_mo) = previous_month(year, mo);
    let (next_year, next_mo) = next_month(year, mo);
    let days_in_prev = days_in_month(prev_year, prev_mo);

    let counts: std::collections::HashMap<String, u32> = [
        (iso_date(year, mo, 3), 2u32),
        (iso_date(year, mo, 7), 1),
        (iso_date(year, mo, 12), 5),
        (iso_date(year, mo, 14), 1),
        (iso_date(year, mo, 18), 3),
        (iso_date(year, mo, 22), 1),
        (iso_date(year, mo, 25), 4),
        (iso_date(year, mo, 28), 1),
        (iso_date(prev_year, prev_mo, days_in_prev), 2),
    ]
    .into_iter()
    .collect();

    let mut records_by_date: std::collections::HashMap<String, Vec<serde_json::Value>> =
        std::collections::HashMap::new();
    for (i, (iso, n)) in counts.iter().enumerate() {
        let mut day_records = Vec::new();
        for k in 0..*n {
            day_records.push(serde_json::json!({
                "id": format!("{:08x}-cal-{}-{}", i as u32, iso, k),
                "label": format!("Sample {singular} {}/{}", i + 1, k + 1),
                "subtitle": format!("Created {iso}"),
            }));
        }
        records_by_date.insert(iso.clone(), day_records);
    }

    let mut weeks: Vec<Vec<serde_json::Value>> = Vec::new();
    let mut day_cursor: i32 = 1 - first_dow as i32;
    for _ in 0..6 {
        let mut row: Vec<serde_json::Value> = Vec::with_capacity(7);
        for _ in 0..7 {
            let (cy, cmo, cd, in_month) = if day_cursor < 1 {
                (
                    prev_year,
                    prev_mo,
                    (days_in_prev as i32 + day_cursor) as u32,
                    false,
                )
            } else if day_cursor > days_in_month_n as i32 {
                (
                    next_year,
                    next_mo,
                    (day_cursor - days_in_month_n as i32) as u32,
                    false,
                )
            } else {
                (year, mo, day_cursor as u32, true)
            };
            let iso = iso_date(cy, cmo, cd);
            let count = *counts.get(&iso).unwrap_or(&0);
            row.push(serde_json::json!({
                "iso": iso,
                "day": cd,
                "in_month": in_month,
                "count": count,
            }));
            day_cursor += 1;
        }
        weeks.push(row);
    }

    let weekday_labels = vec![
        serde_json::json!({"short": "Sun", "full": "Sunday"}),
        serde_json::json!({"short": "Mon", "full": "Monday"}),
        serde_json::json!({"short": "Tue", "full": "Tuesday"}),
        serde_json::json!({"short": "Wed", "full": "Wednesday"}),
        serde_json::json!({"short": "Thu", "full": "Thursday"}),
        serde_json::json!({"short": "Fri", "full": "Friday"}),
        serde_json::json!({"short": "Sat", "full": "Saturday"}),
    ];

    let totals_records: u32 = counts
        .iter()
        .filter(|(iso, _)| iso.starts_with(&format!("{year}-{:02}", mo)))
        .map(|(_, c)| *c)
        .sum();

    let selected_date = counts
        .keys()
        .filter(|iso| iso.starts_with(&format!("{year}-{:02}", mo)))
        .max()
        .cloned()
        .unwrap_or_else(|| iso_date(year, mo, 1));

    ctx.insert("month_label", &month_label);
    ctx.insert("prev_month", &format!("{prev_year}-{:02}", prev_mo));
    ctx.insert("next_month", &format!("{next_year}-{:02}", next_mo));
    ctx.insert(
        "prev_month_label",
        &format!("{} {}", month_name(prev_mo), prev_year),
    );
    ctx.insert(
        "next_month_label",
        &format!("{} {}", month_name(next_mo), next_year),
    );
    ctx.insert("weekday_labels", &weekday_labels);
    ctx.insert("weeks", &weeks);
    ctx.insert("selected_date", &selected_date);
    ctx.insert("records_by_date", &records_by_date);
    ctx.insert("totals", &serde_json::json!({ "records": totals_records }));
    render(&state, "calendar.html.tera", ctx).into_response()
}

fn parse_month(s: Option<&str>) -> Option<(i32, u32)> {
    let s = s?;
    let (y, m) = s.split_once('-')?;
    Some((y.parse().ok()?, m.parse().ok()?))
}

fn month_name(m: u32) -> &'static str {
    [
        "",
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ][m.min(12) as usize]
}

fn previous_month(y: i32, m: u32) -> (i32, u32) {
    if m == 1 {
        (y - 1, 12)
    } else {
        (y, m - 1)
    }
}

fn next_month(y: i32, m: u32) -> (i32, u32) {
    if m == 12 {
        (y + 1, 1)
    } else {
        (y, m + 1)
    }
}

fn iso_date(y: i32, m: u32, d: u32) -> String {
    format!("{y}-{:02}-{:02}", m, d)
}

fn days_in_month(y: i32, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

/// Zeller's congruence → 0=Sun, 1=Mon, … 6=Sat for the given Gregorian date.
fn day_of_week(y: i32, m: u32, d: u32) -> u32 {
    let (y, m) = if m < 3 { (y - 1, m + 12) } else { (y, m) };
    let k = y.rem_euclid(100);
    let j = y.div_euclid(100);
    let h = (d as i32
        + (13 * (m as i32 + 1)) / 5
        + k
        + k / 4
        + j / 4
        + 5 * j)
        .rem_euclid(7);
    ((h + 6) % 7) as u32
}

/// `GET /{entity_plural}/{id}/qr` — QR-code share view.
///
/// Scaffold: server-renders a 21×21 placeholder SVG pattern derived
/// from a stable hash of the share URL plus the three QR finder
/// squares in the top-left, top-right, and bottom-left corners.
/// Production should swap `placeholder_qr_svg(url, modules)` for the
/// `qrcode` crate (or equivalent) so the image is a scannable QR.
pub async fn qr(State(state): State<WebState>, Path(id): Path<String>) -> Response {
    let mut ctx = state.context();
    let plural = state.app.entity_plural;
    let share_url = format!("http://localhost:8080/{plural}/{id}");
    let module_grid: u32 = 21;
    let pixel_size: u32 = 210;
    let qr_svg = placeholder_qr_svg(&share_url, module_grid as usize, pixel_size);

    ctx.insert("record", &seed_record(&state, &id));
    ctx.insert("share_url", &share_url);
    ctx.insert("module_grid", &module_grid);
    ctx.insert("pixel_size", &pixel_size);
    ctx.insert("generated_at", "2026-05-26T00:00:00Z");
    ctx.insert("qr_svg", &qr_svg);
    render(&state, "qr.html.tera", ctx).into_response()
}

/// Generate a deterministic placeholder pattern that looks like a QR.
///
/// This is **not** a real QR code; the dots are derived from a hash
/// of the URL so the same URL always produces the same pattern, and
/// the three "finder" corner squares are drawn in the canonical
/// positions so the result is visually recognizable. Real production
/// QR generation belongs to the `qrcode` crate (which emits a
/// `<polygon>` per dark module). The headless `qr-code` wrapper does
/// not constrain how the inner image is produced.
fn placeholder_qr_svg(text: &str, modules: usize, pixel_size: u32) -> String {
    // Tiny FNV-1a seed → deterministic per-cell bits.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }

    let mut cells = vec![vec![false; modules]; modules];
    for r in 0..modules {
        for c in 0..modules {
            // Per-cell PRNG step: mix coordinates into the running hash.
            hash ^= ((r as u64) << 32) | (c as u64);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            cells[r][c] = (hash >> 17) & 1 == 1;
        }
    }

    // Stamp three QR-style 7×7 finder patterns in the corners.
    let finders = [(0_usize, 0_usize), (0, modules - 7), (modules - 7, 0)];
    for &(or, oc) in &finders {
        for dr in 0..7 {
            for dc in 0..7 {
                let on_border = dr == 0 || dr == 6 || dc == 0 || dc == 6;
                let on_inner = dr >= 2 && dr <= 4 && dc >= 2 && dc <= 4;
                cells[or + dr][oc + dc] = on_border || on_inner;
            }
        }
        // Clear the 8th-row/column "quiet zone" around each finder.
        for d in 0..8 {
            if or + 7 < modules {
                cells[or + 7][oc + d.min(modules - oc - 1)] = false;
            }
            if oc + 7 < modules {
                cells[or + d.min(modules - or - 1)][oc + 7] = false;
            }
        }
    }

    let mut svg = String::with_capacity(modules * modules * 32);
    use std::fmt::Write as _;
    let _ = write!(
        svg,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {modules} {modules}\" \
         width=\"{pixel_size}\" height=\"{pixel_size}\" role=\"img\" \
         aria-label=\"QR placeholder for {}\" \
         shape-rendering=\"crispEdges\">",
        text.replace('"', "&quot;").replace('<', "&lt;"),
    );
    let _ = write!(svg, "<rect width=\"{modules}\" height=\"{modules}\" fill=\"#ffffff\"/>");
    for r in 0..modules {
        for c in 0..modules {
            if cells[r][c] {
                let _ = write!(svg, "<rect x=\"{c}\" y=\"{r}\" width=\"1\" height=\"1\" fill=\"#000000\"/>");
            }
        }
    }
    svg.push_str("</svg>");
    svg
}

/// Query for the signature-capture page.
#[derive(serde::Deserialize)]
pub struct SignQuery {
    #[serde(default)]
    pub purpose: Option<String>,
}

/// `GET /{entity_plural}/:id/sign?purpose=consent|witness|…` — capture a
/// signature against a specific record.
///
/// Scaffold: Alpine-driven canvas (`signature-pad`) with pointer / pen /
/// touch handlers, per-stroke undo, clear, and on-submit `toDataURL`.
/// Production should `POST /api/{plural}/{id}/signatures` with
/// `{ purpose, signed_by_name, signed_at, image_png_base64,
/// audit_user_ip, audit_user_agent }` and append the signature to the
/// audit log + (where applicable) the related `Consent` record.
pub async fn sign(
    State(state): State<WebState>,
    Path(id): Path<String>,
    Query(SignQuery { purpose }): Query<SignQuery>,
) -> Response {
    let mut ctx = state.context();
    let allowed = ["consent", "witness", "acknowledgement", "authorisation", "other"];
    let p = purpose
        .filter(|p| allowed.contains(&p.as_str()))
        .unwrap_or_else(|| "consent".to_string());
    ctx.insert("record", &seed_record(&state, &id));
    ctx.insert("purpose", &p);
    render(&state, "sign.html.tera", ctx).into_response()
}

/// `GET /{entity_plural}/starred` — per-browser starred records.
///
/// Scaffold: seeds 6 candidate records. The page is Alpine-driven —
/// it reads `localStorage["lily-starred-{plural}"]` and `x-show`-filters
/// the rows. Empty state shown when nothing is starred. Production
/// should swap localStorage for a `StarRepository::list_for_user(user_id)`
/// query and a `POST/DELETE /api/{plural}/{id}/star` REST endpoint pair.
pub async fn starred(State(state): State<WebState>) -> Response {
    let mut ctx = state.context();
    let singular = state.app.entity_singular;
    let candidates = serde_json::json!([
        { "id": "star-cand-1", "label": format!("Sample {singular} 1"), "subtitle": "MRN-100201", "active": true },
        { "id": "star-cand-2", "label": format!("Sample {singular} 2"), "subtitle": "MRN-100202", "active": true },
        { "id": "star-cand-3", "label": format!("Sample {singular} 3"), "subtitle": "MRN-100203", "active": true },
        { "id": "star-cand-4", "label": format!("Sample {singular} 4"), "subtitle": "MRN-100204", "active": false },
        { "id": "star-cand-5", "label": format!("Sample {singular} 5"), "subtitle": "MRN-100205", "active": true },
        { "id": "star-cand-6", "label": format!("Sample {singular} 6"), "subtitle": "MRN-100206", "active": true }
    ]);
    ctx.insert("candidates", &candidates);
    render(&state, "starred.html.tera", ctx).into_response()
}

/// `GET /{entity_plural}/trash` — soft-deleted records with restore +
/// permanent-delete actions.
///
/// Scaffold: seeds 4 tombstone rows. Production should read from
/// `Repository::list_soft_deleted(page, limit)` and wire the Restore /
/// Delete-forever buttons to
/// `POST /api/{plural}/{id}/restore` and `DELETE /api/{plural}/{id}/purge`.
pub async fn trash(State(state): State<WebState>) -> Response {
    let mut ctx = state.context();
    let singular = state.app.entity_singular;
    let items = serde_json::json!([
        {
            "id": "tomb-1",
            "label": format!("Sample {singular} 1"),
            "deleted_at": "2026-05-22T14:18:00Z",
            "deleted_by": "u-admin-42",
            "reason": "Confirmed duplicate (merged into sample 8)"
        },
        {
            "id": "tomb-2",
            "label": format!("Sample {singular} 2"),
            "deleted_at": "2026-05-24T09:02:11Z",
            "deleted_by": "u-data-steward-7",
            "reason": "Test record from staging environment"
        },
        {
            "id": "tomb-3",
            "label": format!("Sample {singular} 3"),
            "deleted_at": "2026-05-25T16:44:50Z",
            "deleted_by": serde_json::Value::Null,
            "reason": serde_json::Value::Null
        },
        {
            "id": "tomb-4",
            "label": format!("Sample {singular} 4"),
            "deleted_at": "2026-05-27T11:30:00Z",
            "deleted_by": "u-admin-42",
            "reason": "GDPR Article 17 erasure request"
        }
    ]);
    let pagination = serde_json::json!({
        "page": 1,
        "total_pages": 1,
        "pages": [1],
        "has_prev": false,
        "has_next": false
    });
    ctx.insert("items", &items);
    ctx.insert("pagination", &pagination);
    render(&state, "trash.html.tera", ctx).into_response()
}

/// `GET /{entity_plural}/deduplicate` — batch-dedup scan launcher.
///
/// Scaffold: pure client-side flow (config → 6-digit PIN gate →
/// simulated progress → result summary). The PIN gate is intentional:
/// the underlying `POST /api/{plural}/deduplicate` is a heavy +
/// potentially destructive batch operation and production gates the
/// trigger behind 2FA. Demo PIN is `1357`.
pub async fn deduplicate(State(state): State<WebState>) -> Response {
    render(&state, "deduplicate.html.tera", state.context()).into_response()
}

/// `GET /{entity_plural}/review-queue/kanban` — kanban-board view of
/// the deduplication review queue.
///
/// Scaffold: seeds 8 review items spread across the four statuses
/// (Pending / Confirmed / Rejected / AutoMerged). Alpine state in the
/// template owns the drag-and-drop column reassignments; production
/// should hook the on-drop callback to
/// `POST /api/{plural}/review-queue/{id}/status`.
pub async fn review_queue_kanban(State(state): State<WebState>) -> Response {
    let mut ctx = state.context();
    let singular = state.app.entity_singular;

    let items = vec![
        kanban_item("k-001", "Pending", "definite", "0.99", "tax_id exact match",
                    "aaaa-1", &format!("Sample {singular} 1"),
                    "aaaa-2", &format!("Sample {singular} 1 (duplicate)")),
        kanban_item("k-002", "Pending", "probable", "0.87", "name + DOB",
                    "bbbb-1", &format!("Sample {singular} 2"),
                    "bbbb-2", &format!("Sample {singular} 2 (variant)")),
        kanban_item("k-003", "Pending", "possible", "0.62", "phonetic + address",
                    "cccc-1", &format!("Sample {singular} 3"),
                    "cccc-2", &format!("Sample {singular} 3 (close)")),
        kanban_item("k-004", "Confirmed", "definite", "0.98", "document number match",
                    "dddd-1", &format!("Sample {singular} 4"),
                    "dddd-2", &format!("Sample {singular} 4 (dup)")),
        kanban_item("k-005", "Confirmed", "probable", "0.91", "name + DOB + gender",
                    "eeee-1", &format!("Sample {singular} 5"),
                    "eeee-2", &format!("Sample {singular} 5 (variant)")),
        kanban_item("k-006", "Rejected", "possible", "0.55", "shared postal code",
                    "ffff-1", &format!("Sample {singular} 6"),
                    "ffff-2", &format!("Sample {singular} 6 (unrelated)")),
        kanban_item("k-007", "AutoMerged", "definite", "1.00", "tax_id exact match (auto)",
                    "gggg-1", &format!("Sample {singular} 7"),
                    "gggg-2", &format!("Sample {singular} 7 (auto-merged)")),
        kanban_item("k-008", "AutoMerged", "definite", "0.99", "MRN exact match (auto)",
                    "hhhh-1", &format!("Sample {singular} 8"),
                    "hhhh-2", &format!("Sample {singular} 8 (auto-merged)")),
    ];

    let columns = vec![
        serde_json::json!({ "status": "Pending" }),
        serde_json::json!({ "status": "Confirmed" }),
        serde_json::json!({ "status": "Rejected" }),
        serde_json::json!({ "status": "AutoMerged" }),
    ];

    ctx.insert("items", &items);
    ctx.insert("columns", &columns);
    render(&state, "review_queue_kanban.html.tera", ctx).into_response()
}

#[allow(clippy::too_many_arguments)]
fn kanban_item(
    id: &str,
    status: &str,
    quality: &str,
    score: &str,
    detection_method: &str,
    a_id: &str,
    a_label: &str,
    b_id: &str,
    b_label: &str,
) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "status": status,
        "quality": quality,
        "score": score,
        "detection_method": detection_method,
        "a_id": a_id,
        "a_label": a_label,
        "b_id": b_id,
        "b_label": b_label,
    })
}

/// `GET /{entity_plural}/{id}/notes` — per-record notes / chat thread.
///
/// Scaffold: seeds five role-tagged notes (clinical / admin / system)
/// so the headless `chat-nav` → `chat-list` → `chat-list-item` →
/// `chat-message` chain renders with avatars, role badges, tag-groups,
/// and the role-keyed left-border accent (`data-author-role`). The
/// new-note form is Alpine-driven; in production it HTMX-posts to
/// `POST /api/{plural}/{id}/notes` and the server returns the new
/// `<li class="chat-list-item">` for `hx-swap="beforeend"`.
pub async fn notes(State(state): State<WebState>, Path(id): Path<String>) -> Response {
    let mut ctx = state.context();
    let singular = state.app.entity_singular;
    let notes = vec![
        note_entry(
            "Dr Alice Park",
            "AP",
            "#005eb8",
            "clinical",
            "2026-05-26T09:14:00Z",
            false,
            format!(
                "Reviewed identifiers on intake. {singular} reports a severe penicillin \
                 allergy; verified against the medical-banner overlay. <strong>No \
                 beta-lactam antibiotics</strong>."
            ),
            vec!["allergy", "intake"],
        ),
        note_entry(
            "u-bob",
            "BO",
            "#330072",
            "admin",
            "2026-05-25T13:48:23Z",
            true,
            format!(
                "Corrected mis-typed postal code. Previous value rolled back via the \
                 audit log. {singular} contacted by email to confirm the new address."
            ),
            vec!["address", "data-quality"],
        ),
        note_entry(
            "system",
            "SY",
            "#768692",
            "system",
            "2026-05-25T05:30:01Z",
            false,
            "Nightly normalization job applied: city capitalization (\"bristl\" → \
             \"Bristol\"), country code uppercased (gb → GB)."
                .to_string(),
            vec!["normalization"],
        ),
        note_entry(
            "Dr Carol Vance",
            "CV",
            "#006747",
            "clinical",
            "2026-05-20T16:02:18Z",
            false,
            "Follow-up appointment scheduled 2026-06-05 for post-surgical review. \
             Patient consented to data-sharing with referring GP."
                .to_string(),
            vec!["follow-up"],
        ),
        note_entry(
            "Dr Alice Park",
            "AP",
            "#005eb8",
            "clinical",
            "2026-04-12T09:14:22Z",
            false,
            format!("Initial {singular} record created. Identifiers and primary contact verified."),
            vec!["intake"],
        ),
    ];
    ctx.insert("record", &seed_record(&state, &id));
    ctx.insert("notes", &notes);
    render(&state, "notes.html.tera", ctx).into_response()
}

#[allow(clippy::too_many_arguments)]
fn note_entry(
    author_name: &str,
    initials: &str,
    avatar_bg: &str,
    role: &str,
    at: &str,
    edited: bool,
    body: String,
    tags: Vec<&str>,
) -> serde_json::Value {
    serde_json::json!({
        "author_name": author_name,
        "initials": initials,
        "avatar_bg": avatar_bg,
        "role": role,
        "at": at,
        "edited": edited,
        "body": body,
        "tags": tags,
    })
}

/// `GET /{entity_plural}/{id}/links` — entity-link graph view.
///
/// Scaffold: seeds four link groups (Replaces / ReplacedBy / Refer /
/// Seealso) with sample targets so the headless `tree-nav`,
/// `tree-list`, `tree-list-item`, `tree-link`, kind-keyed `badge`s,
/// per-row remove `action-bar-button` (disabled for merge-derived
/// links), `information-callout`, and the "Add link" `drawer` are
/// visible end-to-end. Wire through to
/// `crate::models::PersonLink` (or `PatientLink`) once the web tier
/// is plumbed into `AppState`.
pub async fn links(State(state): State<WebState>, Path(id): Path<String>) -> Response {
    let mut ctx = state.context();
    let singular = state.app.entity_singular;
    let groups = vec![
        serde_json::json!({
            "kind": "Replaces",
            "description": format!("This {singular} is the surviving record after a merge"),
            "items": [
                {
                    "id": "l-001",
                    "target_id": "aaaa-merged-1",
                    "target_label": format!("Sample {singular} (duplicate)"),
                    "merge_score": "0.99",
                    "note": "Auto-merged after tax-ID exact match"
                }
            ]
        }),
        serde_json::json!({
            "kind": "ReplacedBy",
            "description": format!("This {singular} was merged into another record"),
            "items": []
        }),
        serde_json::json!({
            "kind": "Refer",
            "description": "Manual cross-reference",
            "items": [
                {
                    "id": "l-002",
                    "target_id": "bbbb-refer-1",
                    "target_label": format!("Related {singular} (sibling)"),
                    "merge_score": null,
                    "note": "Asserted as sibling by intake clinician"
                }
            ]
        }),
        serde_json::json!({
            "kind": "Seealso",
            "description": "Possibly relevant record; review when handling this one",
            "items": [
                {
                    "id": "l-003",
                    "target_id": "cccc-seealso-1",
                    "target_label": format!("Adjacent {singular} (same address)"),
                    "merge_score": null,
                    "note": null
                },
                {
                    "id": "l-004",
                    "target_id": "cccc-seealso-2",
                    "target_label": format!("Adjacent {singular} (similar name)"),
                    "merge_score": null,
                    "note": "Different DOB, flagged for review"
                }
            ]
        }),
    ];
    let total: u32 = groups
        .iter()
        .map(|g| g["items"].as_array().map(|a| a.len() as u32).unwrap_or(0))
        .sum();
    let totals = serde_json::json!({ "total": total });
    ctx.insert("record", &seed_record(&state, &id));
    ctx.insert("groups", &groups);
    ctx.insert("totals", &totals);
    render(&state, "links.html.tera", ctx).into_response()
}

/// `GET /{entity_plural}/{id}/fhir` — FHIR R5 resource view.
///
/// Scaffold: seeds a FHIR-shaped JSON payload (per the crate's
/// `fhir_resource_type` — Patient / Person / Organization / Location /
/// Device / Observation, depending on the entity) so the headless
/// `clipboard-copy-button`, `code-block`, `information-callout`, and
/// `summary-list` components render end-to-end. The "Download JSON"
/// link points at the existing FHIR endpoint
/// `GET /fhir/{ResourceType}/{id}`.
pub async fn fhir(State(state): State<WebState>, Path(id): Path<String>) -> Response {
    let mut ctx = state.context();
    let record = seed_record(&state, &id);

    let resource_type = match state.app.entity_singular {
        "patient" => "Patient",
        "person" => "Person",
        "worker" => "Practitioner",
        "place" => "Location",
        "thing" => "Device",
        "event" => "Observation",
        _ => "Basic",
    };

    let payload = serde_json::json!({
        "resourceType": resource_type,
        "id": id,
        "meta": {
            "versionId": "1",
            "lastUpdated": "2026-05-25T07:14:08Z",
            "profile": [format!("http://hl7.org/fhir/StructureDefinition/{resource_type}")]
        },
        "active": true,
        "identifier": [
            {
                "use": "official",
                "type": { "text": "MRN" },
                "system": "urn:oid:1.2.36.146.595.217.0.1",
                "value": "M-001"
            }
        ],
        "name": [
            { "use": "official", "family": "Smith", "given": ["John", "W."] }
        ],
        "telecom": [
            { "system": "phone", "value": "+44 20 7946 0958", "use": "home" },
            { "system": "email", "value": format!("sample.{}@example.test", state.app.entity_singular) }
        ],
        "gender": "male",
        "birthDate": "1980-01-15",
        "address": [
            {
                "use": "home",
                "line": ["10 Downing St"],
                "city": "London",
                "postalCode": "SW1A 2AA",
                "country": "GB"
            }
        ]
    });

    let fhir_json_pretty = serde_json::to_string_pretty(&payload).unwrap_or_default();
    let fhir_json = serde_json::to_string(&payload).unwrap_or_default();

    ctx.insert("record", &record);
    ctx.insert("fhir_resource_type", resource_type);
    ctx.insert("fhir_json", &fhir_json);
    ctx.insert("fhir_json_pretty", &fhir_json_pretty);
    ctx.insert("generated_at", "2026-05-25T07:14:08Z");
    render(&state, "fhir.html.tera", ctx).into_response()
}

pub async fn export(State(state): State<WebState>, Path(id): Path<String>) -> Response {
    let mut ctx = state.context();
    let record = seed_record(&state, &id);

    let mut payload = record.clone();
    if let Some(hc) = seed_healthcare(&state) {
        payload["healthcare"] = hc;
    }
    payload["addresses"] = serde_json::json!([
        {
            "use": "home",
            "line1": "10 Downing St",
            "city": "London",
            "postal_code": "SW1A 2AA",
            "country": "GB"
        }
    ]);
    payload["telecom"] = serde_json::json!([
        { "system": "phone", "value": "+44 20 7946 0958", "use": "home" },
        { "system": "email", "value": format!("sample.{}@example.test", state.app.entity_singular), "use": "work" }
    ]);
    payload["identifiers"] = serde_json::json!([
        { "type": "MRN", "system": "urn:oid:1.2.36.146.595.217.0.1", "value": "M-001" }
    ]);

    let export_json_pretty = serde_json::to_string_pretty(&payload).unwrap_or_default();
    let export_json = serde_json::to_string(&payload).unwrap_or_default();
    let export_size_bytes = export_json_pretty.len();

    let consents = vec![
        serde_json::json!({
            "consent_type": "DataProcessing",
            "status": "Active",
            "badge_type": "success",
        }),
        serde_json::json!({
            "consent_type": "Research",
            "status": "Revoked",
            "badge_type": "error",
        }),
    ];

    ctx.insert("record", &record);
    ctx.insert("export_json", &export_json);
    ctx.insert("export_json_pretty", &export_json_pretty);
    ctx.insert("export_size_bytes", &export_size_bytes);
    ctx.insert("generated_at", "2026-05-25T00:00:00Z");
    ctx.insert("consents", &consents);
    render(&state, "export.html.tera", ctx).into_response()
}

#[derive(Debug, Deserialize)]
pub struct AuditRecentQuery {
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(default = "default_audit_limit")]
    pub limit: u32,
}

fn default_audit_limit() -> u32 {
    50
}

/// `GET /audit` — system-wide recent audit activity.
///
/// Scaffold: seeds eight cross-entity audit entries so the headless
/// `timeline-list`, action-keyed `badge` variants, `text-input` user
/// filter, `select` action + limit filters, `tag-group` active-filter
/// chips, and per-entry deep links to `/{plural}/{id}/audit` are
/// visible end-to-end. Wire through to
/// `crate::db::AuditLogRepository::get_recent(limit)` once the web
/// tier is plumbed into `AppState`.
pub async fn audit_recent(
    State(state): State<WebState>,
    Query(AuditRecentQuery {
        action,
        user_id,
        limit,
    }): Query<AuditRecentQuery>,
) -> Response {
    let mut ctx = state.context();
    let filter_action = action.unwrap_or_default();
    let filter_user = user_id.unwrap_or_default();
    let filter_limit = limit;
    let singular = state.app.entity_singular;

    let mut active_filters: Vec<String> = Vec::new();
    if !filter_action.is_empty() {
        active_filters.push(format!("Action: {filter_action}"));
    }
    if !filter_user.is_empty() {
        active_filters.push(format!("User: {filter_user}"));
    }
    if filter_limit != default_audit_limit() {
        active_filters.push(format!("Limit: {filter_limit}"));
    }

    let all_entries = vec![
        audit_recent_entry(
            "aaaa-1",
            format!("Sample {singular} 1"),
            "Created",
            "2026-05-25T07:01:42Z",
            "u-alice",
            Some("10.0.42.17"),
            Some(format!("New {singular} created via REST POST")),
        ),
        audit_recent_entry(
            "bbbb-2",
            format!("Sample {singular} 2"),
            "Updated",
            "2026-05-25T06:54:09Z",
            "u-bob",
            Some("10.0.42.91"),
            Some("Address corrected".to_string()),
        ),
        audit_recent_entry(
            "cccc-3",
            format!("Sample {singular} 3"),
            "Merged",
            "2026-05-25T06:31:18Z",
            "u-alice",
            Some("10.0.42.17"),
            Some(format!("Merged duplicate {singular}-aaaa-7")),
        ),
        audit_recent_entry(
            "dddd-4",
            format!("Sample {singular} 4"),
            "Deleted",
            "2026-05-25T05:48:51Z",
            "u-bob",
            Some("10.0.42.91"),
            Some(format!("Soft-deleted {singular}; restore window 30 days")),
        ),
        audit_recent_entry(
            "eeee-5",
            format!("Sample {singular} 5"),
            "Linked",
            "2026-05-25T05:22:03Z",
            "u-carol",
            Some("10.0.43.5"),
            Some("Linked to organization main-clinic-42".to_string()),
        ),
        audit_recent_entry(
            "ffff-6",
            format!("Sample {singular} 6"),
            "Updated",
            "2026-05-25T04:09:27Z",
            "u-alice",
            Some("10.0.42.17"),
            Some("Identifier (MRN) added".to_string()),
        ),
        audit_recent_entry(
            "gggg-7",
            format!("Sample {singular} 7"),
            "Created",
            "2026-05-25T03:54:12Z",
            "u-carol",
            Some("10.0.43.5"),
            Some(format!("Bulk-imported {singular} from CSV batch #2026-05-25-a")),
        ),
        audit_recent_entry(
            "hhhh-8",
            format!("Sample {singular} 8"),
            "Updated",
            "2026-05-25T02:31:05Z",
            "system",
            None,
            Some("Nightly normalization job rewrote address (capitalized city)".to_string()),
        ),
    ];

    let entries: Vec<serde_json::Value> = all_entries
        .into_iter()
        .filter(|e| {
            (filter_action.is_empty() || e["action"].as_str() == Some(filter_action.as_str()))
                && (filter_user.is_empty()
                    || e["user_id"].as_str() == Some(filter_user.as_str()))
        })
        .take(filter_limit as usize)
        .collect();

    ctx.insert("filter_action", &filter_action);
    ctx.insert("filter_user", &filter_user);
    ctx.insert("filter_limit", &filter_limit);
    ctx.insert("active_filters", &active_filters);
    ctx.insert("entries", &entries);
    render(&state, "audit_recent.html.tera", ctx).into_response()
}

fn audit_recent_entry(
    entity_id: &str,
    entity_label: String,
    action: &str,
    at: &str,
    user_id: &str,
    user_ip: Option<&str>,
    summary: Option<String>,
) -> serde_json::Value {
    serde_json::json!({
        "entity_id": entity_id,
        "entity_label": entity_label,
        "action": action,
        "at": at,
        "user_id": user_id,
        "user_ip": user_ip,
        "summary": summary,
    })
}

/// `GET /{entity_plural}/{id}/audit` — per-record audit log.
///
/// Scaffold: seeds six placeholder audit entries (one per action type:
/// Created / Updated / Updated / Merged / Linked / Deleted-then-undeleted)
/// so the headless `timeline-list`, action-keyed `badge` variants,
/// `details` + `code-block` old/new value diffs, and `select`-driven
/// HTMX action filter are visible end-to-end. Wire through to the real
/// `crate::db::AuditLogRepository::get_history(entity_id)` once the web
/// tier is plumbed into `AppState`.
pub async fn audit(
    State(state): State<WebState>,
    Path(id): Path<String>,
    Query(AuditQuery { action, page }): Query<AuditQuery>,
) -> Response {
    let mut ctx = state.context();
    let filter_action = action.unwrap_or_default();
    let singular = state.app.entity_singular;

    let all_entries = vec![
        audit_entry(
            "Created",
            "2026-04-12T09:14:22Z",
            "u-alice",
            Some("10.0.42.17"),
            Some("Mozilla/5.0 (Macintosh) Loco/0.14"),
            Some(format!("Initial {singular} created via REST POST")),
            None,
            Some(r#"{"label":"Sample","active":true}"#),
        ),
        audit_entry(
            "Updated",
            "2026-04-18T13:02:51Z",
            "u-alice",
            Some("10.0.42.17"),
            Some("Mozilla/5.0 (Macintosh) Loco/0.14"),
            Some("Address corrected".to_string()),
            Some(r#"{"address":{"city":"Bristl"}}"#),
            Some(r#"{"address":{"city":"Bristol"}}"#),
        ),
        audit_entry(
            "Updated",
            "2026-05-02T17:48:09Z",
            "u-bob",
            Some("10.0.42.91"),
            Some("curl/8.4.0"),
            Some("Identifier added".to_string()),
            Some(r#"{"identifiers":[]}"#),
            Some(r#"{"identifiers":[{"type":"MRN","value":"M-001"}]}"#),
        ),
        audit_entry(
            "Merged",
            "2026-05-10T08:22:14Z",
            "u-alice",
            Some("10.0.42.17"),
            Some("Mozilla/5.0 (Macintosh) Loco/0.14"),
            Some(format!("Merged duplicate {singular} aaaa-2 into this record")),
            None,
            Some(r#"{"merge":{"duplicate_id":"aaaa-2","score":0.99}}"#),
        ),
        audit_entry(
            "Linked",
            "2026-05-15T11:05:33Z",
            "u-carol",
            Some("10.0.43.5"),
            Some("Mozilla/5.0 (X11; Linux)"),
            Some("Linked to organization main-clinic-42".to_string()),
            None,
            Some(r#"{"managing_organization":"main-clinic-42"}"#),
        ),
        audit_entry(
            "Deleted",
            "2026-05-20T19:41:00Z",
            "u-bob",
            Some("10.0.42.91"),
            Some("curl/8.4.0"),
            Some("Soft-deleted, then restored 4 minutes later".to_string()),
            Some(r#"{"active":true}"#),
            Some(r#"{"active":false}"#),
        ),
    ];
    let entries: Vec<serde_json::Value> = all_entries
        .into_iter()
        .filter(|e| {
            filter_action.is_empty() || e["action"].as_str() == Some(filter_action.as_str())
        })
        .collect();

    let total_pages: u32 = 1;
    let pagination = serde_json::json!({
        "page": page,
        "total_pages": total_pages,
        "has_prev": page > 1,
        "has_next": page < total_pages,
        "pages": (1..=total_pages).collect::<Vec<u32>>(),
    });

    ctx.insert("record", &seed_record(&state, &id));
    ctx.insert("entries", &entries);
    ctx.insert("filter_action", &filter_action);
    ctx.insert("pagination", &pagination);
    render(&state, "audit.html.tera", ctx).into_response()
}

#[allow(clippy::too_many_arguments)]
fn audit_entry(
    action: &str,
    at: &str,
    user_id: &str,
    user_ip: Option<&str>,
    user_agent: Option<&str>,
    summary: Option<String>,
    old_value: Option<&str>,
    new_value: Option<&str>,
) -> serde_json::Value {
    serde_json::json!({
        "action": action,
        "at": at,
        "user_id": user_id,
        "user_ip": user_ip,
        "user_agent": user_agent,
        "summary": summary,
        "old_value": old_value,
        "new_value": new_value,
    })
}

#[derive(Debug, Deserialize)]
pub struct ReviewQueueQuery {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub quality: Option<String>,
    #[serde(default = "default_page")]
    pub page: u32,
}

/// `GET /{entity_plural}/review-queue` — deduplication review queue.
///
/// Scaffold: seeds four placeholder review items spanning all four
/// match-quality buckets (definite/probable/possible/unlikely) so the
/// `meter`, `details`, `summary-list`, quality-keyed `badge` variants,
/// per-row `alert-dialog` merge confirmation, `select` filters, and
/// `pagination-nav` are visible end-to-end. Wire this through to the
/// real review-queue repository once the web tier is plumbed into
/// `AppState`.
pub async fn review_queue(
    State(state): State<WebState>,
    Query(ReviewQueueQuery {
        status,
        quality,
        page,
    }): Query<ReviewQueueQuery>,
) -> Response {
    let mut ctx = state.context();
    let filter_status = status.unwrap_or_else(|| "Pending".to_string());
    let filter_quality = quality.unwrap_or_default();
    let singular = state.app.entity_singular;

    let all_items = vec![
        review_item(
            "11111111-1111-1111-1111-111111111111",
            "definite",
            0.99,
            "tax_id exact match",
            ("aaaa-1", format!("Sample {singular} 1")),
            ("aaaa-2", format!("Sample {singular} 1 (duplicate)")),
        ),
        review_item(
            "22222222-2222-2222-2222-222222222222",
            "probable",
            0.87,
            "name + DOB + gender",
            ("bbbb-1", format!("Sample {singular} 2")),
            ("bbbb-2", format!("Sample {singular} 2 (variant)")),
        ),
        review_item(
            "33333333-3333-3333-3333-333333333333",
            "possible",
            0.62,
            "phonetic + address",
            ("cccc-1", format!("Sample {singular} 3")),
            ("cccc-2", format!("Sample {singular} 3 (close)")),
        ),
        review_item(
            "44444444-4444-4444-4444-444444444444",
            "unlikely",
            0.42,
            "shared postal code",
            ("dddd-1", format!("Sample {singular} 4")),
            ("dddd-2", format!("Sample {singular} 4 (weak)")),
        ),
    ];
    let items: Vec<serde_json::Value> = all_items
        .into_iter()
        .filter(|it| {
            filter_quality.is_empty() || it["quality"].as_str() == Some(filter_quality.as_str())
        })
        .collect();

    let stats = serde_json::json!({
        "pending": 4,
        "confirmed": 0,
        "rejected": 0,
        "auto_merged": 0,
    });
    let total_pages: u32 = 1;
    let pagination = serde_json::json!({
        "page": page,
        "total_pages": total_pages,
        "has_prev": page > 1,
        "has_next": page < total_pages,
        "pages": (1..=total_pages).collect::<Vec<u32>>(),
    });

    ctx.insert("filter_status", &filter_status);
    ctx.insert("filter_quality", &filter_quality);
    ctx.insert("items", &items);
    ctx.insert("stats", &stats);
    ctx.insert("pagination", &pagination);
    render(&state, "review_queue.html.tera", ctx).into_response()
}

fn review_item(
    id: &str,
    quality: &str,
    score: f64,
    detection_method: &str,
    a: (&str, String),
    b: (&str, String),
) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "a_id": a.0,
        "a_label": a.1,
        "b_id": b.0,
        "b_label": b.1,
        "score": format!("{:.2}", score),
        "score_pct": (score * 100.0).round() as u32,
        "quality": quality,
        "detection_method": detection_method,
        "created_at": "2026-05-24T00:00:00Z",
        "breakdown": {
            "name": format!("{:.2}", (score - 0.05).max(0.0)),
            "birth_date": format!("{:.2}", (score - 0.03).max(0.0)),
            "address": format!("{:.2}", (score - 0.10).max(0.0)),
        },
    })
}

/// Seed healthcare overlay data for healthcare-aware crates (patient,
/// person). Returns `None` for non-healthcare crates so the
/// `{% if healthcare %}` block in the templates renders nothing.
fn seed_healthcare(state: &WebState) -> Option<serde_json::Value> {
    match state.app.entity_singular {
        "patient" | "person" => Some(serde_json::json!({
            "nhs_number": "943 476 5919",
            "ssn": "***-**-6789",
            "ssn_last4": "6789",
            "birth_date": "1980-01-15",
            "alerts": [
                {
                    "title": "Severe penicillin allergy",
                    "detail": "Anaphylaxis on previous exposure. Do not prescribe beta-lactam antibiotics."
                },
                {
                    "title": "Do not resuscitate",
                    "detail": "DNR order on file as of 2025-11-02."
                }
            ],
            "advice": [
                {
                    "title": "Primary contact",
                    "detail": "Jane Smith (spouse) — 555-0199."
                },
                {
                    "title": "Interpreter required",
                    "detail": "Preferred language: Spanish."
                }
            ],
            "care_instructions": [
                {
                    "title": "Anticoagulant review",
                    "body": "Warfarin INR target 2.0–3.0. Next INR check due within 2 weeks.",
                    "urgency": "Routine",
                    "urgency_type": "info"
                },
                {
                    "title": "Post-surgical follow-up",
                    "body": "Wound check and suture removal scheduled for 2026-06-05.",
                    "urgency": "Scheduled",
                    "urgency_type": "warning"
                }
            ]
        })),
        _ => None,
    }
}

fn seed_record(state: &WebState, id: &str) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "label": format!("Sample {}", state.app.entity_singular),
        "subtitle": "Placeholder record for headless-component verification",
        "active": true,
        "created_at": "2026-05-24T00:00:00Z",
        "updated_at": "2026-05-24T00:00:00Z",
    })
}

#[derive(Debug, Deserialize)]
pub struct CompareQuery {
    pub a: String,
    pub b: String,
    #[serde(default)]
    pub review_id: Option<String>,
}

/// `GET /{entity_plural}/compare?a=&b=&review_id=` — side-by-side match comparison.
///
/// Scaffold: seeds two candidate records and a per-component scoring
/// table so the headless `diff` two-column layout (with two `card`s
/// containing `summary-list`s), the `data-table` of per-component
/// scores with `meter` bars and outcome `badge`s, the overall-match
/// `card`/`meter`/`badge` cluster, and the merge-confirmation
/// `alert-dialog` are visible end-to-end. Wire through to the real
/// matcher and review-queue once the web tier is plumbed into
/// `AppState`.
pub async fn compare(
    State(state): State<WebState>,
    Query(CompareQuery { a, b, review_id }): Query<CompareQuery>,
) -> Response {
    let mut ctx = state.context();
    let singular = state.app.entity_singular;
    let a_record = serde_json::json!({
        "id": a,
        "label": format!("Sample {singular} 1"),
    });
    let b_record = serde_json::json!({
        "id": b,
        "label": format!("Sample {singular} 1 (duplicate)"),
    });

    let fields = vec![
        compare_field("Family name", "Smith", "Smyth", 0.92, "close"),
        compare_field("Given names", "John W.", "John W.", 1.00, "match"),
        compare_field("Birth date", "1980-01-15", "1980-01-15", 1.00, "match"),
        compare_field("Gender", "Male", "Male", 1.00, "match"),
        compare_field(
            "Address",
            "10 Downing St, London SW1A 2AA",
            "10 Downing St, London SW1A 2AA",
            1.00,
            "match",
        ),
        compare_field("Tax ID", "***-**-6789", "***-**-6789", 1.00, "match"),
        compare_field("Phone", "+44 20 7946 0958", "+44 20 7946 0999", 0.30, "differ"),
    ];

    let overall = serde_json::json!({
        "score": "0.94",
        "score_pct": 94,
        "quality": "probable",
        "detection_method": "name (Soundex) + DOB + tax_id"
    });

    ctx.insert("a", &a_record);
    ctx.insert("b", &b_record);
    ctx.insert("fields", &fields);
    ctx.insert("overall", &overall);
    ctx.insert(
        "review_id",
        &review_id.unwrap_or_else(|| "unknown".to_string()),
    );
    render(&state, "compare.html.tera", ctx).into_response()
}

fn compare_field(
    label: &str,
    a_value: &str,
    b_value: &str,
    score: f64,
    outcome: &str,
) -> serde_json::Value {
    serde_json::json!({
        "label": label,
        "a_value": a_value,
        "b_value": b_value,
        "score": format!("{:.2}", score),
        "score_pct": (score * 100.0).round() as u32,
        "outcome": outcome,
    })
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    #[serde(default)]
    pub q: String,
}

#[derive(Debug, Deserialize)]
pub struct SearchPageQuery {
    #[serde(default)]
    pub q: Option<String>,
    #[serde(default)]
    pub fuzzy: Option<String>,
    #[serde(default)]
    pub phonetic: Option<String>,
    #[serde(default)]
    pub mask_sensitive: Option<String>,
    #[serde(default = "default_page")]
    pub page: u32,
}

/// `GET /{entity_plural}/search` — full-page search.
///
/// Scaffold: when `q` is present, seeds three relevance-ranked results
/// so the `search-input`, filter `field`s (`fuzzy`/`phonetic`/`mask_sensitive`
/// checkboxes), `data-table`, `meter` relevance bars, `badge` status,
/// `tag-group` of active filters, and `pagination-nav` are visible
/// end-to-end. Wire through to `crate::search::SearchEngine::search()`
/// once the web tier is plumbed into `AppState`.
pub async fn search_page(
    State(state): State<WebState>,
    Query(SearchPageQuery {
        q,
        fuzzy,
        phonetic,
        mask_sensitive,
        page,
    }): Query<SearchPageQuery>,
) -> Response {
    let mut ctx = state.context();
    let query = q.unwrap_or_default();
    let fuzzy_on = fuzzy.as_deref() == Some("true");
    let phonetic_on = phonetic.as_deref() == Some("true");
    let mask_on = mask_sensitive.as_deref() == Some("true");

    let mut active_filters: Vec<String> = Vec::new();
    if fuzzy_on {
        active_filters.push("Fuzzy".to_string());
    }
    if phonetic_on {
        active_filters.push("Phonetic".to_string());
    }
    if mask_on {
        active_filters.push("Mask sensitive".to_string());
    }

    let results: Vec<serde_json::Value> = if query.is_empty() {
        Vec::new()
    } else {
        let singular = state.app.entity_singular;
        vec![
            serde_json::json!({
                "id": "00000000-0000-0000-0000-000000000001",
                "label": format!("Sample {singular} 1"),
                "subtitle": format!("Strong match for \"{query}\""),
                "active": true,
                "relevance_pct": 92,
            }),
            serde_json::json!({
                "id": "00000000-0000-0000-0000-000000000002",
                "label": format!("Sample {singular} 2"),
                "subtitle": format!("Partial match for \"{query}\""),
                "active": true,
                "relevance_pct": 71,
            }),
            serde_json::json!({
                "id": "00000000-0000-0000-0000-000000000003",
                "label": format!("Sample {singular} 3"),
                "subtitle": "Soft-deleted record",
                "active": false,
                "relevance_pct": 48,
            }),
        ]
    };
    let result_total = results.len() as u32;

    let total_pages: u32 = if result_total == 0 { 0 } else { 1 };
    let pagination = serde_json::json!({
        "page": page,
        "total_pages": total_pages,
        "has_prev": page > 1,
        "has_next": page < total_pages,
        "pages": (1..=total_pages.max(1)).collect::<Vec<u32>>(),
    });

    ctx.insert("query", &query);
    ctx.insert("fuzzy", &fuzzy_on);
    ctx.insert("phonetic", &phonetic_on);
    ctx.insert("mask_sensitive", &mask_on);
    ctx.insert("active_filters", &active_filters);
    ctx.insert("results", &results);
    ctx.insert("result_total", &result_total);
    ctx.insert("pagination", &pagination);
    render(&state, "search.html.tera", ctx).into_response()
}

/// `GET /{entity_plural}/search/partial` — HTMX partial.
///
/// Returns just the inner HTML for `#search-results`. The real
/// implementation should delegate to [`crate::search`] here; this
/// scaffold renders an empty result set so the wiring is verifiable
/// end-to-end without a search index.
pub async fn search_partial(
    State(state): State<WebState>,
    Query(SearchQuery { q }): Query<SearchQuery>,
) -> Response {
    let mut ctx = state.context();
    ctx.insert("query", &q);
    ctx.insert("results", &Vec::<serde_json::Value>::new());
    render(&state, "partials/search.html.tera", ctx).into_response()
}

fn render(state: &WebState, template: &str, ctx: tera::Context) -> Response {
    match state.render(template, &ctx) {
        Ok(html) => Html(html).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!(
                "<h1>Template error</h1><pre>{}</pre>",
                html_escape(&e.to_string())
            )),
        )
            .into_response(),
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// `GET /metrics.prom` — Prometheus text-exposition format for scraping.
///
/// Returns the contents of the process-wide registry (see
/// [`crate::metrics::METRICS`]) with content type
/// `text/plain; version=0.0.4; charset=utf-8`. Configure your scraper
/// with `metrics_path: /metrics.prom` — the canonical `/metrics`
/// endpoint serves the human-readable HTML dashboard.
pub async fn prometheus() -> Response {
    use axum::http::header::CONTENT_TYPE;
    let body = crate::metrics::METRICS.render();
    ([(CONTENT_TYPE, crate::metrics::CONTENT_TYPE)], body).into_response()
}
