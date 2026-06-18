//! Data-quality validation for event records.
//!
//! Validation rules implement the invariants described in
//! `spec.md` section 2 (Domain Model). Returning a non-empty
//! `Vec<ValidationError>` causes the API layer to respond `422`.

use crate::models::{
    Address, ContactPoint, ContactPointSystem, Event, EventAttendanceMode, Location, Offer,
    VirtualLocation,
};

/// A single field-level validation failure.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ValidationError {
    /// Dotted/indexed path to the offending field (e.g. `location[0].name`).
    pub field: String,
    /// Human-readable explanation of why the field is invalid.
    pub message: String,
}

impl ValidationError {
    /// Construct a [`ValidationError`] from a field path and message.
    fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
        }
    }
}

/// Validate an event, collecting every error rather than returning early.
#[must_use]
pub fn validate_event(event: &Event) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // ---- Name ------------------------------------------------------------
    if event.name.trim().is_empty() {
        errors.push(ValidationError::new("name", "Name is required"));
    }

    // ---- Time window ----------------------------------------------------
    errors.extend(validate_time_window(event));

    // ---- Attendance mode <-> location coherence -------------------------
    errors.extend(validate_attendance_location(event));

    // ---- Capacities -----------------------------------------------------
    errors.extend(validate_capacities(event));

    // ---- Languages ------------------------------------------------------
    for (i, lang) in event.in_language.iter().enumerate() {
        if !is_valid_language_code(lang) {
            errors.push(ValidationError::new(
                format!("in_language[{i}]"),
                "expected a 2-letter ISO 639-1 language code",
            ));
        }
    }

    // ---- Locations ------------------------------------------------------
    for (i, loc) in event.location.iter().enumerate() {
        let prefix = format!("location[{i}]");
        match loc {
            Location::Place(place) => {
                if place.name.trim().is_empty() {
                    errors.push(ValidationError::new(
                        format!("{prefix}.name"),
                        "Place name is required",
                    ));
                }
                if let Some(lat) = place.latitude
                    && !(-90.0..=90.0).contains(&lat)
                {
                    errors.push(ValidationError::new(
                        format!("{prefix}.latitude"),
                        "latitude must be between -90 and 90",
                    ));
                }
                if let Some(lon) = place.longitude
                    && !(-180.0..=180.0).contains(&lon)
                {
                    errors.push(ValidationError::new(
                        format!("{prefix}.longitude"),
                        "longitude must be between -180 and 180",
                    ));
                }
                if let Some(ref addr) = place.address {
                    errors.extend(validate_address(addr, &format!("{prefix}.address")));
                }
            }
            Location::PostalAddress(addr) => {
                errors.extend(validate_address(addr, &prefix));
            }
            Location::Virtual(v) => {
                errors.extend(validate_virtual(v, &prefix));
            }
            Location::Text { value } => {
                if value.trim().is_empty() {
                    errors.push(ValidationError::new(
                        format!("{prefix}.value"),
                        "Text location must not be empty",
                    ));
                }
            }
        }
    }

    // ---- Identifiers ----------------------------------------------------
    errors.extend(validate_identifiers(event));

    // ---- Parties (organizer/performer/attendee/...) ---------------------
    errors.extend(validate_parties(event));

    // ---- Offers --------------------------------------------------------
    for (i, offer) in event.offers.iter().enumerate() {
        errors.extend(validate_offer(offer, &format!("offers[{i}]")));
    }

    errors
}

/// Validate each external identifier: `system` and `value` non-empty.
fn validate_identifiers(event: &Event) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    for (i, id) in event.identifiers.iter().enumerate() {
        if id.system.trim().is_empty() {
            errors.push(ValidationError::new(
                format!("identifiers[{i}].system"),
                "identifier.system is required",
            ));
        }
        if id.value.trim().is_empty() {
            errors.push(ValidationError::new(
                format!("identifiers[{i}].value"),
                "identifier.value is required",
            ));
        }
    }
    errors
}

