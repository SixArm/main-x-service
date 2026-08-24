//! Hand-written `OpenAPI` 3 description of the portfolio REST API.
//!
//! The request/response `Plan` body is the `project_portfolio_management_matcher::Plan`
//! shape. That crate is intentionally dependency-light (no `utoipa`), so
//! the schema is authored here by hand rather than derived — which also
//! keeps the doc accurate to the wire format. All plans live in one
//! recursive collection under `/api/plans`; `kind` is an optional
//! descriptive label and any plan may contain any other via `parent_ref`.

use serde_json::{Value, json};

/// The full `OpenAPI` document, served at `/api-docs/openapi.json`.
#[must_use]
pub fn spec() -> Value {
    json!({
        "openapi": "3.0.3",
        "info": {
            "title": "Project Portfolio Management Service API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "Registry of plan identities in one recursive collection (any plan may contain any other via parent_ref; kind is an optional descriptive label): CRUD + matching. The request/response body is the project-portfolio-management-matcher Plan shape. Validation failures (blank name, blank goal title, malformed identifier, malformed parent_ref / in_language, or a containment cycle) return 422. Matching is not gated by kind."
        },
        "paths": paths(),
        "components": components(),
    })
}

/// The `paths` object, composed from the CRUD/matching paths and the
/// auxiliary (auth/audit/events/metrics) paths.
fn paths() -> Value {
    let mut paths = crud_paths();
    merge_object(&mut paths, aux_paths());
    merge_object(&mut paths, insight_paths());
    merge_object(&mut paths, capability_paths());
    merge_object(&mut paths, tba_paths());
    merge_object(&mut paths, tba_plan_paths());
    merge_object(&mut paths, tba_forecast_paths());
    merge_object(&mut paths, tba_rollup_paths());
    paths
}

/// The collaboration / automation / prioritisation capability paths:
/// collaborative review, assignee management, workflow automation, the
/// set-and-forget scheduler, the Smart Score, and bird's-eye lifecycle
/// visibility.
fn capability_paths() -> Value {
    let get = |tag: &str, summary: &str| {
        json!({ "get": { "tags": [tag], "summary": summary,
            "responses": { "200": { "description": "OK" } } } })
    };
    let derived = |tag: &str, summary: &str| {
        json!({ "get": { "tags": [tag], "summary": summary,
            "responses": { "200": { "description": "Derived view (ETag-conditional; carries as_of)" },
                           "304": { "description": "Not modified" } } } })
    };
    json!({
        // --- collaborative review --------------------------------------
        "/api/reviews": {
            "post": { "tags": ["collaboration"],
                "summary": "Delegate an idea / proposal / plan to one internal or external expert",
                "description": "reviewer_ref is an EntityRef URN (person:/worker:/organization:), never a raw email; reviewer_scope records the internal/external disclosure decision explicitly. A second live invitation for the same reviewer + subject is refused.",
                "requestBody": { "required": true, "content": { "application/json": {
                    "schema": { "$ref": "#/components/schemas/ReviewInvite" } } } },
                "responses": { "200": { "description": "The invitation" },
                               "422": { "description": "Validation failure / already invited / unknown subject" } } },
            "get": { "tags": ["collaboration"],
                "summary": "List review invitations (?subject_kind=&subject_pid=&reviewer=&status=; cap 200)",
                "responses": { "200": { "description": "Invitations, newest first" } } } },
        "/api/reviews/consensus": get("collaboration", "Aggregate verdict for one subject (?subject_kind=&subject_pid=): mean score, recommendation counts, strict majority (a tie reports none), outstanding invitations"),
        "/api/reviews/{pid}/respond": { "post": { "tags": ["collaboration"],
            "summary": "Reviewer accepts or declines the invitation",
            "responses": { "200": { "description": "The updated invitation" },
                           "422": { "description": "Illegal transition" } } } },
        "/api/reviews/{pid}/submit": { "post": { "tags": ["collaboration"],
            "summary": "Submit the verdict (score 0-100 optional, recommendation advance|hold|reject); only an accepted invitation may submit",
            "responses": { "200": { "description": "The submitted review" },
                           "422": { "description": "Illegal transition / validation failure" } } } },
        "/api/reviews/{pid}": { "delete": { "tags": ["collaboration"],
            "summary": "Withdraw an invitation; a submitted verdict cannot be withdrawn",
            "responses": { "204": { "description": "Withdrawn" },
                           "422": { "description": "Already final" } } } },
        // --- assignees --------------------------------------------------
        "/api/plans/{pid}/tasks/{t_pid}/assign": { "post": { "tags": ["collaboration"],
            "summary": "Assign or unassign one task (null assignee_ref unassigns); notifies the new assignee",
            "responses": { "200": { "description": "The updated task" },
                           "422": { "description": "assignee_ref is not a person:/worker: URN" } } } },
        "/api/assignees/workload": derived("collaboration", "Open work per assignee, busiest first, including the unassigned pile (?plan=)"),
        // --- notifications ----------------------------------------------
        "/api/notifications": get("collaboration", "One recipient's in-app inbox (?recipient=&unread=; cap 200). In-app only: this service sends no email or push"),
        "/api/notifications/{pid}/read": { "post": { "tags": ["collaboration"],
            "summary": "Mark one notification read (idempotent)",
            "responses": { "200": { "description": "The notification" } } } },
        // --- workflow automation ----------------------------------------
        "/api/automations": {
            "post": { "tags": ["automation"],
                "summary": "Configure one rule: when a trigger fires, apply one action",
                "description": "Triggers: task_moved (with optional from_status/to_status), review_submitted, plan_stage_changed. Actions: assign, add_label, notify, schedule_action, set_task_status. The action shape is validated here, at write time.",
                "requestBody": { "required": true, "content": { "application/json": {
                    "schema": { "$ref": "#/components/schemas/Automation" } } } },
                "responses": { "200": { "description": "The stored rule" },
                               "422": { "description": "Unknown trigger/action or malformed action_value" } } },
            "get": { "tags": ["automation"], "summary": "List rules (?plan=&trigger=&enabled=; cap 200)",
                "responses": { "200": { "description": "Rules" } } } },
        "/api/automations/runs": get("automation", "What the automations actually did (?automation=&subject=&outcome=applied|skipped|failed; cap 200)"),
        "/api/automations/{pid}/enable": { "post": { "tags": ["automation"], "summary": "Switch a rule on",
            "responses": { "200": { "description": "The rule" } } } },
        "/api/automations/{pid}/disable": { "post": { "tags": ["automation"], "summary": "Switch a rule off, keeping it and its run history",
            "responses": { "200": { "description": "The rule" } } } },
        "/api/automations/{pid}": { "delete": { "tags": ["automation"], "summary": "Soft-delete a rule",
            "responses": { "204": { "description": "Deleted" } } } },
        // --- set and forget ----------------------------------------------
        "/api/scheduled-actions": {
            "post": { "tags": ["automation"],
                "summary": "Configure one deadline (notify | expire_review) in_days ahead, then forget it",
                "responses": { "200": { "description": "The queued action" },
                               "422": { "description": "Unknown action_kind, out-of-range in_days, or a notify with no recipient" } } },
            "get": { "tags": ["automation"], "summary": "The deadline queue, soonest first (?status=&subject=&overdue=; cap 200)",
                "responses": { "200": { "description": "Scheduled actions" } } } },
        "/api/scheduled-actions/sweep": { "post": { "tags": ["automation"],
            "summary": "Fire every action now due (claim-based, so a deadline fires exactly once; capped per sweep)",
            "responses": { "200": { "description": "Counts: fired, skipped_already_claimed, capped" } } } },
        "/api/scheduled-actions/{pid}": { "delete": { "tags": ["automation"],
            "summary": "Cancel a pending deadline; an action that already fired cannot be cancelled",
            "responses": { "204": { "description": "Cancelled" },
                           "422": { "description": "Not pending" } } } },
        // --- data-driven prioritisation + bird's-eye visibility ----------
        "/api/plans/{pid}/smart-score": derived("prioritisation", "One plan's Smart Score with the full breakdown: per-component weight/raw/contribution, the components with no evidence, and the coverage those gaps leave. No evidence scores null, never zero"),
        "/api/prioritisation": derived("prioritisation", "Ranked queue, highest Smart Score first (?limit=&band=high|medium|low|unscored); unscored plans sort last rather than as zeros"),
        "/api/lifecycle": derived("prioritisation", "Bird's-eye challenge funnel: live and stalled counts for every lifecycle phase, plus any items in an unknown phase"),
        "/api/plans/{pid}/lifecycle": derived("prioritisation", "One plan's phase, its next gate, the readiness checklist with each blocker named, and its review consensus"),
    })
}

