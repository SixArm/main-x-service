//! HL7 FHIR R5 interop for the `Appointment` resource.
//!
//! A **best-effort, `low`-fidelity** representation of a
//! schema.org/[`Event`](crate::models::Event) as a FHIR R5
//! **`Appointment`** ([`agents/share/fhir.md`](../../../../agents/share/fhir.md)
//! §3): schema.org/Event and FHIR share no single canonical resource, so
//! the mapping is a deliberate approximation, documented gap-by-gap here
//! and never silent. (`Encounter` is a roadmap alternative — not
//! implemented.)
//!
//! It provides the bidirectional mapping between the stored
//! [`Event`](crate::models::Event) and the wire-level
//! [`resources::FhirAppointment`]: [`to_fhir_appointment`] for outbound
//! responses and [`from_fhir_appointment`] for inbound requests.
//! Resource shapes live in [`resources`], search-parameter parsing in
//! [`search`], and the mounted Axum endpoints in
//! [`crate::controllers::fhir`].
//!
//! Field summary (see the per-function docs for the exact gap list):
//!
//! | `Event` | `Appointment` |
//! |---|---|
//! | `start_date` / `end_date` | `start` / `end` |
//! | `name` | `description` |
//! | `event_status` | `status` (via [`event_status_to_appointment_status`]) |
//! | `organizers` / `performers` / `attendees` | `participant` (role-coded) |
//! | `location` | `participant` (`location`-coded, display only) |
//! | `identifiers` | `identifier` (`system|value`; type↔system map) |

/// FHIR resource + envelope wire types (`Appointment`,
/// `OperationOutcome`, `Bundle`).
pub mod resources;
/// FHIR search-parameter parsing + the in-memory match predicate.
pub mod search;

use crate::models::{
    Event, EventStatus, Identifier, IdentifierType, Location, Party, PartyKind, VirtualLocation,
};
use resources::{
    FhirAppointment, FhirCodeableConcept, FhirCoding, FhirIdentifier, FhirMeta, FhirParticipant,
    FhirReference,
};

/// The family code system for the participant role coding.
pub const PARTY_ROLE_SYSTEM: &str = "urn:mxi:event:party-role";

// ---------------------------------------------------------------------------
// Identifier category  <->  FHIR identifier.system  (the "scheme" map)
// ---------------------------------------------------------------------------

/// Map an [`IdentifierType`] to the FHIR `identifier.system` URI, using a
/// family `urn:mxi:event:*` namespace. [`system_to_identifier_type`] is
/// the exact inverse, so a category round-trips through FHIR unchanged.
#[must_use]
pub fn identifier_type_to_system(kind: IdentifierType) -> String {
    match kind {
        IdentifierType::BookingNumber => "urn:mxi:event:booking-number",
        IdentifierType::ConfirmationCode => "urn:mxi:event:confirmation-code",
        IdentifierType::TicketNumber => "urn:mxi:event:ticket-number",
        IdentifierType::EncounterId => "urn:mxi:event:encounter-id",
        IdentifierType::TransactionId => "urn:mxi:event:transaction-id",
        IdentifierType::ExternalRef => "urn:mxi:event:external-ref",
        IdentifierType::Tax => "urn:mxi:event:tax",
        IdentifierType::Other => "urn:mxi:event:other",
    }
    .to_string()
}

/// Map a FHIR `identifier.system` URI back to an [`IdentifierType`] — the
/// inverse of [`identifier_type_to_system`]. An unrecognised system maps
/// to [`IdentifierType::Other`] (the model's own catch-all), so an
/// inbound identifier from a foreign namespace is still carried (its
/// value is preserved).
#[must_use]
pub fn system_to_identifier_type(system: &str) -> IdentifierType {
    match system {
        "urn:mxi:event:booking-number" => IdentifierType::BookingNumber,
        "urn:mxi:event:confirmation-code" => IdentifierType::ConfirmationCode,
        "urn:mxi:event:ticket-number" => IdentifierType::TicketNumber,
        "urn:mxi:event:encounter-id" => IdentifierType::EncounterId,
        "urn:mxi:event:transaction-id" => IdentifierType::TransactionId,
        "urn:mxi:event:external-ref" => IdentifierType::ExternalRef,
        "urn:mxi:event:tax" => IdentifierType::Tax,
        _ => IdentifierType::Other,
    }
}

// ---------------------------------------------------------------------------
// EventStatus  <->  Appointment.status
// ---------------------------------------------------------------------------

