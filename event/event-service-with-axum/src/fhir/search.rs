//! FHIR `Appointment` search parameters + the in-memory match predicate.
//!
//! The supported subset ([`agents/share/fhir.md`](../../../../agents/share/fhir.md)
//! §6): `_id`, `_lastUpdated`, `_count`, `identifier` (token), `status`,
//! and `date`. `_lastUpdated` is accepted and ignored (no `_history`).
//! Unknown parameters are ignored, not rejected (v1). Filtering is
//! in-memory over the active rows the handler loads, matching the native
//! `list_active` scan model.

use serde::Deserialize;

use super::{event_status_to_appointment_status, identifier_type_to_system};
use crate::models::Event;

/// Default page size when `_count` is absent (bounded responses).
pub const DEFAULT_COUNT: usize = 50;

/// Parsed FHIR `Appointment` search query. Every field is optional; a
/// request with none matches all active rows (up to [`DEFAULT_COUNT`]).
#[derive(Debug, Default, Deserialize)]
pub struct FhirAppointmentSearchParams {
    /// `_id` — exact match on the resource id (`pid`).
    #[serde(rename = "_id", default)]
    pub id: Option<String>,
    /// `_lastUpdated` — accepted for compatibility, currently ignored.
    #[serde(rename = "_lastUpdated", default)]
    pub last_updated: Option<String>,
    /// `_count` — page size (result cap).
    #[serde(rename = "_count", default)]
    pub count: Option<usize>,
    /// `identifier` — a token `system|value` (or a bare `value`).
    #[serde(default)]
    pub identifier: Option<String>,
    /// `status` — the FHIR `Appointment.status` code (e.g. `booked`).
    #[serde(default)]
    pub status: Option<String>,
    /// `date` — a calendar day (`yyyy-mm-dd`) the event's start falls on.
    #[serde(default)]
    pub date: Option<String>,
}

impl FhirAppointmentSearchParams {
    /// The effective result cap (`_count`, else [`DEFAULT_COUNT`]).
    #[must_use]
    pub fn limit(&self) -> usize {
        self.count.unwrap_or(DEFAULT_COUNT)
    }

    /// Whether the stored `event` (with public id `pid`) matches every
    /// supplied parameter (conjunction; absent parameters don't
    /// constrain).
    #[must_use]
    pub fn matches(&self, event: &Event, pid: &str) -> bool {
        if let Some(ref id) = self.id
            && pid != id
        {
            return false;
        }
        if let Some(ref status) = self.status
            && &event_status_to_appointment_status(event.event_status) != status
        {
            return false;
        }
        if let Some(ref date) = self.date
            && event.start_date.format("%Y-%m-%d").to_string().as_str() != date.as_str()
        {
            return false;
        }
        if let Some(ref ident) = self.identifier
            && !identifier_matches(event, ident)
        {
            return false;
        }
        true
    }
}

/// Match an `identifier` token: `system|value` matches both parts; a bare
/// `value` matches any identifier with that value.
fn identifier_matches(event: &Event, token: &str) -> bool {
    let (system, value) = token
        .split_once('|')
        .map_or((None, token), |(s, v)| (Some(s), v));
    event.identifiers.iter().any(|id| {
        let value_hit = id.value == value;
        match system {
            Some(sys) => value_hit && identifier_type_to_system(id.identifier_type) == sys,
            None => value_hit,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Event, EventStatus, Identifier, IdentifierType};
    use chrono::{TimeZone, Utc};

    fn sample() -> Event {
        let start = Utc.with_ymd_and_hms(2026, 6, 1, 9, 0, 0).unwrap();
        let mut event = Event::new("Annual Conference", start);
        event.event_status = EventStatus::Scheduled;
        event.identifiers = vec![Identifier::new(
            IdentifierType::TicketNumber,
            identifier_type_to_system(IdentifierType::TicketNumber),
            "T-42".to_string(),
        )];
        event
    }

    #[test]
    fn empty_params_match_everything() {
        let p = FhirAppointmentSearchParams::default();
        assert!(p.matches(&sample(), "pid-1"));
        assert_eq!(p.limit(), DEFAULT_COUNT);
    }

    #[test]
    fn id_must_match_exactly() {
        let p = FhirAppointmentSearchParams {
            id: Some("pid-1".to_string()),
            ..Default::default()
        };
        assert!(p.matches(&sample(), "pid-1"));
        assert!(!p.matches(&sample(), "pid-2"));
    }

    #[test]
    fn status_filters_on_mapped_code() {
        let p = FhirAppointmentSearchParams {
            status: Some("booked".to_string()),
            ..Default::default()
        };
        assert!(p.matches(&sample(), "pid-1"));
        let miss = FhirAppointmentSearchParams {
            status: Some("cancelled".to_string()),
            ..Default::default()
        };
        assert!(!miss.matches(&sample(), "pid-1"));
    }

    #[test]
    fn date_filters_on_start_day() {
        let p = FhirAppointmentSearchParams {
            date: Some("2026-06-01".to_string()),
            ..Default::default()
        };
        assert!(p.matches(&sample(), "pid-1"));
        let miss = FhirAppointmentSearchParams {
            date: Some("2026-06-02".to_string()),
            ..Default::default()
        };
        assert!(!miss.matches(&sample(), "pid-1"));
    }

    #[test]
    fn identifier_token_matches_system_and_value() {
        let sys = identifier_type_to_system(IdentifierType::TicketNumber);
        let p = FhirAppointmentSearchParams {
            identifier: Some(format!("{sys}|T-42")),
            ..Default::default()
        };
        assert!(p.matches(&sample(), "pid-1"));
        // Wrong system ⇒ no match.
        let p2 = FhirAppointmentSearchParams {
            identifier: Some("urn:mxi:event:booking-number|T-42".to_string()),
            ..Default::default()
        };
        assert!(!p2.matches(&sample(), "pid-1"));
        // Bare value ⇒ match.
        let p3 = FhirAppointmentSearchParams {
            identifier: Some("T-42".to_string()),
            ..Default::default()
        };
        assert!(p3.matches(&sample(), "pid-1"));
    }
}
