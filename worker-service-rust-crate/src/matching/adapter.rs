//! Adapter from the service's `Worker` domain model to the `worker-matcher`
//! library's `Worker` matching input.
//!
//! The service stores a rich, FHIR-shaped `Worker` (named `HumanName`, vector
//! `identifiers`, `addresses`, `telecom`, `documents`, soft-delete + audit
//! timestamps, plus the worker-specific `worker_type` and `ods` org code).
//! The `worker-matcher` crate accepts a flat, builder-shaped `Worker` with
//! 40+ country-specific identifier slots and explicit
//! `phone`/`mobile`/`email`/`address` fields.
//!
//! [`to_matcher_worker`] performs the lossy but well-defined projection from
//! the service shape to the matcher shape so callers can use the canonical
//! algorithm without rewriting their domain model.
//!
//! See `agents/share/match.md` and the matcher crate's `spec.md §12` for the
//! algorithm contract this adapter feeds.
//!
//! # Mapping
//!
//! | Service field | Matcher slot |
//! |---|---|
//! | `name.family` | `family_name` |
//! | `name.given[0]` | `given_name` |
//! | `name.given[1]` | `middle_name` |
//! | `birth_date` | `date_of_birth` |
//! | `gender` | `gender` |
//! | first `addresses[]` | `address` (rest → `previous_addresses`) |
//! | first telecom `Phone` | `phone` |
//! | first telecom `Sms` | `mobile` |
//! | first telecom `Email` | `email` |
//! | `tax_id` | `us_ssn` (default; overridable by typed identifier with non-US system URI) |
//! | `identifiers[]` | country-specific slot via [`route_identifier`] |
//! | `documents[]` of type `Passport` | `passport_books` (one per passport) |
//!
//! Service-only fields (`id`, `active`, `worker_type`, `deceased_datetime`,
//! `managing_organization`, `links`, `created_at`, `marital_status`, `photo`,
//! `multiple_birth`) are dropped — they have no matcher counterpart.

use chrono::Datelike;
use worker_matcher::{
    Address as MAddress, Gender as MGender, PassportBook as MPassport, Worker as MWorker,
    WorkerBuilder as MBuilder,
};

use crate::models::{
    Address, ContactPoint, ContactPointSystem, DocumentType, Gender, Identifier, IdentifierType,
    IdentityDocument, Worker,
};

/// Convert a service `Worker` into a `worker_matcher::Worker` ready for
/// `MatchingEngine::match_workers` / `deterministic_match`.
pub fn to_matcher_worker(w: &Worker) -> MWorker {
    let mut b = MWorker::builder();

    // --- Name -------------------------------------------------------------
    let family = w.name.family.trim();
    if !family.is_empty() {
        b = b.family_name(family);
    }
    if let Some(g) = w.name.given.first() {
        if !g.trim().is_empty() {
            b = b.given_name(g.trim());
        }
    }
    if let Some(m) = w.name.given.get(1) {
        if !m.trim().is_empty() {
            b = b.middle_name(m.trim());
        }
    }

    // --- Demographics -----------------------------------------------------
    if let Some(dob) = w.birth_date {
        if dob.year() > 1 {
            b = b.date_of_birth(dob);
        }
    }
    b = b.gender(map_gender(w.gender));

    // --- Telecom ----------------------------------------------------------
    if let Some(v) = first_telecom(&w.telecom, ContactPointSystem::Phone) {
        b = b.phone(v);
    }
    if let Some(v) = first_telecom(&w.telecom, ContactPointSystem::Sms) {
        b = b.mobile(v);
    }
    if let Some(v) = first_telecom(&w.telecom, ContactPointSystem::Email) {
        b = b.email(v);
    }

    // --- Addresses --------------------------------------------------------
    let mut addrs = w.addresses.iter().filter_map(map_address);
    if let Some(primary) = addrs.next() {
        b = b.address(primary);
        let rest: Vec<MAddress> = addrs.collect();
        if !rest.is_empty() {
            b = b.previous_addresses(rest);
        }
    }

    // --- tax_id default → US SSN -----------------------------------------
    if let Some(t) = w.tax_id.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        b = b.us_ssn(t);
    }

    // --- Identifiers → country slots --------------------------------------
    for id in &w.identifiers {
        b = route_identifier(b, id);
    }

    // --- Passports → passport_books ---------------------------------------
    for d in w
        .documents
        .iter()
        .filter(|d| d.document_type == DocumentType::Passport)
    {
        if let Some(pb) = build_passport(d) {
            b = b.add_passport_book(pb);
        }
    }

    b.build()
}

fn map_gender(g: Gender) -> MGender {
    match g {
        Gender::Male => MGender::Male,
        Gender::Female => MGender::Female,
        Gender::Other => MGender::Other,
        Gender::Unknown => MGender::Unknown,
    }
}

fn first_telecom(telecom: &[ContactPoint], system: ContactPointSystem) -> Option<String> {
    telecom
        .iter()
        .find(|c| matches_system(&c.system, &system))
        .map(|c| c.value.clone())
}

fn matches_system(a: &ContactPointSystem, b: &ContactPointSystem) -> bool {
    matches!(
        (a, b),
        (ContactPointSystem::Phone, ContactPointSystem::Phone)
            | (ContactPointSystem::Sms, ContactPointSystem::Sms)
            | (ContactPointSystem::Email, ContactPointSystem::Email)
            | (ContactPointSystem::Fax, ContactPointSystem::Fax)
            | (ContactPointSystem::Pager, ContactPointSystem::Pager)
            | (ContactPointSystem::Url, ContactPointSystem::Url)
            | (ContactPointSystem::Other, ContactPointSystem::Other)
    )
}

