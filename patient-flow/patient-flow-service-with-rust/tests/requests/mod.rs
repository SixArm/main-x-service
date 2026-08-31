//! Request-level test suites: topology CRUD in [`topology`], the
//! admission→discharge journey + the double-placement race in
//! [`flows`], and the board reads (whiteboard shape, ETag conditional
//! GET, locate audit) in [`boards`].

mod boards;
mod flows;
mod topology;

/// Build the demo topology one test needs: a site, one ward (kind /
/// specialty as given), one `flexible` bay, and `beds` beds. Returns
/// `(ward_pid, bed_pids)`.
///
/// A macro, not a function: the `request` handed to each test by
/// loco's `request()` helper is a `loco_rs`-internal-pinned
/// `axum_test::TestServer` — a type this crate cannot name directly
/// without pulling in that exact same `axum-test` version as a direct
/// dependency (defeating the point of tracking a newer one for the
/// dev-dependency's own sake). Expanding inline sidesteps naming the
/// type at all; the actual value's type is inferred at each call site.
macro_rules! seed_ward {
    ($request:expr, $kind:expr, $specialty:expr, $beds:expr) => {
        async {
            let request = $request;
            let kind = $kind;
            let specialty: Option<&str> = $specialty;
            let beds: usize = $beds;
            let site: serde_json::Value = request
                .post("/api/sites")
                .json(&serde_json::json!({ "name": "Test Site" }))
                .await
                .json();
            let ward: serde_json::Value = request
                .post("/api/wards")
                .json(&serde_json::json!({
                    "site_pid": site["pid"],
                    "name": format!("Test Ward ({kind})"),
                    "code": "TW",
                    "kind": kind,
                    "specialty": specialty,
                }))
                .await
                .json();
            let bay: serde_json::Value = request
                .post("/api/bays")
                .json(&serde_json::json!({
                    "ward_pid": ward["pid"],
                    "name": "Bay T",
                    "sex_designation": "flexible",
                }))
                .await
                .json();
            let mut bed_pids = Vec::new();
            for n in 1..=beds {
                let bed: serde_json::Value = request
                    .post("/api/beds")
                    .json(&serde_json::json!({
                        "bay_pid": bay["pid"],
                        "number": format!("TW-{n}"),
                        "oxygen": true,
                        "virtual": kind == "virtual",
                    }))
                    .await
                    .json();
                bed_pids.push(bed["pid"].as_str().expect("bed pid").to_string());
            }
            (
                ward["pid"].as_str().expect("ward pid").to_string(),
                bed_pids,
            )
        }
    };
}
pub(crate) use seed_ward;

/// A fresh synthetic `person:` URN.
pub fn a_person() -> String {
    format!("person:{}", uuid::Uuid::new_v4())
}
