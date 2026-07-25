//! The talent-strategy round-trip: upskilling and reskilling plans
//! (declared vs verified progress), talent pipelines, apprenticeships
//! and internships (including the off-the-job hours gate), succession
//! risk, and the workforce-intelligence views.

use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;
use workforce_planning_management_service::app::App;

use super::{activate, an_org, seed_employee};

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
#[allow(clippy::too_many_lines)] // one seeded team, the whole talent surface
async fn development_plans_track_claimed_and_verified_progress() {
    request::<App, _, _>(|request, _ctx| async move {
        let org = an_org();
        let employee = seed_employee(&request, &org, "T-1", None).await;
        activate(&request, &employee).await;

        let skill: Value = request
            .post("/api/skills")
            .json(&json!({ "name": "Kubernetes", "category": "technical" }))
            .await
            .json();
        let skill_pid = skill["pid"].as_str().expect("skill pid").to_string();

        // ── Upskill vs reskill: the target role decides the kind.
        assert_eq!(
            request
                .post(&format!("/api/employees/{employee}/development-plans"))
                .json(&json!({ "kind": "reskill" }))
                .await
                .status_code(),
            422,
            "a reskill must name its target role"
        );
        assert_eq!(
            request
                .post(&format!("/api/employees/{employee}/development-plans"))
                .json(&json!({ "kind": "upskill", "target_job_title": "Data Engineer" }))
                .await
                .status_code(),
            422,
            "an upskill deepens the current role"
        );
        assert_eq!(
            request
                .post(&format!("/api/employees/{employee}/development-plans"))
                .json(&json!({ "kind": "reskill", "target_job_title": "Engineer" }))
                .await
                .status_code(),
            422,
            "reskilling into the current role is an upskill"
        );
        assert_eq!(
            request
                .post(&format!("/api/employees/{employee}/development-plans"))
                .json(&json!({
                    "kind": "upskill",
                    "items": [{
                        "skill_pid": skill_pid, "current_level": 4, "target_level": 3,
                        "method": "course",
                    }],
                }))
                .await
                .status_code(),
            422,
            "a step must raise the level"
        );

        let plan: Value = request
            .post(&format!("/api/employees/{employee}/development-plans"))
            .json(&json!({
                "kind": "upskill",
                "rationale": "platform ownership",
                "items": [
                    { "skill_pid": skill_pid, "current_level": 2, "target_level": 4, "method": "course" },
                ],
            }))
            .await
            .json();
        let plan_pid = plan["pid"].as_str().expect("plan pid").to_string();

        // ── The item is claimed achieved, but proficiency has not moved.
        let listed: Value = request
            .get(&format!("/api/employees/{employee}/development-plans"))
            .await
            .json();
        let item_pid = listed["plans"][0]["items"][0]["pid"]
            .as_str()
            .expect("item pid")
            .to_string();
        request
            .put(&format!("/api/development-plan-items/{item_pid}"))
            .json(&json!({ "status": "achieved" }))
            .await
            .assert_status_ok();

        let claimed: Value = request
            .get(&format!("/api/employees/{employee}/development-plans"))
            .await
            .json();
        assert_eq!(claimed["plans"][0]["declared_progress"]["numerator"], 1);
        assert_eq!(
            claimed["plans"][0]["verified_progress"]["numerator"], 0,
            "a claim without declared proficiency is not verified"
        );

        // Declaring the proficiency is what makes it verified.
        request
            .put(&format!("/api/employees/{employee}/skills"))
            .json(&json!({ "skill_pid": skill_pid, "proficiency": 4 }))
            .await
            .assert_status_ok();
        let verified: Value = request
            .get(&format!("/api/employees/{employee}/development-plans"))
            .await
            .json();
        assert_eq!(verified["plans"][0]["verified_progress"]["numerator"], 1);

        // ── Lifecycle: activate, then complete once every item is resolved.
        assert_eq!(
            request
                .post(&format!("/api/development-plans/{plan_pid}/status"))
                .json(&json!({ "to": "completed" }))
                .await
                .status_code(),
            422,
            "must activate first"
        );
        request
            .post(&format!("/api/development-plans/{plan_pid}/status"))
            .json(&json!({ "to": "active" }))
            .await
            .assert_status_ok();
        request
            .post(&format!("/api/development-plans/{plan_pid}/status"))
            .json(&json!({ "to": "completed" }))
            .await
            .assert_status_ok();
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
#[allow(clippy::too_many_lines)] // pipelines + early careers + intelligence in one flow
async fn pipelines_apprenticeships_and_intelligence() {
    request::<App, _, _>(|request, _ctx| async move {
        let org = an_org();
        let apprentice = seed_employee(&request, &org, "P-1", None).await;
        activate(&request, &apprentice).await;
        let supervisor = seed_employee(&request, &org, "P-2", None).await;
        activate(&request, &supervisor).await;

        // ── Talent pipeline: stages move forward, and readiness may regress.
        let pipeline: Value = request
            .post("/api/talent-pipelines")
            .json(&json!({
                "name": "Future team leads",
                "purpose": "succession",
                "target_job_title": "Team Lead",
            }))
            .await
            .json();
        let pipeline_pid = pipeline["pid"].as_str().expect("pipeline pid").to_string();
        assert_eq!(
            request
                .post("/api/talent-pipelines")
                .json(&json!({ "name": "Bad", "purpose": "vibes" }))
                .await
                .status_code(),
            422,
            "unknown purpose refused"
        );

        let member: Value = request
            .post(&format!("/api/talent-pipelines/{pipeline_pid}/members"))
            .json(&json!({ "subject_kind": "employee", "subject_pid": apprentice }))
            .await
            .json();
        let member_pid = member["pid"].as_str().expect("member pid").to_string();
        assert_eq!(
            request
                .post(&format!("/api/talent-pipelines/{pipeline_pid}/members"))
                .json(&json!({ "subject_kind": "employee", "subject_pid": apprentice }))
                .await
                .status_code(),
            422,
            "one row per subject"
        );
        assert_eq!(
            request
                .post(&format!("/api/pipeline-members/{member_pid}/stage"))
                .json(&json!({ "to": "placed" }))
                .await
                .status_code(),
            422,
            "no skipping straight to placed"
        );
        for stage in ["assessing", "developing", "ready"] {
            request
                .post(&format!("/api/pipeline-members/{member_pid}/stage"))
                .json(&json!({ "to": stage }))
                .await
                .assert_status_ok();
        }
        let ready: Value = request
            .get(&format!("/api/talent-pipelines/{pipeline_pid}"))
            .await
            .json();
        assert_eq!(ready["health"]["ready"], 1);
        assert_eq!(ready["health"]["live"], 1);
        // Readiness can regress — the pipeline must be able to say so.
        request
            .post(&format!("/api/pipeline-members/{member_pid}/stage"))
            .json(&json!({ "to": "developing", "readiness": "ready_1y" }))
            .await
            .assert_status_ok();
        let regressed: Value = request
            .get(&format!("/api/talent-pipelines/{pipeline_pid}"))
            .await
            .json();
        assert_eq!(regressed["health"]["ready"], 0, "the bench shrank honestly");

        // ── Apprenticeship: the off-the-job hours are the gate.
        assert_eq!(
            request
                .post("/api/early-career-programs")
                .json(&json!({
                    "name": "No hours", "kind": "apprenticeship", "duration_months": 18,
                }))
                .await
                .status_code(),
            422,
            "an apprenticeship must declare its off-the-job hours"
        );
        let program: Value = request
            .post("/api/early-career-programs")
            .json(&json!({
                "name": "Software Developer L4", "kind": "apprenticeship", "level": 4,
                "duration_months": 18, "min_off_the_job_hours": 100,
            }))
            .await
            .json();
        let program_pid = program["pid"].as_str().expect("program pid").to_string();

        let placement: Value = request
            .post(&format!(
                "/api/early-career-programs/{program_pid}/placements"
            ))
            .json(&json!({
                "employee_pid": apprentice,
                "supervisor_pid": supervisor,
                "started_on": "2026-02-02",
            }))
            .await
            .json();
        let placement_pid = placement["pid"]
            .as_str()
            .expect("placement pid")
            .to_string();

        // Hours only accrue on an active placement.
        assert_eq!(
            request
                .post(&format!("/api/program-placements/{placement_pid}/hours"))
                .json(&json!({ "hours": 10 }))
                .await
                .status_code(),
            422,
            "an offered placement does not accrue hours"
        );
        request
            .post(&format!("/api/program-placements/{placement_pid}/status"))
            .json(&json!({ "to": "active" }))
            .await
            .assert_status_ok();
        request
            .post(&format!("/api/program-placements/{placement_pid}/hours"))
            .json(&json!({ "hours": 60 }))
            .await
            .assert_status_ok();
        assert_eq!(
            request
                .post(&format!("/api/program-placements/{placement_pid}/status"))
                .json(&json!({ "to": "completed", "outcome": "converted" }))
                .await
                .status_code(),
            422,
            "60 of 100 off-the-job hours ⇒ cannot complete the apprenticeship"
        );
        request
            .post(&format!("/api/program-placements/{placement_pid}/hours"))
            .json(&json!({ "hours": 40 }))
            .await
            .assert_status_ok();
        request
            .post(&format!("/api/program-placements/{placement_pid}/status"))
            .json(&json!({ "to": "completed", "outcome": "converted" }))
            .await
            .assert_status_ok();

        let placements: Value = request
            .get(&format!("/api/employees/{apprentice}/placements"))
            .await
            .json();
        assert_eq!(placements["placements"][0]["off_the_job"]["hours"], 100);
        assert_eq!(placements["placements"][0]["off_the_job"]["met"], true);

        // ── Workforce intelligence.
        let programs: Value = request
            .get("/api/early-career-programs?kind=apprenticeship")
            .await
            .json();
        let conversion = &programs["programs"][0]["placements"]["conversion_rate"];
        assert_eq!(conversion["numerator"], 1);
        assert_eq!(conversion["denominator"], 1);

        let overview: Value = request
            .get("/api/workforce-intelligence/overview")
            .await
            .json();
        assert!(
            overview["headcount"].as_u64().expect("headcount") >= 2,
            "the seeded team is counted"
        );
        assert!(
            overview["by_department"]["engineering"]
                .as_u64()
                .expect("dept")
                >= 2
        );

        let funnel: Value = request
            .get("/api/workforce-intelligence/pipelines")
            .await
            .json();
        assert_eq!(funnel["by_purpose"]["succession"], 1);
        let apprenticeships = funnel["early_careers"]
            .as_array()
            .expect("kinds")
            .iter()
            .find(|k| k["kind"] == "apprenticeship")
            .expect("apprenticeship rollup")
            .clone();
        assert_eq!(apprenticeships["completed_placements"], 1);
        assert_eq!(apprenticeships["conversion_rate"]["value"], 1.0);

        // ── Succession: risk of loss makes an uncovered role a single
        // point of failure, and a ready successor clears it.
        let plan: Value = request
            .post("/api/succession-plans")
            .json(&json!({
                "role_title": "Head of Platform", "department": "engineering",
                "criticality": 5, "incumbent_pid": supervisor, "risk_of_loss": "high",
            }))
            .await
            .json();
        let plan_pid = plan["pid"].as_str().expect("plan pid").to_string();
        let exposed: Value = request
            .get("/api/workforce-intelligence/succession")
            .await
            .json();
        assert!(
            exposed["single_points_of_failure"]
                .as_array()
                .expect("spofs")
                .iter()
                .any(|s| s["plan_pid"].as_str() == Some(plan_pid.as_str())),
            "an uncovered critical role is a single point of failure"
        );

        let candidate: Value = request
            .post(&format!("/api/succession-plans/{plan_pid}/candidates"))
            .json(&json!({ "employee_pid": apprentice, "readiness": "ready_2y" }))
            .await
            .json();
        let candidate_pid = candidate["pid"]
            .as_str()
            .expect("candidate pid")
            .to_string();
        let developing: Value = request
            .get("/api/workforce-intelligence/succession")
            .await
            .json();
        assert_eq!(
            developing["by_coverage"]["developing"], 1,
            "a ready_2y bench is not cover"
        );

        request
            .put(&format!("/api/succession-candidates/{candidate_pid}"))
            .json(&json!({ "readiness": "ready_now" }))
            .await
            .assert_status_ok();
        let covered: Value = request
            .get("/api/workforce-intelligence/succession")
            .await
            .json();
        assert_eq!(covered["by_coverage"]["covered_now"], 1);
        assert!(
            !covered["single_points_of_failure"]
                .as_array()
                .expect("spofs")
                .iter()
                .any(|s| s["plan_pid"].as_str() == Some(plan_pid.as_str())),
            "a ready successor clears the exposure"
        );
    })
    .await;
}