/// Validate the party role-lists: each organizer / performer / attendee
/// must carry a non-empty `name`.
fn validate_parties(event: &Event) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    for (role, singular, parties) in [
        ("organizers", "organizer", &event.organizers),
        ("performers", "performer", &event.performers),
        ("attendees", "attendee", &event.attendees),
    ] {
        for (i, party) in parties.iter().enumerate() {
            if party.name.trim().is_empty() {
                errors.push(ValidationError::new(
                    format!("{role}[{i}].name"),
                    format!("{singular}.name is required"),
                ));
            }
        }
    }
    errors
}

/// Validate the time-window fields: `end_date >= start_date`,
/// `door_time <= start_date`, ISO 8601 `duration`, non-empty `time_zone`.
fn validate_time_window(event: &Event) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    if let Some(end) = event.end_date
        && end < event.start_date
    {
        errors.push(ValidationError::new(
            "end_date",
            "end_date must be on or after start_date",
        ));
    }
    if let Some(door) = event.door_time
        && door > event.start_date
    {
        errors.push(ValidationError::new(
            "door_time",
            "door_time must be on or before start_date",
        ));
    }
    if let Some(ref duration) = event.duration
        && !is_iso8601_duration(duration)
    {
        errors.push(ValidationError::new(
            "duration",
            "duration must be an ISO 8601 duration (e.g. \"PT1H30M\")",
        ));
    }
    if let Some(ref tz) = event.time_zone
        && tz.trim().is_empty()
    {
        errors.push(ValidationError::new(
            "time_zone",
            "time_zone must be a non-empty IANA name (e.g. \"Europe/London\")",
        ));
    }
    errors
}

/// Validate coherence between `event_attendance_mode` and the location
/// list (online ⇒ a Virtual location; mixed ⇒ physical + Virtual).
fn validate_attendance_location(event: &Event) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    match event.event_attendance_mode {
        EventAttendanceMode::Online => {
            let has_virtual = event
                .location
                .iter()
                .any(|loc| matches!(loc, Location::Virtual(_)));
            if !has_virtual && !event.location.is_empty() {
                errors.push(ValidationError::new(
                    "location",
                    "online events should include at least one Virtual location",
                ));
            }
        }
        EventAttendanceMode::Mixed => {
            let has_physical = event
                .location
                .iter()
                .any(|loc| matches!(loc, Location::Place(_) | Location::PostalAddress(_)));
            let has_virtual = event
                .location
                .iter()
                .any(|loc| matches!(loc, Location::Virtual(_)));
            if !(event.location.is_empty() || has_physical && has_virtual) {
                errors.push(ValidationError::new(
                    "location",
                    "mixed events should include both a physical and a Virtual location",
                ));
            }
        }
        EventAttendanceMode::Offline => {}
    }
    errors
}

/// Validate the attendee-capacity invariants: physical + virtual ≤ total
/// and remaining ≤ total (`u32` already excludes negatives).
fn validate_capacities(event: &Event) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    if let (Some(total), Some(phys), Some(virt)) = (
        event.maximum_attendee_capacity,
        event.maximum_physical_attendee_capacity,
        event.maximum_virtual_attendee_capacity,
    ) && phys.saturating_add(virt) > total
    {
        errors.push(ValidationError::new(
            "maximum_attendee_capacity",
            "physical + virtual capacity exceeds total maximum_attendee_capacity",
        ));
    }
    if let (Some(remaining), Some(total)) = (
        event.remaining_attendee_capacity,
        event.maximum_attendee_capacity,
    ) && remaining > total
    {
        errors.push(ValidationError::new(
            "remaining_attendee_capacity",
            "remaining_attendee_capacity cannot exceed maximum_attendee_capacity",
        ));
    }
    errors
}

/// Validate an [`Address`]. At least one of city / `postal_code` /
/// country must be present.
fn validate_address(addr: &Address, prefix: &str) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let has_location = addr.city.as_ref().is_some_and(|s| !s.trim().is_empty())
        || addr
            .postal_code
            .as_ref()
            .is_some_and(|s| !s.trim().is_empty())
        || addr.country.as_ref().is_some_and(|s| !s.trim().is_empty());
    if !has_location {
        errors.push(ValidationError::new(
            prefix,
            "Address must have at least a city, postal_code, or country",
        ));
    }
    errors
}