/// The executive-insight read paths (CEO / CFO / CTO derived views;
/// all ETag-conditional GETs with an `as_of` stamp).
fn insight_paths() -> Value {
    let get = |tag: &str, summary: &str| {
        json!({ "get": { "tags": [tag], "summary": summary,
            "responses": { "200": { "description": "Derived view (ETag-conditional; carries as_of)" },
                           "304": { "description": "Not modified" } } } })
    };
    json!({
        "/api/executive/health": get("executive", "CEO portfolio-health briefing: per-portfolio RAG rollup, overdue milestones, escalated risks, exposure, overrun currencies, staleness"),
        "/api/executive/decisions": get("executive", "Decision log: gate reviews, scenario commits, decided proposals, merges (newest first; ?limit=)"),
        "/api/executive/benefits": get("executive", "Benefits realization per portfolio: per-currency target vs realized (ratio only with a positive target) + non-financial status counts"),
        "/api/financials/variance": get("financials", "Budget variance (minor units, per currency, never merged) by collection, portfolio, and category"),
        "/api/financials/exposure": get("financials", "Per-currency estate exposure: planned / actual / remaining totals; deliberately no FX conversion"),
        "/api/technology/dependency-risk": get("technology", "Dependency lens: top fan-out items, cross-portfolio edges, edges with a RAG-red predecessor"),
        "/api/technology/radar": get("technology", "Technology radar from tech:<name>[:<ring>] tags; majority ring vote, ties break cautious"),
        "/api/executive/alignment": get("executive", "Strategic-alignment coverage: per-collection aligned/unaligned counts, unaligned spend per currency, ranked unaligned items"),
        "/api/technology/debt": get("technology", "Technical-debt register: risks categorised tech_debt, exposure-sorted, with status counts"),
        "/api/technology/flow": get("technology", "Delivery-flow metrics: milestone throughput per month + median lead days (?months=, cap 24)"),
        "/api/scenarios/compare": get("executive", "Side-by-side scenario comparison (?a=&b=): live evaluations + per-currency planned deltas, exposure/alignment deltas"),
        "/api/board/pack": get("oversight", "Board pack (?from=&to=): window decisions, benefits realized, milestones completed, tranches released + as-of-now health"),
        "/api/board/investments": get("oversight", "Money-moving decisions: scenario commits, tranche releases, approved proposals (newest first, cap 100)"),
        "/api/board/trends": get("oversight", "Stored estate-snapshot series (oldest first; no interpolated history)"),
        "/api/auditor/trail": get("oversight", "Audit explorer (?actor=&action=&entity=&from=&to=&limit=): filtered rows + integrity stats"),
        "/api/auditor/findings": get("oversight", "Segregation-of-duties + hygiene findings over recorded audit actors"),
        "/api/auditor/evidence-pack": get("oversight", "Period evidence bundle (?from=&to=&format=json|csv): audit rows + decisions"),
        "/api/compliance/register": get("oversight", "Compliance risk register (category=compliance, exposure-sorted)"),
        "/api/compliance/findings": get("oversight", "Conformance findings (?review_days=): overdue-unreviewed items, ownerless escalations, overdue reviews, approver-less approvals"),
        "/api/risk/heatmap": get("oversight", "CRO heatmap: probability x impact cells, top risks, posture, concentration, hygiene, declared appetite + breaches"),
        "/api/security/register": get("oversight", "Security risk register + the no-security-risk-at-late-stage heuristic"),
        "/api/regulator/extract": get("oversight", "Deliberately coarse per-portfolio aggregates; ABAC mask obligation withholds names"),
        "/api/plans/{pid}/tasks": {
            "parameters": [{ "name": "pid", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }],
            "get": { "tags": ["engineering"], "summary": "The item's live tasks + per-status board counts", "responses": { "200": { "description": "Tasks + counts" } } },
            "post": { "tags": ["engineering"], "summary": "Create a task (default status todo; done on create stamps done_at)",
                "responses": { "200": { "description": "The task" }, "422": { "description": "Validation failure" } } }
        },
        "/api/plans/{pid}/tasks/{t_pid}": {
            "parameters": [
                { "name": "pid", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } },
                { "name": "t_pid", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }],
            "put": { "tags": ["engineering"], "summary": "Update task fields (status changes go through PATCH)", "responses": { "200": { "description": "The task" }, "422": { "description": "Validation failure" } } },
            "patch": { "tags": ["engineering"], "summary": "Board move: {status}; stamps status_changed_at, first done stamps done_at", "responses": { "200": { "description": "The task" }, "422": { "description": "Unknown status" } } },
            "delete": { "tags": ["engineering"], "summary": "Soft-delete the task", "responses": { "200": { "description": "Deleted" } } }
        },
        "/api/plans/{pid}/sprints": {
            "parameters": [{ "name": "pid", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }],
            "get": { "tags": ["engineering"], "summary": "The item's sprints", "responses": { "200": { "description": "Sprints" } } },
            "post": { "tags": ["engineering"], "summary": "Create a time-boxed sprint", "responses": { "200": { "description": "The sprint" }, "422": { "description": "ends_on before starts_on" } } }
        },
        "/api/plans/{pid}/burndown": get("engineering", "Honest sprint burndown (?sprint=): remaining per day from real done_at stamps; no ideal line"),
        "/api/plans/{pid}/standup": get("engineering", "Last-24h digest: tasks created/moved, current blockers (audit-derived)"),
        "/api/plans/{pid}/velocity": get("engineering", "Per-sprint completed counts + story-point sums (points are team-local; item-own trend only)"),
        "/api/plans/{pid}/sprints/{s_pid}/notes": {
            "get": { "tags": ["engineering"], "summary": "The sprint's retro/feedback notes", "responses": { "200": { "description": "Notes" } } },
            "post": { "tags": ["engineering"], "summary": "Add a note (went_well/improve/action/feedback)", "responses": { "200": { "description": "The note" }, "422": { "description": "Unknown category / blank body" } } }
        },
        "/api/plans/{pid}/sprints/{s_pid}/notes/{n_pid}/convert": { "post": { "tags": ["engineering"],
            "summary": "Convert an action/feedback note into a task (once)",
            "responses": { "200": { "description": "The note + created task" }, "422": { "description": "Not convertible / already converted" } } } },
        "/api/devops/events": { "post": { "tags": ["devops"],
            "summary": "Ingest a deploy/incident/recovery event (deploy needs environment; recovery references its incident)",
            "responses": { "200": { "description": "The stored event" }, "422": { "description": "Validation failure" } } } },
        "/api/devops/metrics": get("devops", "DORA-style metrics from ingested events only (?months=): deploys, incidents, MTTR over linked pairs, declared-cause change failure"),
        "/api/devops/releases": get("devops", "Release register: ingested deploy events, newest first (cap 200)"),
        "/api/engineering/blocked": get("engineering", "Estate blocked-work aging (days since entering blocked)"),
        "/api/engineering/moscow": get("engineering", "MoSCoW scope cut from moscow:<band> tags (untagged counted, never guessed)"),
        "/api/engineering/delivery-links": get("engineering", "External delivery-tracker identifiers per item + the untracked list"),
        "/api/engineering/milestone-calendar": get("engineering", "Estate milestone calendar (?kind=milestone|demo|release|checkpoint)"),
        "/api/board/snapshots": { "post": { "tags": ["oversight"],
            "summary": "Capture one estate snapshot now (portfolio counts, open exposure, per-currency money)",
            "responses": { "200": { "description": "The stored snapshot row" } } } },
    })
}

