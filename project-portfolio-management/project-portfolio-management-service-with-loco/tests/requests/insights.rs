//! Executive-insight request tests (spec §13 — CEO / CFO / CTO derived
//! views): the health briefing, decision log, benefits realization,
//! budget variance + exposure, dependency risk, and the technology
//! radar, exercised end-to-end over the live routes against a seeded
//! estate.
//!
//! `#[ignore]`d: needs PostgreSQL; run with `cargo test -- --ignored`.

use loco_rs::testing::prelude::*;
use project_portfolio_management_service::app::App;
use serde_json::{Value, json};
use serial_test::serial;

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
#[allow(clippy::too_many_lines)] // one seeded estate, seven views
async fn executive_financial_and_technology_views() {
    request::<App, _, _>(|request, _ctx| async move {
        // ── Seed: one portfolio, two children (one overrun + tagged).
        let portfolio: Value = request
            .post("/api/portfolios")
            .json(&json!({ "kind": "Portfolio", "name": "Transformation" }))
            .await
            .json();
        let portfolio_pid = portfolio["pid"].as_str().expect("pid").to_string();
        let project: Value = request
            .post("/api/projects")
            .json(&json!({
                "kind": "Project", "name": "Platform rebuild",
                "portfolio_ref": portfolio_pid,
                "tags": ["tech:rust:adopt", "tech:postgres", "owner:core"],
            }))
            .await
            .json();
        let project_pid = project["pid"].as_str().expect("pid").to_string();
        let product: Value = request
            .post("/api/products")
            .json(&json!({
                "kind": "Product", "name": "Portal",
                "portfolio_ref": portfolio_pid,
                "tags": ["tech:rust:trial"],
            }))
            .await
            .json();
        let product_pid = product["pid"].as_str().expect("pid").to_string();

        // Overrun budget on the project: planned 1_000, actual 1_500.
        let line: Value = request
            .post(&format!("/api/projects/{project_pid}/budget-lines"))
            .json(&json!({ "category": "capex", "description": "Build",
                            "currency": "GBP", "planned_minor": 1_000_i64 }))
            .await
            .json();
        let line_pid = line["pid"].as_str().expect("line pid");
        request
            .post(&format!("/api/projects/{project_pid}/budget-lines/{line_pid}/actual"))
            .json(&json!({ "amount_minor": 1_500_i64 }))
            .await
            .assert_status_ok();
        // A healthy USD line on the product keeps currencies apart.
        request
            .post(&format!("/api/products/{product_pid}/budget-lines"))
            .json(&json!({ "category": "opex", "description": "Run",
                            "currency": "USD", "planned_minor": 2_000_i64 }))
            .await
            .assert_status_ok();

        // A decided gate + a benefit with a target and a realization.
        request
            .post(&format!("/api/projects/{project_pid}/gate-reviews"))
            .json(&json!({ "gate": "g0_concept", "decision": "approved",
                            "approver_ref": "worker:11111111-1111-4111-8111-111111111111" }))
            .await
            .assert_status_ok();
        let benefit: Value = request
            .post(&format!("/api/products/{product_pid}/benefits"))
            .json(&json!({ "title": "Cost saving", "category": "cost_saving",
                            "currency": "GBP", "target_minor": 1_000_i64 }))
            .await
            .json();
        let benefit_pid = benefit["pid"].as_str().expect("benefit pid");
        request
            .post(&format!(
                "/api/products/{product_pid}/benefits/{benefit_pid}/realize"
            ))
            .json(&json!({ "amount_minor": 250_i64 }))
            .await
            .assert_status_ok();

        // A dependency: the product depends on the (overrun ⇒ red) project.
        request
            .post("/api/dependencies")
            .json(&json!({ "predecessor_pid": project_pid, "successor_pid": product_pid }))
            .await
            .assert_status_ok();

        // ── CEO: health — the portfolio bucket is red (overrun member),
        // with three members (portfolio + project + product).
        let health: Value = request.get("/api/executive/health").await.json();
        let bucket = health["portfolios"]
            .as_array()
            .expect("portfolios")
            .iter()
            .find(|b| b["portfolio"]["pid"] == json!(portfolio_pid))
            .expect("portfolio bucket");
        assert_eq!(bucket["status"], "red");
        assert_eq!(bucket["members"], 3);
        assert_eq!(bucket["overrun_currencies"][0], "GBP");
        assert!(health["as_of"].is_string());

        // ── CEO: decisions — the gate review is in the feed.
        let decisions: Value = request.get("/api/executive/decisions").await.json();
        let gate = decisions["decisions"]
            .as_array()
            .expect("decisions")
            .iter()
            .find(|d| d["kind"] == "gate_review")
            .expect("gate review entry");
        assert_eq!(gate["decision"], "approved");
        assert_eq!(gate["actor"], "worker:11111111-1111-4111-8111-111111111111");

        // ── CEO: benefits — GBP 250 / 1_000 ⇒ ratio 0.25.
        let benefits: Value = request.get("/api/executive/benefits").await.json();
        let brow = benefits["portfolios"]
            .as_array()
            .expect("rows")
            .iter()
            .find(|b| b["portfolio"]["pid"] == json!(portfolio_pid))
            .expect("bucket");
        let gbp = &brow["financial"][0];
        assert_eq!(gbp["currency"], "GBP");
        assert_eq!(gbp["target_minor"], 1_000);
        assert_eq!(gbp["realized_minor"], 250);
        assert!((gbp["realization_ratio"].as_f64().expect("ratio") - 0.25).abs() < 1e-9);

        // ── CFO: variance — GBP overrun, USD clean, never merged; the
        // capex category carries the overrun.
        let variance: Value = request.get("/api/financials/variance").await.json();
        let by_category = variance["by_category"].as_array().expect("categories");
        let capex = by_category.iter().find(|c| c["category"] == "capex").expect("capex");
        assert_eq!(capex["variance"][0]["currency"], "GBP");
        assert_eq!(capex["variance"][0]["overrun"], true);
        assert_eq!(capex["variance"][0]["remaining_minor"], -500);

        // ── CFO: exposure — one row per currency, no merged total.
        let exposure: Value = request.get("/api/financials/exposure").await.json();
        let currencies = exposure["currencies"].as_array().expect("currencies");
        assert_eq!(currencies.len(), 2);
        assert_eq!(currencies[0]["currency"], "GBP");
        assert_eq!(currencies[1]["currency"], "USD");
        assert!(exposure["note"].as_str().expect("note").contains("never converted"));

        // ── CTO: dependency risk — the red project tops fan-out and its
        // edge is a red-predecessor edge.
        let risk: Value = request.get("/api/technology/dependency-risk").await.json();
        assert_eq!(risk["top_fan_out"][0]["item"]["pid"], json!(project_pid));
        assert_eq!(risk["top_fan_out"][0]["rag"], "red");
        assert_eq!(
            risk["red_predecessor_edges"][0]["successor"]["pid"],
            json!(product_pid)
        );

        // ── CTO: radar — rust has two items and a majority... one adopt
        // + one trial vote ⇒ tie breaks toward the cautious `trial`;
        // postgres is unclassified; `owner:core` is ignored.
        let radar: Value = request.get("/api/technology/radar").await.json();
        let techs = radar["technologies"].as_array().expect("technologies");
        assert_eq!(techs.len(), 2);
        let postgres = techs.iter().find(|t| t["technology"] == "postgres").expect("pg");
        assert_eq!(postgres["ring"], "unclassified");
        let rust = techs.iter().find(|t| t["technology"] == "rust").expect("rust");
        assert_eq!(rust["ring"], "trial");
        assert_eq!(rust["per_collection"]["Project"], 1);
        assert_eq!(rust["per_collection"]["Product"], 1);

        // ── ETag round-trip: replaying with If-None-Match is a 304.
        let first = request.get("/api/financials/exposure").await;
        let etag = first
            .headers()
            .get("etag")
            .expect("etag header")
            .to_str()
            .expect("ascii")
            .to_string();
        let replay = request
            .get("/api/financials/exposure")
            .add_header("if-none-match", etag)
            .await;
        assert_eq!(replay.status_code(), 304);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
#[allow(clippy::too_many_lines)] // one seeded estate, five moderate-fit views
async fn moderate_fits_tranches_debt_flow_alignment_compare() {
    request::<App, _, _>(|request, _ctx| async move {
        let project: Value = request
            .post("/api/projects")
            .json(&json!({ "kind": "Project", "name": "Ledger rewrite" }))
            .await
            .json();
        let pid = project["pid"].as_str().expect("pid").to_string();

        // ── Funding tranche: gated on g1; held until governance passes.
        let line: Value = request
            .post(&format!("/api/projects/{pid}/budget-lines"))
            .json(&json!({ "category": "capex", "description": "Tranche 2",
                            "currency": "GBP", "planned_minor": 5_000_i64,
                            "gate": "g1_feasibility" }))
            .await
            .json();
        let line_pid = line["pid"].as_str().expect("line pid").to_string();

        // Actuals against a held tranche are refused.
        let held = request
            .post(&format!("/api/projects/{pid}/budget-lines/{line_pid}/actual"))
            .json(&json!({ "amount_minor": 100_i64 }))
            .await;
        assert_eq!(held.status_code(), 422, "held tranche refuses actuals");

        // Release before the gate passes is refused (pre-gate).
        let early = request
            .post(&format!("/api/projects/{pid}/budget-lines/{line_pid}/release"))
            .await;
        assert_eq!(early.status_code(), 422, "pre-gate release refused");

        // Pass g0; g1 not yet reached ⇒ still refused.
        request
            .post(&format!("/api/projects/{pid}/gate-reviews"))
            .json(&json!({ "gate": "g0_concept", "decision": "approved" }))
            .await
            .assert_status_ok();
        let still_early = request
            .post(&format!("/api/projects/{pid}/budget-lines/{line_pid}/release"))
            .await;
        assert_eq!(still_early.status_code(), 422, "g0 does not reach g1");

        // Held planned shows up in the CFO exposure.
        let exposure: Value = request.get("/api/financials/exposure").await.json();
        assert_eq!(exposure["currencies"][0]["held_minor"], 5_000);

        // Pass g1 ⇒ release succeeds exactly once; actuals then flow.
        request
            .post(&format!("/api/projects/{pid}/gate-reviews"))
            .json(&json!({ "gate": "g1_feasibility", "decision": "approved" }))
            .await
            .assert_status_ok();
        let released: Value = request
            .post(&format!("/api/projects/{pid}/budget-lines/{line_pid}/release"))
            .await
            .json();
        assert!(released["released_at"].is_string());
        let again = request
            .post(&format!("/api/projects/{pid}/budget-lines/{line_pid}/release"))
            .await;
        assert_eq!(again.status_code(), 422, "double release refused");
        request
            .post(&format!("/api/projects/{pid}/budget-lines/{line_pid}/actual"))
            .json(&json!({ "amount_minor": 100_i64 }))
            .await
            .assert_status_ok();
        let exposure: Value = request.get("/api/financials/exposure").await.json();
        assert_eq!(exposure["currencies"][0]["held_minor"], 0);

        // ── Tech-debt register: a categorised risk appears; an
        // uncategorised one stays off the view.
        request
            .post(&format!("/api/projects/{pid}/risks"))
            .json(&json!({ "title": "Legacy adapter unmaintained",
                            "probability": 4, "impact": 4,
                            "category": "tech_debt" }))
            .await
            .assert_status_ok();
        request
            .post(&format!("/api/projects/{pid}/risks"))
            .json(&json!({ "title": "Supplier delay", "probability": 2, "impact": 2 }))
            .await
            .assert_status_ok();
        let bad = request
            .post(&format!("/api/projects/{pid}/risks"))
            .json(&json!({ "title": "X", "probability": 1, "impact": 1,
                            "category": "sideways" }))
            .await;
        assert_eq!(bad.status_code(), 422, "unknown category refused");
        let debt: Value = request.get("/api/technology/debt").await.json();
        assert_eq!(debt["register"].as_array().expect("register").len(), 1);
        assert_eq!(debt["register"][0]["exposure"], 16);
        assert_eq!(debt["open_exposure"], 16);

        // ── Flow metrics: a completed milestone is timed via done_at.
        request
            .post(&format!("/api/projects/{pid}/milestones"))
            .json(&json!({ "name": "Cutover", "due": "2026-08-31" }))
            .await
            .assert_status_ok();
        let listed: Value = request.get(&format!("/api/projects/{pid}/milestones")).await.json();
        let m_pid = listed[0]["pid"].as_str().expect("milestone pid");
        request
            .post(&format!("/api/projects/{pid}/milestones/{m_pid}/complete"))
            .await
            .assert_status_ok();
        let flow: Value = request.get("/api/technology/flow").await.json();
        assert_eq!(flow["timed_completions"], 1);
        assert_eq!(flow["median_lead_days"], 0);
        assert_eq!(flow["undated_completions"], 0);

        // ── Alignment coverage: the project is unaligned (with its
        // spend listed); mapping it to an objective flips the counts.
        let alignment: Value = request.get("/api/executive/alignment").await.json();
        let projects = alignment["by_collection"]
            .as_array()
            .expect("collections")
            .iter()
            .find(|c| c["collection"] == "Project")
            .expect("project row")
            .clone();
        assert_eq!(projects["aligned"], 0);
        assert_eq!(alignment["unaligned_spend"][0]["currency"], "GBP");
        assert_eq!(alignment["unaligned_items"][0]["item"]["pid"], json!(pid));

        let objective: Value = request
            .post("/api/objectives")
            .json(&json!({ "title": "Modernise the estate", "period": "2026-H2" }))
            .await
            .json();
        let objective_pid = objective["pid"].as_str().expect("objective pid");
        request
            .post(&format!("/api/projects/{pid}/objectives"))
            .json(&json!({ "objective_pid": objective_pid, "weight": 4 }))
            .await
            .assert_status_ok();
        let alignment: Value = request.get("/api/executive/alignment").await.json();
        let projects = alignment["by_collection"]
            .as_array()
            .expect("collections")
            .iter()
            .find(|c| c["collection"] == "Project")
            .expect("project row")
            .clone();
        assert_eq!(projects["aligned"], 1);
        assert_eq!(projects["unaligned"], 0);

        // ── Scenario compare: two scenarios over the same member, one
        // capped into infeasibility; deltas are per-currency, b - a.
        let roomy: Value = request
            .post("/api/scenarios")
            .json(&json!({ "name": "Roomy", "work_item_pids": [pid],
                            "budget_cap_minor": 10_000, "currency": "GBP" }))
            .await
            .json();
        let tight: Value = request
            .post("/api/scenarios")
            .json(&json!({ "name": "Tight", "work_item_pids": [pid],
                            "budget_cap_minor": 1_000, "currency": "GBP" }))
            .await
            .json();
        let compare: Value = request
            .get(&format!(
                "/api/scenarios/compare?a={}&b={}",
                roomy["pid"].as_str().expect("a"),
                tight["pid"].as_str().expect("b")
            ))
            .await
            .json();
        assert_eq!(compare["a"]["feasible"], true);
        assert_eq!(compare["b"]["feasible"], false);
        assert_eq!(compare["deltas"]["planned_by_currency"][0]["delta_minor"], 0);
        assert_eq!(compare["deltas"]["exposure"], 0);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
#[allow(clippy::too_many_lines)] // one seeded estate, all oversight views
async fn oversight_areas_round_trip() {
    request::<App, _, _>(|request, _ctx| async move {
        // ── Seed: portfolio + project, gated tranche, categorised
        // risks, milestone, benefit, scenario commit.
        let portfolio: Value = request
            .post("/api/portfolios")
            .json(&json!({ "kind": "Portfolio", "name": "Oversight Estate" }))
            .await
            .json();
        let pf_pid = portfolio["pid"].as_str().expect("pid").to_string();
        let project: Value = request
            .post("/api/projects")
            .json(&json!({ "kind": "Project", "name": "Casework digitisation",
                            "portfolio_ref": pf_pid }))
            .await
            .json();
        let pid = project["pid"].as_str().expect("pid").to_string();

        request
            .post(&format!("/api/projects/{pid}/gate-reviews"))
            .json(&json!({ "gate": "g0_concept", "decision": "approved" }))
            .await
            .assert_status_ok();
        let line: Value = request
            .post(&format!("/api/projects/{pid}/budget-lines"))
            .json(&json!({ "category": "capex", "description": "Build",
                            "currency": "GBP", "planned_minor": 4_000_i64,
                            "gate": "g0_concept" }))
            .await
            .json();
        let line_pid = line["pid"].as_str().expect("line pid");
        request
            .post(&format!("/api/projects/{pid}/budget-lines/{line_pid}/release"))
            .await
            .assert_status_ok();
        for (title, category, prob, impact) in [
            ("GDPR basis unclear", "compliance", 4, 5),
            ("Unpatched edge box", "security", 5, 5),
        ] {
            request
                .post(&format!("/api/projects/{pid}/risks"))
                .json(&json!({ "title": title, "probability": prob, "impact": impact,
                                "category": category }))
                .await
                .assert_status_ok();
        }
        request
            .post(&format!("/api/projects/{pid}/milestones"))
            .json(&json!({ "name": "Go-live", "due": "2026-09-30" }))
            .await
            .assert_status_ok();
        let milestones: Value =
            request.get(&format!("/api/projects/{pid}/milestones")).await.json();
        let m_pid = milestones[0]["pid"].as_str().expect("m pid");
        request
            .post(&format!("/api/projects/{pid}/milestones/{m_pid}/complete"))
            .await
            .assert_status_ok();
        let benefit: Value = request
            .post(&format!("/api/projects/{pid}/benefits"))
            .json(&json!({ "title": "Postage saved", "category": "cost_saving",
                            "currency": "GBP", "target_minor": 2_000_i64 }))
            .await
            .json();
        let b_pid = benefit["pid"].as_str().expect("b pid");
        request
            .post(&format!("/api/projects/{pid}/benefits/{b_pid}/realize"))
            .json(&json!({ "amount_minor": 500_i64 }))
            .await
            .assert_status_ok();
        let scenario: Value = request
            .post("/api/scenarios")
            .json(&json!({ "name": "Commit case", "work_item_pids": [pid],
                            "budget_cap_minor": 100_000, "currency": "GBP" }))
            .await
            .json();
        let s_pid = scenario["pid"].as_str().expect("s pid");
        request
            .post(&format!("/api/scenarios/{s_pid}/commit"))
            .await
            .assert_status_ok();

        // ── Board pack: the window captures the decisions + release +
        // milestone + realization; health reflects the estate.
        let pack: Value = request.get("/api/board/pack").await.json();
        assert_eq!(pack["milestones_completed"], 1);
        assert_eq!(pack["tranches_released"]["count"], 1);
        assert_eq!(pack["tranches_released"]["per_currency"][0]["currency"], "GBP");
        assert_eq!(pack["benefits_realized"]["events"], 1);
        assert_eq!(pack["benefits_realized"]["per_currency_minor"]["GBP"], 500);
        assert!(
            pack["decisions"].as_array().expect("decisions").iter()
                .any(|d| d["kind"] == "scenario_commit"),
            "scenario commit is in the pack"
        );

        // ── Board investments: commit + release both present.
        let investments: Value = request.get("/api/board/investments").await.json();
        let kinds: Vec<&str> = investments["investments"]
            .as_array()
            .expect("investments")
            .iter()
            .filter_map(|entry| entry["kind"].as_str())
            .collect();
        assert!(kinds.contains(&"scenario_commit"));
        assert!(kinds.contains(&"tranche_release"));

        // ── Snapshots + trends: capture twice, series holds both.
        request.post("/api/board/snapshots").await.assert_status_ok();
        request.post("/api/board/snapshots").await.assert_status_ok();
        let trends: Value = request.get("/api/board/trends").await.json();
        let series = trends["series"].as_array().expect("series");
        assert_eq!(series.len(), 2);
        assert_eq!(series[0]["body"]["portfolios"], 1);
        assert!(series[0]["body"]["open_exposure"].as_i64().expect("exposure") >= 45);

        // ── Auditor trail: filterable; the release action is recorded.
        let trail: Value = request
            .get("/api/auditor/trail?action=budget_line_released")
            .await
            .json();
        assert_eq!(trail["returned"], 1);
        assert!(trail["stats"]["actorless"].as_u64().expect("count") >= 1);

        // ── Auditor findings: no bearer tokens in this test, so no
        // same-actor conflicts — but actorless actions are counted.
        let findings: Value = request.get("/api/auditor/findings").await.json();
        assert!(findings["actorless_actions"].as_u64().expect("count") > 0);

        // ── Evidence pack: JSON carries audits + decisions; CSV serves.
        let pack: Value = request.get("/api/auditor/evidence-pack").await.json();
        assert!(pack["audit_rows"].as_array().expect("rows").len() > 5);
        assert!(!pack["decisions"].as_array().expect("decisions").is_empty());
        let csv = request.get("/api/auditor/evidence-pack?format=csv").await;
        assert_eq!(csv.status_code(), 200);
        assert!(csv.text().starts_with("created_at,actor,action,entity_pid"));

        // ── Compliance: register carries the GDPR risk; findings flag
        // nothing yet (no overdue targets), so seed one overdue item.
        let register: Value = request.get("/api/compliance/register").await.json();
        assert_eq!(register["register"][0]["title"], "GDPR basis unclear");
        assert_eq!(register["open_exposure"], 20);
        request
            .post("/api/products")
            .json(&json!({ "kind": "Product", "name": "Stale product",
                            "portfolio_ref": pf_pid, "target_date": "2020-01-01" }))
            .await
            .assert_status_ok();
        let findings: Value = request.get("/api/compliance/findings").await.json();
        assert!(
            findings["findings"].as_array().expect("findings").iter().any(
                |f| f["rule"] == "overdue_item_without_recent_gate_review"
            ),
            "the overdue, unreviewed product is a finding"
        );

        // ── CRO heatmap: cells + posture + no appetite configured.
        let heatmap: Value = request.get("/api/risk/heatmap").await.json();
        assert_eq!(heatmap["cells"]["p5i5"], 1);
        assert_eq!(heatmap["cells"]["p4i5"], 1);
        assert_eq!(heatmap["estate_open_exposure"], 45);
        assert!(heatmap["appetite"].is_null());
        assert_eq!(heatmap["breaches"].as_array().expect("breaches").len(), 0);

        // ── CISO register: the security risk + no unreviewed-late-stage
        // items (the project is only at g0).
        let security: Value = request.get("/api/security/register").await.json();
        assert_eq!(security["register"][0]["title"], "Unpatched edge box");
        assert_eq!(
            security["unreviewed_at_late_stage"]["items"]
                .as_array()
                .expect("items")
                .len(),
            0
        );

        // ── Regulator extract: coarse aggregates, unmasked (auth off).
        let extract: Value = request.get("/api/regulator/extract").await.json();
        assert_eq!(extract["masked"], false);
        let pf = &extract["portfolios"][0];
        assert_eq!(pf["name"], "Oversight Estate");
        assert_eq!(pf["members"]["Project"], 1);
        assert_eq!(pf["gate_decisions"]["approved"], 1);
        assert_eq!(pf["spend"][0]["currency"], "GBP");
        assert!(pf.get("owner_ref").is_none(), "no person references");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
#[allow(clippy::too_many_lines)] // one seeded estate, the engineering surface
async fn engineering_tasks_sprints_and_views() {
    request::<App, _, _>(|request, _ctx| async move {
        let project: Value = request
            .post("/api/projects")
            .json(&json!({ "kind": "Project", "name": "Payments revamp",
                            "tags": ["moscow:must"],
                            "identifiers": [{ "scheme": "GitHubProjectId", "value": "42" }] }))
            .await
            .json();
        let pid = project["pid"].as_str().expect("pid").to_string();
        request
            .post("/api/products")
            .json(&json!({ "kind": "Product", "name": "Untracked idea" }))
            .await
            .assert_status_ok();

        // ── Sprint + tasks.
        let sprint: Value = request
            .post(&format!("/api/projects/{pid}/sprints"))
            .json(&json!({ "name": "Sprint 1", "starts_on": "2026-07-13",
                            "ends_on": "2026-07-26" }))
            .await
            .json();
        let sprint_pid = sprint["pid"].as_str().expect("sprint pid").to_string();
        let bad_sprint = request
            .post(&format!("/api/projects/{pid}/sprints"))
            .json(&json!({ "name": "Backwards", "starts_on": "2026-07-26",
                            "ends_on": "2026-07-13" }))
            .await;
        assert_eq!(bad_sprint.status_code(), 422);

        let task_a: Value = request
            .post(&format!("/api/projects/{pid}/tasks"))
            .json(&json!({ "title": "Wire the API", "sprint_pid": sprint_pid,
                            "assignee_ref": "worker:11111111-1111-4111-8111-111111111111" }))
            .await
            .json();
        let a_pid = task_a["pid"].as_str().expect("task pid").to_string();
        assert_eq!(task_a["status"], "todo");
        let task_b: Value = request
            .post(&format!("/api/projects/{pid}/tasks"))
            .json(&json!({ "title": "Ship it", "sprint_pid": sprint_pid }))
            .await
            .json();
        let b_pid = task_b["pid"].as_str().expect("task pid").to_string();
        let bad_status = request
            .post(&format!("/api/projects/{pid}/tasks"))
            .json(&json!({ "title": "X", "status": "sideways" }))
            .await;
        assert_eq!(bad_status.status_code(), 422);
        let bad_assignee = request
            .post(&format!("/api/projects/{pid}/tasks"))
            .json(&json!({ "title": "X", "assignee_ref": "team:core" }))
            .await;
        assert_eq!(bad_assignee.status_code(), 422);

        // Board moves: a -> in_progress -> blocked; b -> done (stamps).
        request
            .patch(&format!("/api/projects/{pid}/tasks/{a_pid}"))
            .json(&json!({ "status": "in_progress" }))
            .await
            .assert_status_ok();
        let blocked_task: Value = request
            .patch(&format!("/api/projects/{pid}/tasks/{a_pid}"))
            .json(&json!({ "status": "blocked" }))
            .await
            .json();
        assert_eq!(blocked_task["blocked_days"], 0);
        let done_task: Value = request
            .patch(&format!("/api/projects/{pid}/tasks/{b_pid}"))
            .json(&json!({ "status": "done" }))
            .await
            .json();
        assert!(done_task["done_at"].is_string(), "first done stamps done_at");

        // PUT refuses status changes (flow stamps stay true).
        let sneaky = request
            .put(&format!("/api/projects/{pid}/tasks/{a_pid}"))
            .json(&json!({ "title": "Wire the API", "status": "done" }))
            .await;
        assert_eq!(sneaky.status_code(), 422);

        // List + counts.
        let listed: Value = request.get(&format!("/api/projects/{pid}/tasks")).await.json();
        assert_eq!(listed["counts"]["blocked"], 1);
        assert_eq!(listed["counts"]["done"], 1);

        // ── Burndown: 2 tasks, one done today ⇒ ends at 1; derivation
        // string is served.
        let burndown: Value = request
            .get(&format!("/api/projects/{pid}/burndown?sprint={sprint_pid}"))
            .await
            .json();
        assert_eq!(burndown["total_tasks"], 2);
        let points = burndown["points"].as_array().expect("points");
        assert_eq!(points.len(), 14);
        assert_eq!(points[0]["remaining"], 2, "before any completion");
        assert_eq!(points[points.len() - 1]["remaining"], 1, "one real completion");
        assert!(burndown["derivation"].as_str().expect("d").contains("no ideal line"));

        // ── Standup digest: creations + moves + the current blocker.
        let standup: Value = request.get(&format!("/api/projects/{pid}/standup")).await.json();
        assert_eq!(standup["tasks_created"].as_array().expect("c").len(), 2);
        assert!(standup["tasks_moved"].as_array().expect("m").len() >= 3);
        assert_eq!(standup["blocked_now"][0]["title"], "Wire the API");

        // ── Estate views.
        let blocked: Value = request.get("/api/engineering/blocked").await.json();
        assert_eq!(blocked["blocked"][0]["title"], "Wire the API");
        assert_eq!(blocked["blocked"][0]["item"]["name"], "Payments revamp");

        let moscow: Value = request.get("/api/engineering/moscow").await.json();
        assert_eq!(moscow["bands"]["must"][0]["name"], "Payments revamp");
        assert_eq!(moscow["untagged"], 1);

        let links: Value = request.get("/api/engineering/delivery-links").await.json();
        assert_eq!(links["tracked"][0]["links"][0]["scheme"], "GitHubProjectId");
        assert_eq!(links["untracked"][0]["name"], "Untracked idea");

        // ── Milestone kinds + the calendar.
        request
            .post(&format!("/api/projects/{pid}/milestones"))
            .json(&json!({ "name": "Sprint demo", "due": "2026-07-24", "kind": "demo" }))
            .await
            .assert_status_ok();
        request
            .post(&format!("/api/projects/{pid}/milestones"))
            .json(&json!({ "name": "Plain milestone", "due": "2026-08-01" }))
            .await
            .assert_status_ok();
        let bad_kind = request
            .post(&format!("/api/projects/{pid}/milestones"))
            .json(&json!({ "name": "X", "due": "2026-08-01", "kind": "party" }))
            .await;
        assert_eq!(bad_kind.status_code(), 422);
        let calendar: Value = request
            .get("/api/engineering/milestone-calendar?kind=demo")
            .await
            .json();
        let entries = calendar["milestones"].as_array().expect("entries");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0]["name"], "Sprint demo");
        let all: Value = request.get("/api/engineering/milestone-calendar").await.json();
        assert_eq!(all["milestones"].as_array().expect("all").len(), 2);
    })
    .await;
}