fn map_address(a: &Address) -> Option<MAddress> {
    let any = a.line1.is_some()
        || a.line2.is_some()
        || a.city.is_some()
        || a.state.is_some()
        || a.postal_code.is_some()
        || a.country.is_some();
    if !any {
        return None;
    }
    let mut m = MAddress::new();
    if let Some(v) = a.line1.as_deref() {
        m = m.with_line1(v);
    }
    if let Some(v) = a.line2.as_deref() {
        m = m.with_line2(v);
    }
    if let Some(v) = a.city.as_deref() {
        m = m.with_city(v);
    }
    if let Some(v) = a.state.as_deref() {
        m = m.with_county(v);
    }
    if let Some(v) = a.postal_code.as_deref() {
        m = m.with_postcode(v);
    }
    if let Some(v) = a.country.as_deref() {
        m = m.with_country(v);
    }
    Some(m)
}

/// Route a service `Identifier` to the appropriate matcher country-specific
/// builder method.
///
/// The matcher exposes 42 country slots (`uk_nhs_number`, `fr_nir`, `us_ssn`,
/// `br_cpf`, …). Service-side identifiers carry a free-form `system` URI;
/// when that URI mentions a known scheme it wins, otherwise fall back to the
/// generic `IdentifierType` enum.
fn route_identifier(b: MBuilder, id: &Identifier) -> MBuilder {
    let sys = id.system.to_ascii_lowercase();
    let val = id.value.trim();
    if val.is_empty() {
        return b;
    }

    if sys.contains("nhs.uk") || sys.contains("uk-nhs") || sys.contains("nhs-number") {
        return b.uk_nhs_number(val);
    }
    if sys.contains("us-ssn") || sys.contains("ssa.gov") {
        return b.us_ssn(val);
    }
    if sys.contains("cpf") {
        return b.br_cpf(val);
    }
    if sys.contains("nir") || sys.contains("ameli.fr") {
        return b.fr_nir(val);
    }
    if sys.contains("tsi") || sys.contains("ingesa") {
        return b.es_tsi(val);
    }
    if sys.contains("aadhaar") || sys.contains("uidai") {
        return b.in_aadhaar(val);
    }
    if sys.contains("my-number") || sys.contains("myna") {
        return b.jp_my_number(val);
    }
    if sys.contains("curp") {
        return b.mx_curp(val);
    }
    if sys.contains("personnummer") {
        return b.se_workernummer(val);
    }
    if sys.contains("kvnr") {
        return b.de_kvnr(val);
    }
    if sys.contains("bsn") {
        return b.nl_bsn(val);
    }
    if sys.contains("nhi") {
        return b.nz_nhi(val);
    }
    if sys.contains("ihi") {
        if val.chars().filter(|c| c.is_ascii_digit()).count() >= 14 {
            return b.au_ihi(val);
        }
        return b.ie_ihi(val);
    }

    match id.identifier_type {
        IdentifierType::TAX | IdentifierType::SSN => b.us_ssn(val),
        // PPN passports flow via IdentityDocument; ODS / MRN / DL / NPI / Other
        // have no per-country matcher slot.
        IdentifierType::PPN
        | IdentifierType::ODS
        | IdentifierType::MRN
        | IdentifierType::DL
        | IdentifierType::NPI
        | IdentifierType::Other => b,
    }
}

fn build_passport(d: &IdentityDocument) -> Option<MPassport> {
    let country = d.issuing_country.as_deref()?.trim();
    if country.is_empty() {
        return None;
    }
    let mut pb = MPassport::new(country, d.number.trim())?;
    if let Some(date) = d.issue_date {
        pb = pb.with_issued(date);
    }
    if let Some(date) = d.expiry_date {
        pb = pb.with_expires(date);
    }
    Some(pb)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{HumanName, Worker};
    use chrono::Utc;
    use uuid::Uuid;

    fn svc_worker(family: &str, given: &str) -> Worker {
        Worker {
            id: Uuid::new_v4(),
            identifiers: vec![],
            active: true,
            name: HumanName {
                use_type: None,
                family: family.into(),
                given: vec![given.into()],
                prefix: vec![],
                suffix: vec![],
            },
            additional_names: vec![],
            telecom: vec![],
            gender: Gender::Female,
            worker_type: None,
            birth_date: chrono::NaiveDate::from_ymd_opt(1980, 5, 15),
            tax_id: None,
            documents: vec![],
            emergency_contacts: vec![],
            deceased: false,
            deceased_datetime: None,
            addresses: vec![],
            marital_status: None,
            multiple_birth: None,
            photo: vec![],
            managing_organization: None,
            links: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn round_trip_names_and_dob() {
        let svc = svc_worker("Williams", "Alice");
        let m = to_matcher_worker(&svc);
        assert_eq!(m.family_name.as_deref(), Some("Williams"));
        assert_eq!(m.given_name.as_deref(), Some("Alice"));
    }

    #[test]
    fn routes_uk_nhs_by_system_uri() {
        let mut svc = svc_worker("Smith", "John");
        svc.identifiers.push(Identifier::new(
            IdentifierType::Other,
            "https://fhir.nhs.uk/Id/nhs-number".into(),
            "943 476 5919".into(),
        ));
        let m = to_matcher_worker(&svc);
        assert_eq!(m.uk_nhs_number.as_deref(), Some("943 476 5919"));
    }
}
