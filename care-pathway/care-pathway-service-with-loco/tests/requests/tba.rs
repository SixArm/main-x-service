//! The time-based-analysis round trip: set a clock, record segments,
//! and read the derived per-instance, timeline, cohort, constraint and
//! flow views. Pins the contract in `spec/time-based-analysis.md` §14.3.

use care_pathway_service::app::App;
use care_pathway_service::tba::DAY_MS;
use loco_rs::TestServer;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

/// A fixed reference day, so the assertions are about durations rather
/// than about when the suite happened to run.
const DAY0: &str = "2026-01-01T00:00:00Z";

/// `DAY0` plus `days`, as an RFC 3339 instant.
fn day(days: i64) -> String {
    let base: chrono::DateTime<chrono::Utc> = DAY0.parse().expect("parse DAY0");
    (base + chrono::Duration::days(days)).to_rfc3339()
}

/// Seed a pathway template and one enrolled instance.
async fn seed(request: &TestServer) -> (String, String) {
    let created = request
        .post("/api/care-pathways")
        .json(&json!({
            "name": format!("TBA pathway {}", uuid::Uuid::new_v4()),
            "care_setting": "Outpatient",
            "condition_codes": [{"system": "Icd10", "code": "M54"}],
        }))
        .await;
    created.assert_status_ok();
    let template: Value = created.json();
    let pathway = template["pid"].as_str().expect("pathway pid").to_string();
    let enrolled = request
        .post(&format!("/api/care-pathways/{pathway}/instances"))
        .json(&json!({ "subject_ref": format!("person:{}", uuid::Uuid::new_v4()) }))
        .await;
    enrolled.assert_status_ok();
    let instance: Value = enrolled.json();
    let instance_pid = instance["pid"].as_str().expect("instance pid").to_string();
    (pathway, instance_pid)
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
#[allow(clippy::too_many_lines)] // one seeded journey, the whole TBA surface
async fn time_based_analysis_round_trip() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let (pathway, pid) = seed(&request).await;

        // ── The clock: a 100-day journey, set explicitly.
        request
            .post(&format!("/api/instances/{pid}/clock"))
            .json(&json!({ "event": "start", "at": day(0) }))
            .await
            .assert_status_ok();
        request
            .post(&format!("/api/instances/{pid}/clock"))
            .json(&json!({ "event": "stop", "at": day(100) }))
            .await
            .assert_status_ok();
        assert_eq!(
            request
                .post(&format!("/api/instances/{pid}/clock"))
                .json(&json!({ "event": "pause", "at": day(50) }))
                .await
                .status_code(),
            422,
            "there is no clock pause — spec §12.3"
        );

        // ── Fourteen days of value-adding care inside 100 days: the
        // Barker case, end to end through HTTP.
        for (label, stage, start, end) in [
            ("first consultation", "treatment", 0, 7),
            ("scan", "diagnostics", 60, 67),
        ] {
            request
                .post(&format!("/api/instances/{pid}/segments"))
                .json(&json!({
                    "label": label, "stage": stage, "category": "value_adding",
                    "started_at": day(start), "ended_at": day(end),
                    "actor_ref": format!("worker:{}", uuid::Uuid::new_v4()),
                }))
                .await
                .assert_status_ok();
        }
        request
            .post(&format!("/api/instances/{pid}/segments"))
            .json(&json!({
                "label": "wait for scan slot", "stage": "diagnostics",
                "category": "unnecessary_non_value_adding", "waste": "waiting",
                "started_at": day(7), "ended_at": day(60),
            }))
            .await
            .assert_status_ok();

        // ── The §5.1 invariants are refused at the boundary.
        let bad = [
            (
                json!({"label": "x", "stage": "treatment", "category": "value_adding",
                       "waste": "waiting", "started_at": day(1), "ended_at": day(2)}),
                "waste on a value-adding segment",
            ),
            (
                json!({"label": "x", "stage": "treatment",
                       "category": "unnecessary_non_value_adding",
                       "started_at": day(1), "ended_at": day(2)}),
                "unnecessary without a waste type",
            ),
            (
                json!({"label": "x", "stage": "sideways", "category": "value_adding",
                       "started_at": day(1), "ended_at": day(2)}),
                "unknown stage",
            ),
            (
                json!({"label": "x", "stage": "treatment", "category": "value_adding",
                       "started_at": day(5), "ended_at": day(2)}),
                "reversed interval",
            ),
            (
                json!({"label": "", "stage": "treatment", "category": "value_adding",
                       "started_at": day(1), "ended_at": day(2)}),
                "blank label",
            ),
        ];
        for (body, why) in bad {
            assert_eq!(
                request
                    .post(&format!("/api/instances/{pid}/segments"))
                    .json(&body)
                    .await
                    .status_code(),
                422,
                "refused: {why}"
            );
        }

        // ── Per-instance analysis: 14 of 100 days is the headline.
        let analysis: Value = request
            .get(&format!("/api/instances/{pid}/time-analysis"))
            .await
            .json();
        let a = &analysis["analysis"];
        assert_eq!(a["clock"]["start_source"], "clock_start_at");
        assert_eq!(a["lead_time_days"], 100.0);
        let ratio = a["value_adding_ratio"]["value"].as_f64().expect("ratio");
        assert!(
            (ratio - 0.14).abs() < 1e-9,
            "14 value-adding days in 100: got {ratio}"
        );
        // 67 of 100 days are covered by a segment, which is below the
        // 80% `mapped` threshold — so the ratio is reported as
        // `partial`, not presented as fully evidenced (spec §6.6).
        let coverage = a["coverage_ratio"]["value"].as_f64().expect("coverage");
        assert!((coverage - 0.67).abs() < 1e-9, "got {coverage}");
        assert_eq!(a["confidence"], "partial");
        // The four buckets partition the clock exactly (§12.3).
        let buckets: i64 = a["by_category"]
            .as_array()
            .expect("by_category")
            .iter()
            .map(|c| c["ms"].as_i64().unwrap_or(0))
            .sum();
        assert_eq!(buckets, a["lead_time_ms"].as_i64().expect("lead time"));
        // The biggest queue is named, not merely counted.
        let longest = &a["gaps"][0];
        assert_eq!(longest["days"], 33.0, "day 67 → day 100");
        assert_eq!(longest["after"], "scan");

        // ── An open segment blocks a second one until it is closed.
        let open: Value = request
            .post(&format!("/api/instances/{pid}/segments"))
            .json(&json!({
                "label": "in theatre", "stage": "treatment",
                "category": "value_adding", "started_at": day(80),
            }))
            .await
            .json();
        let seg = open["pid"].as_str().expect("segment pid").to_string();
        assert_eq!(open["ended_at"], Value::Null, "still running");
        assert_eq!(
            request
                .post(&format!("/api/instances/{pid}/segments"))
                .json(&json!({
                    "label": "also open", "stage": "treatment",
                    "category": "value_adding", "started_at": day(81),
                }))
                .await
                .status_code(),
            422,
            "only one open segment at a time"
        );
        request
            .post(&format!("/api/instances/{pid}/segments/{seg}/close"))
            .json(&json!({ "ended_at": day(82) }))
            .await
            .assert_status_ok();
        assert_eq!(
            request
                .post(&format!("/api/instances/{pid}/segments/{seg}/close"))
                .json(&json!({ "ended_at": day(83) }))
                .await
                .status_code(),
            422,
            "already closed"
        );

        // ── The timeline wall interleaves segments and gaps in order.
        let timeline: Value = request
            .get(&format!("/api/instances/{pid}/timeline"))
            .await
            .json();
        let wall = timeline["wall"].as_array().expect("wall");
        assert!(wall.len() >= 4);
        assert!(wall.iter().any(|e| e["kind"] == "segment"));
        assert!(wall.iter().any(|e| e["kind"] == "gap"));

        // ── Cohort: one instance, so percentile detail is suppressed.
        let cohort: Value = request
            .get(&format!(
                "/api/care-pathways/{pathway}/time-analysis?standard=rtt_18_weeks"
            ))
            .await
            .json();
        assert_eq!(cohort["cohort"]["instances"], 1);
        assert_eq!(cohort["suppressed"], true, "n=1 identifies the patient");
        assert_eq!(cohort["cohort"]["lead_time"], Value::Null);
        assert_eq!(cohort["compliance"]["standard"], "rtt_18_weeks");
        assert_eq!(cohort["compliance"]["within"], 1, "100 days is under 126");
        assert_eq!(cohort["compliance"]["threshold_days"], 126.0);
        assert_eq!(
            cohort["suppression_note"],
            "withheld: fewer than the minimum cell count"
        );

        // `?mode=remove` drops the key entirely instead of nulling it.
        let removed_cohort: Value = request
            .get(&format!(
                "/api/care-pathways/{pathway}/time-analysis?mode=remove"
            ))
            .await
            .json();
        assert!(
            removed_cohort["cohort"].get("lead_time").is_none(),
            "remove mode drops the key"
        );

        assert_eq!(
            request
                .get(&format!(
                    "/api/care-pathways/{pathway}/time-analysis?standard=nope"
                ))
                .await
                .status_code(),
            422,
            "unknown standard is refused, not ignored"
        );

        // ── Constraints: this cohort is one instance too, so the
        // findings (which would otherwise describe this one patient's
        // journey precisely) are withheld the same way the cohort
        // view's percentiles are — a real fix, not a design choice this
        // test merely observes (spec T-14k; this endpoint previously
        // returned `findings` unsuppressed at any cohort size). The
        // ordering/naming logic itself is proven DB-free in
        // `src/tba.rs`'s `constraints_rank_by_recoverable_time_and_name_their_rule`.
        let constraints: Value = request
            .get(&format!("/api/care-pathways/{pathway}/constraints"))
            .await
            .json();
        assert_eq!(constraints["instances"], 1);
        assert_eq!(
            constraints["suppressed"], true,
            "n=1 identifies the patient"
        );
        assert_eq!(constraints["findings"], Value::Null);

        // `?mode=remove` drops the key entirely instead of nulling it.
        let removed: Value = request
            .get(&format!(
                "/api/care-pathways/{pathway}/constraints?mode=remove"
            ))
            .await
            .json();
        assert!(
            removed.get("findings").is_none(),
            "remove mode drops the key"
        );
        assert_eq!(
            removed["suppressed"], true,
            "still reported, just not the value"
        );

        // ── Flow: Little's Law over the window.
        let flow: Value = request
            .get("/api/instances/flow?window_days=90")
            .await
            .json();
        assert_eq!(flow["flow"]["window_days"], 90);
        assert!(flow["flow"]["interpretation"].is_string());
        assert_eq!(
            request
                .get("/api/instances/flow?window_days=0")
                .await
                .status_code(),
            422,
            "window must be at least a day"
        );

        // ── The standards catalogue carries its citation dates.
        let standards: Value = request.get("/api/instances/time-standards").await.json();
        let list = standards["standards"].as_array().expect("standards");
        assert!(list.iter().all(|s| s["as_of"].is_string()));
        assert!(list.iter().any(|s| s["id"] == "ae_4_hours"));

        // ── Recording is audited (HIPAA §164.312(b)).
        let audit: Value = request
            .get(&format!("/api/care-pathways/{pid}/audit"))
            .await
            .json();
        let actions: Vec<&str> = audit
            .as_array()
            .map(|rows| rows.iter().filter_map(|r| r["action"].as_str()).collect())
            .unwrap_or_default();
        assert!(actions.contains(&"instance_segment_recorded"));
        assert!(actions.contains(&"instance_clock_set"));
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn an_unmapped_journey_reads_as_unknown_not_as_inefficient() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let (_pathway, pid) = seed(&request).await;
        let analysis: Value = request
            .get(&format!("/api/instances/{pid}/time-analysis"))
            .await
            .json();
        let a = &analysis["analysis"];
        // Enrolment sets the clock, so the start is measured, not
        // inferred; nothing else is recorded yet.
        assert_eq!(a["clock"]["start_source"], "clock_start_at");
        assert_eq!(a["clock"]["stop_source"], "as_of", "still running");
        assert_eq!(a["confidence"], "unmapped");
        assert_eq!(a["coverage_ratio"]["value"], 0.0);
        assert_eq!(
            a["segments"], 0,
            "an unmapped journey reports zero coverage, not a bad score"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn the_flow_gauges_publish_only_what_may_be_published() {
    super::isolate_search_index();
    request::<App, _, _>(|request, ctx| async move {
        let (pathway, _pid) = seed(&request).await;

        // One instance: below the cohort floor, so the pathway is
        // counted as suppressed rather than exported. `/metrics.prom`
        // stays scrapeable under enforcement, so a p90 lead time over
        // one patient must not leave by that door.
        let set = care_pathway_service::flow_metrics::refresh_once(&ctx)
            .await
            .expect("refresh");
        assert!(
            !set.rows.iter().any(|row| row.pathway_pid == pathway),
            "a one-instance cohort must not be labelled: {set:?}"
        );
        assert!(set.suppressed_pathways >= 1, "and it must be counted");

        // Enrol four more so the cohort clears the floor of five.
        for _ in 0..4 {
            request
                .post(&format!("/api/care-pathways/{pathway}/instances"))
                .json(&json!({ "subject_ref": format!("person:{}", uuid::Uuid::new_v4()) }))
                .await
                .assert_status_ok();
        }
        let set = care_pathway_service::flow_metrics::refresh_once(&ctx)
            .await
            .expect("refresh");
        let row = set
            .rows
            .iter()
            .find(|row| row.pathway_pid == pathway)
            .expect("the pathway is exported once its cohort clears the floor");
        assert_eq!(row.instances, 5);

        // The same floor, generalised (spec T-14k): once this cohort
        // reaches 5, both TBA cohort endpoints stop suppressing too —
        // one shared decision (`suppression::is_suppressed`), not two
        // floors that could drift apart.
        let cohort: Value = request
            .get(&format!("/api/care-pathways/{pathway}/time-analysis"))
            .await
            .json();
        assert_eq!(cohort["suppressed"], false);
        assert!(!cohort["cohort"]["lead_time"].is_null());

        let constraints: Value = request
            .get(&format!("/api/care-pathways/{pathway}/constraints"))
            .await
            .json();
        assert_eq!(constraints["suppressed"], false);
        assert!(constraints["findings"].is_array());

        // The gauges carry it, labelled by pid — never by name, which a
        // rename would fork.
        let body = care_pathway_service::metrics::Metrics::global().render();
        assert!(
            body.contains(&format!(
                r#"care_pathway_flow_instances{{pathway="{pathway}"}} 5"#
            )),
            "missing the labelled series in: {body}"
        );
        assert!(
            body.contains("care_pathway_flow_last_refresh_timestamp_seconds"),
            "a scraper alerts on this going stale: {body}"
        );
        // Both bounds travel with the rows, so the gauges cannot be read
        // as the whole estate.
        assert!(body.contains("care_pathway_flow_pathways_suppressed"));
        assert!(body.contains("care_pathway_flow_pathways_dropped"));
    })
    .await;
}

/// The T-14b directly-follows process map: stage and step level, an
/// unrecognised `?level=`, and the T-14k suppression it shares with
/// the cohort views — withheld at `n = 1`, visible once the cohort
/// clears the floor, and `?mode=remove` dropping every entry while
/// suppressed.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn process_map_round_trip() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let created = request
            .post("/api/care-pathways")
            .json(&json!({
                "name": format!("process-map pathway {}", uuid::Uuid::new_v4()),
                "care_setting": "Outpatient",
                "condition_codes": [{"system": "Icd10", "code": "M54"}],
            }))
            .await;
        created.assert_status_ok();
        let template: Value = created.json();
        let pathway = template["pid"].as_str().expect("pathway pid").to_string();

        // One rich instance: two steps declared at enrolment, and two
        // segments (a self-loop-free triage -> treatment chain).
        let enrolled = request
            .post(&format!("/api/care-pathways/{pathway}/instances"))
            .json(&json!({
                "subject_ref": format!("person:{}", uuid::Uuid::new_v4()),
                "steps": ["consent", "review"],
            }))
            .await;
        enrolled.assert_status_ok();
        let instance: Value = enrolled.json();
        let pid = instance["pid"].as_str().expect("instance pid").to_string();

        request
            .post(&format!("/api/instances/{pid}/clock"))
            .json(&json!({ "event": "start", "at": day(0) }))
            .await
            .assert_status_ok();
        request
            .post(&format!("/api/instances/{pid}/clock"))
            .json(&json!({ "event": "stop", "at": day(20) }))
            .await
            .assert_status_ok();
        request
            .post(&format!("/api/instances/{pid}/segments"))
            .json(&json!({
                "label": "triage", "stage": "triage", "category": "value_adding",
                "started_at": day(1), "ended_at": day(2),
            }))
            .await
            .assert_status_ok();
        request
            .post(&format!("/api/instances/{pid}/segments"))
            .json(&json!({
                "label": "treatment", "stage": "treatment", "category": "value_adding",
                "started_at": day(5), "ended_at": day(8),
            }))
            .await
            .assert_status_ok();

        // ── n=1: below the floor, so every node/edge is withheld.
        let stage_map: Value = request
            .get(&format!("/api/care-pathways/{pathway}/process-map"))
            .await
            .json();
        assert_eq!(stage_map["level"], "stage");
        assert_eq!(stage_map["instances"], 1);
        let nodes = stage_map["nodes"].as_array().expect("nodes");
        assert!(!nodes.is_empty());
        for node in nodes {
            assert_eq!(node["suppressed"], true, "{node}");
            assert_eq!(node["instance_count"], Value::Null);
        }
        for edge in stage_map["edges"].as_array().expect("edges") {
            assert_eq!(edge["suppressed"], true, "{edge}");
        }

        // `?mode=remove` drops every suppressed entry entirely.
        let removed: Value = request
            .get(&format!(
                "/api/care-pathways/{pathway}/process-map?mode=remove"
            ))
            .await
            .json();
        assert!(removed["nodes"].as_array().expect("nodes").is_empty());
        assert!(removed["edges"].as_array().expect("edges").is_empty());

        // An unrecognised level is refused, not silently defaulted.
        assert_eq!(
            request
                .get(&format!(
                    "/api/care-pathways/{pathway}/process-map?level=nonsense"
                ))
                .await
                .status_code(),
            422
        );

        // ── Complete both declared steps on the rich instance.
        let detail: Value = request.get(&format!("/api/instances/{pid}")).await.json();
        for step in detail["steps"].as_array().expect("steps") {
            let step_pid = step["pid"].as_str().expect("step pid");
            request
                .post(&format!("/api/instances/{pid}/steps/{step_pid}/complete"))
                .await
                .assert_status_ok();
        }

        // ── Four more instances, each with a `triage` segment and a
        // completed `consent` step — same activities as the rich
        // instance, so `triage`/`consent` clear the per-node floor of
        // five while `treatment`/`review` (rich-instance-only) do not.
        // Suppression here is per node/edge, not per cohort: reaching
        // five *instances* is not the same as reaching five *visits to
        // this activity*, and that distinction is the point of the test.
        for i in 0..4 {
            let enrolled = request
                .post(&format!("/api/care-pathways/{pathway}/instances"))
                .json(&json!({
                    "subject_ref": format!("person:{}", uuid::Uuid::new_v4()),
                    "steps": ["consent"],
                }))
                .await;
            enrolled.assert_status_ok();
            let other: Value = enrolled.json();
            let other_pid = other["pid"].as_str().expect("instance pid").to_string();
            request
                .post(&format!("/api/instances/{other_pid}/segments"))
                .json(&json!({
                    "label": "triage", "stage": "triage", "category": "value_adding",
                    "started_at": day(1), "ended_at": day(2 + i),
                }))
                .await
                .assert_status_ok();
            let other_detail: Value = request
                .get(&format!("/api/instances/{other_pid}"))
                .await
                .json();
            let step_pid = other_detail["steps"][0]["pid"].as_str().expect("step pid");
            request
                .post(&format!(
                    "/api/instances/{other_pid}/steps/{step_pid}/complete"
                ))
                .await
                .assert_status_ok();
        }

        let by_activity = |nodes: &[Value], activity: &str| -> Value {
            nodes
                .iter()
                .find(|n| n["activity"] == activity)
                .unwrap_or(&Value::Null)
                .clone()
        };

        // ── Stage level: `triage` (5 visits) is now visible; `treatment`
        // (1 visit) stays withheld even though the cohort itself is now
        // well above the floor.
        let stage_map: Value = request
            .get(&format!("/api/care-pathways/{pathway}/process-map"))
            .await
            .json();
        assert_eq!(stage_map["instances"], 5);
        let stage_nodes = stage_map["nodes"].as_array().expect("nodes");
        let triage = by_activity(stage_nodes, "triage");
        assert_eq!(triage["suppressed"], false);
        assert_eq!(triage["instance_count"], 5);
        assert_eq!(triage["occurrence_count"], 5);
        assert!(triage["median_duration_days"].as_f64().unwrap() > 0.0);
        let treatment = by_activity(stage_nodes, "treatment");
        assert_eq!(
            treatment["suppressed"], true,
            "only one instance ever reaches treatment"
        );
        assert_eq!(treatment["instance_count"], Value::Null);
        assert_eq!(by_activity(stage_nodes, "start")["instance_count"], 5);
        assert_eq!(by_activity(stage_nodes, "end")["instance_count"], 5);
        let stage_edges = stage_map["edges"].as_array().expect("edges");
        let start_to_triage = stage_edges
            .iter()
            .find(|e| e["from"] == "start" && e["to"] == "triage")
            .expect("start->triage edge");
        assert_eq!(start_to_triage["suppressed"], false);
        assert_eq!(start_to_triage["instance_count"], 5);
        let triage_to_treatment = stage_edges
            .iter()
            .find(|e| e["from"] == "triage" && e["to"] == "treatment")
            .expect("triage->treatment edge is still listed, just withheld");
        assert_eq!(triage_to_treatment["suppressed"], true);
        assert_eq!(triage_to_treatment["median_gap_days"], Value::Null);

        // ── Step level: `consent` (5 visits) is visible; `review` (1
        // visit) stays withheld, same distinction as stage level.
        let step_map: Value = request
            .get(&format!(
                "/api/care-pathways/{pathway}/process-map?level=step"
            ))
            .await
            .json();
        assert_eq!(step_map["level"], "step");
        let step_nodes = step_map["nodes"].as_array().expect("nodes");
        let consent = by_activity(step_nodes, "consent");
        assert_eq!(consent["suppressed"], false, "{consent}");
        assert_eq!(consent["instance_count"], 5);
        assert_eq!(
            consent["median_duration_days"],
            Value::Null,
            "a step has no duration"
        );
        let review = by_activity(step_nodes, "review");
        assert_eq!(
            review["suppressed"], true,
            "only the rich instance declared review"
        );
        assert_eq!(by_activity(step_nodes, "start")["instance_count"], 5);
    })
    .await;
}