/// Map an [`EventStatus`] to a FHIR `Appointment.status` code. The
/// mapping is lossy: `MovedOnline` and `Rescheduled` both fold onto
/// `booked` (no distinct FHIR code), so they do not round-trip — see
/// [`appointment_status_to_event_status`].
#[must_use]
pub fn event_status_to_appointment_status(status: EventStatus) -> String {
    match status {
        EventStatus::Scheduled | EventStatus::MovedOnline | EventStatus::Rescheduled => "booked",
        EventStatus::Cancelled => "cancelled",
        EventStatus::Postponed => "waitlist",
        EventStatus::Completed => "fulfilled",
    }
    .to_string()
}

/// Map a FHIR `Appointment.status` code back to an [`EventStatus`] — the
/// partial inverse of [`event_status_to_appointment_status`]. Unknown or
/// unmapped codes default to [`EventStatus::Scheduled`].
#[must_use]
pub fn appointment_status_to_event_status(status: &str) -> EventStatus {
    match status {
        "cancelled" => EventStatus::Cancelled,
        "waitlist" => EventStatus::Postponed,
        "fulfilled" => EventStatus::Completed,
        _ => EventStatus::Scheduled,
    }
}

// ---------------------------------------------------------------------------
// Party / Location  <->  Appointment.participant
// ---------------------------------------------------------------------------

/// The FHIR resource type name for a party kind (`Person` /
/// `Organization`), used as `Reference.type` so the kind survives the
/// round-trip even for id-less parties.
fn party_type_name(kind: PartyKind) -> &'static str {
    match kind {
        PartyKind::Person => "Person",
        PartyKind::Organization => "Organization",
    }
}

/// Render a party as a role-coded FHIR participant.
fn party_participant(party: &Party, role: &str) -> FhirParticipant {
    let type_name = party_type_name(party.kind);
    FhirParticipant {
        role_type: vec![role_concept(role)],
        actor: Some(FhirReference {
            reference: party.id.map(|id| format!("{type_name}/{id}")),
            ref_type: Some(type_name.to_string()),
            display: Some(party.name.clone()),
        }),
        status: "accepted".to_string(),
    }
}

/// A single-coding `CodeableConcept` in the party-role system.
fn role_concept(role: &str) -> FhirCodeableConcept {
    FhirCodeableConcept {
        coding: vec![FhirCoding {
            system: Some(PARTY_ROLE_SYSTEM.to_string()),
            code: Some(role.to_string()),
        }],
    }
}

/// A best-effort display label for a location variant (used as the
/// participant's `actor.display`).
fn location_label(loc: &Location) -> String {
    match loc {
        Location::Place(p) => p.name.clone(),
        Location::PostalAddress(a) => {
            let parts = [a.line1.as_deref(), a.city.as_deref(), a.country.as_deref()];
            let joined = parts.into_iter().flatten().collect::<Vec<_>>().join(", ");
            if joined.is_empty() {
                "Address".to_string()
            } else {
                joined
            }
        }
        Location::Virtual(VirtualLocation { name, url }) => {
            name.clone().unwrap_or_else(|| url.clone())
        }
        Location::Text { value } => value.clone(),
    }
}

/// Read the party-role code from a participant's `type` codings.
fn participant_role(p: &FhirParticipant) -> Option<String> {
    p.role_type
        .iter()
        .flat_map(|c| &c.coding)
        .find(|c| c.system.as_deref() == Some(PARTY_ROLE_SYSTEM))
        .and_then(|c| c.code.clone())
}

/// Rebuild a domain [`Party`] from a participant's `actor` reference,
/// recovering `kind` from `Reference.type` (defaulting to `Person`).
fn participant_to_party(actor: &FhirReference) -> Party {
    let kind = match actor.ref_type.as_deref() {
        Some("Organization") => PartyKind::Organization,
        _ => PartyKind::Person,
    };
    let id = actor
        .reference
        .as_deref()
        .and_then(|r| r.rsplit('/').next())
        .and_then(|s| uuid::Uuid::parse_str(s).ok());
    Party {
        kind,
        id,
        name: actor.display.clone().unwrap_or_default(),
        email: None,
        url: None,
    }
}

// ---------------------------------------------------------------------------
// Event  <->  FhirAppointment
// ---------------------------------------------------------------------------

