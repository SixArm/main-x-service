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
    merge_object(&mut paths, tpc_paths());
    merge_object(&mut paths, control_paths());
    merge_object(&mut paths, phase_paths());
    merge_object(&mut paths, distribution_paths());
    merge_object(&mut paths, okr_paths());
    merge_object(&mut paths, workflow_paths());
    merge_object(&mut paths, effort_paths());
    merge_object(&mut paths, ceremony_paths());
    merge_object(&mut paths, value_paths());
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
                "summary": "Configure one rule: when a trigger fires, apply its actions in declared order (FR-32)",
                "description": "Triggers: task_moved (with optional from_status/to_status), review_submitted, plan_stage_changed, plan_phase_changed (with optional from_status/to_status naming project phases, not task statuses). Actions (1-20, applied in array order, each outcome logged separately): assign, add_label, notify, schedule_action, set_task_status. Every action's shape is validated here, at write time.",
                "requestBody": { "required": true, "content": { "application/json": {
                    "schema": { "$ref": "#/components/schemas/Automation" } } } },
                "responses": { "200": { "description": "The stored rule" },
                               "422": { "description": "Unknown trigger, empty/oversized actions array, or a malformed action in it" } } },
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
        "/api/automations/milestones/sweep": { "post": { "tags": ["automation"],
            "summary": "Fire every enabled milestone_due rule matching a milestone whose due date has arrived (FR-32; claim-based, so each rule/milestone pair fires exactly once, ever — not once per sweep)",
            "responses": { "200": { "description": "Counts: fired, already_claimed, capped" } } } },
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
                "Automation": { "type": "object", "required": ["name", "trigger_kind", "actions"], "properties": {
                    "plan_pid": { "type": "string", "format": "uuid", "nullable": true, "description": "Scope to one plan's board; absent = every plan" },
                    "name": { "type": "string" },
                    "trigger_kind": { "type": "string", "enum": ["task_moved", "review_submitted", "plan_stage_changed", "plan_phase_changed"] },
                    "from_status": { "type": "string", "nullable": true, "description": "task_moved (a task status) or plan_phase_changed (a project phase) only; absent = any" },
                    "to_status": { "type": "string", "nullable": true, "description": "task_moved (a task status) or plan_phase_changed (a project phase) only; absent = any" },
                    "actions": { "type": "array", "minItems": 1, "maxItems": 20, "description": "Applied in declared order; each action's outcome is logged separately (FR-32)", "items": { "type": "object", "required": ["kind"], "properties": {
                        "kind": { "type": "string", "enum": ["assign", "add_label", "notify", "schedule_action", "set_task_status"] },
                        "value": { "type": "object", "description": "Action-specific: assign {assignee_ref}, add_label {label}, notify {recipient_ref, message?}, schedule_action {action_kind, in_days, recipient_ref?}, set_task_status {status}" } } } } } },
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

/// The Total Project Control paths (entity spec §9.2c / FR-37).
fn tpc_paths() -> Value {
    let plan = json!({
        "name": "pid", "in": "path", "required": true,
        "schema": { "type": "string", "format": "uuid" }
    });
    json!({
        "/api/plans/{pid}/tpc": {
            "post": {
                "tags": ["total-project-control"],
                "summary": "Record a TPC observation (EMV, cost estimate to complete, DIPP)",
                "description": "Money in minor units, ratios in basis points — no float touches a currency figure. A negative expected monetary value is accepted deliberately: a project can be worth less than nothing to finish, and refusing to record that would hide the one case the metric exists to expose. A negative cost estimate to complete is refused.",
                "parameters": [plan],
                "responses": {
                    "200": { "description": "Recorded" },
                    "404": { "description": "Unknown plan" },
                    "422": { "description": "Bad currency, or a negative cost estimate to complete" }
                }
            },
            "get": {
                "tags": ["total-project-control"],
                "summary": "TPC observation history, newest first",
                "parameters": [plan],
                "responses": { "200": { "description": "Observations" }, "404": { "description": "Unknown plan" } }
            }
        },
        "/api/plans/{pid}/tpc/report": {
            "get": {
                "tags": ["total-project-control"],
                "summary": "DIPP, the progress index, and the stored-versus-computed divergence",
                "description": "DIPP = EMV / CEC asks whether the value still to come is worth the money still to spend; sunk cost appears nowhere in it. A cost estimate to complete of zero reports null with a reason, never infinity. A plan with no observation reports unmeasured, never a zero.",
                "parameters": [plan],
                "responses": { "200": { "description": "Derived report" }, "404": { "description": "Unknown plan" } }
            }
        },
        "/api/tpc": {
            "get": {
                "tags": ["total-project-control"],
                "summary": "Portfolio triage ranked by DIPP descending, within one currency",
                "description": "Plans recorded in another currency and plans whose DIPP is undefined are excluded and reported, rather than sorted last as though measured and bad. This service never converts currency.",
                "parameters": [
                    { "name": "currency", "in": "query", "schema": { "type": "string", "default": "GBP" } }
                ],
                "responses": { "200": { "description": "Ranked triage" } }
            }
        }
    })
}