/// Validate a [`VirtualLocation`]: URL must be syntactically reasonable.
fn validate_virtual(v: &VirtualLocation, prefix: &str) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    if v.url.trim().is_empty() {
        errors.push(ValidationError::new(
            format!("{prefix}.url"),
            "Virtual location URL is required",
        ));
    } else if !v.url.starts_with("http://") && !v.url.starts_with("https://") {
        errors.push(ValidationError::new(
            format!("{prefix}.url"),
            "Virtual location URL must start with http:// or https://",
        ));
    }
    errors
}

/// Validate an [`Offer`]: paired price + currency, `valid_from` <= `valid_through`.
fn validate_offer(offer: &Offer, prefix: &str) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    match (offer.price.as_ref(), offer.price_currency.as_ref()) {
        (Some(price), None) if !price.trim().is_empty() => errors.push(ValidationError::new(
            format!("{prefix}.price_currency"),
            "price_currency is required when price is set",
        )),
        (Some(price), _) if price.parse::<f64>().is_err() => {
            errors.push(ValidationError::new(
                format!("{prefix}.price"),
                "price must be a decimal number",
            ));
        }
        _ => {}
    }
    if let Some(ref c) = offer.price_currency
        && (c.len() != 3 || !c.chars().all(|c| c.is_ascii_alphabetic()))
    {
        errors.push(ValidationError::new(
            format!("{prefix}.price_currency"),
            "price_currency must be a 3-letter ISO 4217 code",
        ));
    }
    if let (Some(from), Some(through)) = (offer.valid_from, offer.valid_through)
        && through < from
    {
        errors.push(ValidationError::new(
            format!("{prefix}.valid_through"),
            "valid_through must be on or after valid_from",
        ));
    }
    errors
}

/// Validate a contact point (used by Party.email / Place.url scenarios
/// and by callers outside this module).
#[must_use]
pub fn validate_contact_point(cp: &ContactPoint, prefix: &str) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    if cp.value.trim().is_empty() {
        errors.push(ValidationError::new(
            format!("{prefix}.value"),
            "Contact value is required",
        ));
        return errors;
    }
    match cp.system {
        ContactPointSystem::Email if (!cp.value.contains('@') || !cp.value.contains('.')) => {
            errors.push(ValidationError::new(
                format!("{prefix}.value"),
                "Invalid email format",
            ));
        }
        ContactPointSystem::Phone | ContactPointSystem::Sms | ContactPointSystem::Fax => {
            let digits: String = cp.value.chars().filter(char::is_ascii_digit).collect();
            if digits.len() < 7 {
                errors.push(ValidationError::new(
                    format!("{prefix}.value"),
                    "Phone number must have at least 7 digits",
                ));
            }
        }
        _ => {}
    }
    errors
}

/// Cheap ISO 8601 duration check: must start with "P" and contain
/// at least one designator letter.
fn is_iso8601_duration(s: &str) -> bool {
    let s = s.trim();
    if !s.starts_with('P') || s.len() < 2 {
        return false;
    }
    s.chars()
        .skip(1)
        .any(|c| matches!(c, 'Y' | 'M' | 'W' | 'D' | 'H' | 'S' | 'T'))
}

/// Surface-level check for ISO 639-1 codes: 2 ASCII letters.
fn is_valid_language_code(s: &str) -> bool {
    s.len() == 2 && s.chars().all(|c| c.is_ascii_alphabetic())
}

// ---------------------------------------------------------------------------
// Normalization helpers (kept from prior version; useful for inbound data)
// ---------------------------------------------------------------------------

/// Normalize/standardize a phone number to E.164-like format.
#[must_use]
pub fn normalize_phone(phone: &str, default_country_code: &str) -> String {
    let digits: String = phone.chars().filter(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return String::new();
    }
    if digits.len() >= 10 && digits.starts_with(default_country_code) {
        return format!("+{digits}");
    }
    if digits.len() == 10 {
        return format!("+{default_country_code}{digits}");
    }
    if phone.starts_with('+') {
        return format!("+{digits}");
    }
    format!("+{default_country_code}{digits}")
}

