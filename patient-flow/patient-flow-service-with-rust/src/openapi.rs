//! Hand-written `OpenAPI` 3 description of the patient-flow REST API.
//!
//! Summary-level by design: every path and verb is present with its
//! request/response essentials; the full field-by-field shapes live in
//! the spec (`../spec/domain-model.md`, `../spec/whiteboard.md`).

use serde_json::{Value, json};

/// The full `OpenAPI` document, served at `/api-docs/openapi.json`.
#[must_use]
pub fn spec() -> Value {
    let ok = |desc: &str| json!({ "200": { "description": desc } });
    let created = json!({
        "200": { "description": "Created: {pid}" },
        "422": { "description": "Validation failure" }
    });
    json!({
        "openapi": "3.0.3",
        "info": {
            "title": "Patient Flow Service API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "Hospital patient flow & bed management: topology (sites/wards/bays/beds), the bed state machine, inpatient stays (SAFER, Red2Green, DTOC), bed requests with rule-checked allocation, infection control, virtual wards, and derived whiteboard / at-a-glance / locate / capacity reads. Patients and staff are referenced by EntityRef URNs (person:<uuid>, worker:<uuid>). Validation failures return 422. API version is negotiated with the Accepts-version header (1.0)."
        },
        "paths": {
            "/api/sites": {
                "post": { "tags": ["topology"], "summary": "Create a site", "responses": created },
                "get": { "tags": ["topology"], "summary": "List active sites", "responses": ok("Sites") }
            },
            "/api/sites/{pid}": { "delete": { "tags": ["topology"], "summary": "Soft-delete a site", "responses": ok("Deleted") } },
            "/api/wards": {
                "post": { "tags": ["topology"], "summary": "Create a ward (kind: inpatient|assessment|virtual)", "responses": created },
                "get": { "tags": ["topology"], "summary": "List active wards", "responses": ok("Wards") }
            },
            "/api/wards/{pid}": {
                "get": { "tags": ["topology"], "summary": "Fetch a ward", "responses": ok("Ward") },
                "put": { "tags": ["topology"], "summary": "Update ward (open/escalation/closed_to_admissions/…)", "responses": ok("Updated ward") },
                "delete": { "tags": ["topology"], "summary": "Soft-delete a ward", "responses": ok("Deleted") }
            },
            "/api/bays": { "post": { "tags": ["topology"], "summary": "Create a bay (sex_designation, side_room)", "responses": created } },
            "/api/bays/{pid}": {
                "put": { "tags": ["topology"], "summary": "Update bay (designation / closed_to_admissions)", "responses": ok("Updated bay") },
                "delete": { "tags": ["topology"], "summary": "Soft-delete a bay", "responses": ok("Deleted") }
            },
            "/api/beds": { "post": { "tags": ["topology"], "summary": "Create a bed (starts available)", "responses": created } },
            "/api/beds/{pid}": {
                "get": { "tags": ["topology"], "summary": "Fetch a bed", "responses": ok("Bed") },
                "delete": { "tags": ["topology"], "summary": "Soft-delete a bed", "responses": ok("Deleted") }
            },
            "/api/beds/{pid}/state": { "post": {
                "tags": ["beds"],
                "summary": "Apply a bed state transition (allocate|release|clean_start|clean_complete|close|reopen)",
                "responses": { "200": { "description": "The updated bed" }, "422": { "description": "Illegal transition (names the current state)" } }
            } },
            "/api/stays": { "post": {
                "tags": ["stays"],
                "summary": "Admit a patient (person_ref URN) into an available/reserved bed",
                "responses": { "200": { "description": "{pid, ward_pid, edd_missing}" }, "422": { "description": "Validation / occupancy failure" } }
            } },
            "/api/stays/{pid}": {
                "get": { "tags": ["stays"], "summary": "Stay detail (journey, Red2Green run, flags; audited sensitive read; honours mask)", "responses": ok("StayDetail") },
                "put": { "tags": ["stays"], "summary": "Update whiteboard fields (EDD, CCD, named staff, alerts, senior review)", "responses": ok("Updated stay") }
            },
            "/api/stays/{pid}/transfer": { "post": { "tags": ["stays"], "summary": "Move to another bed (rule-checked; overrides audited)", "responses": ok("Updated stay") } },
            "/api/stays/{pid}/discharge-ready": { "post": { "tags": ["stays"], "summary": "Mark discharge-ready (requires EDD + CCD met; sets pathway p0–p3; starts the DTOC clock)", "responses": ok("Updated stay") } },
            "/api/stays/{pid}/discharge": { "post": { "tags": ["stays"], "summary": "Discharge (destination; vacates the bed; clears flags)", "responses": ok("Updated stay") } },
            "/api/stays/{pid}/red-green": { "post": { "tags": ["stays"], "summary": "Record today's Red2Green day (≤2 coded delay reasons; same-day editable)", "responses": ok("The day row") } },
            "/api/stays/{pid}/infection-flags": { "post": { "tags": ["infection"], "summary": "Raise a precaution flag (contact|droplet|airborne|protective)", "responses": created } },
            "/api/stays/{pid}/infection-flags/{flag_pid}/clear": { "post": { "tags": ["infection"], "summary": "Clear a flag", "responses": ok("Cleared") } },
            "/api/bed-requests": {
                "post": { "tags": ["bed-requests"], "summary": "Queue a bed request (origin, priority, requirements)", "responses": created },
                "get": { "tags": ["bed-requests"], "summary": "The demand board (priority then wait; live eligible-bed counts)", "responses": ok("Requests") }
            },
            "/api/bed-requests/{pid}/eligible": { "get": { "tags": ["bed-requests"], "summary": "Ranked eligible beds (rules 1–5; side-room conservation)", "responses": ok("Eligible beds") } },
            "/api/bed-requests/{pid}/allocate": { "post": { "tags": ["bed-requests"], "summary": "Reserve a chosen bed (rule-checked; sex/ward-fit overridable with reason)", "responses": ok("Updated request") } },
            "/api/bed-requests/{pid}/cancel": { "post": { "tags": ["bed-requests"], "summary": "Cancel (releases a reserved bed)", "responses": ok("Updated request") } },
            "/api/whiteboard/{ward_pid}": { "get": { "tags": ["boards"], "summary": "Ward whiteboard: bay-ordered bed cards + as_of (mask obligation honoured)", "responses": ok("Whiteboard") } },
            "/api/at-a-glance": { "get": { "tags": ["boards"], "summary": "Hospital at a glance: per-ward rows + site tiles", "responses": ok("AtAGlance") } },
            "/api/capacity/metrics": { "get": { "tags": ["boards"], "summary": "Capacity snapshot (flat; refreshes Prometheus gauges)", "responses": ok("Capacity") } },
            "/api/locate/{person_ref}": { "get": { "tags": ["boards"], "summary": "Where is patient X right now? (audited sensitive read)", "responses": ok("Location") } },
            "/api/audits/recent": { "get": { "tags": ["audit"], "summary": "Recent audit entries", "responses": ok("Audit entries") } },
            "/api/audits": { "get": { "tags": ["audit"], "summary": "Ward-scoped handover trail (?ward=&since=)", "responses": ok("Audit entries") } },
            "/api/audits/{entity_pid}": { "get": { "tags": ["audit"], "summary": "One record's audit trail", "responses": ok("Audit entries") } },
            "/api/events/recent": { "get": { "tags": ["events"], "summary": "Recent events (memory ring or outbox)", "responses": ok("Events") } },
            "/metrics.prom": { "get": { "tags": ["ops"], "summary": "Prometheus metrics (public)", "responses": ok("Exposition text") } }
        }
    })
}

#[cfg(test)]
mod tests {
    /// The document parses, declares `OpenAPI` 3, and covers the mounted
    /// API surface (spot-checked against the route table).
    #[test]
    fn spec_shape() {
        let doc = super::spec();
        assert_eq!(doc["openapi"], "3.0.3");
        let paths = doc["paths"].as_object().unwrap();
        for p in [
            "/api/stays",
            "/api/beds/{pid}/state",
            "/api/whiteboard/{ward_pid}",
            "/api/at-a-glance",
            "/api/locate/{person_ref}",
            "/api/bed-requests/{pid}/allocate",
        ] {
            assert!(paths.contains_key(p), "missing {p}");
        }
    }
}
