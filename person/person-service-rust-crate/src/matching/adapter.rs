//! Adapter from the service's `Person` domain model to the `person-matcher`
//! library's `Person` matching input.
//!
//! The service stores a rich, FHIR-shaped `Person` (named `HumanName`, vector
//! `identifiers`, `addresses`, `telecom`, `documents`, soft-delete + audit
//! timestamps). The `person-matcher` crate accepts a flat, builder-shaped
//! `Person` with country-specific identifier slots and explicit
//! `phone`/`mobile`/`email`/`address` fields.
//!
//! [`to_matcher_person`](crate::matching::adapter::to_matcher_person) performs the lossy but well-defined projection from
//! the service shape to the matcher shape so callers can use the canonical
//! algorithm without rewriting their domain model.
//!
//! See `agents/share/match.md` and the matcher crate's `spec.md §12` for the
//! algorithm contract this adapter feeds.
//!
//! # Example
//!
//! ```ignore
//! use person_service::matching::adapter::to_matcher_person;
//! use person_matcher::{MatchingEngine, MatchConfig};
//!
//! let engine = MatchingEngine::new(MatchConfig::default());
//! let result = engine.match_persons(
//!     &to_matcher_person(&svc_a),
//!     &to_matcher_person(&svc_b),
//! );
//! ```
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
//! | first `addresses[]` | `address` (rest become `previous_addresses`) |
//! | first telecom `Phone` | `phone` |
//! | first telecom `Sms` (mobile) | `mobile` |
//! | first telecom `Email` | `email` |
//! | `tax_id` | `us_ssn` (default; overridden if a TAX identifier carries a non-US system URI) |
//! | `identifiers[]` with `IdentifierType` + `system` URI | country-specific slot via `route_identifier` |
//! | `documents[]` of type `Passport` | `passport_books` (one per passport) |

use person_matcher::{
    Address as MAddress, Gender as MGender, PassportBook as MPassport, Person as MPerson,
    PersonBuilder as MBuilder,
};

use crate::models::{
    Address, ContactPoint, ContactPointSystem, DocumentType, Gender, Identifier, IdentifierType,
    IdentityDocument, Person,
};

