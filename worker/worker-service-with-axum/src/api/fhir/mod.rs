//! HL7 FHIR R5 API: resource types, conversions, and endpoint handlers.
//!
//! Lets FHIR-aware clients (EHRs, integration engines) talk to the service in
//! their native format. This module owns the bidirectional mapping between the
//! internal [`Worker`] model and the wire-format [`FhirWorker`] resource
//! ([`to_fhir_worker`] / [`from_fhir_worker`]); the resource structs live in
//! [`resources`], `Bundle` handling in [`bundle`], search parameters in
//! [`search_parameters`], and the Axum handlers in [`handlers`].
//!
//! The conversions are field-by-field and lossy in both directions — several
//! internal fields have no FHIR slot here and several FHIR fields are not yet
//! parsed back (marked `TODO` inline). No doctests: constructing a meaningful
//! [`Worker`]/[`FhirWorker`] pair is verbose and the mapping is covered by the
//! crate's tests.

use crate::Result;
use crate::api::fhir::resources::{
    FhirCodeableConcept, FhirCoding, FhirHumanName, FhirIdentifier, FhirReference,
};
use crate::models::{Address, ContactPoint, Gender, HumanName, Identifier, Worker};

/// FHIR `Bundle` handling for search responses.
pub mod bundle;
/// Axum handlers for the FHIR R5 `Worker` endpoints.
pub mod handlers;
/// FHIR resource structs and converters.
pub mod resources;
/// FHIR search-parameter parsing.
pub mod search_parameters;

pub use resources::{FhirOperationOutcome, FhirWorker};

/// Maps an internal [`HumanName`] to a FHIR `HumanName`, omitting empty lists.
fn to_fhir_human_name(name: &HumanName, text: String) -> FhirHumanName {
    FhirHumanName {
        use_: name
            .use_type
            .as_ref()
            .map(|u| format!("{u:?}").to_lowercase()),
        text: Some(text),
        family: Some(name.family.clone()),
        given: (!name.given.is_empty()).then(|| name.given.clone()),
        prefix: (!name.prefix.is_empty()).then(|| name.prefix.clone()),
        suffix: (!name.suffix.is_empty()).then(|| name.suffix.clone()),
    }
}

/// Builds the FHIR identifier list for `worker`.
fn to_fhir_identifiers(worker: &Worker) -> Vec<FhirIdentifier> {
    worker
        .identifiers
        .iter()
        .map(|id| FhirIdentifier {
            use_: id
                .use_type
                .as_ref()
                .map(|u| format!("{u:?}").to_lowercase()),
            type_: Some(FhirCodeableConcept {
                coding: Some(vec![FhirCoding {
                    system: Some(id.system.clone()),
                    code: Some(id.identifier_type.to_string()),
                    display: Some(id.identifier_type.to_string()),
                }]),
                text: Some(id.identifier_type.to_string()),
            }),
            system: Some(id.system.clone()),
            value: Some(id.value.clone()),
            assigner: id.assigner.as_ref().map(|a| FhirReference {
                reference: None,
                display: Some(a.clone()),
            }),
        })
        .collect()
}

/// Builds the FHIR telecom list for `worker`.
fn to_fhir_telecom_points(worker: &Worker) -> Vec<resources::FhirContactPoint> {
    worker
        .telecom
        .iter()
        .map(|cp| resources::FhirContactPoint {
            system: Some(format!("{:?}", cp.system).to_lowercase()),
            value: Some(cp.value.clone()),
            use_: cp
                .use_type
                .as_ref()
                .map(|u| format!("{u:?}").to_lowercase()),
        })
        .collect()
}

/// Builds the FHIR name list (primary name first, then additional names).
fn to_fhir_names(worker: &Worker) -> Vec<FhirHumanName> {
    let mut names = vec![to_fhir_human_name(&worker.name, worker.full_name())];
    for add_name in &worker.additional_names {
        let text = format!("{} {}", add_name.given.join(" "), add_name.family);
        names.push(to_fhir_human_name(add_name, text));
    }
    names
}

