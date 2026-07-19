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
