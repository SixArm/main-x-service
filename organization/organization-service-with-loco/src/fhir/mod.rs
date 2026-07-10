//! HL7 FHIR R5 interop for the `Organization` resource.
//!
//! The **reference implementation** of the family FHIR contract
//! ([`agents/share/fhir.md`](../../../../agents/share/fhir.md)) — copied by
//! the other loco-idiomatic entity services. It provides the bidirectional
//! mapping between the stored `organization_matcher::Organization` DTO and
//! the wire-level [`resources::FhirOrganization`]: [`to_fhir_organization`]
//! for outbound responses and [`from_fhir_organization`] for inbound
//! requests. Resource shapes live in [`resources`], search-parameter
//! parsing in [`search`], and the mounted Axum endpoints in
//! [`crate::controllers::fhir`].
//!
//! Conversions are **lossy where the DTO has no FHIR home** — documented
//! inline and gathered in [`from_fhir_organization`]'s doc — never silent.

/// FHIR resource + envelope wire types (`Organization`, `OperationOutcome`,
/// `Bundle`).
pub mod resources;
/// FHIR search-parameter parsing + the in-memory match predicate.
pub mod search;

use organization_matcher::{IdentifierScheme, OrgIdentifier, Organization, PostalAddress};
use resources::{FhirAddress, FhirContactPoint, FhirIdentifier, FhirMeta, FhirOrganization};

/// Map an identifier `scheme` to the FHIR `identifier.system` URI. The
/// well-known registries use their canonical namespace; the rest use a
/// family `urn:mxi:org:*` namespace. [`system_to_scheme`] is the exact
/// inverse, so a scheme round-trips through FHIR unchanged.
#[must_use]
pub fn scheme_to_system(scheme: &IdentifierScheme) -> String {
    match scheme {
        IdentifierScheme::Lei => "https://www.gleif.org/lei".to_string(),
        IdentifierScheme::Duns => "https://www.dnb.com/duns".to_string(),
        IdentifierScheme::Gln => "https://www.gs1.org/gln".to_string(),
        IdentifierScheme::Wikidata => "https://www.wikidata.org/entity".to_string(),
        IdentifierScheme::Ror => "https://ror.org".to_string(),
        IdentifierScheme::Isni => "https://isni.org".to_string(),
        IdentifierScheme::Iso6523 => "urn:mxi:org:iso6523".to_string(),
        IdentifierScheme::Vat => "urn:mxi:org:vat".to_string(),
        IdentifierScheme::TaxId => "urn:mxi:org:taxid".to_string(),
        IdentifierScheme::Naics => "urn:mxi:org:naics".to_string(),
        IdentifierScheme::IsicV4 => "urn:mxi:org:isicv4".to_string(),
        IdentifierScheme::Sic => "urn:mxi:org:sic".to_string(),
        IdentifierScheme::Custom(label) => format!("urn:mxi:org:custom:{label}"),
    }
}

/// Map a FHIR `identifier.system` URI back to an identifier `scheme` — the
/// inverse of [`scheme_to_system`]. An unrecognised system is preserved as
/// `Custom(system)` so an inbound identifier from a foreign namespace is
/// never dropped.
#[must_use]
pub fn system_to_scheme(system: &str) -> IdentifierScheme {
    match system {
        "https://www.gleif.org/lei" => IdentifierScheme::Lei,
        "https://www.dnb.com/duns" => IdentifierScheme::Duns,
        "https://www.gs1.org/gln" => IdentifierScheme::Gln,
        "https://www.wikidata.org/entity" => IdentifierScheme::Wikidata,
        "https://ror.org" => IdentifierScheme::Ror,
        "https://isni.org" => IdentifierScheme::Isni,
        "urn:mxi:org:iso6523" => IdentifierScheme::Iso6523,
        "urn:mxi:org:vat" => IdentifierScheme::Vat,
        "urn:mxi:org:taxid" => IdentifierScheme::TaxId,
        "urn:mxi:org:naics" => IdentifierScheme::Naics,
        "urn:mxi:org:isicv4" => IdentifierScheme::IsicV4,
        "urn:mxi:org:sic" => IdentifierScheme::Sic,
        other => other.strip_prefix("urn:mxi:org:custom:").map_or_else(
            || IdentifierScheme::Custom(other.to_string()),
            |label| IdentifierScheme::Custom(label.to_string()),
        ),
    }
}