/// Maps an internal [`Worker`] to a FHIR R5 `Practitioner`-shaped [`FhirWorker`].
///
/// Copies identity, names (primary plus additional), telecom, gender, birth
/// date, deceased state, addresses, marital status, multiple-birth flag,
/// links, and managing organization, formatting enums as lowercase FHIR codes
/// and building references as `Resource/{id}` strings. Fields the internal
/// model does not carry (e.g. address `use`/`type`) are left `None`.
#[must_use]
pub fn to_fhir_worker(worker: &Worker) -> FhirWorker {
    use resources::{
        FhirAddress, FhirDeceased, FhirMeta, FhirMultipleBirth, FhirWorker, FhirWorkerLink,
    };

    let mut fhir_worker = FhirWorker::new();

    // Basic fields
    fhir_worker.id = Some(worker.id.to_string());
    fhir_worker.active = Some(worker.active);

    // Meta
    fhir_worker.meta = Some(FhirMeta {
        version_id: None,
        last_updated: Some(worker.updated_at.to_string()),
    });

    // Identifiers
    if !worker.identifiers.is_empty() {
        fhir_worker.identifier = Some(to_fhir_identifiers(worker));
    }

    // Names (primary plus any additional)
    fhir_worker.name = Some(to_fhir_names(worker));

    // Telecom
    if !worker.telecom.is_empty() {
        fhir_worker.telecom = Some(to_fhir_telecom_points(worker));
    }

    // Gender
    fhir_worker.gender = Some(format!("{:?}", worker.gender).to_lowercase());

    // Birth date
    fhir_worker.birth_date = worker.birth_date.map(|d| d.to_string());

    // Deceased
    if worker.deceased {
        fhir_worker.deceased = Some(if let Some(dt) = worker.deceased_datetime {
            FhirDeceased::DateTime(dt.to_rfc3339())
        } else {
            FhirDeceased::Boolean(true)
        });
    }

    // Addresses
    if !worker.addresses.is_empty() {
        fhir_worker.address = Some(
            worker
                .addresses
                .iter()
                .map(|addr| {
                    let mut lines = Vec::new();
                    if let Some(ref l1) = addr.line1 {
                        lines.push(l1.clone());
                    }
                    if let Some(ref l2) = addr.line2 {
                        lines.push(l2.clone());
                    }

                    FhirAddress {
                        use_: None,  // Not stored in our model
                        type_: None, // Not stored in our model
                        text: None,  // Not stored in our model
                        line: if lines.is_empty() { None } else { Some(lines) },
                        city: addr.city.clone(),
                        state: addr.state.clone(),
                        postal_code: addr.postal_code.clone(),
                        country: addr.country.clone(),
                    }
                })
                .collect(),
        );
    }

    // Marital status
    if let Some(ref status) = worker.marital_status {
        fhir_worker.marital_status = Some(FhirCodeableConcept {
            coding: Some(vec![FhirCoding {
                system: Some("http://terminology.hl7.org/CodeSystem/v3-MaritalStatus".to_string()),
                code: Some(status.clone()),
                display: Some(status.clone()),
            }]),
            text: Some(status.clone()),
        });
    }

    // Multiple birth
    if let Some(mb) = worker.multiple_birth {
        fhir_worker.multiple_birth = Some(FhirMultipleBirth::Boolean(mb));
    }

    // Links
    if !worker.links.is_empty() {
        fhir_worker.link = Some(
            worker
                .links
                .iter()
                .map(|link| FhirWorkerLink {
                    other: FhirReference {
                        reference: Some(format!("Worker/{}", link.other_worker_id)),
                        display: None,
                    },
                    type_: format!("{:?}", link.link_type).to_lowercase(),
                })
                .collect(),
        );
    }

    // Managing organization
    if let Some(ref org_id) = worker.managing_organization {
        fhir_worker.managing_organization = Some(FhirReference {
            reference: Some(format!("Organization/{org_id}")),
            display: None,
        });
    }

    fhir_worker
}