/// The Controlling-process paths (entity spec §9.2c / FR-38, FR-39).
fn control_paths() -> Value {
    let plan = json!({
        "name": "pid", "in": "path", "required": true,
        "schema": { "type": "string", "format": "uuid" }
    });
    json!({
        "/api/plans/{pid}/controls": {
            "post": {
                "tags": ["controls"],
                "summary": "Register a control: standard, timing, source, cadence",
                "description": "The timing decides what a failing control may do: feedforward may block a write, concurrent may warn and escalate but never silently undo the operator's action, feedback may only record. A control naming a metric this service does not produce is refused here rather than left registered — a check nobody can evaluate is indistinguishable from one that passes.",
                "parameters": [plan],
                "responses": {
                    "200": { "description": "Registered, with the response its timing permits" },
                    "404": { "description": "Unknown plan" },
                    "422": { "description": "Unknown metric, bad timing or comparator, or a within-control with no tolerance" }
                }
            },
            "get": {
                "tags": ["controls"],
                "summary": "The control register for one plan",
                "parameters": [plan],
                "responses": { "200": { "description": "Controls" }, "404": { "description": "Unknown plan" } }
            }
        },
        "/api/controls/{pid}": {
            "delete": {
                "tags": ["controls"],
                "summary": "Withdraw a control",
                "description": "Soft-delete. The readings stay: they are the history of what was measured, and withdrawing a control does not unmeasure the past.",
                "parameters": [plan],
                "responses": { "204": { "description": "Withdrawn" }, "404": { "description": "Unknown control" } }
            }
        },
        "/api/controls/{pid}/readings": {
            "post": {
                "tags": ["controls"],
                "summary": "Measure and compare in one step",
                "description": "The verdict is derived at write from the standard in force, so a reading can never disagree with the standard it was taken against. A reading with no value is unmeasured — a third verdict, never a pass — and is excluded from pass rates rather than counted as either half.",
                "parameters": [plan],
                "responses": { "200": { "description": "Reading with verdict and gap" }, "404": { "description": "Unknown control" } }
            },
            "get": {
                "tags": ["controls"],
                "summary": "Readings for one control, newest first",
                "description": "Append-only: correcting a reading means recording another one, because a control history that can be rewritten measures whatever the editor wanted.",
                "parameters": [plan],
                "responses": { "200": { "description": "Readings" }, "404": { "description": "Unknown control" } }
            }
        },
        "/api/readings/{pid}/actions": {
            "post": {
                "tags": ["controls"],
                "summary": "Record what a failing reading provoked",
                "description": "The fourth step of the Controlling process. An action of kind accept also stamps the reading as explicitly accepted, which is what stops it being reported as unanswered.",
                "parameters": [plan],
                "responses": {
                    "200": { "description": "Action recorded" },
                    "404": { "description": "Unknown reading" },
                    "422": { "description": "Bad kind, or a missing description" }
                }
            }
        },
        "/api/plans/{pid}/controls/coverage": {
            "get": {
                "tags": ["controls"],
                "summary": "What is not being controlled on one plan",
                "description": "Controls that have never produced a reading, controls whose cadence has lapsed, unanswered failures, and the count per timing with every timing present even at zero — an empty cell is a finding, not a row to omit.",
                "parameters": [plan],
                "responses": { "200": { "description": "Coverage" }, "404": { "description": "Unknown plan" } }
            }
        },
        "/api/controls/coverage": {
            "get": {
                "tags": ["controls"],
                "summary": "Portfolio-wide control coverage, naming plans with no controls at all",
                "responses": { "200": { "description": "Coverage" } }
            }
        }
    })
}

/// The project-phase paths (entity spec §9.2b / FR-30).
fn phase_paths() -> Value {
    let plan = json!({
        "name": "pid", "in": "path", "required": true,
        "schema": { "type": "string", "format": "uuid" }
    });
    json!({
        "/api/plans/{pid}/phase": {
            "put": {
                "tags": ["phase"],
                "summary": "Advance one step, or move back with a stated reason",
                "description": "Advancement is one step at a time and a skip is refused, naming the phase that would have been jumped. A backward move is permitted but must carry a reason: re-planning is normal, an unexplained regression is not. The phase never gates an operational write — tasks may be created in initiating and issues raised in closing, because refusing writes on that basis teaches operators to misreport the phase. An unrecognised token is refused, never coerced to a default.",
                "parameters": [plan],
                "responses": {
                    "200": { "description": "New phase, and the next one available" },
                    "404": { "description": "Unknown plan" },
                    "422": { "description": "A skipped phase, a silent regression, no change, or an unknown token" }
                }
            }
        },
        "/api/plans/{pid}/phase-history": {
            "get": {
                "tags": ["phase"],
                "summary": "Transitions and time spent in each phase",
                "description": "The log is append-only — there is no edit or delete route, because a phase history that can be rewritten cannot support a duration claim. Every phase is reported even at zero, and a phase entered more than once reports its visit count: two visits of 50 and one of 100 are different stories that the total alone would hide.",
                "parameters": [plan],
                "responses": { "200": { "description": "History and durations" }, "404": { "description": "Unknown plan" } }
            }
        }
    })
}

