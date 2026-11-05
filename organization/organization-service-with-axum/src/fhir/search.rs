//! FHIR `Organization` search parameters + the in-memory match predicate.
//!
//! The supported subset ([`agents/share/fhir.md`](../../../../agents/share/fhir.md)
//! §6): `_id`, `_count`, `identifier` (token), `name`, `address`,
//! `address-city`, `address-postalcode`. `_lastUpdated` is accepted and
//! ignored (no `_history`). Unknown parameters are ignored, not rejected
//! (v1). Filtering is in-memory over the active rows the handler loads,
//! matching the native `check-duplicates` scan model.

use organization_matcher::Organization;
use serde::Deserialize;

use super::scheme_to_system;

/// Default page size when `_count` is absent (mirrors the native list cap
/// intent — bounded responses).
pub const DEFAULT_COUNT: usize = 50;

/// Parsed FHIR `Organization` search query. Every field is optional; a
/// request with none matches all active rows (up to [`DEFAULT_COUNT`]).
#[derive(Debug, Default, Deserialize)]
pub struct FhirOrgSearchParams {
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
    /// `name` — case-insensitive substring over name + aliases.
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

impl FhirOrgSearchParams {
    /// The effective result cap (`_count`, else [`DEFAULT_COUNT`]).
    #[must_use]
    pub fn limit(&self) -> usize {
        self.count.unwrap_or(DEFAULT_COUNT)
    }

    /// Whether the stored `org` (with public id `pid`) matches every
    /// supplied parameter (conjunction; absent parameters don't constrain).
    #[must_use]
    pub fn matches(&self, org: &Organization, pid: &str) -> bool {
        if let Some(ref id) = self.id
            && pid != id
        {
            return false;
        }
        if let Some(ref name) = self.name
            && !name_matches(org, name)
        {
            return false;
        }
        if let Some(ref ident) = self.identifier
            && !identifier_matches(org, ident)
        {
            return false;
        }
        if let Some(ref city) = self.address_city
            && !contains_ci(
                org.address.as_ref().and_then(|a| a.locality.as_deref()),
                city,
            )
        {
            return false;
        }
        if let Some(ref pc) = self.address_postalcode
            && !contains_ci(
                org.address.as_ref().and_then(|a| a.postal_code.as_deref()),
                pc,
            )
        {
            return false;
        }
        if let Some(ref addr) = self.address
            && !address_matches(org, addr)
        {
            return false;
        }
        true
    }
}

/// Case-insensitive substring of `name` over the org's name, legal name,
/// and alternate names.
fn name_matches(org: &Organization, name: &str) -> bool {
    contains_ci(Some(&org.name), name)
        || contains_ci(org.legal_name.as_deref(), name)
        || org
            .alternate_names
            .iter()
            .any(|a| contains_ci(Some(a), name))
}

/// Match an `identifier` token: `system|value` matches both parts; a bare
/// `value` matches any identifier with that value.
fn identifier_matches(org: &Organization, token: &str) -> bool {
    let (system, value) = token
        .split_once('|')
        .map_or((None, token), |(s, v)| (Some(s), v));
    org.identifiers.iter().any(|id| {
        let value_hit = id.value == value;
        match system {
            Some(sys) => value_hit && scheme_to_system(&id.scheme) == sys,
            None => value_hit,
        }
    })
}

/// Match the general `address` param against any address part.
fn address_matches(org: &Organization, needle: &str) -> bool {
    let Some(a) = org.address.as_ref() else {
        return false;
    };
    contains_ci(a.street_address.as_deref(), needle)
        || contains_ci(a.locality.as_deref(), needle)
        || contains_ci(a.region.as_deref(), needle)
        || contains_ci(a.postal_code.as_deref(), needle)
        || contains_ci(a.country.as_deref(), needle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use organization_matcher::{IdentifierScheme, OrgIdentifier, PostalAddress};

    fn acme() -> Organization {
        let mut org = Organization::new("Acme, Inc.");
        org.alternate_names = vec!["Acme".to_string()];
        org.identifiers = vec![OrgIdentifier {
            scheme: IdentifierScheme::Lei,
            value: "5493001KJTIIGC8Y1R12".to_string(),
        }];
        org.address = Some(PostalAddress {
            locality: Some("Springfield".to_string()),
            postal_code: Some("62704".to_string()),
            ..Default::default()
        });
        org
    }

    #[test]
    fn empty_params_match_everything() {
        let p = FhirOrgSearchParams::default();
        assert!(p.matches(&acme(), "pid-1"));
        assert_eq!(p.limit(), DEFAULT_COUNT);
    }

    #[test]
    fn name_matches_alias_case_insensitively() {
        let p = FhirOrgSearchParams {
            name: Some("acme".to_string()),
            ..Default::default()
        };
        assert!(p.matches(&acme(), "pid-1"));
    }

    #[test]
    fn id_must_match_exactly() {
        let p = FhirOrgSearchParams {
            id: Some("pid-1".to_string()),
            ..Default::default()
        };
        assert!(p.matches(&acme(), "pid-1"));
        assert!(!p.matches(&acme(), "pid-2"));
    }

    #[test]
    fn identifier_token_matches_system_and_value() {
        let p = FhirOrgSearchParams {
            identifier: Some("https://www.gleif.org/lei|5493001KJTIIGC8Y1R12".to_string()),
            ..Default::default()
        };
        assert!(p.matches(&acme(), "pid-1"));
        // Wrong system ⇒ no match.
        let p2 = FhirOrgSearchParams {
            identifier: Some("https://www.dnb.com/duns|5493001KJTIIGC8Y1R12".to_string()),
            ..Default::default()
        };
        assert!(!p2.matches(&acme(), "pid-1"));
        // Bare value ⇒ match.
        let p3 = FhirOrgSearchParams {
            identifier: Some("5493001KJTIIGC8Y1R12".to_string()),
            ..Default::default()
        };
        assert!(p3.matches(&acme(), "pid-1"));
    }

    #[test]
    fn address_city_and_postalcode_filter() {
        let p = FhirOrgSearchParams {
            address_city: Some("spring".to_string()),
            address_postalcode: Some("62704".to_string()),
            ..Default::default()
        };
        assert!(p.matches(&acme(), "pid-1"));
        let miss = FhirOrgSearchParams {
            address_city: Some("gotham".to_string()),
            ..Default::default()
        };
        assert!(!miss.matches(&acme(), "pid-1"));
    }
}
