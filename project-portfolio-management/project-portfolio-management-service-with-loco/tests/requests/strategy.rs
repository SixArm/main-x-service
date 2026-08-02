//! PPM Phase-C strategy request tests (spec/15-roadmap PPM-2/4/5/11):
//! the idea funnel end-to-end (idea → proposal → plan), scenario
//! evaluation + the infeasible-commit refusal, OKR alignment rollups,
//! and benefits with ROI against recorded spend.
//!
//! `#[ignore]`d: needs PostgreSQL; run with `cargo test -- --ignored`.

use loco_rs::testing::prelude::*;
use project_portfolio_management_service::app::App;
use serde_json::{Value, json};
use serial_test::serial;

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// The full funnel: capture → vote (ranked) → convert to a draft
// proposal → the proposal promotes into the registry; dismissed and
// converted ideas refuse further actions.
async fn idea_funnel_flows_into_the_intake_pipeline() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let quiet: Value = request
            .post("/api/ideas")
            .json(&json!({ "title": "Modest idea" }))
            .await
            .json();
        let loud: Value = request
            .post("/api/ideas")
            .json(
                &json!({ "title": "Popular idea", "pitch": "Everyone wants this",
                            "tags": ["platform", "q3"] }),
            )
            .await
            .json();
        let loud_pid = loud["pid"].as_str().expect("pid").to_string();
        for _ in 0..3 {
            request
                .post(&format!("/api/ideas/{loud_pid}/vote"))
                .await
                .assert_status_ok();
        }
        let board: Value = request.get("/api/ideas").await.json();
        assert_eq!(board[0]["pid"], loud_pid.as_str(), "most-voted first");
        assert_eq!(board[0]["votes"], 3);

        // Convert the popular one; the draft proposal carries the pitch.
        let converted: Value = request
            .post(&format!("/api/ideas/{loud_pid}/convert"))
            .json(&json!({ "kind_target": "projects" }))
            .await
            .json();
        assert_eq!(converted["status"], "converted");
        let proposal_pid = converted["proposal_pid"].as_str().expect("proposal pid");
        let proposal: Value = request
            .get(&format!("/api/proposals/{proposal_pid}"))
            .await
            .json();
        assert_eq!(proposal["title"], "Popular idea");
        assert_eq!(proposal["summary"], "Everyone wants this");
        assert_eq!(proposal["status"], "draft");

        // Converted ideas take no more votes; dismiss the quiet one.
        assert_eq!(
            request
                .post(&format!("/api/ideas/{loud_pid}/vote"))
                .await
                .status_code(),
            422
        );
        let quiet_pid = quiet["pid"].as_str().expect("pid");
        request
            .post(&format!("/api/ideas/{quiet_pid}/dismiss"))
            .await
            .assert_status_ok();

        // The funnel completes: submit → review → approve → promote.
        for action in ["submit", "review", "approve", "promote"] {
            request
                .post(&format!("/api/proposals/{proposal_pid}/{action}"))
                .await
                .assert_status_ok();
        }
        let done: Value = request
            .get(&format!("/api/proposals/{proposal_pid}"))
            .await
            .json();
        assert_eq!(done["status"], "promoted");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Scenario evaluation sums live budgets/risks per member, names cap
