//! Request-level test suites: the sales journey in [`sales`], the
//! consent-gated marketing journey in [`marketing`], and the support
//! journey + contract pins in [`support`].

mod insights;
mod marketing;
mod sales;
mod support;

use serde_json::{Value, json};

/// A fresh synthetic `person:` URN.
pub fn a_person() -> String {
    format!("person:{}", uuid::Uuid::new_v4())
}

/// A fresh synthetic `worker:` URN.
pub fn a_worker() -> String {
    format!("worker:{}", uuid::Uuid::new_v4())
}

/// Create the demo pipeline (3 open stages + Won + Lost). Returns
/// `(pipeline_pid, stage_pids)`.
pub async fn seed_pipeline(request: &axum_test::TestServer) -> (String, Vec<String>) {
    let created: Value = request
        .post("/api/pipelines")
        .json(&json!({
            "name": "Test Pipeline",
            "stages": [
                { "name": "Qualification", "probability_percent": 10 },
                { "name": "Proposal", "probability_percent": 50 },
                { "name": "Won", "probability_percent": 100, "is_won": true },
                { "name": "Lost", "probability_percent": 0, "is_lost": true },
            ],
        }))
        .await
        .json();
    let stages = created["stage_pids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p.as_str().unwrap().to_string())
        .collect();
    (created["pid"].as_str().unwrap().to_string(), stages)
}

/// Create a consented contact. Returns the pid.
pub async fn seed_contact(request: &axum_test::TestServer, name: &str) -> String {
    let created: Value = request
        .post("/api/contacts")
        .json(&json!({ "person_ref": a_person(), "display_name": name }))
        .await
        .json();
    let pid = created["pid"].as_str().unwrap().to_string();
    request
        .post(&format!("/api/contacts/{pid}/consent"))
        .json(&json!({ "action": "granted", "source": "web form" }))
        .await
        .assert_status_ok();
    pid
}
