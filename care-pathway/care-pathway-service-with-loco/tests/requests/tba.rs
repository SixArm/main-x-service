//! The time-based-analysis round trip: set a clock, record segments,
//! and read the derived per-instance, timeline, cohort, constraint and
//! flow views. Pins the contract in `spec/time-based-analysis.md` §14.3.

use axum_test::TestServer;
use care_pathway_service::app::App;
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
            request
                .get(&format!(
                    "/api/care-pathways/{pathway}/time-analysis?standard=nope"
                ))
                .await
                .status_code(),
            422,
            "unknown standard is refused, not ignored"
        );

        // ── Constraints name their rule and rank by recoverable time.
        let constraints: Value = request
            .get(&format!("/api/care-pathways/{pathway}/constraints"))
            .await
            .json();
        let findings = constraints["findings"].as_array().expect("findings");
        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f["rule"] == "longest_gap"));
        let recoverable: Vec<i64> = findings
            .iter()
            .map(|f| f["recoverable_ms"].as_i64().unwrap_or(0))
            .collect();
        assert!(
            recoverable.windows(2).all(|w| w[0] >= w[1]),
            "ordered by recoverable time: {recoverable:?}"
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