/// Convert a service `Person` into a `person_matcher::Person` ready for
/// `MatchingEngine::match_persons` / `deterministic_match`.
///
/// This is a *projection* — fields the matcher does not consume (UUID,
/// `active`, `deceased_datetime`, `managing_organization`, `links`,
/// `created_at`, …) are dropped. Fields the matcher consumes but the service
/// stores in a collection are sampled (first phone, first email, first
/// address with the rest going to `previous_addresses`).
pub fn to_matcher_person(p: &Person) -> MPerson {
    let mut b = MPerson::builder();

    // --- Name -------------------------------------------------------------
    let family = p.name.family.trim();
    if !family.is_empty() {
        b = b.family_name(family);
    }
    if let Some(g) = p.name.given.first() {
        if !g.trim().is_empty() {
            b = b.given_name(g.trim());
        }
    }
    if let Some(m) = p.name.given.get(1) {
        if !m.trim().is_empty() {
            b = b.middle_name(m.trim());
        }
    }

    // --- Demographics -----------------------------------------------------
    if let Some(dob) = p.birth_date {
        // Guard against placeholder dates (e.g. year 1).
        if dob.year() > 1 {
            b = b.date_of_birth(dob);
        }
    }
    b = b.gender(map_gender(p.gender));

    // --- Telecom (first Phone/Sms/Email) ---------------------------------
    if let Some(v) = first_telecom(&p.telecom, ContactPointSystem::Phone) {
        b = b.phone(v);
    }
    if let Some(v) = first_telecom(&p.telecom, ContactPointSystem::Sms) {
        b = b.mobile(v);
    }
    if let Some(v) = first_telecom(&p.telecom, ContactPointSystem::Email) {
        b = b.email(v);
    }

    // --- Addresses (first → primary; rest → previous) ---------------------
    let mut addrs = p.addresses.iter().filter_map(map_address);
    if let Some(primary) = addrs.next() {
        b = b.address(primary);
        let rest: Vec<MAddress> = addrs.collect();
        if !rest.is_empty() {
            b = b.previous_addresses(rest);
        }
    }

    // --- tax_id (default-routed to US SSN unless overridden below) --------
    if let Some(t) = p.tax_id.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        b = b.us_ssn(t);
    }

    // --- Identifiers → country-specific slots -----------------------------
    for id in &p.identifiers {
        b = route_identifier(b, id);
    }

    // --- Passports → passport_books ---------------------------------------
    for d in p
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
        m = m.with_county(v); // matcher uses "county"; service uses "state"
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
/// Routing key is `(IdentifierType, system)` — the URI takes precedence so a
/// `SSN` carrying a Brazilian CPF system URI maps to `br_cpf`, not `us_ssn`.
/// Unknown system URIs for typed identifiers fall back to the type's default
/// country (SSN→US, PPN→passport_book, etc.).
fn route_identifier(b: MBuilder, id: &Identifier) -> MBuilder {
    let sys = id.system.to_ascii_lowercase();
    let val = id.value.trim();
    if val.is_empty() {
        return b;
    }

    // System-URI fast paths (most specific first).
    if sys.contains("nhs.uk") || sys.contains("uk-nhs") || sys.contains("nhs-number") {
        return b.united_kingdom_national_health_service_number(val);
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
        return b.se_personnummer(val);
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
        // AU and IE both call theirs IHI; AU is 16 digits, IE is 7.
        if val.chars().filter(|c| c.is_ascii_digit()).count() >= 14 {
            return b.au_ihi(val);
        }
        return b.ie_ihi(val);
    }

    // Type-based defaults.
    match id.identifier_type {
        IdentifierType::TAX => b.us_ssn(val),
        IdentifierType::SSN => b.us_ssn(val),
        IdentifierType::PPN => b, // passports are handled via IdentityDocument
        IdentifierType::MRN | IdentifierType::DL | IdentifierType::NPI | IdentifierType::Other => b,
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
    use crate::models::{HumanName, Identifier, IdentifierType, Person};
    use jiff::Timestamp;
    use uuid::Uuid;

    fn svc_person(family: &str, given: &str) -> Person {
        Person {
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
            birth_date: Some(jiff::civil::date(1980, 5, 15)),
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
            created_at: Timestamp::now(),
            updated_at: Timestamp::now(),
        }
    }

    #[test]
    fn round_trip_names_and_dob() {
        let svc = svc_person("Williams", "Alice");
        let m = to_matcher_person(&svc);
        assert_eq!(m.family_name.as_deref(), Some("Williams"));
        assert_eq!(m.given_name.as_deref(), Some("Alice"));
        assert_eq!(m.date_of_birth, Some(jiff::civil::date(1980, 5, 15)));
    }

    #[test]
    fn routes_uk_nhs_by_system_uri() {
        let mut svc = svc_person("Smith", "John");
        svc.identifiers.push(Identifier::new(
            IdentifierType::Other,
            "https://fhir.nhs.uk/Id/nhs-number".into(),
            "943 476 5919".into(),
        ));
        let m = to_matcher_person(&svc);
        assert_eq!(
            m.united_kingdom_national_health_service_number.as_deref(),
            Some("943 476 5919")
        );
    }

    #[test]
    fn tax_id_defaults_to_us_ssn() {
        let mut svc = svc_person("Smith", "John");
        svc.tax_id = Some("123-45-6789".into());
        let m = to_matcher_person(&svc);
        assert_eq!(m.us_ssn.as_deref(), Some("123-45-6789"));
    }

    #[test]
    fn passport_document_maps_to_passport_book() {
        let mut svc = svc_person("Smith", "John");
        svc.documents.push(IdentityDocument {
            document_type: DocumentType::Passport,
            number: "X12345678".into(),
            issuing_country: Some("US".into()),
            issuing_authority: None,
            issue_date: None,
            expiry_date: None,
            verified: false,
        });
        let m = to_matcher_person(&svc);
        assert_eq!(m.passport_books.len(), 1);
        assert_eq!(m.passport_books[0].country, "US");
        assert_eq!(m.passport_books[0].number, "X12345678");
    }
}
