//! HL7 FHIR R5 interop for the `Device` resource.
//!
//! Maps the stored [`crate::models::thing::Thing`] DTO to the wire-level
//! [`resources::FhirDevice`] and back: [`to_fhir_device`] for outbound
//! responses and [`from_fhir_device`] for inbound requests. Resource
//! shapes live in [`resources`], search-parameter parsing in [`search`],
//! and the mounted Axum endpoints in [`crate::controllers::fhir`]. This
//! follows the family FHIR contract
//! ([`agents/share/fhir.md`](../../../../agents/share/fhir.md), `medium`
//! fidelity — the core fields map, but `Device`'s clinical structure is
//! only partly populated).
//!
//! Conversions are **lossy where the DTO has no FHIR home** — documented
//! inline and gathered in [`from_fhir_device`]'s doc — never silent.
//! `Substance`/`Medication` (the other FHIR device-adjacent resources)
//! are out of v1 scope.

/// FHIR resource + envelope wire types (`Device`, `OperationOutcome`,
/// `Bundle`).
pub mod resources;
/// FHIR search-parameter parsing + the in-memory match predicate.
pub mod search;

use crate::models::identifier::{IdentifierType, ThingIdentifier};
use crate::models::thing::Thing;
use resources::{
    FhirAnnotation, FhirCodeableConcept, FhirDevice, FhirDeviceName, FhirIdentifier, FhirMeta,
};

/// `Device.name.type` for the primary name (the domain `name`).
const PRIMARY_NAME_TYPE: &str = "registered-name";
/// `Device.name.type` for the alias names (the domain `alternate_names`).
const ALIAS_NAME_TYPE: &str = "user-friendly-name";

/// Map an identifier [`IdentifierType`] to the FHIR `identifier.system`
/// URI. Well-known schemes use their canonical namespace; the rest use a
/// family `urn:mxi:thing:*` namespace. [`system_to_scheme`] is the exact
/// inverse, so a scheme round-trips through FHIR unchanged.
#[must_use]
pub fn scheme_to_system(scheme: &IdentifierType) -> String {
    match scheme {
        IdentifierType::Doi => "https://doi.org".to_string(),
        IdentifierType::Isbn => "urn:isbn".to_string(),
        IdentifierType::Issn => "urn:issn".to_string(),
        IdentifierType::Gtin => "https://www.gs1.org/gtin".to_string(),
        IdentifierType::Sku => "urn:mxi:thing:sku".to_string(),
        IdentifierType::Mpn => "urn:mxi:thing:mpn".to_string(),
        IdentifierType::SerialNumber => "urn:mxi:thing:serial".to_string(),
        IdentifierType::Uri => "urn:mxi:thing:uri".to_string(),
        IdentifierType::Uuid => "urn:ietf:rfc:4122".to_string(),
        IdentifierType::Custom(label) => format!("urn:mxi:thing:custom:{label}"),
    }
}

/// Map a FHIR `identifier.system` URI back to an [`IdentifierType`] — the
/// inverse of [`scheme_to_system`]. An unrecognised system is preserved as
/// `Custom(system)` so an inbound identifier from a foreign namespace is
/// never dropped.
#[must_use]
pub fn system_to_scheme(system: &str) -> IdentifierType {
    match system {
        "https://doi.org" => IdentifierType::Doi,
        "urn:isbn" => IdentifierType::Isbn,
        "urn:issn" => IdentifierType::Issn,
        "https://www.gs1.org/gtin" => IdentifierType::Gtin,
        "urn:mxi:thing:sku" => IdentifierType::Sku,
        "urn:mxi:thing:mpn" => IdentifierType::Mpn,
        "urn:mxi:thing:serial" => IdentifierType::SerialNumber,
        "urn:mxi:thing:uri" => IdentifierType::Uri,
        "urn:ietf:rfc:4122" => IdentifierType::Uuid,
        other => other.strip_prefix("urn:mxi:thing:custom:").map_or_else(
            || IdentifierType::Custom(other.to_string()),
            |label| IdentifierType::Custom(label.to_string()),
        ),
    }
}