/// Journey variants (T-14c): the named parameters are echoed, an
/// overlap above `combination_window_days` combines into a canonical
/// alphabetical step, an unrecognised `filter` is refused, and
/// suppression (T-14k) folds a rare variant into `suppressed_instances`
/// while renormalising the visible variants' shares to still sum to 1.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn variants_round_trip() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let created = request
            .post("/api/care-pathways")
            .json(&json!({
                "name": format!("variants pathway {}", uuid::Uuid::new_v4()),
                "care_setting": "Outpatient",
                "condition_codes": [{"system": "Icd10", "code": "M54"}],
            }))
            .await;
        created.assert_status_ok();
        let template: Value = created.json();
        let pathway = template["pid"].as_str().expect("pathway pid").to_string();

        // Five instances sharing the same overlapping triage/treatment
        // pair — a 5-day overlap, which combines under a 1-day window.
        for _ in 0..5 {
            let enrolled = request
                .post(&format!("/api/care-pathways/{pathway}/instances"))
                .json(&json!({ "subject_ref": format!("person:{}", uuid::Uuid::new_v4()) }))
                .await;
            enrolled.assert_status_ok();
            let instance: Value = enrolled.json();
            let pid = instance["pid"].as_str().expect("instance pid").to_string();
            for (stage, start, end) in [("triage", 0, 10), ("treatment", 5, 15)] {
                request
                    .post(&format!("/api/instances/{pid}/segments"))
                    .json(&json!({
                        "label": stage, "stage": stage, "category": "value_adding",
                        "started_at": day(start), "ended_at": day(end),
                    }))
                    .await
                    .assert_status_ok();
            }
        }

        let report: Value = request
            .get(&format!(
                "/api/care-pathways/{pathway}/variants?combination_window_days=1"
            ))
            .await
            .json();
        assert_eq!(report["instances"], 5);
        assert_eq!(report["suppressed_instances"], 0);
        assert_eq!(report["params"]["combination_window_days"], 1.0);
        assert_eq!(report["params"]["filter"], "all");
        let visible = report["variants"].as_array().expect("variants");
        assert_eq!(
            visible.len(),
            1,
            "every instance shares one variant: {visible:?}"
        );
        assert_eq!(visible[0]["variant"], "treatment+triage");
        assert_eq!(visible[0]["frequency"], 5);
        assert_eq!(visible[0]["share"].as_f64().unwrap(), 1.0);
        assert_eq!(visible[0]["cumulative_share"].as_f64().unwrap(), 1.0);
        let overall_line = report["lines"]
            .as_array()
            .expect("lines")
            .iter()
            .find(|l| l["position"] == "overall")
            .expect("overall line");
        assert_eq!(overall_line["n"], 5);

        // An unrecognised filter is refused, not silently defaulted.
        assert_eq!(
            request
                .get(&format!(
                    "/api/care-pathways/{pathway}/variants?filter=nonsense"
                ))
                .await
                .status_code(),
            422
        );

        // A sixth instance with its own, unique journey stays below the
        // floor: its variant is suppressed, folded into
        // suppressed_instances, and the five-instance variant's share
        // is renormalised over the unsuppressed instances alone, so it
        // still reads as 1.0.
        let enrolled = request
            .post(&format!("/api/care-pathways/{pathway}/instances"))
            .json(&json!({ "subject_ref": format!("person:{}", uuid::Uuid::new_v4()) }))
            .await;
        enrolled.assert_status_ok();
        let sixth: Value = enrolled.json();
        let sixth_pid = sixth["pid"].as_str().expect("instance pid").to_string();
        request
            .post(&format!("/api/instances/{sixth_pid}/segments"))
            .json(&json!({
                "label": "discharge", "stage": "discharge", "category": "value_adding",
                "started_at": day(0), "ended_at": day(1),
            }))
            .await
            .assert_status_ok();

        let report: Value = request
            .get(&format!(
                "/api/care-pathways/{pathway}/variants?combination_window_days=1"
            ))
            .await
            .json();
        assert_eq!(report["instances"], 6);
        assert_eq!(
            report["suppressed_instances"], 1,
            "the lone discharge-only journey"
        );
        let visible = report["variants"].as_array().expect("variants");
        assert_eq!(
            visible.len(),
            1,
            "the rare variant is folded, not listed: {visible:?}"
        );
        assert_eq!(
            visible[0]["share"].as_f64().unwrap(),
            1.0,
            "renormalised over the unsuppressed"
        );
    })
    .await;
}