/// Render a stored [`Event`] as a FHIR R5 [`FhirAppointment`].
///
/// `id` is the event's `id`; `meta.lastUpdated` comes from `updated_at`.
/// `start`/`end` carry the time window as RFC 3339; `description` carries
/// `name`; `status` is the mapped `event_status`; `participant` carries
/// `organizers` / `performers` / `attendees` (role-coded) followed by the
/// `location` list (as `location`-coded participants, display only);
/// `identifier` carries `identifiers` (category → `system`, value →
/// `value`).
///
/// **Fidelity gaps** (no `Appointment` home): `description` (the event's
/// long-form text — distinct from `name`), `keywords`, `image`,
/// `same_as`, `url`, `offers`, capacity, audience fields, `sponsors` /
/// `funders` / `contributors`, `about` / `works`, `super_event` /
/// `sub_events`, `door_time` / `duration` / `time_zone`, and per-party
/// `email` / `url` are not emitted. Location structure beyond a display
/// label (address, geo, place id) is not carried.
#[must_use]
pub fn to_fhir_appointment(event: &Event) -> FhirAppointment {
    let mut fhir = FhirAppointment::new();
    fhir.id = Some(event.id.to_string());
    fhir.meta = Some(FhirMeta {
        version_id: None,
        last_updated: Some(event.updated_at.to_rfc3339()),
    });
    fhir.status = event_status_to_appointment_status(event.event_status);
    fhir.description = Some(event.name.clone());
    fhir.start = Some(event.start_date.to_rfc3339());
    fhir.end = event.end_date.map(|d| d.to_rfc3339());

    fhir.identifier = event
        .identifiers
        .iter()
        .map(|id| FhirIdentifier {
            system: Some(identifier_type_to_system(id.identifier_type)),
            value: Some(id.value.clone()),
        })
        .collect();

    let mut participant = Vec::new();
    for p in &event.organizers {
        participant.push(party_participant(p, "organizer"));
    }
    for p in &event.performers {
        participant.push(party_participant(p, "performer"));
    }
    for p in &event.attendees {
        participant.push(party_participant(p, "attendee"));
    }
    for loc in &event.location {
        participant.push(FhirParticipant {
            role_type: vec![role_concept("location")],
            actor: Some(FhirReference {
                reference: None,
                ref_type: Some("Location".to_string()),
                display: Some(location_label(loc)),
            }),
            status: "accepted".to_string(),
        });
    }
    fhir.participant = participant;

    fhir
}