/// Render a stored [`Thing`] as a FHIR R5 [`FhirDevice`].
///
/// `id`, `status` (from `is_deleted`), and `meta.lastUpdated` (from
/// `updated_at`) come from the record. `name` carries the primary `name`
/// first (`registered-name`) then each `alternate_names` entry
/// (`user-friendly-name`); `identifier` carries the typed identifiers as
/// `system|value` tokens; `type.text` carries `additional_type`;
/// `manufacturer` carries `owner`; `modelNumber` carries
/// `disambiguating_description`; `note` carries `description`.
///
/// **Fidelity gaps** (the DTO field has no FHIR `Device` home and is not
/// emitted): `url`, `images`, `main_entity_of_page`, `same_as`,
/// `subject_of`, `potential_action`, and the optional per-identifier
/// `name`/`url`. The `manufacturer`/`modelNumber` mappings are
/// **approximate** (the nearest available domain fields), reflecting the
/// `medium` fidelity of a generic-thing → clinical-device projection.
#[must_use]
pub fn to_fhir_device(thing: &Thing) -> FhirDevice {
    let mut fhir = FhirDevice::new();
    fhir.id = Some(thing.id.to_string());
    fhir.status = Some(if thing.is_deleted { "inactive" } else { "active" }.to_string());
    fhir.meta = Some(FhirMeta {
        version_id: None,
        last_updated: Some(thing.updated_at.to_rfc3339()),
    });

    let mut names = vec![FhirDeviceName {
        value: thing.name.clone(),
        name_type: PRIMARY_NAME_TYPE.to_string(),
    }];
    names.extend(thing.alternate_names.iter().map(|a| FhirDeviceName {
        value: a.clone(),
        name_type: ALIAS_NAME_TYPE.to_string(),
    }));
    fhir.name = names;

    fhir.identifier = thing
        .identifiers
        .iter()
        .map(|id| FhirIdentifier {
            system: Some(scheme_to_system(&id.property_id)),
            value: Some(id.value.clone()),
        })
        .collect();

    fhir.device_type = thing.additional_type.clone().map(|t| FhirCodeableConcept {
        coding: Vec::new(),
        text: Some(t),
    });
    fhir.manufacturer.clone_from(&thing.owner);
    fhir.model_number.clone_from(&thing.disambiguating_description);
    fhir.note = thing
        .description
        .clone()
        .map(|text| vec![FhirAnnotation { text }])
        .unwrap_or_default();

    fhir
}

/// Parse an inbound [`FhirDevice`] into a stored [`Thing`].
///
/// A non-empty primary `name` is required (a `Device` with no usable
/// `name` is a `400`). The first `name` entry becomes `name`; the rest
/// become `alternate_names`. `identifier` → `identifiers` (system →
/// scheme); `type.text` (or the first coding's `display`/`code`) →
/// `additional_type`; `manufacturer` → `owner`; `modelNumber` →
/// `disambiguating_description`; the first `note` → `description`.
///
/// **Fidelity gaps** (no inbound source, defaulted): `url`, `images`,
/// `main_entity_of_page`, `same_as`, `subject_of`, `potential_action`,
/// and the per-identifier `name`/`url`.
///
/// # Errors
///
/// Returns the missing-`name` diagnostic string when the resource has no
/// non-empty primary `name`.
pub fn from_fhir_device(fhir: &FhirDevice) -> Result<Thing, String> {
    let name = fhir
        .name
        .iter()
        .map(|n| n.value.trim())
        .find(|v| !v.is_empty())
        .ok_or_else(|| "Device.name is required".to_string())?;

    let mut thing = Thing::new(name);
    // Every name entry after the first non-empty primary becomes an alias.
    let mut seen_primary = false;
    for entry in &fhir.name {
        let value = entry.value.trim();
        if value.is_empty() {
            continue;
        }
        if !seen_primary && value == name {
            seen_primary = true;
            continue;
        }
        thing.alternate_names.push(value.to_string());
    }

    thing.identifiers = fhir
        .identifier
        .iter()
        .filter_map(|id| {
            let value = id.value.clone()?;
            let scheme = id
                .system
                .as_deref()
                .map_or(IdentifierType::Custom(String::new()), system_to_scheme);
            Some(ThingIdentifier::new(scheme, &value))
        })
        .collect();

    thing.additional_type = fhir.device_type.as_ref().and_then(codeable_concept_text);
    thing.owner.clone_from(&fhir.manufacturer);
    thing
        .disambiguating_description
        .clone_from(&fhir.model_number);
    thing.description = fhir.note.first().map(|n| n.text.clone());

    Ok(thing)
}