/// The Flow Distribution path (entity spec §9.2b / FR-31).
fn distribution_paths() -> Value {
    json!({
        "/api/plans/{pid}/flow-distribution": {
            "get": {
                "tags": ["flow-distribution"],
                "summary": "The feature / defect / risk / debt mix of completed work",
                "description": "The fifth Flow Framework metric, and the only one not already computed under time-based-analysis vocabulary — Flow Time, Velocity, Efficiency and Load are served by /api/plans/{pid}/time-analysis and /flow, and are deliberately not republished here under Flow-Framework names. Work with no declared type is reported as unclassified and counted separately, never folded into feature, because absorbing it would flatter the share a reader is most likely to act on. An intended mix is reported against only when a deployment declares one: absent that, the mix is reported without judgement, since an unlabelled target is how a measurement becomes a quota.",
                "parameters": [
                    { "name": "pid", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } },
                    { "name": "window_days", "in": "query", "schema": { "type": "integer", "default": 90, "minimum": 1, "maximum": 3650 } },
                    { "name": "rollup", "in": "query", "schema": { "type": "boolean", "default": false } },
                    { "name": "depth", "in": "query", "schema": { "type": "integer" } }
                ],
                "responses": { "200": { "description": "The mix" }, "404": { "description": "Unknown plan" } }
            }
        }
    })
}

/// The OKR-engine paths (entity spec §9.2b / FR-27).
fn okr_paths() -> Value {
    let id = |name: &str| {
        json!({ "name": name, "in": "path", "required": true,
                "schema": { "type": "string", "format": "uuid" } })
    };
    json!({
        "/api/objectives/{pid}/key-results": {
            "post": {
                "tags": ["okr"],
                "summary": "Declare a key result on an objective",
                "description": "Key results hang off an objective, which has a pid, a period and weighted plan alignment — not off a plan's goals[], whose entries carry no identifier and would be orphaned by any reordering. current_value starts at the baseline, so a key result running from 100 defects to 0 reads 0% on the day it is created rather than complete. A maintain key result needs a tolerance band, a currency-valued one must name its currency, and one whose start equals its target is refused: it has no distance to travel and could never report progress.",
                "parameters": [id("pid")],
                "responses": {
                    "200": { "description": "Declared" },
                    "404": { "description": "Unknown objective" },
                    "422": { "description": "Unmeasurable: no range, no tolerance, or no currency" }
                }
            },
            "get": {
                "tags": ["okr"],
                "summary": "Key results with derived progress, and the reason where absent",
                "parameters": [id("pid")],
                "responses": { "200": { "description": "Key results" }, "404": { "description": "Unknown objective" } }
            }
        },
        "/api/key-results/{pid}/check-ins": {
            "post": {
                "tags": ["okr"],
                "summary": "Record an observation",
                "description": "Advances current_value and never start_value: progress measured from a moving baseline is not progress, so there is no path here that touches it. Confidence is recorded and never blended into any score — a self-report and a measurement are different kinds of evidence, and averaging them would make the measured half unfalsifiable.",
                "parameters": [id("pid")],
                "responses": {
                    "200": { "description": "Recorded, with recomputed progress" },
                    "404": { "description": "Unknown key result" },
                    "422": { "description": "Confidence out of range" }
                }
            },
            "get": {
                "tags": ["okr"],
                "summary": "Check-ins, newest first",
                "parameters": [id("pid")],
                "responses": { "200": { "description": "Check-ins" }, "404": { "description": "Unknown key result" } }
            }
        },
        "/api/plans/{pid}/okr": {
            "get": {
                "tags": ["okr"],
                "summary": "The plan's alignment-weighted OKR score",
                "description": "Weighted by the existing objective_links weight rather than a second notion of importance. An objective with no measurable key result reports unmeasured and is excluded from both halves of the mean — it must neither drag the plan down nor silently lift it — and is reported rather than hidden. Every score is derived on read, so recording a check-in corrects every figure resting on it.",
                "parameters": [id("pid")],
                "responses": { "200": { "description": "Score and the objectives behind it" }, "404": { "description": "Unknown plan" } }
            }
        }
    })
}

/// The custom-workflow paths (entity spec §9.2b / FR-26).
fn workflow_paths() -> Value {
    let plan = json!({
        "name": "pid", "in": "path", "required": true,
        "schema": { "type": "string", "format": "uuid" }
    });
    json!({
        "/api/workflows": {
            "post": {
                "tags": ["workflow"],
                "summary": "Register a state vocabulary for a plan's tasks or issues",
                "description": "Every state must declare a category (todo, active, waiting, done): the board, the burndown and every flow figure are computed from what a state means, not its name, so a state without one is refused by name here and by the schema too. A plan-scoped workflow is never the deployment default — default means the fallback when a plan has none.",
                "responses": {
                    "200": { "description": "Registered" },
                    "404": { "description": "plan_pid names no live plan" },
                    "422": { "description": "applies_to not task or issue, a blank or over-long name, a state without a recognised category, or a workflow that fails validation" }
                }
            },
            "get": {
                "tags": ["workflow"],
                "summary": "Every registered workflow",
                "responses": { "200": { "description": "Workflows" } }
            }
        },
        "/api/workflows/{pid}": {
            "delete": {
                "tags": ["workflow"],
                "summary": "Withdraw a workflow",
                "description": "Refused while any live task sits in a state only this workflow declares: withdrawing it would leave that work in a state no vocabulary explains — exactly the uncategorised-state problem the category column exists to prevent.",
                "parameters": [plan],
                "responses": {
                    "200": { "description": "Withdrawn (soft-delete)" },
                    "404": { "description": "Unknown workflow" },
                    "422": { "description": "Live work sits in a state only this workflow declares" }
                }
            }
        },
        "/api/plans/{pid}/workflow": {
            "get": {
                "tags": ["workflow"],
                "summary": "The vocabulary actually in force, and where it came from",
                "description": "Resolution order: the plan's own workflow if it has one, else the deployment default, else the built-in vocabulary — never none, which is what keeps every existing board working with nothing configured. The source is named because 'why can I not move this card' is answered by which vocabulary is in force, not by the vocabulary alone.",
                "parameters": [
                    plan,
                    { "name": "applies_to", "in": "query",
                      "schema": { "type": "string", "enum": ["task", "issue"], "default": "task" } }
                ],
                "responses": {
                    "200": { "description": "Derived view (ETag-conditional)" },
                    "304": { "description": "Not modified" },
                    "404": { "description": "Unknown plan" },
                    "422": { "description": "applies_to not task or issue" }
                }
            }
        }
    })
}