/// Shallow-merge the top-level keys of `src` into `dst`.
fn merge_object(dst: &mut Value, src: Value) {
    if let (Some(dst), Value::Object(src)) = (dst.as_object_mut(), src) {
        for (k, v) in src {
            dst.insert(k, v);
        }
    }
}

/// The CRUD + matching + merge paths over the single `/api/plans`
/// collection.
fn crud_paths() -> Value {
    json!({
            "/api/plans": {
                "get": {
                    "tags": ["plans"],
                    "summary": "List active plans (cap 100; ?parent= rolls up a plan children)",
                    "responses": { "200": { "description": "List of references",
                        "content": { "application/json": { "schema": { "type": "array", "items": { "$ref": "#/components/schemas/PlanRef" } } } } } }
                },
                "post": {
                    "tags": ["plans"],
                    "summary": "Create a plan",
                    "requestBody": { "required": true, "content": { "application/json": { "schema": { "$ref": "#/components/schemas/Plan" } } } },
                    "responses": {
                        "200": { "description": "Created", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/PlanRef" } } } },
                        "422": { "description": "Validation failure: blank name, a containment cycle, or a malformed goal/identifier/parent_ref/in_language" }
                    }
                }
            },
            "/api/plans/search": {
                "get": {
                    "tags": ["plans"],
                    "summary": "Case-insensitive name search (Postgres ILIKE, cap 50)",
                    "parameters": [{ "name": "q", "in": "query", "required": true, "schema": { "type": "string" } }],
                    "responses": {
                        "200": { "description": "Matches", "content": { "application/json": { "schema": { "type": "array", "items": { "$ref": "#/components/schemas/PlanRef" } } } } },
                        "400": { "description": "Missing or blank `q`" }
                    }
                }
            },
            "/api/plans/match": {
                "post": {
                    "tags": ["matching"],
                    "summary": "Rank a query against an explicit candidate list (no persistence; kind does not gate matching)",
                    "requestBody": { "required": true, "content": { "application/json": { "schema": { "$ref": "#/components/schemas/MatchRequest" } } } },
                    "responses": { "200": { "description": "Ranked results (index + MatchResult)" } }
                }
            },
            "/api/plans/check-duplicates": {
                "post": {
                    "tags": ["matching"],
                    "summary": "Match a query against stored plans",
                    "requestBody": { "required": true, "content": { "application/json": { "schema": { "$ref": "#/components/schemas/Plan" } } } },
                    "responses": { "200": { "description": "Scored matches",
                        "content": { "application/json": { "schema": { "type": "array", "items": { "$ref": "#/components/schemas/ScoredRef" } } } } } }
                }
            },
            "/api/plans/merge": {
                "post": {
                    "tags": ["matching"],
                    "summary": "Merge a confirmed duplicate into a surviving plan",
                    "requestBody": { "required": true, "content": { "application/json": { "schema": { "$ref": "#/components/schemas/MergeRequest" } } } },
                    "responses": {
                        "200": { "description": "The survivor's merged payload + the merged pids" },
                        "404": { "description": "main_pid or duplicate_pid not found" },
                        "422": { "description": "main_pid and duplicate_pid are equal" }
                    }
                }
            },
            "/api/plans/merges/recent": {
                "get": { "tags": ["matching"], "summary": "Recent merge-history records", "responses": { "200": { "description": "Merge records" } } }
            }
    })
}

/// The auth / audit / events / single-record / metrics paths.
fn aux_paths() -> Value {
    json!({
            "/api/plans/whoami": {
                "get": {
                    "tags": ["auth"],
                    "summary": "Echo the verified claims of the bearer token",
                    "security": [{ "bearer": [] }],
                    "responses": {
                        "200": { "description": "Verified token claims" },
                        "401": { "description": "Missing or invalid bearer token" }
                    }
                }
            },
            "/api/plans/audit/recent": {
                "get": { "tags": ["audit"], "summary": "Recent audit-log entries", "responses": { "200": { "description": "Audit entries" } } }
            },
            "/api/plans/events/recent": {
                "get": { "tags": ["audit"], "summary": "Recent events from the in-memory stream", "responses": { "200": { "description": "Events" } } }
            },
            "/api/plans/{pid}": {
                "parameters": [{ "name": "pid", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }],
                "get": { "tags": ["plans"], "summary": "Fetch the stored plan",
                    "responses": { "200": { "description": "Plan", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/Plan" } } } }, "404": { "description": "Not found" } } },
                "put": { "tags": ["plans"], "summary": "Replace a plan's payload",
                    "requestBody": { "required": true, "content": { "application/json": { "schema": { "$ref": "#/components/schemas/Plan" } } } },
                    "responses": { "200": { "description": "Updated", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/PlanRef" } } } }, "404": { "description": "Not found" }, "422": { "description": "Validation failure" } } },
                "delete": { "tags": ["plans"], "summary": "Soft-delete a plan", "responses": { "200": { "description": "Deleted" } } }
            },
            "/api/plans/{pid}/audit": {
                "parameters": [{ "name": "pid", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }],
                "get": { "tags": ["audit"], "summary": "Audit trail for one plan", "responses": { "200": { "description": "Audit entries" } } }
            },
            "/metrics.prom": {
                "get": {
                    "tags": ["observability"],
                    "summary": "Prometheus metrics (text-exposition format)",
                    "description": "Process-wide metric registry rendered as text/plain; version=0.0.4. Mounted at the root (not under /api) and public even under blanket auth enforcement. Configure your scraper with metrics_path: /metrics.prom.",
                    "responses": { "200": { "description": "Prometheus text exposition",
                        "content": { "text/plain": { "schema": { "type": "string" } } } } }
                }
            }
    })
}

/// The `components` object of the `OpenAPI` document.
fn components() -> Value {
    json!({
            "securitySchemes": {
                "bearer": { "type": "http", "scheme": "bearer", "bearerFormat": "PASETO",
                    "description": "Short-lived PASETO v4.public token from the authentication-service, verified offline against its published Ed25519 key." }
            },
            "schemas": {
                "PlanRef": { "type": "object", "required": ["pid", "name"], "properties": {
                    "pid": { "type": "string", "format": "uuid" }, "name": { "type": "string" } } },
                "ScoredRef": { "type": "object", "properties": {
                    "pid": { "type": "string" }, "name": { "type": "string" },
                    "score": { "type": "number", "format": "double" }, "confidence": { "type": "string" },
                    "is_match": { "type": "boolean" } } },
                "MatchRequest": { "type": "object", "required": ["query", "candidates"], "properties": {
                    "query": { "$ref": "#/components/schemas/Plan" },
                    "candidates": { "type": "array", "items": { "$ref": "#/components/schemas/Plan" } } } },
                "ReviewInvite": { "type": "object", "required": ["subject_kind", "subject_pid", "reviewer_ref"], "properties": {
                    "subject_kind": { "type": "string", "enum": ["idea", "proposal", "plan"] },
                    "subject_pid": { "type": "string", "format": "uuid" },
                    "reviewer_ref": { "type": "string", "description": "EntityRef URN: person:<uuid> / worker:<uuid> / organization:<uuid>" },
                    "reviewer_scope": { "type": "string", "enum": ["internal", "external"], "default": "internal" },
                    "expertise": { "type": "string", "nullable": true, "description": "Why this expert: the specialism the delegation is for" },
                    "due_on": { "type": "string", "format": "date", "nullable": true } } },
                "Automation": { "type": "object", "required": ["name", "trigger_kind", "action_kind"], "properties": {
                    "plan_pid": { "type": "string", "format": "uuid", "nullable": true, "description": "Scope to one plan's board; absent = every plan" },
                    "name": { "type": "string" },
                    "trigger_kind": { "type": "string", "enum": ["task_moved", "review_submitted", "plan_stage_changed"] },
                    "from_status": { "type": "string", "nullable": true, "description": "task_moved only; absent = any column" },
                    "to_status": { "type": "string", "nullable": true, "description": "task_moved only; absent = any column" },
                    "action_kind": { "type": "string", "enum": ["assign", "add_label", "notify", "schedule_action", "set_task_status"] },
                    "action_value": { "type": "object", "description": "Action-specific: assign {assignee_ref}, add_label {label}, notify {recipient_ref, message?}, schedule_action {action_kind, in_days, recipient_ref?}, set_task_status {status}" } } },
                "MergeRequest": { "type": "object", "required": ["main_pid", "duplicate_pid"], "properties": {
                    "main_pid": { "type": "string", "format": "uuid" },
                    "duplicate_pid": { "type": "string", "format": "uuid" },
                    "reason": { "type": "string", "nullable": true } } },
                "PlanIdentifier": { "type": "object", "required": ["scheme", "value"], "properties": {
                    "scheme": { "description": "Uri | Uuid | JiraProjectKey | AsanaGid | TrelloBoardId | MsProjectId | GitHubProjectId | LinearId | Code | LocalId | {Custom: string}" },
                    "value": { "type": "string", "description": "Must be non-blank." } } },
                "Plan": { "type": "object", "required": ["name"], "properties": {
                    "kind": { "type": "string", "enum": ["Portfolio", "Project", "Product", "Program", "Practice", "Process", "Purpose", "Pathway", "Proposal"], "nullable": true, "description": "Optional descriptive label; does not gate matching or fix a collection" },
                    "name": { "type": "string" },
                    "alternate_names": { "type": "array", "items": { "type": "string" } },
                    "code": { "type": "string", "nullable": true, "description": "Owner-scoped code, e.g. PROJ-2026" },
                    "owner_org_id": { "type": "string", "nullable": true, "description": "EntityRef organization:<id>" },
                    "owner_org_name": { "type": "string", "nullable": true },
                    "lead_ref": { "type": "string", "nullable": true, "description": "EntityRef person:<id> | worker:<id>" },
                    "parent_ref": { "type": "string", "nullable": true, "description": "Parent plan pid (the containment link; any plan may contain any other; UUID)" },
                    "status": { "type": "string", "nullable": true, "description": "Proposed | Active | OnHold | Completed | Cancelled | {Custom: string}" },
                    "goals": { "type": "array", "items": { "type": "object", "properties": { "title": { "type": "string" }, "description": { "type": "string", "nullable": true }, "target_date": { "type": "string", "nullable": true }, "status": { "type": "string", "nullable": true } } } },
                    "start_date": { "type": "string", "nullable": true, "description": "ISO-8601 YYYY / YYYY-MM / YYYY-MM-DD" },
                    "target_date": { "type": "string", "nullable": true },
                    "keywords": { "type": "array", "items": { "type": "string" } },
                    "tags": { "type": "array", "items": { "type": "string" } },
                    "identifiers": { "type": "array", "items": { "$ref": "#/components/schemas/PlanIdentifier" } },
                    "same_as": { "type": "array", "items": { "type": "string" } },
                    "in_language": { "type": "string", "nullable": true, "description": "BCP-47 language tag" },
                    "relationships": { "type": "array", "items": { "type": "object", "properties": { "relation": { "type": "string" }, "plan_id": { "type": "string" } } } } } }
            }
    })
}

/// The time-based-analysis paths (`spec/time-based-analysis.md` §10).
///
/// Read-only by design. Transitions are written by the existing
/// `POST /api/plans/{pid}/tasks` and `PATCH .../{t_pid}` calls, so the
/// measurement is a by-product of moving the card rather than another
/// thing to keep up to date — and there is deliberately no edit or
/// delete, because an editable flow log measures whatever the editor
/// wanted.
fn tba_paths() -> Value {
    let plan = json!({
        "name": "pid", "in": "path", "required": true,
        "schema": { "type": "string", "format": "uuid" }
    });
    let task = json!({
        "name": "t_pid", "in": "path", "required": true,
        "schema": { "type": "string", "format": "uuid" }
    });
    json!({
        "/api/flow-classes": {
            "get": {
                "tags": ["time-based-analysis"],
                "summary": "The status → VSM category map in force, and the vocabularies behind it",
                "description": "`todo` is inventory waste rather than merely not-started: work bought and not yet used, aging while it waits. An unclassified status falls back to unnecessary_non_value_adding, so adding a board column cannot silently improve the flow efficiency. Override with PROJECT_PORTFOLIO_MANAGEMENT_FLOW_CLASSES; an unparsable or unknown-category override falls back whole rather than half-applying.",
                "responses": { "200": { "description": "The classification in force" } }
            }
        },
        "/api/plans/{pid}/tasks/{t_pid}/transitions": {
            "get": {
                "tags": ["time-based-analysis"],
                "summary": "The append-only status-transition log for one task",
                "description": "Read-only: there is no edit or delete route. Correcting history means moving the card, which is itself recorded. `backfilled` marks a transition synthesised by the migration rather than observed.",
                "parameters": [plan, task],
                "responses": { "200": { "description": "Transitions in time order" },
                               "404": { "description": "Unknown plan or task" } }
            }
        },
        "/api/plans/{pid}/tasks/{t_pid}/time-analysis": {
            "get": {
                "tags": ["time-based-analysis"],
                "summary": "Per-task time-based analysis",
                "description": "Cycle time is what the team controls (first started → finished); lead time is what the requester waits (created → finished). They are different numbers and the difference is the backlog dwell — quoting the first as delivery time is the commonest misreport in flow measurement, so both are always returned. flow_efficiency is work over cycle time; by_status and by_category partition the lead time, so no time is lost.",
                "parameters": [plan, task],
                "responses": { "200": { "description": "Analysis" },
                               "404": { "description": "Unknown plan or task" } }
            }
        },
    })
}

/// The **plan-level** time-based-analysis paths
/// (`spec/time-based-analysis.md` §10.2).
fn tba_plan_paths() -> Value {
    let plan = json!({
        "name": "pid", "in": "path", "required": true,
        "schema": { "type": "string", "format": "uuid" }
    });
    let sprint = json!({
        "name": "sprint", "in": "query",
        "schema": { "type": "string", "format": "uuid" }
    });
    let plan_for_cfd = json!({
        "name": "pid", "in": "path", "required": true,
        "schema": { "type": "string", "format": "uuid" }
    });
    json!({
        "/api/plans/{pid}/time-analysis": {
            "get": {
                "tags": ["time-based-analysis"],
                "summary": "Plan-level flow: cycle/lead distributions, flow efficiency, first-pass yield, and the service level expectation",
                "description": "Percentiles are nearest-rank, so every one is an observed item. The SLE — 'p% of items finish within N days' — is derived from the plan's own history and is refused below a minimum sample rather than computed from noise. Throughput is reported beside rolled_first_pass_yield: throughput rising while yield falls is not going faster, it is shipping work back to yourself.",
                "parameters": [
                    plan,
                    { "name": "sle_percentile", "in": "query", "schema": { "type": "number", "default": 0.85, "minimum": 0, "maximum": 1 } },
                    { "name": "target_days", "in": "query", "schema": { "type": "number" }, "description": "Score an existing commitment instead of, or as well as, the derived expectation" },
                    sprint
                ],
                "responses": {
                    "200": { "description": "Plan analysis" },
                    "404": { "description": "Unknown plan" },
                    "422": { "description": "sle_percentile outside [0,1], a non-positive target_days, or a malformed sprint" }
                }
            }
        },
        "/api/plans/{pid}/constraints": {
            "get": {
                "tags": ["time-based-analysis"],
                "summary": "Ranked constraints: where the plan's time goes",
                "description": "Disclosed-rule findings ordered by recoverable time. Deliberately not a composite score, and deliberately never per-person — flow is a property of the system, and measuring individuals on card movement destroys the data it depends on.",
                "parameters": [plan, sprint],
                "responses": { "200": { "description": "Findings" },
                               "404": { "description": "Unknown plan" } }
            }
        },
        "/api/plans/{pid}/aging-wip": {
            "get": {
                "tags": ["time-based-analysis"],
                "summary": "Open items ranked by age against the plan's service level expectation",
                "description": "The only view here about work that can still be helped: cycle time, throughput and WIP are all history, while an item's age is a fact about today. Items with no expectation to compare against are listed with a null ratio rather than dropped.",
                "parameters": [
                    plan,
                    { "name": "sle_percentile", "in": "query", "schema": { "type": "number", "default": 0.85 } },
                    sprint
                ],
                "responses": { "200": { "description": "Aging work in progress" },
                               "404": { "description": "Unknown plan" } }
            }
        },
        "/api/plans/{pid}/cumulative-flow": {
            "get": {
                "tags": ["time-based-analysis"],
                "summary": "The board's composition sampled daily — the cumulative flow diagram",
                "description": "Every status band is present at every sample, including at zero, so a stacked chart never has to decide whether a missing band means zero. A task does not appear before it was created, and one whose history predates its first recorded transition reads as `todo` rather than vanishing and reappearing mid-chart. The vertical gap between the total and the done band is work in progress, and its width is approximately the cycle time — Little's Law read straight off the chart. Served here rather than assembled in the browser because it needs every task's whole history at once.",
                "parameters": [
                    plan_for_cfd,
                    { "name": "days", "in": "query", "schema": { "type": "integer", "default": 60, "minimum": 1, "maximum": 365 } }
                ],
                "responses": {
                    "200": { "description": "Daily samples" },
                    "404": { "description": "Unknown plan" },
                    "422": { "description": "days out of range" }
                }
            }
        },
        "/api/plans/{pid}/flow": {
            "get": {
                "tags": ["time-based-analysis"],
                "summary": "Queueing-theory flow: arrival rate, throughput, utilisation, WIP, and Little's Law",
                "description": "Little's Law κ = λτ used as a consistency check on observed figures, not as a forecast — it assumes arrivals and departures balance over the window. Column occupancy against the configured WIP limits is reported alongside, because lowering a cap shortens cycle time without anyone working faster.",
                "parameters": [
                    plan,
                    { "name": "window_days", "in": "query", "schema": { "type": "integer", "default": 90, "minimum": 1, "maximum": 3650 } }
                ],
                "responses": {
                    "200": { "description": "Flow analysis" },
                    "404": { "description": "Unknown plan" },
                    "422": { "description": "Window out of range" }
                }
            }
        }
    })
}

/// The cross-plan rollup path (`spec/time-based-analysis.md` §15
/// TBA-9).
fn tba_rollup_paths() -> Value {
    let plan_for_rollup = json!({
        "name": "pid", "in": "path", "required": true,
        "schema": { "type": "string", "format": "uuid" }
    });
    json!({
        "/api/plans/{pid}/rollup": {
            "get": {
                "tags": ["time-based-analysis"],
                "summary": "Flow across a plan and everything it contains",
                "description": "The combined figures are the **union of every task under this plan**, not an average of the children's ratios — averaging would weight a five-task plan equally with a five-hundred-task one. The per-plan table is returned alongside and, for a portfolio, is usually the more useful half: a rollup mixes boards whose teams mean different things by `in_progress`, so which child differs is a firmer finding than the combined number. The walk is bounded by depth and node caps and reports `truncated` when one fires; `revisits` is non-zero when containment is not a tree, which the write path should have refused.",
                "parameters": [
                    plan_for_rollup,
                    { "name": "depth", "in": "query", "schema": { "type": "integer", "default": 32, "minimum": 1, "maximum": 32 } }
                ],
                "responses": {
                    "200": { "description": "Combined figures, the walked tree, and the per-plan comparison" },
                    "404": { "description": "Unknown plan" },
                    "422": { "description": "depth out of range" }
                }
            }
        }
    })
}

/// The Monte-Carlo forecasting path (`spec/time-based-analysis.md` §15
/// TBA-11), its own function because the description carries the two
/// things a reader gets wrong: the input is throughput, not cycle time,
/// and the conservative percentile flips between its two answers.
fn tba_forecast_paths() -> Value {
    let plan_for_forecast = json!({
        "name": "pid", "in": "path", "required": true,
        "schema": { "type": "string", "format": "uuid" }
    });
    json!({
        "/api/plans/{pid}/forecast": {
            "get": {
                "tags": ["time-based-analysis"],
                "summary": "Monte-Carlo delivery forecast: how long for N items, and how many in N periods",
                "description": "Samples the plan's own **throughput** history — how many items it actually finished per period — not its cycle-time distribution. Cycle time answers a question about one item (that is the service level expectation); using it for a batch assumes items are worked one at a time, which for a team running several in parallel is pessimistic by roughly that factor. Both directions are returned together because quoting one without the other is how a forecast gets misread, and the percentile direction reverses between them: for `how long` the higher percentile is conservative, for `how many` it is the lower one (`at_least_items` is the 15th percentile, not the 85th). Sampling is with replacement and the seed is fixed unless supplied, so the same question gives the same answer — a forecast that moves on every reload is not one anybody will act on.",
                "parameters": [
                    plan_for_forecast,
                    { "name": "items", "in": "query", "schema": { "type": "integer" }, "description": "Batch size; defaults to the plan's open items" },
                    { "name": "periods", "in": "query", "schema": { "type": "integer", "default": 4 }, "description": "Horizon for the how-many forecast" },
                    { "name": "history_periods", "in": "query", "schema": { "type": "integer", "default": 12, "minimum": 1, "maximum": 260 } },
                    { "name": "period_days", "in": "query", "schema": { "type": "integer", "default": 7, "minimum": 1, "maximum": 90 } },
                    { "name": "trials", "in": "query", "schema": { "type": "integer", "default": 10_000, "maximum": 100_000 } },
                    { "name": "seed", "in": "query", "schema": { "type": "integer", "format": "int64" } }
                ],
                "responses": {
                    "200": { "description": "Both forecasts, each carrying a `reason` instead of a figure when the history is too thin to forecast from" },
                    "404": { "description": "Unknown plan" },
                    "422": { "description": "A parameter outside its range" }
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pins the document's basic shape and the core schemas.
    #[test]
    fn spec_is_wellformed() {
        let s = spec();
        assert_eq!(s["openapi"], "3.0.3");
        assert!(s["paths"]["/api/plans"]["post"].is_object());
        assert!(s["paths"]["/api/plans/check-duplicates"]["post"].is_object());
        assert!(s["components"]["schemas"]["Plan"]["properties"]["name"].is_object());
        assert!(s["components"]["schemas"]["Plan"]["properties"]["kind"].is_object());
        assert!(s["components"]["schemas"]["PlanIdentifier"]["properties"]["value"].is_object());
    }

    /// Pins the time-based-analysis surface, including the two
    /// statements the API makes about itself that a client needs: that
    /// cycle and lead time are different numbers, and that the log is
    /// append-only.
    #[test]
    fn spec_documents_the_time_based_analysis_surface() {
        let s = spec();
        let paths = &s["paths"];
        for path in [
            "/api/flow-classes",
            "/api/plans/{pid}/tasks/{t_pid}/transitions",
            "/api/plans/{pid}/tasks/{t_pid}/time-analysis",
            "/api/plans/{pid}/time-analysis",
            "/api/plans/{pid}/constraints",
            "/api/plans/{pid}/aging-wip",
            "/api/plans/{pid}/flow",
            "/api/plans/{pid}/cumulative-flow",
            "/api/plans/{pid}/forecast",
            "/api/plans/{pid}/rollup",
        ] {
            assert!(paths[path]["get"].is_object(), "{path} is undocumented");
        }
        let task_analysis =
            paths["/api/plans/{pid}/tasks/{t_pid}/time-analysis"]["get"]["description"]
                .as_str()
                .unwrap_or_default();
        assert!(
            task_analysis.contains("different numbers"),
            "the cycle-versus-lead distinction must be stated, not assumed"
        );
        let log = paths["/api/plans/{pid}/tasks/{t_pid}/transitions"]["get"]["description"]
            .as_str()
            .unwrap_or_default();
        assert!(
            log.contains("no edit or delete"),
            "the append-only property must be documented: it is what makes the \
             figures trustworthy"
        );
        // Every path here is a GET; a write route would mean the log had
        // gained an editing surface.
        for (path, item) in paths.as_object().expect("paths") {
            if path.contains("time-analysis") || path.contains("transitions") {
                assert!(
                    item.as_object().expect("item").keys().all(|m| m == "get"),
                    "{path} must be read-only"
                );
            }
        }
    }

    /// The forecast endpoint must document the two things a reader
    /// gets wrong: that it samples throughput rather than cycle time,
    /// and that the conservative percentile flips between its two
    /// answers.
    #[test]
    fn the_forecast_documents_its_input_and_its_percentile_direction() {
        let s = spec();
        let description = s["paths"]["/api/plans/{pid}/forecast"]["get"]["description"]
            .as_str()
            .unwrap_or_default();
        assert!(
            description.contains("throughput"),
            "the input must be named: {description}"
        );
        assert!(
            description.contains("15th percentile"),
            "the reversed direction must be stated, not left to the reader"
        );
    }

    /// Pins that the seven core CRUD + matching operations are documented.
    #[test]
    fn spec_documents_core_endpoints() {
        let s = spec();
        let paths = &s["paths"];
        assert!(paths["/api/plans"]["get"].is_object());
        assert!(paths["/api/plans"]["post"].is_object());
        assert!(paths["/api/plans/match"]["post"].is_object());
        assert!(paths["/api/plans/check-duplicates"]["post"].is_object());
        assert!(paths["/api/plans/{pid}"]["get"].is_object());
        assert!(paths["/api/plans/{pid}"]["put"].is_object());
        assert!(paths["/api/plans/{pid}"]["delete"].is_object());
    }

    /// Pins that the audit + event-stream endpoints are documented.
    #[test]
    fn spec_documents_audit_and_event_endpoints() {
        let s = spec();
        let paths = &s["paths"];
        assert!(paths["/api/plans/audit/recent"]["get"].is_object());
        assert!(paths["/api/plans/events/recent"]["get"].is_object());
        assert!(paths["/api/plans/{pid}/audit"]["get"].is_object());
    }

    /// Pins that the name-search endpoint is documented with its `q` param.
    #[test]
    fn spec_documents_search_endpoint() {
        let s = spec();
        let op = &s["paths"]["/api/plans/search"]["get"];
        assert!(op.is_object());
        assert_eq!(op["parameters"][0]["name"], "q");
    }

    /// Pins the merge endpoints + `MergeRequest` schema.
    #[test]
    fn spec_documents_merge_endpoints() {
        let s = spec();
        assert!(s["paths"]["/api/plans/merge"]["post"].is_object());
        assert!(s["paths"]["/api/plans/merges/recent"]["get"].is_object());
        assert!(s["components"]["schemas"]["MergeRequest"]["properties"]["main_pid"].is_object());
    }

    /// Pins the Prometheus `/metrics.prom` endpoint under the root path.
    #[test]
    fn spec_documents_metrics_endpoint() {
        let s = spec();
        let op = &s["paths"]["/metrics.prom"]["get"];
        assert!(op.is_object());
        assert!(op["responses"]["200"]["content"]["text/plain"].is_object());
    }

    /// Pins that the capability endpoints are documented.
    #[test]
    fn spec_documents_capability_endpoints() {
        let s = spec();
        let paths = &s["paths"];
        for path in [
            "/api/reviews",
            "/api/reviews/consensus",
            "/api/assignees/workload",
            "/api/notifications",
            "/api/automations",
            "/api/automations/runs",
            "/api/scheduled-actions",
            "/api/scheduled-actions/sweep",
            "/api/prioritisation",
            "/api/lifecycle",
            "/api/plans/{pid}/smart-score",
            "/api/plans/{pid}/lifecycle",
        ] {
            assert!(paths[path].is_object(), "{path} is undocumented");
        }
        assert!(paths["/api/reviews"]["post"].is_object());
        assert!(paths["/api/automations"]["post"].is_object());
        assert!(
            s["components"]["schemas"]["ReviewInvite"]["properties"]["reviewer_scope"].is_object()
        );
        assert!(s["components"]["schemas"]["Automation"]["properties"]["trigger_kind"].is_object());
    }

    /// Pins that `/whoami` carries a bearer security requirement.
    #[test]
    fn spec_documents_whoami_with_bearer_security() {
        let s = spec();
        assert!(s["paths"]["/api/plans/whoami"]["get"]["security"][0]["bearer"].is_array());
        assert_eq!(
            s["components"]["securitySchemes"]["bearer"]["scheme"],
            "bearer"
        );
    }
}
