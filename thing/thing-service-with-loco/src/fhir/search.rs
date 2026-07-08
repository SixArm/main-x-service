//! FHIR `Device` search parameters + the in-memory match predicate.
//!
//! The supported subset ([`agents/share/fhir.md`](../../../../agents/share/fhir.md)
//! §6): `_id`, `_lastUpdated`, `_count`, `identifier` (token), `type`,
//! `manufacturer`. `_lastUpdated` is accepted and ignored (no
//! `_history`). Unknown parameters are ignored, not rejected (v1).
//! Filtering is in-memory over the active rows the handler loads,
//! matching the native duplicate-scan model.

use serde::Deserialize;

use super::scheme_to_system;
use crate::models::thing::Thing;

/// Default page size when `_count` is absent (mirrors the native list cap
/// intent — bounded responses).
pub const DEFAULT_COUNT: usize = 50;

/// Parsed FHIR `Device` search query. Every field is optional; a request
/// with none matches all active rows (up to [`DEFAULT_COUNT`]).
#[derive(Debug, Default, Deserialize)]
pub struct FhirDeviceSearchParams {
    /// `_id` — exact match on the resource id.
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
    /// `type` — case-insensitive substring over the device type
    /// (`additional_type`).
    #[serde(rename = "type", default)]
    pub device_type: Option<String>,
    /// `manufacturer` — case-insensitive substring over the manufacturer
    /// (`owner`).
    #[serde(default)]
    pub manufacturer: Option<String>,
}

/// Case-insensitive substring test where an absent haystack never matches.
fn contains_ci(haystack: Option<&str>, needle: &str) -> bool {
    haystack.is_some_and(|h| h.to_lowercase().contains(&needle.to_lowercase()))
}

impl FhirDeviceSearchParams {
    /// The effective result cap (`_count`, else [`DEFAULT_COUNT`]).
    #[must_use]
    pub fn limit(&self) -> usize {
        self.count.unwrap_or(DEFAULT_COUNT)
    }

    /// Whether the stored `thing` matches every supplied parameter
    /// (conjunction; absent parameters don't constrain).
    #[must_use]
    pub fn matches(&self, thing: &Thing) -> bool {
        if let Some(ref id) = self.id
            && thing.id.to_string() != *id
        {
            return false;
        }
        if let Some(ref ident) = self.identifier
            && !identifier_matches(thing, ident)
        {
            return false;
        }
        if let Some(ref ty) = self.device_type
            && !contains_ci(thing.additional_type.as_deref(), ty)
        {
            return false;
        }
        if let Some(ref mfr) = self.manufacturer
            && !contains_ci(thing.owner.as_deref(), mfr)
        {
            return false;
        }
        true
    }
}

/// Match an `identifier` token: `system|value` matches both parts; a bare
/// `value` matches any identifier with that value.
fn identifier_matches(thing: &Thing, token: &str) -> bool {
    let (system, value) = token
        .split_once('|')
        .map_or((None, token), |(s, v)| (Some(s), v));
    thing.identifiers.iter().any(|id| {
        let value_hit = id.value == value;
        match system {
            Some(sys) => value_hit && scheme_to_system(&id.property_id) == sys,
            None => value_hit,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::identifier::ThingIdentifier;

    fn book() -> Thing {
        let mut thing = Thing::new("Pride and Prejudice");
        thing.additional_type = Some("https://schema.org/Book".to_string());
        thing.owner = Some("Penguin Random House".to_string());
        thing.identifiers = vec![ThingIdentifier::isbn("9780141439518")];
        thing
    }

    #[test]
    fn empty_params_match_everything() {
        let p = FhirDeviceSearchParams::default();
        assert!(p.matches(&book()));
        assert_eq!(p.limit(), DEFAULT_COUNT);
    }

    #[test]
    fn id_must_match_exactly() {
        let thing = book();
        let hit = FhirDeviceSearchParams {
            id: Some(thing.id.to_string()),
            ..Default::default()
        };
        assert!(hit.matches(&thing));
        let miss = FhirDeviceSearchParams {
            id: Some("00000000-0000-0000-0000-000000000000".to_string()),
            ..Default::default()
        };
        assert!(!miss.matches(&thing));
    }

    #[test]
    fn identifier_token_matches_system_and_value() {
        let p = FhirDeviceSearchParams {
            identifier: Some("urn:isbn|9780141439518".to_string()),
            ..Default::default()
        };
        assert!(p.matches(&book()));
        // Wrong system ⇒ no match.
        let wrong = FhirDeviceSearchParams {
            identifier: Some("https://doi.org|9780141439518".to_string()),
            ..Default::default()
        };
        assert!(!wrong.matches(&book()));
        // Bare value ⇒ match.
        let bare = FhirDeviceSearchParams {
            identifier: Some("9780141439518".to_string()),
            ..Default::default()
        };
        assert!(bare.matches(&book()));
    }

    #[test]
    fn type_and_manufacturer_filter_case_insensitively() {
        let p = FhirDeviceSearchParams {
            device_type: Some("book".to_string()),
            manufacturer: Some("penguin".to_string()),
            ..Default::default()
        };
        assert!(p.matches(&book()));
        let miss = FhirDeviceSearchParams {
            manufacturer: Some("acme".to_string()),
            ..Default::default()
        };
        assert!(!miss.matches(&book()));
    }
}