// and must-include violations, and refuses committing an infeasible
// scenario; a feasible one commits with the evaluation audited.
async fn scenarios_evaluate_and_commit_feasibly() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        // Two projects with budgets; one carries an open risk.
        let mk = |name: &str| {
            let request = &request;
            let name = name.to_string();
            async move {
                let created: Value = request
                    .post("/api/plans")
                    .json(&json!({ "kind": "Project", "name": name }))
                    .await
                    .json();
                created["pid"].as_str().expect("pid").to_string()
            }
        };
        let alpha = mk("Scenario Alpha").await;
        let beta = mk("Scenario Beta").await;
        for (pid, amount) in [(&alpha, 600_000), (&beta, 500_000)] {
            request
                .post(&format!("/api/plans/{pid}/budget-lines"))
                .json(&json!({ "category": "capex", "description": "build",
                                "currency": "GBP", "planned_minor": amount }))
                .await
                .assert_status_ok();
        }
        request
            .post(&format!("/api/plans/{alpha}/risks"))
            .json(&json!({ "title": "Supply risk", "probability": 3, "impact": 4 }))
            .await
            .assert_status_ok();

        // Unknown member pid refuses at create.
        assert_eq!(
            request
                .post("/api/scenarios")
                .json(&json!({ "name": "Ghost", "plan_pids": [uuid::Uuid::new_v4()] }))
                .await
                .status_code(),
            422
        );

        // A capped scenario over both projects: 1.1m > 1m cap ⇒
        // infeasible; commit refuses.
        let tight: Value = request
            .post("/api/scenarios")
            .json(&json!({
                "name": "Tight",
                "plan_pids": [alpha, beta],
                "budget_cap_minor": 1_000_000, "currency": "GBP",
            }))
            .await
            .json();
        let tight_pid = tight["pid"].as_str().expect("pid");
        let evaluated: Value = request
            .get(&format!("/api/scenarios/{tight_pid}/evaluate"))
            .await
            .json();
        assert_eq!(evaluated["feasible"], false);
        assert_eq!(evaluated["evaluation"]["total_exposure"], 12);
        assert_eq!(
            evaluated["evaluation"]["planned_by_currency"][0],
            json!(["GBP", 1_100_000])
        );
        assert_eq!(
            request
                .post(&format!("/api/scenarios/{tight_pid}/commit"))
                .await
                .status_code(),
            422,
            "infeasible scenarios cannot commit"
        );

        // Alpha alone fits the cap and must-include; commits.
        let fits: Value = request
            .post("/api/scenarios")
            .json(&json!({
                "name": "Fits",
                "plan_pids": [alpha],
                "budget_cap_minor": 1_000_000, "currency": "GBP",
                "must_include": [alpha],
            }))
            .await
            .json();
        let fits_pid = fits["pid"].as_str().expect("pid");
        let committed: Value = request
            .post(&format!("/api/scenarios/{fits_pid}/commit"))
            .await
            .json();
        assert_eq!(committed["status"], "committed");
        assert_eq!(
            request
                .post(&format!("/api/scenarios/{fits_pid}/commit"))
                .await
                .status_code(),
            422,
            "already committed"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// OKR alignment: weighted links upsert per (objective, item); the
// objective rolls weights up per collection; the item lists its
// mappings; weight bounds refuse.
async fn okr_alignment_rolls_up() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let objective: Value = request
            .post("/api/objectives")
            .json(&json!({ "title": "Reduce cost to serve", "period": "2026-H2" }))
            .await
            .json();
        let objective_pid = objective["pid"].as_str().expect("pid").to_string();
        let project: Value = request
            .post("/api/plans")
            .json(&json!({ "kind": "Project", "name": "Aligned project" }))
            .await
            .json();
        let project_pid = project["pid"].as_str().expect("pid").to_string();
        let program: Value = request
            .post("/api/plans")
            .json(&json!({ "kind": "Program", "name": "Aligned programme" }))
            .await
            .json();
        let program_pid = program["pid"].as_str().expect("pid").to_string();

        assert_eq!(
            request
                .post(&format!("/api/plans/{project_pid}/objectives"))
                .json(&json!({ "objective_pid": objective_pid, "weight": 9 }))
                .await
                .status_code(),
            422,
            "weight bounds"
        );
        request
            .post(&format!("/api/plans/{project_pid}/objectives"))
            .json(&json!({ "objective_pid": objective_pid, "weight": 2 }))
            .await
            .assert_status_ok();
        // Re-linking the same pair updates the weight (upsert).
        request
            .post(&format!("/api/plans/{project_pid}/objectives"))
            .json(&json!({ "objective_pid": objective_pid, "weight": 5 }))
            .await
            .assert_status_ok();
        request
            .post(&format!("/api/plans/{program_pid}/objectives"))
            .json(&json!({ "objective_pid": objective_pid, "weight": 3 }))
            .await
            .assert_status_ok();

        let alignment: Value = request
            .get(&format!("/api/objectives/{objective_pid}/alignment"))
            .await
            .json();
        assert_eq!(alignment["total_weight"], 8, "5 (upserted) + 3");
        assert_eq!(alignment["weight_by_collection"]["Project"], 5);
        assert_eq!(alignment["weight_by_collection"]["Program"], 3);
        let item_view: Value = request
            .get(&format!("/api/plans/{project_pid}/objectives"))
            .await
            .json();
        assert_eq!(item_view[0]["weight"], 5);
        assert_eq!(item_view[0]["title"], "Reduce cost to serve");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Benefits: declared targets, realized accumulation, and ROI in
// basis points against the item's recorded budget actuals.
async fn benefits_track_realization_and_roi() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let item: Value = request
            .post("/api/plans")
            .json(&json!({ "kind": "Product", "name": "Benefit bearer" }))
            .await
            .json();
        let pid = item["pid"].as_str().expect("pid").to_string();

        // A benefit needs a financial target or a note.
        assert_eq!(
            request
                .post(&format!("/api/plans/{pid}/benefits"))
                .json(&json!({ "title": "Vague", "category": "other" }))
                .await
                .status_code(),
            422
        );
        let benefit: Value = request
            .post(&format!("/api/plans/{pid}/benefits"))
            .json(
                &json!({ "title": "Support cost saving", "category": "cost_saving",
                            "currency": "GBP", "target_minor": 2_000_000,
                            "expected_on": "2026-12-31" }),
            )
            .await
            .json();
        let benefit_pid = benefit["pid"].as_str().expect("pid");

        // Spend 1m (budget line + actual), realize 1.5m ⇒ ROI +50%.
        let line: Value = request
            .post(&format!("/api/plans/{pid}/budget-lines"))
            .json(&json!({ "category": "opex", "description": "run",
                            "currency": "GBP", "planned_minor": 1_200_000 }))
            .await
            .json();
        let line_pid = line["pid"].as_str().expect("line pid");
        request
            .post(&format!("/api/plans/{pid}/budget-lines/{line_pid}/actual"))
            .json(&json!({ "amount_minor": 1_000_000 }))
            .await
            .assert_status_ok();
        request
            .post(&format!("/api/plans/{pid}/benefits/{benefit_pid}/realize"))
            .json(&json!({ "amount_minor": 1_500_000, "status": "on_track" }))
            .await
            .assert_status_ok();

        let listed: Value = request
            .get(&format!("/api/plans/{pid}/benefits"))
            .await
            .json();
        assert_eq!(listed["totals"][0]["currency"], "GBP");
        assert_eq!(listed["totals"][0]["target_minor"], 2_000_000);
        assert_eq!(listed["totals"][0]["realized_minor"], 1_500_000);
        assert_eq!(listed["totals"][0]["spend_minor"], 1_000_000);
        assert_eq!(listed["totals"][0]["roi_basis_points"], 5_000, "+50%");
        assert_eq!(listed["benefits"][0]["status"], "on_track");
    })
    .await;
}
