//! FHIR R5 mapping (stub).
//!
//! schema.org/Event and FHIR R5 don't share a single canonical
//! resource: depending on context, an event maps to FHIR
//! `Appointment` (booking), `Encounter` (clinical), `Schedule` /
//! `Slot` (calendaring), or one of several event-pattern resources.
//! The mapping needs domain decisions (which subtype? how to map
//! parties to participants?) that aren't yet captured in spec.md.
//!
//! Until those decisions land, this module exposes a minimal
//! [`FhirEvent`](crate::api::fhir::FhirEvent) shape and stub handlers
//! that return 501. The route
//! surface is preserved so callers and the Swagger docs see the
//! shape they expect; the bodies just say "not implemented yet".

/// FHIR R5 stub handlers (all return `501 Not Implemented`).
pub mod handlers;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A minimal FHIR-flavored event payload. Only the fields the stub
/// handlers echo back are populated.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FhirEvent {
    /// FHIR `resourceType` discriminator (`"Event"` in this stub).
    #[serde(rename = "resourceType")]
    pub resource_type: String,
    /// Logical resource id, when known.
    pub id: Option<String>,
    /// FHIR status code, when known.
    pub status: Option<String>,
    /// schema.org/startDate — FHIR Period.start when used.
    pub start: Option<String>,
    /// schema.org/endDate — FHIR Period.end when used.
    pub end: Option<String>,
    /// Free-text description, when known.
    pub description: Option<String>,
}

impl FhirEvent {
    /// Construct an empty `Event`-typed payload with all optional
    /// fields unset.
    #[must_use]
    pub fn new() -> Self {
        Self {
            resource_type: "Event".into(),
            id: None,
            status: None,
            start: None,
            end: None,
            description: None,
        }
    }
}

impl Default for FhirEvent {
    /// Same as [`FhirEvent::new`]: an empty `Event`-typed payload.
    fn default() -> Self {
        Self::new()
    }
}
