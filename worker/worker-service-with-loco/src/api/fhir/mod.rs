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

/// Extracts the primary [`HumanName`] from the first FHIR name entry.
///
/// # Errors
///
/// Returns [`crate::Error::Validation`] if the worker has no name entries.
fn parse_fhir_name(fhir_worker: &FhirWorker) -> Result<HumanName> {
    use crate::models::NameUse;

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

    Ok(HumanName {
        use_type: first_name.use_.as_ref().and_then(|u| match u.as_str() {
            "usual" => Some(NameUse::Usual),
            "official" => Some(NameUse::Official),
            "temp" => Some(NameUse::Temp),
            "nickname" => Some(NameUse::Nickname),
            "anonymous" => Some(NameUse::Anonymous),
            "old" => Some(NameUse::Old),
            "maiden" => Some(NameUse::Maiden),
            _ => None,
        }),
        family: first_name.family.clone().unwrap_or_default(),
        given: first_name.given.clone().unwrap_or_default(),
        prefix: first_name.prefix.clone().unwrap_or_default(),
        suffix: first_name.suffix.clone().unwrap_or_default(),
    })
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

/// Maps the FHIR identifier list onto internal [`Identifier`]s, skipping any
/// entry missing a system or value.
fn parse_fhir_identifiers(fhir_worker: &FhirWorker) -> Vec<Identifier> {
    let Some(ids) = fhir_worker.identifier.as_ref() else {
        return vec![];
    };
    ids.iter()
        .filter_map(|fid| {
            Some(Identifier::new(
                crate::models::IdentifierType::Other, // TODO: Parse from coding
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

/// Maps a FHIR [`FhirWorker`] back to an internal [`Worker`].
///
/// Parses the id (generating a fresh UUID when absent), takes the first name
/// entry (erroring if there is none), and decodes gender / birth date /
/// deceased / identifiers / addresses / telecom from their FHIR codes.
/// Several fields are not yet round-tripped (see the `TODO` markers) and are
/// left at their defaults.
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

    Ok(Worker {
        id,
        identifiers,
        active: fhir_worker.active.unwrap_or(true),
        name,
        additional_names: vec![], // TODO: Parse additional names from FHIR
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
        marital_status: None, // TODO: Parse marital status
        multiple_birth: None, // TODO: Parse multiple birth
        photo: vec![],
        managing_organization: None, // TODO: Parse organization reference
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
}