/// Extract a plain-text label from a [`FhirCodeableConcept`]: prefer
/// `text`, else the first coding's `display`, else its `code`.
fn codeable_concept_text(cc: &FhirCodeableConcept) -> Option<String> {
    if let Some(text) = cc.text.as_deref()
        && !text.trim().is_empty()
    {
        return Some(text.to_string());
    }
    cc.coding
        .first()
        .and_then(|c| c.display.clone().or_else(|| c.code.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every identifier scheme round-trips through the FHIR `system` URI
    /// unchanged (`scheme → system → scheme` is the identity).
    #[test]
    fn scheme_system_round_trips() {
        let schemes = [
            IdentifierType::Doi,
            IdentifierType::Isbn,
            IdentifierType::Issn,
            IdentifierType::Gtin,
            IdentifierType::Sku,
            IdentifierType::Mpn,
            IdentifierType::SerialNumber,
            IdentifierType::Uri,
            IdentifierType::Uuid,
            IdentifierType::Custom("open-library".to_string()),
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
            IdentifierType::Custom("https://example.org/foo".to_string())
        );
    }

    /// The losslessly-reversible fields survive `DTO → FHIR → DTO`.
    #[test]
    fn dto_fhir_round_trip_preserves_core_fields() {
        let mut thing = Thing::new("Pride and Prejudice");
        thing.alternate_names = vec!["First Impressions".to_string()];
        thing.additional_type = Some("https://schema.org/Book".to_string());
        thing.owner = Some("Penguin".to_string());
        thing.disambiguating_description = Some("Penguin Classics ed.".to_string());
        thing.description = Some("A novel by Jane Austen.".to_string());
        thing.identifiers = vec![
            ThingIdentifier::isbn("9780141439518"),
            ThingIdentifier::doi("10.1000/xyz123"),
        ];

        let fhir = to_fhir_device(&thing);
        assert_eq!(fhir.resource_type, "Device");
        assert_eq!(fhir.id.as_deref(), Some(thing.id.to_string().as_str()));
        assert_eq!(fhir.status.as_deref(), Some("active"));

        let back = from_fhir_device(&fhir).expect("valid resource");
        assert_eq!(back.name, thing.name);
        assert_eq!(back.alternate_names, thing.alternate_names);
        assert_eq!(back.additional_type, thing.additional_type);
        assert_eq!(back.owner, thing.owner);
        assert_eq!(
            back.disambiguating_description,
            thing.disambiguating_description
        );
        assert_eq!(back.description, thing.description);
        assert_eq!(back.identifiers.len(), 2);
        assert_eq!(back.identifiers[0].property_id, IdentifierType::Isbn);
        assert_eq!(back.identifiers[0].value, "9780141439518");
        assert_eq!(back.identifiers[1].property_id, IdentifierType::Doi);
    }

    /// A resource with no usable `name` is rejected (maps to a `400`).
    #[test]
    fn missing_name_is_rejected() {
        let fhir = FhirDevice::new();
        assert!(from_fhir_device(&fhir).is_err());
        // A blank-only name is also rejected.
        let mut blank = FhirDevice::new();
        blank.name = vec![FhirDeviceName {
            value: "   ".to_string(),
            name_type: PRIMARY_NAME_TYPE.to_string(),
        }];
        assert!(from_fhir_device(&blank).is_err());
    }

    /// A soft-deleted record renders `status = inactive`.
    #[test]
    fn soft_deleted_renders_inactive_status() {
        let mut thing = Thing::new("Obsolete");
        thing.soft_delete();
        assert_eq!(to_fhir_device(&thing).status.as_deref(), Some("inactive"));
    }
}