/// Stage anchors, delay decomposition, and anchored standards (T-14d):
/// a `?from_anchor=&to_anchor=` pair scores each instance's own
/// referral -> diagnostics interval rather than the whole clock; an
/// instance that never reaches the `to_anchor` counts as `unreached`
/// (a third verdict, excluded from `within`/`breached` but disclosed);
/// naming `cancer_fds_28_days` alone scores the same anchored interval
/// automatically, from its own catalogue-declared anchor, with no
/// anchor query at all; an explicit query anchor pair overrides even a
/// standard's own declared anchor; an unrecognised or one-sided anchor
/// pair falls back to the ordinary whole-clock score with a disclosed
/// `anchor_note`; and the whole-clock `rtt_18_weeks` standard is
/// unaffected throughout.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn anchored_compliance_round_trip() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let created = request
            .post("/api/care-pathways")
            .json(&json!({
                "name": format!("anchors pathway {}", uuid::Uuid::new_v4()),
                "care_setting": "Outpatient",
                "condition_codes": [{"system": "Icd10", "code": "M54"}],
            }))
            .await;
        created.assert_status_ok();
        let template: Value = created.json();
        let pathway = template["pid"].as_str().expect("pathway pid").to_string();

        // Instance A: referral on day 0, diagnostics on day 20 — a
        // 20-day referral -> diagnostics interval, inside a 100-day clock.
        let enrolled_a = request
            .post(&format!("/api/care-pathways/{pathway}/instances"))
            .json(&json!({ "subject_ref": format!("person:{}", uuid::Uuid::new_v4()) }))
            .await;
        enrolled_a.assert_status_ok();
        let a: Value = enrolled_a.json();
        let pid_a = a["pid"].as_str().expect("instance pid").to_string();
        request
            .post(&format!("/api/instances/{pid_a}/clock"))
            .json(&json!({ "event": "start", "at": day(0) }))
            .await
            .assert_status_ok();
        request
            .post(&format!("/api/instances/{pid_a}/clock"))
            .json(&json!({ "event": "stop", "at": day(100) }))
            .await
            .assert_status_ok();
        request
            .post(&format!("/api/instances/{pid_a}/segments"))
            .json(&json!({
                "label": "referred", "stage": "referral", "category": "value_adding",
                "started_at": day(0), "ended_at": day(1),
            }))
            .await
            .assert_status_ok();
        request
            .post(&format!("/api/instances/{pid_a}/segments"))
            .json(&json!({
                "label": "scanned", "stage": "diagnostics", "category": "value_adding",
                "started_at": day(20), "ended_at": day(21),
            }))
            .await
            .assert_status_ok();

        // Instance B: referred, but never reaches diagnostics.
        let enrolled_b = request
            .post(&format!("/api/care-pathways/{pathway}/instances"))
            .json(&json!({ "subject_ref": format!("person:{}", uuid::Uuid::new_v4()) }))
            .await;
        enrolled_b.assert_status_ok();
        let b: Value = enrolled_b.json();
        let pid_b = b["pid"].as_str().expect("instance pid").to_string();
        request
            .post(&format!("/api/instances/{pid_b}/clock"))
            .json(&json!({ "event": "start", "at": day(0) }))
            .await
            .assert_status_ok();
        request
            .post(&format!("/api/instances/{pid_b}/clock"))
            .json(&json!({ "event": "stop", "at": day(100) }))
            .await
            .assert_status_ok();
        request
            .post(&format!("/api/instances/{pid_b}/segments"))
            .json(&json!({
                "label": "referred", "stage": "referral", "category": "value_adding",
                "started_at": day(0), "ended_at": day(1),
            }))
            .await
            .assert_status_ok();
        request
            .post(&format!("/api/instances/{pid_b}/segments"))
            .json(&json!({
                "label": "triaged", "stage": "triage", "category": "value_adding",
                "started_at": day(3), "ended_at": day(4),
            }))
            .await
            .assert_status_ok();

        // ── Anchored: referral -> diagnostics, 28-day threshold.
        // Instance A (20 days) is within; instance B never reaches
        // diagnostics, so it is `unreached`, not `breached`.
        let anchored: Value = request
            .get(&format!(
                "/api/care-pathways/{pathway}/time-analysis\
                 ?from_anchor=referral&to_anchor=diagnostics&target_days=28"
            ))
            .await
            .json();
        let compliance = &anchored["compliance"];
        assert_eq!(compliance["within"], 1);
        assert_eq!(compliance["breached"], 0);
        assert_eq!(
            compliance["unreached"], 1,
            "excluded from within/breached, but disclosed"
        );
        assert_eq!(compliance["anchor_note"], Value::Null);

        // ── The whole-clock rtt_18_weeks standard, no anchor named:
        // both instances sit well inside 100 days against a 126-day
        // threshold, and `unreached` never applies to a whole-clock score.
        let whole_clock: Value = request
            .get(&format!(
                "/api/care-pathways/{pathway}/time-analysis?standard=rtt_18_weeks"
            ))
            .await
            .json();
        let compliance = &whole_clock["compliance"];
        assert_eq!(compliance["within"], 2);
        assert_eq!(compliance["breached"], 0);
        assert_eq!(compliance["unreached"], 0);
        assert_eq!(compliance["anchor_note"], Value::Null);

        // ── `cancer_fds_28_days` declares its own referral ->
        // diagnostics anchor in the catalogue (T-14d), so naming just
        // the standard — no `from_anchor`/`to_anchor` in the query at
        // all — scores the same anchored interval automatically.
        let catalogue_anchored: Value = request
            .get(&format!(
                "/api/care-pathways/{pathway}/time-analysis?standard=cancer_fds_28_days"
            ))
            .await
            .json();
        let compliance = &catalogue_anchored["compliance"];
        assert_eq!(compliance["standard"], "cancer_fds_28_days");
        assert_eq!(compliance["within"], 1);
        assert_eq!(compliance["breached"], 0);
        assert_eq!(
            compliance["unreached"], 1,
            "instance B never reaches diagnostics, via the catalogue's own anchor"
        );
        assert_eq!(compliance["anchor_note"], Value::Null);

        // ── An explicit query anchor pair overrides even a standard's
        // own declared anchor: referral -> triage instead of ->
        // diagnostics swaps which instance is within/unreached, while
        // cancer_fds_28_days's own 28-day threshold still applies.
        let overridden: Value = request
            .get(&format!(
                "/api/care-pathways/{pathway}/time-analysis\
                 ?standard=cancer_fds_28_days&from_anchor=referral&to_anchor=triage"
            ))
            .await
            .json();
        let compliance = &overridden["compliance"];
        assert_eq!(compliance["standard"], "cancer_fds_28_days");
        assert_eq!(
            compliance["within"], 1,
            "instance B reaches triage on day 3; instance A never does"
        );
        assert_eq!(compliance["unreached"], 1);

        // ── An unrecognised `to_anchor` falls back to the whole-clock
        // score, with the fallback disclosed rather than silent.
        let unrecognised: Value = request
            .get(&format!(
                "/api/care-pathways/{pathway}/time-analysis\
                 ?from_anchor=referral&to_anchor=not_a_stage&target_days=28"
            ))
            .await
            .json();
        let compliance = &unrecognised["compliance"];
        assert_eq!(compliance["anchor_note"], "unknown_to_anchor");
        assert_eq!(
            compliance["unreached"], 0,
            "fell back to the whole-clock score, which has no unreached"
        );

        // ── A one-sided anchor pair (only `from_anchor` given) is the
        // same disclosed fallback, not a hard error.
        let one_sided: Value = request
            .get(&format!(
                "/api/care-pathways/{pathway}/time-analysis\
                 ?from_anchor=referral&target_days=28"
            ))
            .await
            .json();
        assert_eq!(one_sided["compliance"]["anchor_note"], "to_anchor_missing");
    })
    .await;
}

