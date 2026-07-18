//! Request-level test suites: topology CRUD in [`topology`], the
//! admission→discharge journey + the double-placement race in
//! [`flows`], and the board reads (whiteboard shape, ETag conditional
//! GET, locate audit) in [`boards`].

mod boards;
mod flows;
mod topology;

use serde_json::{Value, json};

/// Build the demo topology one test needs: a site, one ward (kind /
/// specialty as given), one `flexible` bay, and `beds` beds. Returns
/// `(ward_pid, bed_pids)`.
pub async fn seed_ward(
    request: &axum_test::TestServer,
    kind: &str,
    specialty: Option<&str>,
    beds: usize,
) -> (String, Vec<String>) {
    let site: Value = request
        .post("/api/sites")
        .json(&json!({ "name": "Test Site" }))
        .await
        .json();
    let ward: Value = request
        .post("/api/wards")
        .json(&json!({
            "site_pid": site["pid"],
            "name": format!("Test Ward ({kind})"),
            "code": "TW",
            "kind": kind,
            "specialty": specialty,
        }))
        .await
        .json();
    let bay: Value = request
        .post("/api/bays")
        .json(&json!({
            "ward_pid": ward["pid"],
            "name": "Bay T",
            "sex_designation": "flexible",
        }))
        .await
        .json();
    let mut bed_pids = Vec::new();
    for n in 1..=beds {
        let bed: Value = request
            .post("/api/beds")
            .json(&json!({
                "bay_pid": bay["pid"],
                "number": format!("TW-{n}"),
                "oxygen": true,
                "virtual": kind == "virtual",
            }))
            .await
            .json();
        bed_pids.push(bed["pid"].as_str().expect("bed pid").to_string());
    }
    (ward["pid"].as_str().expect("ward pid").to_string(), bed_pids)
}

/// A fresh synthetic `person:` URN.
pub fn a_person() -> String {
    format!("person:{}", uuid::Uuid::new_v4())
}