/// The recorded-effort and utilisation paths (entity spec §9.2c /
/// FR-28, FR-35; `agents/share/time-based-analysis.md` §7.1).
fn effort_paths() -> Value {
    let plan = json!({
        "name": "pid", "in": "path", "required": true,
        "schema": { "type": "string", "format": "uuid" }
    });
    json!({
        "/api/plans/{pid}/time-entries": {
            "post": {
                "tags": ["effort"],
                "summary": "Record effort against a plan (and optionally one task)",
                "description": "actor_ref is a person: or worker: URN, never a raw name. Minutes are capped at 1440 — a single day cannot hold more, so anything above is a typo or a fabrication, and either way it is not effort.",
                "parameters": [plan],
                "responses": {
                    "200": { "description": "Recorded" },
                    "404": { "description": "Unknown plan" },
                    "422": { "description": "actor_ref not a person:/worker: URN, or minutes outside 1..=1440" }
                }
            },
            "get": {
                "tags": ["effort"],
                "summary": "The plan's time entries, newest first (cap 5000)",
                "parameters": [plan],
                "responses": { "200": { "description": "Entries" }, "404": { "description": "Unknown plan" } }
            }
        },
        "/api/plans/{pid}/effort": {
            "get": {
                "tags": ["effort"],
                "summary": "Effort roll-ups per plan, per task and per assignee — every figure labelled asserted",
                "description": "Recorded effort is what people said they spent, not an observation, so the roll-up says asserted rather than presenting it as measurement. The per-assignee table returns in stable actor order and is deliberately not sorted by size — a size-ranked effort table is a league table.",
                "parameters": [plan],
                "responses": {
                    "200": { "description": "Derived view (ETag-conditional; carries as_of)" },
                    "304": { "description": "Not modified" },
                    "404": { "description": "Unknown plan" }
                }
            }
        },
        "/api/working-time": {
            "post": {
                "tags": ["effort"],
                "summary": "Declare the capacity basis: minutes per day and working days per week",
                "description": "The denominator every utilisation figure divides by. Declared, not inferred — a capacity basis reverse-engineered from effort entries would make 100% true by construction.",
                "responses": {
                    "200": { "description": "Declared" },
                    "422": { "description": "minutes_per_day outside 1..=1440, or working_days_per_week outside 1..=7" }
                }
            }
        },
        "/api/non-working": {
            "post": {
                "tags": ["effort"],
                "summary": "Record leave, holiday, study leave or non-project duty",
                "description": "This is what stops somebody on leave reporting 0% utilisation, which would read as measured idleness. The period's minutes are deducted from the person's denominator, and a person entirely absent still appears in the utilisation view — precisely so the answer is 'on leave' rather than a silent absence.",
                "responses": {
                    "200": { "description": "Recorded" },
                    "422": { "description": "kind not leave, holiday, study_leave or non_project_duty, or ends_on precedes starts_on" }
                }
            }
        },
        "/api/capacity/utilization": {
            "get": {
                "tags": ["effort"],
                "summary": "Recorded effort against declared capacity, by plan, team or person",
                "description": "Per-person utilisation is served under the five stated obligations (time-based-analysis §7.1): each figure ships beside its numerator, its denominator and the non-working deduction that produced the denominator; the per-person view returns in stable actor order, never sorted by utilisation; a person under the suppression floor is withheld rather than reported on noise; and utilisation near or above 100% is a warning about the queue, not an achievement. No per-person cycle time, throughput or flow efficiency is served here or derivable from what it returns — that refusal was narrowed, not repealed.",
                "parameters": [
                    { "name": "by", "in": "query",
                      "schema": { "type": "string", "enum": ["plan", "team", "person"], "default": "team" } },
                    { "name": "window_days", "in": "query",
                      "schema": { "type": "integer", "default": 28, "minimum": 1, "maximum": 366 } },
                    { "name": "plan_pid", "in": "query",
                      "schema": { "type": "string", "format": "uuid" } }
                ],
                "responses": {
                    "200": { "description": "Derived view (ETag-conditional)" },
                    "304": { "description": "Not modified" },
                    "422": { "description": "by is not plan, team or person" }
                }
            }
        }
    })
}

