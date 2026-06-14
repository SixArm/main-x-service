//! Privacy and data-masking utilities for events.
//!
//! Events are less PII-heavy than people: most fields (name,
//! description, location) are public. The remaining sensitive
//! surfaces are party emails / URLs / personal-identifier IDs, plus
//! identifier values when they double as access tokens (booking
//! numbers, confirmation codes).

use crate::models::{Event, Party};

/// Return a copy of `event` with sensitive fields masked.
pub fn mask_event(event: &Event) -> Event {
    let mut masked = event.clone();

    // Mask all identifier values — they may double as access tokens.
    for id in &mut masked.identifiers {
        id.value = mask_value(&id.value, 4);
    }

    // Mask attendee / performer / organizer party emails.
    mask_parties(&mut masked.attendees);
    mask_parties(&mut masked.performers);
    mask_parties(&mut masked.organizers);
    mask_parties(&mut masked.sponsors);
    mask_parties(&mut masked.funders);
    mask_parties(&mut masked.contributors);

    masked
}

/// Mask each party's email in place and strip its external `id`, so a
/// masked view never leaks personal contact details or cross-service
/// references.
fn mask_parties(parties: &mut [Party]) {
    for p in parties {
        if let Some(ref email) = p.email {
            p.email = Some(mask_value(email, 4));
        }
        // External IDs should not be exposed in masked view.
        p.id = None;
    }
}

/// Mask a value, keeping only the last `visible_chars` characters
/// visible. Non-alphanumeric characters are preserved as separators.
fn mask_value(value: &str, visible_chars: usize) -> String {
    if value.len() <= visible_chars {
        return value.to_string();
    }
    let visible_start = value.len() - visible_chars;
    let masked_part: String = value[..visible_start]
        .chars()
        .map(|c| if c.is_alphanumeric() { '*' } else { c })
        .collect();
    format!("{masked_part}{}", &value[visible_start..])
}

/// Whether `consents` contains an active, non-expired consent of the
/// given type.
pub fn has_active_consent(
    consents: &[crate::models::Consent],
    consent_type: crate::models::ConsentType,
) -> bool {
    let today = chrono::Utc::now().date_naive();
    consents.iter().any(|c| {
        c.consent_type == consent_type
            && c.status == crate::models::ConsentStatus::Active
            && c.expiry_date.map_or(true, |exp| exp >= today)
    })
}

/// GDPR right-of-access export: every field stored about an event.
pub fn export_event_data(event: &Event) -> serde_json::Value {
    serde_json::to_value(event).unwrap_or(serde_json::Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Identifier, IdentifierType, Party, PartyKind};
    use chrono::TimeZone;

    /// Only the last 4 chars stay visible; separators are preserved and
    /// short values pass through unchanged.
    #[test]
    fn mask_value_keeps_last_4() {
        // Last 4 chars remain visible; alphanumerics before are masked,
        // non-alphanumeric separators (the "-") are preserved.
        assert_eq!(mask_value("ABC-12345", 4), "***-*2345");
        assert_eq!(mask_value("short", 10), "short");
    }

    /// `mask_event` masks identifier values (potential access tokens).
    #[test]
    fn mask_event_masks_identifiers() {
        let mut event = Event::new(
            "Concert",
            chrono::Utc.with_ymd_and_hms(2026, 3, 1, 9, 0, 0).unwrap(),
        );
        event.identifiers.push(Identifier::new(
            IdentifierType::TicketNumber,
            "sys".into(),
            "T-987654".into(),
        ));
        let masked = mask_event(&event);
        // Last 4 visible (7654); the "-" separator survives.
        assert_eq!(masked.identifiers[0].value, "*-**7654");
    }

    /// `mask_event` masks party emails, strips external IDs, keeps names.
    #[test]
    fn mask_event_masks_party_emails() {
        let mut event = Event::new(
            "X",
            chrono::Utc.with_ymd_and_hms(2026, 3, 1, 9, 0, 0).unwrap(),
        );
        event.attendees.push(Party {
            kind: PartyKind::Person,
            id: Some(uuid::Uuid::new_v4()),
            name: "Alice".into(),
            email: Some("alice@example.test".into()),
            url: None,
        });
        let masked = mask_event(&event);
        assert!(
            masked.attendees[0]
                .email
                .as_ref()
                .unwrap()
                .ends_with("test")
        );
        assert!(masked.attendees[0].email.as_ref().unwrap().contains('*'));
        // External ID should be stripped from the masked view.
        assert!(masked.attendees[0].id.is_none());
        // Name is not PII for event display.
        assert_eq!(masked.attendees[0].name, "Alice");
    }

    /// The GDPR export includes the core stored fields.
    #[test]
    fn export_includes_all_fields() {
        let event = Event::new(
            "X",
            chrono::Utc.with_ymd_and_hms(2026, 3, 1, 9, 0, 0).unwrap(),
        );
        let exported = export_event_data(&event);
        let obj = exported.as_object().unwrap();
        assert!(obj.contains_key("name"));
        assert!(obj.contains_key("start_date"));
        assert!(obj.contains_key("event_status"));
    }
}