/// Trim/title-case/uppercase an address for consistent storage.
#[must_use]
pub fn standardize_address(addr: &Address) -> Address {
    Address {
        use_type: addr.use_type.clone(),
        line1: addr.line1.as_ref().map(|s| normalize_street_address(s)),
        line2: addr.line2.as_ref().map(|s| s.trim().to_string()),
        city: addr.city.as_ref().map(|s| title_case(s.trim())),
        state: addr.state.as_ref().map(|s| s.trim().to_uppercase()),
        postal_code: addr.postal_code.as_ref().map(|s| s.trim().to_string()),
        country: addr.country.as_ref().map(|s| s.trim().to_uppercase()),
    }
}

/// Expand common street-type abbreviations (St. → Street, Ave. →
/// Avenue, …) and trim surrounding whitespace.
fn normalize_street_address(street: &str) -> String {
    street
        .trim()
        .replace("St.", "Street")
        .replace("St ", "Street ")
        .replace("Ave.", "Avenue")
        .replace("Ave ", "Avenue ")
        .replace("Rd.", "Road")
        .replace("Rd ", "Road ")
        .replace("Dr.", "Drive")
        .replace("Blvd.", "Boulevard")
        .replace("Ln.", "Lane")
        .replace("Ct.", "Court")
}

/// Title-case each whitespace-separated word (first letter upper, rest
/// lower), collapsing runs of whitespace to single spaces.
fn title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    let upper: String = first.to_uppercase().collect();
                    let rest: String = chars.collect::<String>().to_lowercase();
                    format!("{upper}{rest}")
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Event, EventAttendanceMode, Location, Place, VirtualLocation};
    use chrono::{DateTime, TimeZone, Utc};

    /// A fixed start instant for building test events.
    fn start() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 3, 1, 12, 0, 0).unwrap()
    }

    /// A minimal valid event produces no validation errors.
    #[test]
    fn valid_event_passes() {
        let event = Event::new("Test", start());
        let errors = validate_event(&event);
        assert!(errors.is_empty(), "expected no errors, got {errors:?}");
    }

    /// An empty name yields a `name` error.
    #[test]
    fn empty_name_fails() {
        let event = Event::new("", start());
        let errors = validate_event(&event);
        assert!(errors.iter().any(|e| e.field == "name"));
    }

    /// `end_date` before `start_date` yields an `end_date` error.
    #[test]
    fn end_before_start_fails() {
        let mut event = Event::new("Test", start());
        event.end_date = Some(start() - chrono::Duration::hours(1));
        let errors = validate_event(&event);
        assert!(errors.iter().any(|e| e.field == "end_date"));
    }

    /// `door_time` after `start_date` yields a `door_time` error.
    #[test]
    fn door_after_start_fails() {
        let mut event = Event::new("Test", start());
        event.door_time = Some(start() + chrono::Duration::hours(1));
        let errors = validate_event(&event);
        assert!(errors.iter().any(|e| e.field == "door_time"));
    }

    /// Physical + virtual capacity exceeding total yields an error.
    #[test]
    fn capacity_breakdown_must_sum() {
        let mut event = Event::new("Test", start());
        event.maximum_attendee_capacity = Some(100);
        event.maximum_physical_attendee_capacity = Some(80);
        event.maximum_virtual_attendee_capacity = Some(50);
        let errors = validate_event(&event);
        assert!(
            errors
                .iter()
                .any(|e| e.field == "maximum_attendee_capacity")
        );
    }

    /// An online event with no virtual location yields a `location` error.
    #[test]
    fn online_requires_virtual_location() {
        let mut event = Event::new("Test", start());
        event.event_attendance_mode = EventAttendanceMode::Online;
        event.location.push(Location::Text {
            value: "physical-only".into(),
        });
        let errors = validate_event(&event);
        assert!(errors.iter().any(|e| e.field == "location"));
    }

    /// An online event with a virtual location validates cleanly.
    #[test]
    fn online_passes_with_virtual_location() {
        let mut event = Event::new("Test", start());
        event.event_attendance_mode = EventAttendanceMode::Online;
        event.location.push(Location::Virtual(VirtualLocation {
            name: None,
            url: "https://example.test/zoom".into(),
        }));
        let errors = validate_event(&event);
        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    }

    /// A `Mixed` event with only a virtual location (no physical) yields
    /// a `location` error: mixed requires both a physical and a virtual.
    #[test]
    fn mixed_requires_physical_and_virtual_location() {
        let mut event = Event::new("Test", start());
        event.event_attendance_mode = EventAttendanceMode::Mixed;
        event.location.push(Location::Virtual(VirtualLocation {
            name: None,
            url: "https://example.test/stream".into(),
        }));
        let errors = validate_event(&event);
        assert!(
            errors.iter().any(|e| e.field == "location"),
            "mixed-with-only-virtual should fail, got {errors:?}"
        );
    }

    /// A `Mixed` event with only a physical location (no virtual) also
    /// yields a `location` error.
    #[test]
    fn mixed_requires_virtual_when_only_physical() {
        let mut event = Event::new("Test", start());
        event.event_attendance_mode = EventAttendanceMode::Mixed;
        event.location.push(Location::Place(Place {
            id: None,
            name: "Main Hall".into(),
            address: None,
            latitude: None,
            longitude: None,
            url: None,
        }));
        let errors = validate_event(&event);
        assert!(
            errors.iter().any(|e| e.field == "location"),
            "mixed-with-only-physical should fail, got {errors:?}"
        );
    }

    /// A `Mixed` event with both a physical and a virtual location
    /// validates cleanly.
    #[test]
    fn mixed_passes_with_physical_and_virtual() {
        let mut event = Event::new("Test", start());
        event.event_attendance_mode = EventAttendanceMode::Mixed;
        event.location.push(Location::Place(Place {
            id: None,
            name: "Main Hall".into(),
            address: None,
            latitude: None,
            longitude: None,
            url: None,
        }));
        event.location.push(Location::Virtual(VirtualLocation {
            name: None,
            url: "https://example.test/stream".into(),
        }));
        let errors = validate_event(&event);
        assert!(
            !errors.iter().any(|e| e.field == "location"),
            "mixed-with-both should pass, got {errors:?}"
        );
    }

    /// `remaining_attendee_capacity` exceeding the total maximum yields a
    /// `remaining_attendee_capacity` error.
    #[test]
    fn remaining_capacity_cannot_exceed_total() {
        let mut event = Event::new("Test", start());
        event.maximum_attendee_capacity = Some(100);
        event.remaining_attendee_capacity = Some(150);
        let errors = validate_event(&event);
        assert!(
            errors
                .iter()
                .any(|e| e.field == "remaining_attendee_capacity"),
            "expected remaining-capacity error, got {errors:?}"
        );
    }

    /// Non-ISO durations fail; a valid `PT1H30M` passes.
    #[test]
    fn iso_duration_validation() {
        let mut event = Event::new("Test", start());
        event.duration = Some("90 minutes".into());
        let errors = validate_event(&event);
        assert!(errors.iter().any(|e| e.field == "duration"));

        event.duration = Some("PT1H30M".into());
        let errors = validate_event(&event);
        assert!(!errors.iter().any(|e| e.field == "duration"));
    }

    /// A non-ISO-639-1 language string yields an `in_language` error.
    #[test]
    fn language_code_validation() {
        let mut event = Event::new("Test", start());
        event.in_language.push("English".into());
        let errors = validate_event(&event);
        assert!(errors.iter().any(|e| e.field.starts_with("in_language")));
    }

    /// A US phone number normalizes to E.164-like `+1…` form.
    #[test]
    fn normalize_phone_us() {
        assert_eq!(normalize_phone("(555) 123-4567", "1"), "+15551234567");
    }

    /// Address standardization title-cases city and upper-cases
    /// state/country.
    #[test]
    fn standardize_address_works() {
        let addr = Address {
            use_type: None,
            line1: Some("123 main st.".into()),
            line2: None,
            city: Some("new york".into()),
            state: Some("ny".into()),
            postal_code: Some("10001".into()),
            country: Some("us".into()),
        };
        let std = standardize_address(&addr);
        assert_eq!(std.city.as_deref(), Some("New York"));
        assert_eq!(std.state.as_deref(), Some("NY"));
        assert_eq!(std.country.as_deref(), Some("US"));
    }
}