/// The sprint-ceremony and commitment paths (entity spec §9.2b /
/// FR-29).
fn ceremony_paths() -> Value {
    let sprint = json!({
        "name": "pid", "in": "path", "required": true,
        "schema": { "type": "string", "format": "uuid" }
    });
    json!({
        "/api/sprints/{pid}/ceremonies": {
            "post": {
                "tags": ["ceremony"],
                "summary": "Hold a ceremony: planning, daily, review or retrospective",
                "description": "A second planning or review on the same sprint is refused — that is a re-plan, which is a new sprint. Dailies and retrospectives may repeat.",
                "parameters": [sprint],
                "responses": {
                    "200": { "description": "Held" },
                    "404": { "description": "Unknown sprint" },
                    "422": { "description": "Unknown kind, or a second planning/review on the same sprint" }
                }
            },
            "get": {
                "tags": ["ceremony"],
                "summary": "The sprint's ceremonies, with per-kind counts and the retrospective notes",
                "description": "Every kind is reported even at zero — a sprint that never held a retrospective is a finding, not a missing row.",
                "parameters": [sprint],
                "responses": { "200": { "description": "Ceremonies" }, "404": { "description": "Unknown sprint" } }
            }
        },
        "/api/sprints/{pid}/commit": {
            "post": {
                "tags": ["ceremony"],
                "summary": "Snapshot the committed task set — once",
                "description": "Refuses a second call: a rewritable commitment is not a commitment, and a sprint that grew by half would otherwise look like one that was always that size.",
                "parameters": [sprint],
                "responses": {
                    "200": { "description": "The count committed" },
                    "404": { "description": "Unknown sprint" },
                    "422": { "description": "Already committed" }
                }
            }
        },
        "/api/sprints/{pid}/commitment": {
            "get": {
                "tags": ["ceremony"],
                "summary": "The committed set beside the current one, so a scope change reads as a change",
                "description": "Tasks added or removed after commitment are named, not just counted — 'what was added' is the question. Sprint velocity and burndown are sprint-scoped and count-based; the Flow Framework metrics are item-scoped and time-based. Neither is derived from the other.",
                "parameters": [sprint],
                "responses": {
                    "200": { "description": "Derived view (ETag-conditional)" },
                    "304": { "description": "Not modified" },
                    "404": { "description": "Unknown sprint" }
                }
            }
        }
    })
}