/// Maps one FHIR `HumanName` entry onto an internal [`HumanName`]. Shared by
/// [`parse_fhir_name`] (the first entry) and [`parse_fhir_additional_names`]
/// (every entry after it) so the `use` vocabulary is decoded in one place.
fn parse_one_fhir_name(fname: &FhirHumanName) -> HumanName {
    use crate::models::NameUse;

    HumanName {
        use_type: fname.use_.as_ref().and_then(|u| match u.as_str() {
            "usual" => Some(NameUse::Usual),
            "official" => Some(NameUse::Official),
            "temp" => Some(NameUse::Temp),
            "nickname" => Some(NameUse::Nickname),
            "anonymous" => Some(NameUse::Anonymous),
            "old" => Some(NameUse::Old),
            "maiden" => Some(NameUse::Maiden),
            _ => None,
        }),
        family: fname.family.clone().unwrap_or_default(),
        given: fname.given.clone().unwrap_or_default(),
        prefix: fname.prefix.clone().unwrap_or_default(),
        suffix: fname.suffix.clone().unwrap_or_default(),
    }
}

/// Extracts the primary [`HumanName`] from the first FHIR name entry.
///
/// # Errors
///
/// Returns [`crate::Error::Validation`] if the worker has no name entries.
fn parse_fhir_name(fhir_worker: &FhirWorker) -> Result<HumanName> {
    let Some(names) = fhir_worker.name.as_ref() else {
        return Err(crate::Error::Validation(
            "Worker must have at least one name".to_string(),
        ));
    };
    let Some(first_name) = names.first() else {
        return Err(crate::Error::Validation(
            "Worker must have at least one name".to_string(),
        ));
    };

    Ok(parse_one_fhir_name(first_name))
}

/// Extracts every FHIR name entry **after** the first (the primary name
/// [`parse_fhir_name`] already handles) as internal `additional_names` —
/// the direction `to_fhir_names` builds going out. Empty when the worker
/// has zero or one name entry.
fn parse_fhir_additional_names(fhir_worker: &FhirWorker) -> Vec<HumanName> {
    let Some(names) = fhir_worker.name.as_ref() else {
        return vec![];
    };
    names.iter().skip(1).map(parse_one_fhir_name).collect()
}

/// Decodes a FHIR gender code into a [`Gender`], defaulting to `Unknown`.
fn parse_fhir_gender(code: Option<&str>) -> Gender {
    match code {
        Some("male") => Gender::Male,
        Some("female") => Gender::Female,
        Some("other") => Gender::Other,
        _ => Gender::Unknown,
    }
}

/// Decodes the FHIR `deceased[x]` choice element into a `(deceased, datetime)`
/// pair.
fn parse_fhir_deceased(fhir_worker: &FhirWorker) -> (bool, Option<chrono::DateTime<chrono::Utc>>) {
    use crate::api::fhir::resources::FhirDeceased;

    match &fhir_worker.deceased {
        Some(FhirDeceased::Boolean(b)) => (*b, None),
        Some(FhirDeceased::DateTime(dt)) => {
            let parsed_dt = dt.parse::<chrono::DateTime<chrono::Utc>>().ok();
            (true, parsed_dt)
        }
        None => (false, None),
    }
}

/// Recovers the [`IdentifierType`](crate::models::IdentifierType) `to_fhir_identifiers`
/// encoded into `Identifier.type.coding[0].code` (`id.identifier_type.to_string()` —
/// the UPPERCASE wire token, e.g. `"NPI"`).
///
/// This is deliberately **not** read from `Identifier.system`: `system` carries
/// the identifier's own assigning-authority string (round-tripped as-is,
/// below), a value the type vocabulary has never lived in on the way out —
/// reading it here would degrade every identifier to `Other` regardless of
/// what was actually encoded. `IdentifierType`'s own `#[serde(rename_all =
/// "UPPERCASE")]` + `#[serde(other)]` derive is the "existing serde
/// vocabulary": deserializing through it (rather than a second hand-rolled
/// match, as `db/repositories.rs::identifiers_from_db` uses for the same
/// mapping) means a variant added there is picked up here for free, and an
/// unrecognised code still lands on `Other` rather than an error.
fn parse_fhir_identifier_type(fid: &FhirIdentifier) -> crate::models::IdentifierType {
    fid.type_
        .as_ref()
        .and_then(|t| t.coding.as_ref())
        .and_then(|codings| codings.first())
        .and_then(|c| c.code.as_deref())
        .and_then(|code| serde_json::from_value(serde_json::Value::String(code.to_string())).ok())
        .unwrap_or(crate::models::IdentifierType::Other)
}

