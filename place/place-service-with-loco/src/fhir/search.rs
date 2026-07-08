//! FHIR `Location` search parameters + the in-memory match predicate.
//!
//! The supported subset ([`agents/share/fhir.md`](../../../../agents/share/fhir.md)
//! §6): `_id`, `_count`, `identifier` (token), `name`, `address`,
//! `address-city`, `address-postalcode`. `_lastUpdated` is accepted and
//! ignored (no `_history`). Unknown parameters are ignored, not rejected
//! (v1). Filtering is in-memory over the active rows the handler loads,
//! matching the native list/scan model.

use serde::Deserialize;

use super::identifier_type_to_system;
use crate::models::place::Place;

/// Default page size when `_count` is absent (mirrors the native list cap
/// intent — bounded responses).
pub const DEFAULT_COUNT: usize = 50;

/// Parsed FHIR `Location` search query. Every field is optional; a request
/// with none matches all active rows (up to [`DEFAULT_COUNT`]).
#[derive(Debug, Default, Deserialize)]
pub struct FhirLocationSearchParams {
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
    /// `name` — case-insensitive substring over name + alternate name.
    #[serde(default)]
    pub name: Option<String>,
    /// `address` — case-insensitive substring over any address part.
    #[serde(default)]
    pub address: Option<String>,
    /// `address-city` — case-insensitive substring over locality.
    #[serde(rename = "address-city", default)]
    pub address_city: Option<String>,
    /// `address-postalcode` — case-insensitive substring over postal code.
    #[serde(rename = "address-postalcode", default)]
    pub address_postalcode: Option<String>,
}

/// Case-insensitive substring test where a blank needle matches nothing
/// meaningful (callers only pass `Some` when they mean to filter).
fn contains_ci(haystack: Option<&str>, needle: &str) -> bool {
    haystack.is_some_and(|h| h.to_lowercase().contains(&needle.to_lowercase()))
}

impl FhirLocationSearchParams {
    /// The effective result cap (`_count`, else [`DEFAULT_COUNT`]).
    #[must_use]
    pub fn limit(&self) -> usize {
        self.count.unwrap_or(DEFAULT_COUNT)
    }

    /// Whether the stored `place` (with id `pid`) matches every supplied
    /// parameter (conjunction; absent parameters don't constrain).
    #[must_use]
    pub fn matches(&self, place: &Place, pid: &str) -> bool {
        if let Some(ref id) = self.id
            && pid != id
        {
            return false;
        }
        if let Some(ref name) = self.name
            && !name_matches(place, name)
        {
            return false;
        }
        if let Some(ref ident) = self.identifier
            && !identifier_matches(place, ident)
        {
            return false;
        }
        if let Some(ref city) = self.address_city
            && !contains_ci(
                place.address.as_ref().and_then(|a| a.address_locality.as_deref()),
                city,
            )
        {
            return false;
        }
        if let Some(ref pc) = self.address_postalcode
            && !contains_ci(
                place.address.as_ref().and_then(|a| a.postal_code.as_deref()),
                pc,
            )
        {
            return false;
        }
        if let Some(ref addr) = self.address
            && !address_matches(place, addr)
        {
            return false;
        }
        true
    }
}

/// Case-insensitive substring of `name` over the place's name and alternate
/// name.
fn name_matches(place: &Place, name: &str) -> bool {
    contains_ci(Some(&place.name), name) || contains_ci(place.alternate_name.as_deref(), name)
}

/// Match an `identifier` token: `system|value` matches both parts; a bare
/// `value` matches any identifier with that value. The scalar GLN and
/// branch-code fields participate alongside the `identifiers` vec.
fn identifier_matches(place: &Place, token: &str) -> bool {
    let (system, value) = token
        .split_once('|')
        .map_or((None, token), |(s, v)| (Some(s), v));

    // (system_uri, value) pairs for every identifier the place carries.
    let mut pairs: Vec<(String, &str)> = Vec::new();
    if let Some(ref gln) = place.global_location_number {
        pairs.push(("https://www.gs1.org/gln".to_string(), gln.as_str()));
    }
    if let Some(ref branch) = place.branch_code {
        pairs.push(("urn:mxi:place:branchcode".to_string(), branch.as_str()));
    }
    for id in &place.identifiers {
        pairs.push((identifier_type_to_system(&id.identifier_type), id.value.as_str()));
    }

    pairs.iter().any(|(sys, val)| {
        let value_hit = *val == value;
        match system {
            Some(want) => value_hit && sys == want,
            None => value_hit,
        }
    })
}

/// Match the general `address` param against any address part.
fn address_matches(place: &Place, needle: &str) -> bool {
    let Some(a) = place.address.as_ref() else {
        return false;
    };
    contains_ci(a.street_address.as_deref(), needle)
        || contains_ci(a.address_locality.as_deref(), needle)
        || contains_ci(a.address_region.as_deref(), needle)
        || contains_ci(a.postal_code.as_deref(), needle)
        || contains_ci(a.address_country.as_deref(), needle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::address::PostalAddress;
    use crate::models::identifier::{IdentifierType, PlaceIdentifier};

    fn central_park() -> Place {
        let mut place = Place::new("Central Park");
        place.alternate_name = Some("The Park".to_string());
        place.global_location_number = Some("1234567890123".to_string());
        place.identifiers = vec![PlaceIdentifier::new(IdentifierType::Fips, "36061")];
        place.address = Some(PostalAddress {
            address_locality: Some("New York".to_string()),
            postal_code: Some("10022".to_string()),
            ..Default::default()
        });
        place
    }

    #[test]
    fn empty_params_match_everything() {
        let p = FhirLocationSearchParams::default();
        assert!(p.matches(&central_park(), "pid-1"));
        assert_eq!(p.limit(), DEFAULT_COUNT);
    }

    #[test]
    fn name_matches_alternate_case_insensitively() {
        let p = FhirLocationSearchParams {
            name: Some("the park".to_string()),
            ..Default::default()
        };
        assert!(p.matches(&central_park(), "pid-1"));
    }

    #[test]
    fn id_must_match_exactly() {
        let p = FhirLocationSearchParams {
            id: Some("pid-1".to_string()),
            ..Default::default()
        };
        assert!(p.matches(&central_park(), "pid-1"));
        assert!(!p.matches(&central_park(), "pid-2"));
    }

    #[test]
    fn identifier_token_matches_system_and_value() {
        // GLN system + value hits the scalar GLN field.
        let p = FhirLocationSearchParams {
            identifier: Some("https://www.gs1.org/gln|1234567890123".to_string()),
            ..Default::default()
        };
        assert!(p.matches(&central_park(), "pid-1"));
        // Wrong system ⇒ no match.
        let p2 = FhirLocationSearchParams {
            identifier: Some("urn:mxi:place:fips|1234567890123".to_string()),
            ..Default::default()
        };
        assert!(!p2.matches(&central_park(), "pid-1"));
        // Bare value against a vec identifier ⇒ match.
        let p3 = FhirLocationSearchParams {
            identifier: Some("36061".to_string()),
            ..Default::default()
        };
        assert!(p3.matches(&central_park(), "pid-1"));
    }

    #[test]
    fn address_city_and_postalcode_filter() {
        let p = FhirLocationSearchParams {
            address_city: Some("new".to_string()),
            address_postalcode: Some("10022".to_string()),
            ..Default::default()
        };
        assert!(p.matches(&central_park(), "pid-1"));
        let miss = FhirLocationSearchParams {
            address_city: Some("boston".to_string()),
            ..Default::default()
        };
        assert!(!miss.matches(&central_park(), "pid-1"));
    }
}