/// Map a domain [`PostalAddress`] to a FHIR `Address` (splitting
/// `street_address` on newlines into the `line` array).
fn to_fhir_address(addr: &PostalAddress) -> FhirAddress {
    let line = addr
        .street_address
        .as_deref()
        .map(|s| s.lines().map(str::to_string).collect::<Vec<_>>())
        .unwrap_or_default();
    FhirAddress {
        line,
        city: addr.locality.clone(),
        state: addr.region.clone(),
        postal_code: addr.postal_code.clone(),
        country: addr.country.clone(),
    }
}

/// Map a FHIR `Address` back to a domain [`PostalAddress`] (joining the
/// `line` array with newlines).
fn from_fhir_address(addr: &FhirAddress) -> PostalAddress {
    PostalAddress {
        street_address: (!addr.line.is_empty()).then(|| addr.line.join("\n")),
        locality: addr.city.clone(),
        region: addr.state.clone(),
        postal_code: addr.postal_code.clone(),
        country: addr.country.clone(),
    }
}

/// Render a stored [`Organization`] as a FHIR R5 [`FhirOrganization`].
///
/// `id` is the record's public `pid`; `active` and `last_updated` come
/// from the row. `alias` carries `legal_name` (first, when present)
/// followed by `alternate_names`; `telecom` carries `telephone`/`email`/
/// `url`. `jurisdiction`, `founding_date`, `keywords`, and `same_as` have
/// no FHIR `Organization` element and are not emitted (documented gaps).
#[must_use]
pub fn to_fhir_organization(
    org: &Organization,
    id: &str,
    active: bool,
    last_updated: Option<String>,
) -> FhirOrganization {
    let mut fhir = FhirOrganization::new();
    fhir.id = Some(id.to_string());
    fhir.active = Some(active);
    fhir.meta = last_updated.map(|lu| FhirMeta {
        version_id: None,
        last_updated: Some(lu),
    });
    fhir.name = Some(org.name.clone());

    // alias = legal_name (if any) then alternate_names.
    let mut alias = Vec::new();
    if let Some(ref legal) = org.legal_name {
        alias.push(legal.clone());
    }
    alias.extend(org.alternate_names.iter().cloned());
    fhir.alias = alias;

    fhir.identifier = org
        .identifiers
        .iter()
        .map(|id| FhirIdentifier {
            system: Some(scheme_to_system(&id.scheme)),
            value: Some(id.value.clone()),
        })
        .collect();

    let mut telecom = Vec::new();
    if let Some(ref phone) = org.telephone {
        telecom.push(FhirContactPoint {
            system: Some("phone".to_string()),
            value: Some(phone.clone()),
        });
    }
    if let Some(ref email) = org.email {
        telecom.push(FhirContactPoint {
            system: Some("email".to_string()),
            value: Some(email.clone()),
        });
    }
    if let Some(ref url) = org.url {
        telecom.push(FhirContactPoint {
            system: Some("url".to_string()),
            value: Some(url.clone()),
        });
    }
    fhir.telecom = telecom;

    if let Some(ref addr) = org.address {
        fhir.address = vec![to_fhir_address(addr)];
    }

    fhir
}