/// Censoring-aware cohort statistics (T-14e): `survival.time_to_close`
/// treats every open instance as right-censored rather than mixing a
/// running lead time in as if it had actually closed; `?discontinued=`
/// selects whether a discontinued closure counts as the event or a
/// censoring; `survival.time_to_anchor` reuses T-14d's own
/// `from_anchor`/`to_anchor` pair and excludes an instance that never
/// reached `from_anchor` entirely; and the whole `survival` block is
/// withheld under the identical suppression decision as the
/// percentile detail.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
#[allow(clippy::too_many_lines)] // five instances, three assertions each
async fn censoring_aware_survival_round_trip() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let created = request
            .post("/api/care-pathways")
            .json(&json!({
                "name": format!("survival pathway {}", uuid::Uuid::new_v4()),
                "care_setting": "Outpatient",
                "condition_codes": [{"system": "Icd10", "code": "M54"}],
            }))
            .await;
        created.assert_status_ok();
        let template: Value = created.json();
        let pathway = template["pid"].as_str().expect("pathway pid").to_string();

        let enroll_with_clock = |stop_day: Option<i64>| {
            let request = &request;
            let pathway = pathway.clone();
            async move {
                let enrolled = request
                    .post(&format!("/api/care-pathways/{pathway}/instances"))
                    .json(&json!({ "subject_ref": format!("person:{}", uuid::Uuid::new_v4()) }))
                    .await;
                enrolled.assert_status_ok();
                let instance: Value = enrolled.json();
                let pid = instance["pid"].as_str().expect("instance pid").to_string();
                request
                    .post(&format!("/api/instances/{pid}/clock"))
                    .json(&json!({ "event": "start", "at": day(0) }))
                    .await
                    .assert_status_ok();
                if let Some(stop_day) = stop_day {
                    request
                        .post(&format!("/api/instances/{pid}/clock"))
                        .json(&json!({ "event": "stop", "at": day(stop_day) }))
                        .await
                        .assert_status_ok();
                }
                pid
            }
        };

        // A: closes (completed) at day 10 -- always an event. Closing
        // stamps `clock_stop_at` to the real moment of closure (spec
        // §12.3), so the explicit day-10 stop has to be set *after*
        // `/status`, not before, or `/status` overwrites it.
        let pid_a = enroll_with_clock(None).await;
        request
            .post(&format!("/api/instances/{pid_a}/status"))
            .json(&json!({ "to": "completed", "outcome": "improved" }))
            .await
            .assert_status_ok();
        request
            .post(&format!("/api/instances/{pid_a}/clock"))
            .json(&json!({ "event": "stop", "at": day(10) }))
            .await
            .assert_status_ok();

        // B: closes (discontinued) at day 20 -- event or censor,
        // depending on `?discontinued=`. Same ordering as A.
        let pid_b = enroll_with_clock(None).await;
        request
            .post(&format!("/api/instances/{pid_b}/status"))
            .json(&json!({ "to": "discontinued", "reason": "patient moved away" }))
            .await
            .assert_status_ok();
        request
            .post(&format!("/api/instances/{pid_b}/clock"))
            .json(&json!({ "event": "stop", "at": day(20) }))
            .await
            .assert_status_ok();

        // C: never closes -- always right-censored, whatever the
        // real elapsed time since day 0 turns out to be.
        let _pid_c = enroll_with_clock(None).await;

        // D: reaches referral (day 0) and diagnostics (day 5) -- a
        // 5-day time-to-anchor event.
        let pid_d = enroll_with_clock(Some(100)).await;
        request
            .post(&format!("/api/instances/{pid_d}/segments"))
            .json(&json!({
                "label": "referred", "stage": "referral", "category": "value_adding",
                "started_at": day(0), "ended_at": day(1),
            }))
            .await
            .assert_status_ok();
        request
            .post(&format!("/api/instances/{pid_d}/segments"))
            .json(&json!({
                "label": "scanned", "stage": "diagnostics", "category": "value_adding",
                "started_at": day(5), "ended_at": day(6),
            }))
            .await
            .assert_status_ok();

        // E: reaches referral but never diagnostics -- excluded from
        // nothing (time-to-close still applies), but right-censored,
        // relative to referral, at the clock's own day-30 stop, in
        // time-to-anchor.
        let pid_e = enroll_with_clock(Some(30)).await;
        request
            .post(&format!("/api/instances/{pid_e}/segments"))
            .json(&json!({
                "label": "referred", "stage": "referral", "category": "value_adding",
                "started_at": day(0), "ended_at": day(1),
            }))
            .await
            .assert_status_ok();

        // ── Default `?discontinued=event`: A (completed) and B
        // (discontinued) are both events; C, D, and E are all
        // right-censored, because none of them ever closed via
        // `/status` — an explicit clock stop controls *when*, not
        // *whether*, this instance is closed.
        let default_mode: Value = request
            .get(&format!("/api/care-pathways/{pathway}/time-analysis"))
            .await
            .json();
        assert_eq!(default_mode["survival"]["discontinued"], "event");
        let close = &default_mode["survival"]["time_to_close"];
        assert_eq!(close["events"], 2, "A and B close");
        assert_eq!(close["censored"], 3, "C, D, and E stay active");
        let steps = close["steps"].as_array().expect("steps");
        assert!(
            steps
                .iter()
                .any(|s| s["time_ms"] == 10 * DAY_MS && s["events"] == 1),
            "A's event at day 10: {steps:?}"
        );
        assert!(
            steps
                .iter()
                .any(|s| s["time_ms"] == 20 * DAY_MS && s["events"] == 1),
            "B's event at day 20 (discontinued counts as event): {steps:?}"
        );

        // ── `?discontinued=censor`: B moves from an event to a
        // censoring, so the count of events drops by one and the
        // count of censored rises by one; B's own time no longer
        // shows up as an event step.
        let censor_mode: Value = request
            .get(&format!(
                "/api/care-pathways/{pathway}/time-analysis?discontinued=censor"
            ))
            .await
            .json();
        assert_eq!(censor_mode["survival"]["discontinued"], "censor");
        let close = &censor_mode["survival"]["time_to_close"];
        assert_eq!(close["events"], 1, "only A still closes as an event");
        assert_eq!(close["censored"], 4, "B moved from event to censored");
        let steps = close["steps"].as_array().expect("steps");
        assert!(
            !steps.iter().any(|s| s["time_ms"] == 20 * DAY_MS),
            "B no longer produces an event step: {steps:?}"
        );

        // ── An unrecognised `discontinued` value is refused, not
        // silently defaulted.
        assert_eq!(
            request
                .get(&format!(
                    "/api/care-pathways/{pathway}/time-analysis?discontinued=nonsense"
                ))
                .await
                .status_code(),
            422
        );

        // ── time-to-anchor (referral -> diagnostics): D is a 5-day
        // event; E is censored at day 30 (E reached referral on day 0
        // but never diagnostics); A, B, C never reached referral at
        // all and are excluded entirely, not assigned an arbitrary
        // origin.
        let anchored: Value = request
            .get(&format!(
                "/api/care-pathways/{pathway}/time-analysis\
                 ?from_anchor=referral&to_anchor=diagnostics"
            ))
            .await
            .json();
        let time_to_anchor = &anchored["survival"]["time_to_anchor"];
        assert_eq!(time_to_anchor["events"], 1, "only D reaches diagnostics");
        assert_eq!(
            time_to_anchor["censored"], 1,
            "only E reached referral without diagnostics"
        );
        let steps = time_to_anchor["steps"].as_array().expect("steps");
        assert!(
            steps
                .iter()
                .any(|s| s["time_ms"] == 5 * DAY_MS && s["events"] == 1),
            "D's 5-day event: {steps:?}"
        );

        // ── Naming no anchor pair at all omits time_to_anchor
        // entirely -- it is not a null placeholder for a feature that
        // wasn't asked for.
        assert_eq!(default_mode["survival"]["time_to_anchor"], Value::Null);

        // ── Suppression: a single-instance pathway withholds the
        // whole survival block under the identical decision that
        // withholds the percentile detail.
        let solo = request
            .post("/api/care-pathways")
            .json(&json!({
                "name": format!("solo survival pathway {}", uuid::Uuid::new_v4()),
                "care_setting": "Outpatient",
                "condition_codes": [{"system": "Icd10", "code": "M54"}],
            }))
            .await;
        solo.assert_status_ok();
        let solo_template: Value = solo.json();
        let solo_pathway = solo_template["pid"].as_str().expect("pathway pid");
        let solo_enrolled = request
            .post(&format!("/api/care-pathways/{solo_pathway}/instances"))
            .json(&json!({ "subject_ref": format!("person:{}", uuid::Uuid::new_v4()) }))
            .await;
        solo_enrolled.assert_status_ok();
        let solo_report: Value = request
            .get(&format!("/api/care-pathways/{solo_pathway}/time-analysis"))
            .await
            .json();
        assert_eq!(solo_report["suppressed"], true);
        assert_eq!(
            solo_report["survival"],
            Value::Null,
            "withheld, same as the percentile detail"
        );
    })
    .await;
}