/// Maps the FHIR identifier list onto internal [`Identifier`]s, skipping any
/// entry missing a system or value.
fn parse_fhir_identifiers(fhir_worker: &FhirWorker) -> Vec<Identifier> {
    let Some(ids) = fhir_worker.identifier.as_ref() else {
        return vec![];
    };
    ids.iter()
        .filter_map(|fid| {
            Some(Identifier::new(
                parse_fhir_identifier_type(fid),
                fid.system.clone()?,
                fid.value.clone()?,
            ))
        })
        .collect()
}

/// Maps the FHIR address list onto internal [`Address`]es.
fn parse_fhir_addresses(fhir_worker: &FhirWorker) -> Vec<Address> {
    let Some(addrs) = fhir_worker.address.as_ref() else {
        return vec![];
    };
    addrs
        .iter()
        .map(|faddr| {
            let lines = faddr.line.clone().unwrap_or_default();
            Address {
                use_type: None,
                line1: lines.first().cloned(),
                line2: lines.get(1).cloned(),
                city: faddr.city.clone(),
                state: faddr.state.clone(),
                postal_code: faddr.postal_code.clone(),
                country: faddr.country.clone(),
            }
        })
        .collect()
}

/// Maps the FHIR telecom list onto internal [`ContactPoint`]s, skipping any
/// entry with an unrecognized system or missing value.
fn parse_fhir_telecom(fhir_worker: &FhirWorker) -> Vec<ContactPoint> {
    use crate::models::{ContactPointSystem, ContactPointUse};

    let Some(tels) = fhir_worker.telecom.as_ref() else {
        return vec![];
    };
    tels.iter()
        .filter_map(|ftel| {
            let system = ftel.system.as_ref().and_then(|s| match s.as_str() {
                "phone" => Some(ContactPointSystem::Phone),
                "fax" => Some(ContactPointSystem::Fax),
                "email" => Some(ContactPointSystem::Email),
                "pager" => Some(ContactPointSystem::Pager),
                "url" => Some(ContactPointSystem::Url),
                "sms" => Some(ContactPointSystem::Sms),
                "other" => Some(ContactPointSystem::Other),
                _ => None,
            })?;

            let value = ftel.value.clone()?;

            Some(ContactPoint {
                system,
                value,
                use_type: ftel.use_.as_ref().and_then(|u| match u.as_str() {
                    "home" => Some(ContactPointUse::Home),
                    "work" => Some(ContactPointUse::Work),
                    "temp" => Some(ContactPointUse::Temp),
                    "old" => Some(ContactPointUse::Old),
                    "mobile" => Some(ContactPointUse::Mobile),
                    _ => None,
                }),
            })
        })
        .collect()
}

/// Recovers `marital_status` from the FHIR `maritalStatus` `CodeableConcept`
/// `to_fhir_worker` builds — preferring the plain-text form (`text`, set to
/// the same string `to_fhir_worker` coded), falling back to the coding's
/// `code` when `text` is absent.
fn parse_fhir_marital_status(fhir_worker: &FhirWorker) -> Option<String> {
    let concept = fhir_worker.marital_status.as_ref()?;
    concept.text.clone().or_else(|| {
        concept
            .coding
            .as_ref()
            .and_then(|codings| codings.first())
            .and_then(|c| c.code.clone())
    })
}

/// Recovers `multiple_birth` from the FHIR `multipleBirth[x]` choice element.
/// A boolean round-trips directly; a birth-**order** integer (e.g. "second
/// twin") has no slot in the internal model beyond the flag itself, so any
/// nonzero order is treated as `true` — the fact of a multiple birth is
/// preserved even though the order is not.
fn parse_fhir_multiple_birth(fhir_worker: &FhirWorker) -> Option<bool> {
    use crate::api::fhir::resources::FhirMultipleBirth;

    match fhir_worker.multiple_birth {
        Some(FhirMultipleBirth::Boolean(b)) => Some(b),
        Some(FhirMultipleBirth::Integer(n)) => Some(n != 0),
        None => None,
    }
}