/// Parse an inbound [`FhirOrganization`] into a stored [`Organization`].
///
/// `name` is required (a FHIR `Organization` with no `name` is a `400`).
/// `alias` → `alternate_names`; `telecom` phone/email/url → the scalar
/// fields; `identifier` → `identifiers` (system → scheme). The first
/// address is taken.
///
/// **Fidelity gaps** (the DTO has no FHIR `Organization` home, or the
/// reverse is ambiguous): `legal_name` is not recovered from `alias`
/// (all aliases become `alternate_names`); `jurisdiction`,
/// `founding_date`, `keywords`, and `same_as` are not carried by the
/// resource and default to empty.
///
/// # Errors
///
/// Returns the missing-`name` diagnostic string when the resource has no
/// non-empty `name`.
pub fn from_fhir_organization(fhir: &FhirOrganization) -> Result<Organization, String> {
    let name = fhir
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .ok_or_else(|| "Organization.name is required".to_string())?;

    let mut org = Organization::new(name);
    org.alternate_names.clone_from(&fhir.alias);
    org.identifiers = fhir
        .identifier
        .iter()
        .filter_map(|id| {
            let value = id.value.clone()?;
            let scheme = id
                .system
                .as_deref()
                .map_or(IdentifierScheme::Custom(String::new()), system_to_scheme);
            Some(OrgIdentifier { scheme, value })
        })
        .collect();

    for cp in &fhir.telecom {
        let (Some(system), Some(value)) = (cp.system.as_deref(), cp.value.clone()) else {
            continue;
        };
        match system {
            "phone" => org.telephone = Some(value),
            "email" => org.email = Some(value),
            "url" => org.url = Some(value),
            _ => {}
        }
    }

    if let Some(addr) = fhir.address.first() {
        org.address = Some(from_fhir_address(addr));
    }

    Ok(org)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every identifier scheme round-trips through the FHIR `system` URI
    /// unchanged (`scheme → system → scheme` is the identity).
    #[test]
    fn scheme_system_round_trips() {
        let schemes = [
            IdentifierScheme::Lei,
            IdentifierScheme::Duns,
            IdentifierScheme::Iso6523,
            IdentifierScheme::Gln,
            IdentifierScheme::Wikidata,
            IdentifierScheme::Ror,
            IdentifierScheme::Isni,
            IdentifierScheme::Vat,
            IdentifierScheme::TaxId,
            IdentifierScheme::Naics,
            IdentifierScheme::IsicV4,
            IdentifierScheme::Sic,
            IdentifierScheme::Custom("acme-internal".to_string()),
        ];
        for scheme in schemes {
            let system = scheme_to_system(&scheme);
            assert_eq!(system_to_scheme(&system), scheme, "round-trip {scheme:?}");
        }
    }

    /// An unknown inbound `system` is preserved as `Custom(system)`, never
    /// dropped.
    #[test]
    fn unknown_system_becomes_custom() {
        assert_eq!(
            system_to_scheme("https://example.org/foo"),
            IdentifierScheme::Custom("https://example.org/foo".to_string())
        );
    }

    /// The losslessly-reversible fields survive `DTO → FHIR → DTO`.
    #[test]
    fn dto_fhir_round_trip_preserves_core_fields() {
        let mut org = Organization::new("Acme, Inc.");
        org.alternate_names = vec!["Acme".to_string(), "ACME Corp".to_string()];
        org.identifiers = vec![OrgIdentifier {
            scheme: IdentifierScheme::Lei,
            value: "5493001KJTIIGC8Y1R12".to_string(),
        }];
        org.telephone = Some("+1-555-0100".to_string());
        org.email = Some("hello@acme.example".to_string());
        org.url = Some("https://acme.example".to_string());
        org.address = Some(PostalAddress {
            street_address: Some("1 Main St".to_string()),
            locality: Some("Springfield".to_string()),
            region: Some("IL".to_string()),
            postal_code: Some("62704".to_string()),
            country: Some("US".to_string()),
        });

        let fhir = to_fhir_organization(&org, "pid-123", true, None);
        assert_eq!(fhir.resource_type, "Organization");
        assert_eq!(fhir.id.as_deref(), Some("pid-123"));

        let back = from_fhir_organization(&fhir).expect("valid resource");
        assert_eq!(back.name, org.name);
        assert_eq!(back.alternate_names, org.alternate_names);
        assert_eq!(back.identifiers.len(), 1);
        assert_eq!(back.identifiers[0].scheme, IdentifierScheme::Lei);
        assert_eq!(back.identifiers[0].value, org.identifiers[0].value);
        assert_eq!(back.telephone, org.telephone);
        assert_eq!(back.email, org.email);
        assert_eq!(back.url, org.url);
        assert_eq!(back.address, org.address);
    }

    /// A resource with no `name` is rejected (maps to a `400`).
    #[test]
    fn missing_name_is_rejected() {
        let fhir = FhirOrganization::new();
        assert!(from_fhir_organization(&fhir).is_err());
    }
}