/// The realized-gains and strategic-performance paths (entity spec
/// §9.2c / FR-33, FR-34, FR-36).
fn value_paths() -> Value {
    let plan = json!({
        "name": "pid", "in": "path", "required": true,
        "schema": { "type": "string", "format": "uuid" }
    });
    json!({
        "/api/plans/{pid}/business-case": {
            "post": {
                "tags": ["value"],
                "summary": "Record a promised business-case target",
                "description": "approved_at is stamped here and has no update path: it is the Time-to-Value clock start, and a clock start that can move is not a measurement.",
                "parameters": [plan],
                "responses": {
                    "200": { "description": "Recorded" },
                    "404": { "description": "Unknown plan" },
                    "422": { "description": "source not charter or gate_review, or a blank metric" }
                }
            }
        },
        "/api/plans/{pid}/value-points": {
            "post": {
                "tags": ["value"],
                "summary": "Record observed value",
                "description": "A second first-measurable point is refused: the Time-to-Value clock stops once.",
                "parameters": [plan],
                "responses": {
                    "200": { "description": "Recorded" },
                    "404": { "description": "Unknown plan" },
                    "422": { "description": "A second first-measurable value point" }
                }
            }
        },
        "/api/plans/{pid}/adoption": {
            "post": {
                "tags": ["value"],
                "summary": "Record an adoption snapshot",
                "description": "A zero or negative target is refused here rather than divided at read, and the definition of 'active user' is required and stored beside the rate — it is the term most easily redefined between two readings.",
                "parameters": [plan],
                "responses": {
                    "200": { "description": "Recorded" },
                    "404": { "description": "Unknown plan" },
                    "422": { "description": "Non-positive target_users or window_days, negative active_users, or a blank definition" }
                }
            }
        },
        "/api/plans/{pid}/satisfaction": {
            "post": {
                "tags": ["value"],
                "summary": "Record a satisfaction response (NPS or CSAT)",
                "parameters": [plan],
                "responses": {
                    "200": { "description": "Recorded" },
                    "404": { "description": "Unknown plan" },
                    "422": { "description": "instrument not nps or csat, score outside 0..=10, or an unknown respondent_role" }
                }
            }
        },
        "/api/plans/{pid}/value-realization": {
            "get": {
                "tags": ["value"],
                "summary": "The realized-gains view: transformation ROI, Time-to-Value, adoption, performance to business case",
                "description": "Every figure is derived on read, so recording a value point corrects every number resting on it. Investment is actual cost, not planned — ROI on money not yet spent is a forecast, and this view reports what happened. A mixed-currency plan has no single investment figure, so the ROI is withheld with mixed_currency rather than silently adding pounds to euros. Absent evidence reports null with a reason and sorts last, never 0: a plan with no value points has not failed to deliver — it has not been measured, and those are different findings.",
                "parameters": [plan],
                "responses": {
                    "200": { "description": "Derived view (ETag-conditional; carries as_of)" },
                    "304": { "description": "Not modified" },
                    "404": { "description": "Unknown plan" }
                }
            }
        },
        "/api/plans/{pid}/performance": {
            "get": {
                "tags": ["value"],
                "summary": "The strategic-performance view: stakeholder NPS, schedule and financial indices",
                "description": "SPI and CPI need a phased budget baseline this service does not yet hold, so they are reported as unmeasured with the reason — rather than omitted (which would look like nothing to say) or defaulted to 1.0 (which would say 'exactly on plan').",
                "parameters": [plan],
                "responses": {
                    "200": { "description": "Derived view (ETag-conditional; carries as_of)" },
                    "304": { "description": "Not modified" },
                    "404": { "description": "Unknown plan" }
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
            "/api/automations/milestones/sweep",
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

    /// Every Total Project Control and Controlling-process route is
    /// documented, and the two properties a reader must not have to
    /// guess at are stated rather than implied: that DIPP excludes sunk
    /// cost, and that the timing decides what a failing control may do.
    #[test]
    fn spec_documents_the_tpc_and_control_surface() {
        let s = spec();
        let paths = &s["paths"];
        for (path, method) in [
            ("/api/plans/{pid}/tpc", "post"),
            ("/api/plans/{pid}/tpc", "get"),
            ("/api/plans/{pid}/tpc/report", "get"),
            ("/api/tpc", "get"),
            ("/api/plans/{pid}/controls", "post"),
            ("/api/plans/{pid}/controls", "get"),
            ("/api/controls/{pid}", "delete"),
            ("/api/controls/{pid}/readings", "post"),
            ("/api/controls/{pid}/readings", "get"),
            ("/api/readings/{pid}/actions", "post"),
            ("/api/plans/{pid}/controls/coverage", "get"),
            ("/api/controls/coverage", "get"),
        ] {
            assert!(
                paths[path][method].is_object(),
                "{method} {path} is undocumented"
            );
        }

        let report = paths["/api/plans/{pid}/tpc/report"]["get"]["description"]
            .as_str()
            .unwrap_or_default();
        assert!(
            report.contains("sunk cost"),
            "DIPP's whole point is that sunk cost appears nowhere; say so"
        );
        assert!(
            report.contains("never infinity"),
            "a zero cost estimate to complete must document its null, not leave a sentinel implied"
        );

        let register = paths["/api/plans/{pid}/controls"]["post"]["description"]
            .as_str()
            .unwrap_or_default();
        assert!(
            register.contains("feedforward") && register.contains("feedback"),
            "the timing decides what a failing control may do: state it"
        );

        let reading = paths["/api/controls/{pid}/readings"]["get"]["description"]
            .as_str()
            .unwrap_or_default();
        assert!(
            reading.contains("Append-only"),
            "the append-only property is what makes a control history trustworthy"
        );
    }

    /// The phase surface is documented, and the two properties a reader
    /// must not have to guess are stated: that the log is append-only,
    /// and that the phase does not gate operational writes.
    #[test]
    fn spec_documents_the_phase_surface() {
        let s = spec();
        let paths = &s["paths"];
        assert!(paths["/api/plans/{pid}/phase"]["put"].is_object());
        assert!(paths["/api/plans/{pid}/phase-history"]["get"].is_object());

        let set = paths["/api/plans/{pid}/phase"]["put"]["description"]
            .as_str()
            .unwrap_or_default();
        assert!(
            set.contains("never gates an operational write"),
            "a reader must not have to discover that phase does not block work"
        );
        assert!(
            set.contains("must carry a reason"),
            "the regression rule is the non-obvious half of the contract"
        );

        let history = paths["/api/plans/{pid}/phase-history"]["get"]["description"]
            .as_str()
            .unwrap_or_default();
        assert!(
            history.contains("append-only"),
            "the append-only property is what makes a duration claim trustworthy"
        );
    }

    /// Flow Distribution is documented, and says both of the things a
    /// reader would otherwise have to discover: that unclassified work
    /// is counted separately, and that the other four Flow Framework
    /// metrics are not republished here under new names.
    #[test]
    fn spec_documents_flow_distribution() {
        let s = spec();
        let d = s["paths"]["/api/plans/{pid}/flow-distribution"]["get"]["description"]
            .as_str()
            .unwrap_or_default();
        assert!(!d.is_empty(), "flow-distribution is undocumented");
        assert!(
            d.contains("never folded into feature"),
            "the unclassified rule is the one that keeps the mix honest"
        );
        assert!(
            d.contains("not republished here"),
            "say that the other four metrics live elsewhere, or they get built twice"
        );
    }

    /// The OKR surface is documented, and states the two rules a reader
    /// would otherwise have to discover: the baseline never moves, and
    /// an unmeasured objective is excluded rather than scored zero.
    #[test]
    fn spec_documents_the_okr_surface() {
        let s = spec();
        let paths = &s["paths"];
        for (path, method) in [
            ("/api/objectives/{pid}/key-results", "post"),
            ("/api/objectives/{pid}/key-results", "get"),
            ("/api/key-results/{pid}/check-ins", "post"),
            ("/api/key-results/{pid}/check-ins", "get"),
            ("/api/plans/{pid}/okr", "get"),
        ] {
            assert!(
                paths[path][method].is_object(),
                "{method} {path} is undocumented"
            );
        }

        let check_in = paths["/api/key-results/{pid}/check-ins"]["post"]["description"]
            .as_str()
            .unwrap_or_default();
        assert!(
            check_in.contains("never start_value"),
            "the immutable baseline is the rule most worth stating"
        );
        assert!(
            check_in.contains("never blended"),
            "confidence not entering the score must be documented, not assumed"
        );

        let plan = paths["/api/plans/{pid}/okr"]["get"]["description"]
            .as_str()
            .unwrap_or_default();
        assert!(
            plan.contains("excluded from both halves"),
            "an unmeasured objective is neither a drag nor a lift; say so"
        );
    }

    /// The workflow, effort/utilisation, ceremony and value surfaces are
    /// documented, and each states the rule a reader would otherwise
    /// have to discover: every state declares a category, per-person
    /// flow figures are refused, the commitment is written once, and
    /// absent evidence is null with a reason.
    #[test]
    fn spec_documents_the_workflow_effort_ceremony_value_surfaces() {
        let s = spec();
        let paths = &s["paths"];
        for (path, method) in [
            ("/api/workflows", "post"),
            ("/api/workflows", "get"),
            ("/api/workflows/{pid}", "delete"),
            ("/api/plans/{pid}/workflow", "get"),
            ("/api/plans/{pid}/time-entries", "post"),
            ("/api/plans/{pid}/time-entries", "get"),
            ("/api/plans/{pid}/effort", "get"),
            ("/api/working-time", "post"),
            ("/api/non-working", "post"),
            ("/api/capacity/utilization", "get"),
            ("/api/sprints/{pid}/ceremonies", "post"),
            ("/api/sprints/{pid}/ceremonies", "get"),
            ("/api/sprints/{pid}/commit", "post"),
            ("/api/sprints/{pid}/commitment", "get"),
            ("/api/plans/{pid}/business-case", "post"),
            ("/api/plans/{pid}/value-points", "post"),
            ("/api/plans/{pid}/adoption", "post"),
            ("/api/plans/{pid}/satisfaction", "post"),
            ("/api/plans/{pid}/value-realization", "get"),
            ("/api/plans/{pid}/performance", "get"),
        ] {
            assert!(
                paths[path][method].is_object(),
                "{method} {path} is undocumented"
            );
        }

        let register = paths["/api/workflows"]["post"]["description"]
            .as_str()
            .unwrap_or_default();
        assert!(
            register.contains("Every state must declare a category"),
            "the category rule is what every flow figure rests on: state it"
        );

        let utilization = paths["/api/capacity/utilization"]["get"]["description"]
            .as_str()
            .unwrap_or_default();
        assert!(
            utilization.contains("No per-person cycle time, throughput or flow efficiency"),
            "the §7.1 refusal was narrowed, not repealed — the doc must say so"
        );
        assert!(
            utilization.contains("never sorted by utilisation"),
            "obligation 4: utilisation is never the sole ranking key"
        );

        let commit = paths["/api/sprints/{pid}/commit"]["post"]["description"]
            .as_str()
            .unwrap_or_default();
        assert!(
            commit.contains("rewritable commitment is not a commitment"),
            "the once-only rule is the whole point of the snapshot"
        );

        let realization = paths["/api/plans/{pid}/value-realization"]["get"]["description"]
            .as_str()
            .unwrap_or_default();
        assert!(
            realization.contains("mixed_currency"),
            "the ROI is withheld, never converted; a reader must not have to discover that"
        );
        assert!(
            realization.contains("has not been measured"),
            "no-evidence is a different finding from no-delivery; say so"
        );

        let performance = paths["/api/plans/{pid}/performance"]["get"]["description"]
            .as_str()
            .unwrap_or_default();
        assert!(
            performance.contains("no_baseline") || performance.contains("phased budget baseline"),
            "SPI/CPI absence must carry its reason, not default to on-plan"
        );
    }

    /// Mounted-but-undocumented paths: the pre-existing gap
    /// (governance / strategy / visibility, the compliance verifiers,
    /// the masked/export views, and the docs surface itself, which
    /// documents the API rather than its own pages).
    const KNOWN_UNDOCUMENTED: &[&str] = &[
        // docs surface
        "/api-docs/openapi.json",
        "/swagger-ui",
        // privacy views on the plan record
        "/api/plans/{pid}/masked",
        "/api/plans/{pid}/export",
        // integrity verification
        "/api/compliance/records/verify",
        "/api/compliance/audit/verify",
        // governance (PPM Phase A)
        "/api/proposals",
        "/api/proposals/{pid}",
        "/api/proposals/{pid}/submit",
        "/api/proposals/{pid}/review",
        "/api/proposals/{pid}/approve",
        "/api/proposals/{pid}/reject",
        "/api/proposals/{pid}/promote",
        "/api/proposals/{pid}/duplicates",
        "/api/plans/{pid}/gate-reviews",
        "/api/plans/{pid}/risks",
        "/api/plans/{pid}/risks/{risk_pid}",
        "/api/plans/{pid}/risks/{risk_pid}/escalate",
        "/api/plans/{pid}/budget-lines",
        "/api/plans/{pid}/budget-lines/{line_pid}/actual",
        "/api/plans/{pid}/budget-lines/{line_pid}/release",
        "/api/plans/{pid}/governance",
        // strategy (PPM Phase C)
        "/api/ideas",
        "/api/ideas/{pid}/vote",
        "/api/ideas/{pid}/dismiss",
        "/api/ideas/{pid}/convert",
        "/api/scenarios",
        "/api/scenarios/{pid}/evaluate",
        "/api/scenarios/{pid}/commit",
        "/api/objectives",
        "/api/objectives/{pid}/alignment",
        "/api/plans/{pid}/objectives",
        "/api/plans/{pid}/benefits",
        "/api/plans/{pid}/benefits/{b_pid}/realize",
        // visibility (PPM Phase B)
        "/api/dependencies",
        "/api/dependencies/{pid}",
        "/api/plans/{pid}/schedule",
        "/api/plans/{pid}/milestones",
        "/api/plans/{pid}/milestones/{m_pid}/complete",
        "/api/plans/{pid}/allocations",
        "/api/plans/{pid}/allocations/{a_pid}",
        "/api/capacity",
        "/api/reports",
        "/api/reports/{pid}",
        "/api/reports/{pid}/run",
        "/api/at-a-glance",
    ];

    /// Every route the controllers mount, as `(path, method)` pairs —
    /// the same `routes()` functions `app.rs` wires, so the list below
    /// is kept in step with `App::routes`. (Loco's default `/_health`
    /// and `/_ping` are added by `AppRoutes`, not a controller, and are
    /// out of scope here.)
    fn mounted_routes() -> std::collections::BTreeSet<(String, String)> {
        let groups = [
            crate::controllers::plans::routes(),
            crate::controllers::compliance::routes(),
            crate::controllers::governance::routes(),
            crate::controllers::visibility::routes(),
            crate::controllers::insights::routes(),
            crate::controllers::oversight::routes(),
            crate::controllers::tba::routes(),
            crate::controllers::tpc::routes(),
            crate::controllers::controls::routes(),
            crate::controllers::phase::routes(),
            crate::controllers::distribution::routes(),
            crate::controllers::workflow::routes(),
            crate::controllers::okr::routes(),
            crate::controllers::effort::routes(),
            crate::controllers::ceremony::routes(),
            crate::controllers::value::routes(),
            crate::controllers::engineering::routes(),
            crate::controllers::strategy::routes(),
            crate::controllers::collaboration::routes(),
            crate::controllers::automation::routes(),
            crate::controllers::prioritisation::routes(),
            crate::controllers::docs::routes(),
            crate::controllers::metrics::routes(),
        ];
        let mut mounted = std::collections::BTreeSet::new();
        for group in &groups {
            let prefix = group.prefix.as_deref().unwrap_or("");
            for handler in &group.handlers {
                let mut path = format!("{prefix}/{}", handler.uri.trim_start_matches('/'));
                while path.len() > 1 && path.ends_with('/') {
                    path.pop();
                }
                for action in &handler.actions {
                    mounted.insert((path.clone(), action.as_str().to_lowercase()));
                }
            }
        }
        mounted
    }

    /// Every documented `(path, method)` pair in the spec's `paths`
    /// object (non-operation keys such as `parameters` are skipped).
    fn documented_routes(s: &Value) -> std::collections::BTreeSet<(String, String)> {
        let mut documented = std::collections::BTreeSet::new();
        for (path, item) in s["paths"].as_object().expect("paths") {
            for method in item.as_object().expect("path item").keys() {
                if ["get", "post", "put", "patch", "delete"].contains(&method.as_str()) {
                    documented.insert((path.clone(), method.clone()));
                }
            }
        }
        documented
    }

    /// Two-way parity between the mounted router and this document.
    ///
    /// **Forward**: every route the controllers mount (enumerated by
    /// [`mounted_routes`]) appears in the document, unless its
    /// path sits in the explicit `KNOWN_UNDOCUMENTED` register.
    /// **Reverse**: every documented path + method is actually mounted,
    /// so the doc cannot describe a route that does not exist.
    ///
    /// The register is a debt list, not an allowance: an entry must be
    /// genuinely mounted (a stale entry fails) and must stay
    /// undocumented (documenting one without removing it fails), so it
    /// can only shrink as documentation lands. A brand-new route is
    /// therefore documented or explicitly registered — never silent.
    #[test]
    fn spec_and_mounted_routes_agree_both_ways() {
        use std::collections::BTreeSet;

        let mounted = mounted_routes();
        assert!(!mounted.is_empty(), "route enumeration returned nothing");
        let documented = documented_routes(&spec());

        let registered: BTreeSet<&str> = KNOWN_UNDOCUMENTED.iter().copied().collect();
        let mounted_paths: BTreeSet<&str> = mounted.iter().map(|(path, _)| path.as_str()).collect();
        let documented_paths: BTreeSet<&str> =
            documented.iter().map(|(path, _)| path.as_str()).collect();

        // Forward: mounted ⇒ documented or registered.
        let missing: Vec<String> = mounted
            .iter()
            .filter(|(path, _)| !registered.contains(path.as_str()))
            .filter(|entry| !documented.contains(*entry))
            .map(|(path, method)| format!("{method} {path}"))
            .collect();
        assert!(
            missing.is_empty(),
            "mounted but neither documented nor registered as known debt:\n{}",
            missing.join("\n")
        );

        // Reverse: documented ⇒ mounted (exact path + method).
        let phantom: Vec<String> = documented
            .iter()
            .filter(|entry| !mounted.contains(*entry))
            .map(|(path, method)| format!("{method} {path}"))
            .collect();
        assert!(
            phantom.is_empty(),
            "documented but not mounted (a doc typo, or a removed route):\n{}",
            phantom.join("\n")
        );

        // The register can only shrink: every entry is really mounted…
        let stale: Vec<&str> = registered
            .iter()
            .filter(|path| !mounted_paths.contains(**path))
            .copied()
            .collect();
        assert!(
            stale.is_empty(),
            "registered as undocumented but not mounted at all — remove:\n{}",
            stale.join("\n")
        );
        // …and still undocumented (documenting one must delete its entry).
        let done: Vec<&str> = registered
            .iter()
            .filter(|path| documented_paths.contains(**path))
            .copied()
            .collect();
        assert!(
            done.is_empty(),
            "registered as undocumented but now documented — remove from the register:\n{}",
            done.join("\n")
        );
    }
}