/// Strips a FHIR literal reference's `{resource_type}/` prefix, so
/// `"Organization/9a2f…"` recovers the bare id `to_fhir_worker` started
/// from. A reference not carrying that prefix (or carrying none) is
/// returned as-is rather than dropped — lenient, since an id is still
/// better than nothing for a field this model stores as an opaque string.
fn strip_reference_prefix<'a>(reference: &'a str, resource_type: &str) -> &'a str {
    reference
        .strip_prefix(resource_type)
        .and_then(|rest| rest.strip_prefix('/'))
        .unwrap_or(reference)
}

/// Recovers `managing_organization` from the FHIR `managingOrganization`
/// reference `to_fhir_worker` builds as `Organization/{org_id}`. The
/// internal field is a [`uuid::Uuid`], not an opaque string, so a
/// reference whose id half is not a valid UUID (a malformed or
/// hand-edited resource) is dropped rather than stored malformed.
fn parse_fhir_managing_organization(fhir_worker: &FhirWorker) -> Option<uuid::Uuid> {
    let reference = fhir_worker
        .managing_organization
        .as_ref()?
        .reference
        .as_deref()?;
    uuid::Uuid::parse_str(strip_reference_prefix(reference, "Organization")).ok()
}

/// Maps a FHIR [`FhirWorker`] back to an internal [`Worker`].
///
/// Parses the id (generating a fresh UUID when absent), takes the first name
/// entry (erroring if there is none), and decodes gender / birth date /
/// deceased / identifiers / addresses / telecom / additional names / marital
/// status / multiple-birth flag / managing organization from their FHIR
/// codes (T-19). `worker_type`, `tax_id`, `documents`, `emergency_contacts`,
/// `photo`, and `links` have no FHIR slot this conversion populates yet and
/// are left at their defaults — `links` in particular is a real,
/// **unmarked** round-trip gap `to_fhir_worker` emits going out (spec §13,
/// a follow-up beyond this task's five `TODO`-marked fields).
///
/// # Errors
///
/// Returns [`crate::Error::Validation`] on an invalid UUID or a missing name.
pub fn from_fhir_worker(fhir_worker: &FhirWorker) -> Result<Worker> {
    use chrono::Utc;
    use uuid::Uuid;

    // Parse ID
    let id = if let Some(ref id_str) = fhir_worker.id {
        Uuid::parse_str(id_str)
            .map_err(|e| crate::Error::Validation(format!("Invalid UUID: {e}")))?
    } else {
        Uuid::new_v4()
    };

    let name = parse_fhir_name(fhir_worker)?;
    let gender = parse_fhir_gender(fhir_worker.gender.as_deref());
    let birth_date = fhir_worker
        .birth_date
        .as_ref()
        .and_then(|d| d.parse::<chrono::NaiveDate>().ok());
    let (deceased, deceased_datetime) = parse_fhir_deceased(fhir_worker);
    let identifiers = parse_fhir_identifiers(fhir_worker);
    let addresses = parse_fhir_addresses(fhir_worker);
    let telecom = parse_fhir_telecom(fhir_worker);
    let additional_names = parse_fhir_additional_names(fhir_worker);
    let marital_status = parse_fhir_marital_status(fhir_worker);
    let multiple_birth = parse_fhir_multiple_birth(fhir_worker);
    let managing_organization = parse_fhir_managing_organization(fhir_worker);

    Ok(Worker {
        id,
        identifiers,
        active: fhir_worker.active.unwrap_or(true),
        name,
        additional_names,
        telecom,
        gender,
        worker_type: None,
        birth_date,
        deceased,
        deceased_datetime,
        addresses,
        tax_id: None,
        documents: vec![],
        emergency_contacts: vec![],
        marital_status,
        multiple_birth,
        photo: vec![],
        managing_organization,
        links: vec![],
        created_at: Utc::now(),
        updated_at: Utc::now(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Gender, HumanName, Worker};

    fn sample_worker() -> Worker {
        let name = HumanName {
            use_type: None,
            family: "Smith".to_string(),
            given: vec!["John".to_string()],
            prefix: vec![],
            suffix: vec![],
        };
        let mut worker = Worker::new(name, Gender::Male);
        worker.birth_date = "1980-01-15".parse::<chrono::NaiveDate>().ok();
        worker
    }

    #[test]
    fn to_fhir_worker_emits_practitioner_resource_type() {
        let fhir = to_fhir_worker(&sample_worker());
        assert_eq!(fhir.resource_type, "Practitioner");
    }

    #[test]
    fn round_trip_preserves_core_fields() {
        let worker = sample_worker();
        let fhir = to_fhir_worker(&worker);
        let back = from_fhir_worker(&fhir).expect("round-trip should succeed");
        assert_eq!(back.name.family, worker.name.family);
        assert_eq!(back.name.given, worker.name.given);
        assert_eq!(back.gender, worker.gender);
        assert_eq!(back.birth_date, worker.birth_date);
        assert_eq!(back.id, worker.id);
    }

    #[test]
    fn from_fhir_worker_rejects_missing_name() {
        let fhir = FhirWorker::new(); // no name set
        let err = from_fhir_worker(&fhir);
        assert!(err.is_err(), "a resource with no name must be rejected");
    }

    /// T-19: the five fields `from_fhir_worker` used to silently drop
    /// (identifier type beyond `Other`, additional names, marital status,
    /// multiple-birth flag, managing organization) all survive a
    /// `to_fhir_worker` → `from_fhir_worker` round trip instead of
    /// degrading to `Other`/`None`/`vec![]`.
    #[test]
    fn round_trip_preserves_previously_dropped_fields() {
        use crate::models::IdentifierType;

        let mut worker = sample_worker();
        worker.identifiers = vec![
            Identifier::new(
                IdentifierType::NPI,
                "http://hl7.org/fhir/sid/us-npi".to_string(),
                "1234567893".to_string(),
            ),
            Identifier::new(
                IdentifierType::TAX,
                "http://example.org/tax-id".to_string(),
                "99-1234567".to_string(),
            ),
        ];
        worker.additional_names = vec![HumanName {
            use_type: None,
            family: "Doe".to_string(),
            given: vec!["Jonathan".to_string()],
            prefix: vec![],
            suffix: vec![],
        }];
        worker.marital_status = Some("Married".to_string());
        worker.multiple_birth = Some(true);
        worker.managing_organization = Some(uuid::Uuid::new_v4());

        let fhir = to_fhir_worker(&worker);
        let back = from_fhir_worker(&fhir).expect("round-trip should succeed");

        let back_types: Vec<_> = back
            .identifiers
            .iter()
            .map(|i| i.identifier_type.clone())
            .collect();
        assert_eq!(
            back_types,
            vec![IdentifierType::NPI, IdentifierType::TAX],
            "identifier types must survive, not degrade to Other: {back_types:?}"
        );
        assert_eq!(back.additional_names.len(), 1);
        assert_eq!(back.additional_names[0].family, "Doe");
        assert_eq!(back.additional_names[0].given, vec!["Jonathan".to_string()]);
        assert_eq!(back.marital_status, worker.marital_status);
        assert_eq!(back.multiple_birth, worker.multiple_birth);
        assert_eq!(back.managing_organization, worker.managing_organization);
    }

    /// An identifier type FHIR never declared this crate's vocabulary for
    /// still degrades to `Other` (the fail-safe, not a regression) rather
    /// than an error or a panic.
    #[test]
    fn unrecognised_fhir_identifier_type_code_is_other() {
        let fid = resources::FhirIdentifier {
            use_: None,
            type_: Some(FhirCodeableConcept {
                coding: Some(vec![FhirCoding {
                    system: None,
                    code: Some("NOT-A-REAL-CODE".to_string()),
                    display: None,
                }]),
                text: None,
            }),
            system: Some("urn:example".to_string()),
            value: Some("x".to_string()),
            assigner: None,
        };
        assert_eq!(
            parse_fhir_identifier_type(&fid),
            crate::models::IdentifierType::Other
        );
    }
}