/// Parse an inbound [`FhirAppointment`] into a stored [`Event`].
///
/// `description` is required and becomes `name` (a FHIR `Appointment`
/// with no non-empty `description` is a `400`/`422`). `start` is required
/// and must be an RFC 3339 timestamp; it becomes `start_date`. `end` →
/// `end_date`; `status` → `event_status`; `identifier` → `identifiers`
/// (`system` → category); role-coded participants → `organizers` /
/// `performers` / `attendees`; `location`-coded participants →
/// [`Location::Text`] (display label only).
///
/// **Fidelity gaps** (the reverse is lossy or the field has no wire
/// carrier): the event's own long-form `description`, `keywords`, and the
/// other unmapped `Event` fields default to empty; a location is
/// recovered only as free `Location::Text`, never as a structured
/// `Place` / `PostalAddress` / `Virtual`; a participant with an
/// unrecognised (or absent) role falls into `attendees`; per-party
/// `email` / `url` are not recovered.
///
/// # Errors
///
/// Returns a diagnostic string when the resource has no non-empty
/// `description`, or when `start` is missing or not a valid RFC 3339
/// timestamp.
pub fn from_fhir_appointment(fhir: &FhirAppointment) -> Result<Event, String> {
    let name = fhir
        .description
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .ok_or_else(|| "Appointment.description is required (maps to Event.name)".to_string())?;

    let start_raw = fhir
        .start
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "Appointment.start is required (maps to Event.start_date)".to_string())?;
    let start_date = chrono::DateTime::parse_from_rfc3339(start_raw)
        .map_err(|e| format!("Appointment.start is not a valid RFC 3339 instant: {e}"))?
        .with_timezone(&chrono::Utc);

    let mut event = Event::new(name, start_date);

    if let Some(end_raw) = fhir.end.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        event.end_date = Some(
            chrono::DateTime::parse_from_rfc3339(end_raw)
                .map_err(|e| format!("Appointment.end is not a valid RFC 3339 instant: {e}"))?
                .with_timezone(&chrono::Utc),
        );
    }

    event.event_status = appointment_status_to_event_status(&fhir.status);

    event.identifiers = fhir
        .identifier
        .iter()
        .filter_map(|id| {
            let value = id.value.clone()?;
            let system = id.system.clone().unwrap_or_default();
            let identifier_type = system_to_identifier_type(&system);
            Some(Identifier::new(identifier_type, system, value))
        })
        .collect();

    for p in &fhir.participant {
        let Some(actor) = p.actor.as_ref() else {
            continue;
        };
        match participant_role(p).as_deref() {
            Some("organizer") => event.organizers.push(participant_to_party(actor)),
            Some("performer") => event.performers.push(participant_to_party(actor)),
            Some("location") => event.location.push(Location::Text {
                value: actor.display.clone().unwrap_or_default(),
            }),
            // "attendee" and any unrecognised / absent role default here.
            _ => event.attendees.push(participant_to_party(actor)),
        }
    }

    Ok(event)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    /// Every identifier category round-trips through the FHIR `system`
    /// URI unchanged (`type → system → type` is the identity).
    #[test]
    fn identifier_type_system_round_trips() {
        let kinds = [
            IdentifierType::BookingNumber,
            IdentifierType::ConfirmationCode,
            IdentifierType::TicketNumber,
            IdentifierType::EncounterId,
            IdentifierType::TransactionId,
            IdentifierType::ExternalRef,
            IdentifierType::Tax,
            IdentifierType::Other,
        ];
        for kind in kinds {
            let system = identifier_type_to_system(kind);
            assert_eq!(
                system_to_identifier_type(&system),
                kind,
                "round-trip {kind:?}"
            );
        }
    }

    /// An unknown inbound `system` folds to the model's `Other` category
    /// (never dropped — its value is still carried).
    #[test]
    fn unknown_system_becomes_other() {
        assert_eq!(
            system_to_identifier_type("https://example.org/foo"),
            IdentifierType::Other
        );
    }

    /// The cleanly-mapped statuses round-trip; the folded ones are
    /// documented gaps.
    #[test]
    fn status_round_trips_for_mapped_values() {
        for status in [
            EventStatus::Scheduled,
            EventStatus::Cancelled,
            EventStatus::Postponed,
            EventStatus::Completed,
        ] {
            let code = event_status_to_appointment_status(status);
            assert_eq!(appointment_status_to_event_status(&code), status);
        }
    }

    /// The losslessly-reversible fields survive `Event → FHIR → Event`.
    #[test]
    fn dto_fhir_round_trip_preserves_core_fields() {
        let start = Utc.with_ymd_and_hms(2026, 6, 1, 9, 0, 0).unwrap();
        let mut event = Event::new("Annual Conference", start);
        event.end_date = Some(start + chrono::Duration::hours(8));
        event.event_status = EventStatus::Completed;
        event.identifiers = vec![Identifier::new(
            IdentifierType::TicketNumber,
            identifier_type_to_system(IdentifierType::TicketNumber),
            "T-42".to_string(),
        )];
        event.organizers = vec![Party {
            kind: PartyKind::Organization,
            id: None,
            name: "Cal Performances".to_string(),
            email: None,
            url: None,
        }];
        event.performers = vec![Party {
            kind: PartyKind::Person,
            id: None,
            name: "The Quartet".to_string(),
            email: None,
            url: None,
        }];
        event.attendees = vec![Party {
            kind: PartyKind::Person,
            id: None,
            name: "Jane Doe".to_string(),
            email: None,
            url: None,
        }];
        event.location = vec![Location::Text {
            value: "Main Hall".to_string(),
        }];

        let fhir = to_fhir_appointment(&event);
        assert_eq!(fhir.resource_type, "Appointment");
        assert_eq!(fhir.id.as_deref(), Some(event.id.to_string().as_str()));
        assert_eq!(fhir.status, "fulfilled");

        let back = from_fhir_appointment(&fhir).expect("valid resource");
        assert_eq!(back.name, event.name);
        assert_eq!(back.start_date, event.start_date);
        assert_eq!(back.end_date, event.end_date);
        assert_eq!(back.event_status, EventStatus::Completed);
        assert_eq!(back.identifiers.len(), 1);
        assert_eq!(
            back.identifiers[0].identifier_type,
            IdentifierType::TicketNumber
        );
        assert_eq!(back.identifiers[0].value, "T-42");
        assert_eq!(back.organizers.len(), 1);
        assert_eq!(back.organizers[0].name, "Cal Performances");
        assert_eq!(back.organizers[0].kind, PartyKind::Organization);
        assert_eq!(back.performers.len(), 1);
        assert_eq!(back.performers[0].name, "The Quartet");
        assert_eq!(back.attendees.len(), 1);
        assert_eq!(back.attendees[0].name, "Jane Doe");
        assert_eq!(back.location.len(), 1);
        assert_eq!(
            back.location[0],
            Location::Text {
                value: "Main Hall".to_string()
            }
        );
    }

    /// A resource with no `description` is rejected (maps to a `400`).
    #[test]
    fn missing_description_is_rejected() {
        let mut fhir = FhirAppointment::new();
        fhir.start = Some("2026-06-01T09:00:00Z".to_string());
        assert!(from_fhir_appointment(&fhir).is_err());
    }

    /// A resource with no `start` is rejected (Event requires a start).
    #[test]
    fn missing_start_is_rejected() {
        let mut fhir = FhirAppointment::new();
        fhir.description = Some("Some appointment".to_string());
        assert!(from_fhir_appointment(&fhir).is_err());
    }
}
