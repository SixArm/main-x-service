//! Cross-service journey links: the `continues_as` write side
//! (`agents/share/cross-service-linking.md` §4.1/§4.2) and the
//! reconciliation pull the aggregator uses.

use axum_test::TestServer;
use care_pathway_service::app::App;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

/// Seed a pathway template and one enrolled instance.
async fn seed(request: &TestServer) -> (String, String) {
    let created = request
        .post("/api/care-pathways")
        .json(&json!({
            "name": format!("Journey pathway {}", uuid::Uuid::new_v4()),
            "care_setting": "Outpatient",
            "condition_codes": [{"system": "Icd10", "code": "I63"}],
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
    (pathway, instance["pid"].as_str().expect("pid").to_string())
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
#[allow(clippy::too_many_lines)] // one seeded journey, the whole link surface
async fn a_journey_links_into_the_next_episode() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let (_pathway, instance) = seed(&request).await;
        let stay = format!("patient_flow_stay:{}", uuid::Uuid::new_v4());

        // ── Assert the journey edge.
        let created = request
            .post(&format!("/api/instances/{instance}/links"))
            .json(&json!({
                "kind": "continues_as",
                "to_ref": stay,
                "valid_from": "2026-02-01",
            }))
            .await;
        created.assert_status_ok();
        let edge: Value = created.json();
        assert_eq!(edge["kind"], "continues_as");
        assert_eq!(edge["to_ref"], stay);
        assert_eq!(edge["provenance"], "operator");
        // The edge originates from the *instance*, not the template — a
        // journey belongs to an enrolment.
        assert_eq!(
            edge["from_ref"],
            format!("care_pathway_instance:{instance}")
        );
        let edge_id = edge["id"].as_str().expect("edge id").to_string();

        // ── Re-asserting the same edge is idempotent: same id, no
        // duplicate. The aggregator dedupes on edge_id, so a retried
        // write must not produce a second edge.
        let again: Value = request
            .post(&format!("/api/instances/{instance}/links"))
            .json(&json!({
                "kind": "continues_as", "to_ref": stay, "valid_from": "2026-02-01",
            }))
            .await
            .json();
        assert_eq!(again["id"], edge_id, "re-assertion is idempotent");
        let listed: Value = request
            .get(&format!("/api/instances/{instance}/links"))
            .await
            .json();
        assert_eq!(listed.as_array().expect("links").len(), 1);

        // ── The closed registry is enforced at the boundary.
        for (kind, to_ref, why) in [
            (
                "continues_as",
                format!("person:{}", uuid::Uuid::new_v4()),
                "a person is not an episode",
            ),
            (
                "same_identity",
                format!("worker:{}", uuid::Uuid::new_v4()),
                "not a kind this service originates",
            ),
            (
                "teleports_to",
                format!("case:{}", uuid::Uuid::new_v4()),
                "unknown kind",
            ),
            ("continues_as", "not-a-ref".to_string(), "malformed ref"),
        ] {
            assert_eq!(
                request
                    .post(&format!("/api/instances/{instance}/links"))
                    .json(&json!({ "kind": kind, "to_ref": to_ref }))
                    .await
                    .status_code(),
                422,
                "refused: {why}"
            );
        }

        // A journey cannot continue as itself — that edge would be a
        // one-node cycle and a stitched timeline would not terminate.
        assert_eq!(
            request
                .post(&format!("/api/instances/{instance}/links"))
                .json(&json!({
                    "kind": "continues_as",
                    "to_ref": format!("care_pathway_instance:{instance}"),
                }))
                .await
                .status_code(),
            422
        );

        // ── A transfer to another pathway is permitted.
        let (_p2, other) = seed(&request).await;
        request
            .post(&format!("/api/instances/{instance}/links"))
            .json(&json!({
                "kind": "continues_as",
                "to_ref": format!("care_pathway_instance:{other}"),
            }))
            .await
            .assert_status_ok();

        // ── The reconciliation pull returns the canonical §4.2 shape,
        // which the aggregator deserializes directly.
        let bulk: Value = request.get("/api/instances/links").await.json();
        let edges = bulk["edges"].as_array().expect("edges");
        assert!(edges.len() >= 2);
        let mine = edges
            .iter()
            .find(|e| e["edge_id"] == edge_id)
            .expect("our edge is in the pull");
        for key in ["edge_id", "from_ref", "to_ref", "edge_kind", "provenance"] {
            assert!(mine[key].is_string(), "missing {key} in {mine}");
        }
        assert_eq!(mine["edge_kind"], "continues_as");
        assert_eq!(bulk["capped"], false);

        // ── The write is audited (high-sensitivity kind, §10).
        let audit: Value = request
            .get(&format!("/api/care-pathways/{instance}/audit"))
            .await
            .json();
        let actions: Vec<&str> = audit
            .as_array()
            .map(|rows| rows.iter().filter_map(|r| r["action"].as_str()).collect())
            .unwrap_or_default();
        assert!(actions.contains(&"linked"), "got {actions:?}");

        // ── Withdrawing removes it from the active list.
        request
            .delete(&format!("/api/instances/{instance}/links/{edge_id}"))
            .await
            .assert_status_ok();
        let after: Value = request
            .get(&format!("/api/instances/{instance}/links"))
            .await
            .json();
        assert_eq!(after.as_array().expect("links").len(), 1, "one left");

        // Withdrawing twice is a 404, not a silent success.
        assert_eq!(
            request
                .delete(&format!("/api/instances/{instance}/links/{edge_id}"))
                .await
                .status_code(),
            404
        );

        // ── An edge cannot be withdrawn through another journey.
        let live: Value = request
            .get(&format!("/api/instances/{instance}/links"))
            .await
            .json();
        let live_id = live[0]["id"].as_str().expect("live edge").to_string();
        assert_eq!(
            request
                .delete(&format!("/api/instances/{other}/links/{live_id}"))
                .await
                .status_code(),
            404,
            "an edge is scoped to its own journey"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
#[allow(clippy::too_many_lines)] // one seeded chain, the whole stitch surface
async fn a_journey_stitches_across_its_links() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let (_p1, first) = seed(&request).await;
        let (_p2, second) = seed(&request).await;

        // A single-leg journey is already a complete journey.
        let solo: Value = request
            .get(&format!("/api/instances/{first}/journey"))
            .await
            .json();
        assert_eq!(solo["legs"].as_array().expect("legs").len(), 1);
        assert_eq!(solo["totals"]["legs_unresolved"], 0);
        assert!(solo["totals"]["lead_time_ms"].is_i64());

        // ── Transfer to another pathway: a local leg, needing no
        // configuration and no HTTP at all.
        request
            .post(&format!("/api/instances/{first}/links"))
            .json(&json!({
                "kind": "continues_as",
                "to_ref": format!("care_pathway_instance:{second}"),
            }))
            .await
            .assert_status_ok();

        let stitched: Value = request
            .get(&format!("/api/instances/{first}/journey"))
            .await
            .json();
        let legs = stitched["legs"].as_array().expect("legs");
        assert_eq!(legs.len(), 2, "the transfer is followed");
        assert!(legs.iter().all(|l| l["status"] == "resolved"));
        assert_eq!(legs[0]["hop"], 0);
        assert_eq!(legs[1]["hop"], 1);
        assert_eq!(
            legs[1]["entity_ref"],
            format!("care_pathway_instance:{second}")
        );
        assert_eq!(stitched["totals"]["legs_resolved"], 2);
        assert_eq!(stitched["totals"]["legs_unresolved"], 0);
        assert!(
            stitched["totals"]["lead_time_ms"].is_i64(),
            "every leg resolved, so the totals stand"
        );
        assert_eq!(stitched["totals"]["reason"], Value::Null);

        // ── A leg into an unconfigured peer: the link is known, the
        // timeline is not requested, and every combined figure is
        // withheld rather than understated.
        request
            .post(&format!("/api/instances/{second}/links"))
            .json(&json!({
                "kind": "continues_as",
                "to_ref": format!("patient_flow_stay:{}", uuid::Uuid::new_v4()),
            }))
            .await
            .assert_status_ok();

        let partial: Value = request
            .get(&format!("/api/instances/{first}/journey"))
            .await
            .json();
        let legs = partial["legs"].as_array().expect("legs");
        assert_eq!(legs.len(), 3);
        let remote = legs
            .iter()
            .find(|l| {
                l["entity_ref"]
                    .as_str()
                    .unwrap_or("")
                    .starts_with("patient_flow_stay:")
            })
            .expect("the remote leg is listed, not dropped");
        assert_eq!(remote["status"], "not_configured");
        assert_eq!(remote["hop"], 2);
        assert!(
            remote["detail"]
                .as_str()
                .unwrap_or("")
                .contains("not requested")
        );

        // The resolved legs still report their own figures...
        assert_eq!(partial["totals"]["legs_resolved"], 2);
        assert_eq!(partial["totals"]["legs_unresolved"], 1);
        for leg in legs.iter().filter(|l| l["status"] == "resolved") {
            assert!(leg["lead_time_ms"].is_i64());
        }
        // ...but no combined total is published, because it would
        // understate the journey by exactly the leg nobody could read.
        assert_eq!(partial["totals"]["lead_time_ms"], Value::Null);
        assert_eq!(partial["totals"]["value_adding_ratio"], Value::Null);
        let reason = partial["totals"]["reason"]
            .as_str()
            .expect("a null says why");
        assert!(reason.contains("could not be read"), "{reason}");

        // ── A ring terminates. The write path refuses a self-link, but
        // second → first is invisible from either end at write time.
        request
            .post(&format!("/api/instances/{second}/links"))
            .json(&json!({
                "kind": "continues_as",
                "to_ref": format!("care_pathway_instance:{first}"),
            }))
            .await
            .assert_status_ok();
        let ringed: Value = request
            .get(&format!("/api/instances/{first}/journey"))
            .await
            .json();
        let refs: Vec<&str> = ringed["legs"]
            .as_array()
            .expect("legs")
            .iter()
            .filter_map(|l| l["entity_ref"].as_str())
            .filter(|r| !r.is_empty())
            .collect();
        let start = format!("care_pathway_instance:{first}");
        assert_eq!(
            refs.iter().filter(|r| **r == start).count(),
            1,
            "the start is visited once, not revisited round the ring: {refs:?}"
        );

        // ── The response explains its own semantics rather than leaving
        // them to be inferred.
        let note = ringed["note"].as_str().unwrap_or_default();
        assert!(note.contains("your** credential"), "{note}");
        assert!(note.contains("earliest clock start"), "{note}");
    })
    .await;
}
