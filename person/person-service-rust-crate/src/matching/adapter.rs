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
//!
//! # Identifier scheme-routing audit (spec §5.3.1 / E-8)
//!
//! The matcher exposes **26** national-ID builder slots. [`route_identifier`]
//! reaches **all 26** via `system`-URI substring fast paths (plus the
//! type-based `tax_id` / `SSN` / `TAX` → `us_ssn` defaults). No slot is
//! unreachable from service data.
//!
//! ## Routable (`system` URI substring → matcher slot)
//!
//! | URI contains | Matcher slot |
//! |---|---|
//! | `nhs.uk` / `uk-nhs` / `nhs-number` | `united_kingdom_national_health_service_number` |
//! | `us-ssn` / `ssa.gov` (+ type SSN/TAX, + `tax_id`) | `us_ssn` |
//! | `cpf` | `br_cpf` |
//! | `nir` / `ameli.fr` | `fr_nir` |
//! | `tsi` / `ingesa` | `es_tsi` |
//! | `aadhaar` / `uidai` | `in_aadhaar` |
//! | `my-number` / `myna` | `jp_my_number` |
//! | `curp` | `mx_curp` |
//! | `personnummer` | `se_personnummer` |
//! | `kvnr` | `de_kvnr` |
//! | `bsn` | `nl_bsn` |
//! | `nhi` | `nz_nhi` |
//! | `ihi` (≥14 digits) | `au_ihi` |
//! | `ihi` (<14 digits) | `ie_ihi` |
//! | `hc-number` / `health-and-care` | `uk_hc_number` |
//! | `chi-number` / `:chi` / `/chi` | `uk_chi_number` |
//! | `nino` / `national-insurance` | `uk_nino` |
//! | `codice` / `it-cf` / `:cf` | `it_cf` |
//! | `egn` | `bg_egn` |
//! | `dni` | `es_dni` |
//! | `oib` | `hr_oib` |
//! | `fnr` / `fodselsnummer` | `no_fnr` |
//! | `pesel` | `pl_pesel` |
//! | `cnp` | `ro_cnp` |
//! | `emso` | `si_emso` |
//! | `rrn` | `cn_rrn` |
//!
//! Routing order is **most-specific-first**: the `nhs-number` /`ihi` /`nir`
//! fast paths run before the shorter fragments below them, so e.g. an
//! `nhs-number` URI never falls through to the bare `chi`/`nir` checks, and
//! `cpf` is matched before the bare `:cf` codice-fiscale fragment.
//!
//! Every routable scheme is pinned by `tests/duplicate_detection.rs`
//! (`routable_identifier_systems_reach_their_matcher_slot`,
//! `all_national_id_schemes_route_to_their_slot`,
//! `ihi_disambiguates_au_vs_ie_by_digit_count`). Adding a scheme is a
//! three-part change: a fast path here, a table row here + in spec §5.3.1,
//! and a test case.

use chrono::Datelike;
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
    // Service `HumanName` is structured (family + ordered given vector);
    // the matcher wants flat `family_name` / `given_name` / `middle_name`
    // scalars. Every value is trimmed and empties are skipped so a blank
    // service field never sets a (would-be-empty) matcher slot.
    //
    // `name.family`   → matcher `family_name`
    let family = p.name.family.trim();
    if !family.is_empty() {
        b = b.family_name(family);
    }
    // `name.given[0]` → matcher `given_name` (the first/primary forename)
    if let Some(g) = p.name.given.first() {
        if !g.trim().is_empty() {
            b = b.given_name(g.trim());
        }
    }
    // `name.given[1]` → matcher `middle_name` (the second forename, if any).
    // Any further given names (`given[2..]`) have no matcher slot and are
    // intentionally dropped.
    if let Some(m) = p.name.given.get(1) {
        if !m.trim().is_empty() {
            b = b.middle_name(m.trim());
        }
    }

    // --- Demographics -----------------------------------------------------
    // `birth_date` → matcher `date_of_birth` (same `chrono::NaiveDate`).
    if let Some(dob) = p.birth_date {
        // Guard against placeholder dates (e.g. year 1) that some source
        // systems use as a "no DOB" sentinel — feeding those to the matcher
        // would create spurious DOB agreement between unrelated records.
        if dob.year() > 1 {
            b = b.date_of_birth(dob);
        }
    }
    // `gender` → matcher `gender` (variant-for-variant via `map_gender`).
    b = b.gender(map_gender(p.gender));

    // --- Telecom (first Phone/Sms/Email) ---------------------------------
    // The service holds a heterogeneous `Vec<ContactPoint>`; the matcher
    // wants three distinct scalar slots. Split the vector by channel and
    // sample the FIRST entry of each into its slot:
    //   first telecom with system Phone → matcher `phone`
    if let Some(v) = first_telecom(&p.telecom, ContactPointSystem::Phone) {
        b = b.phone(v);
    }
    //   first telecom with system Sms   → matcher `mobile` (SMS-capable ≈ cell)
    if let Some(v) = first_telecom(&p.telecom, ContactPointSystem::Sms) {
        b = b.mobile(v);
    }
    //   first telecom with system Email → matcher `email`
    // Fax / Pager / Url / Other channels have no matcher slot and are dropped.
    if let Some(v) = first_telecom(&p.telecom, ContactPointSystem::Email) {
        b = b.email(v);
    }

    // --- Addresses (first → primary; rest → previous) ---------------------
    // `map_address` drops wholly-empty addresses; the first surviving one
    // becomes the matcher's primary `address`, and every remaining one is
    // collected into `previous_addresses` so historical addresses still
    // contribute to the address-overlap signal.
    let mut addrs = p.addresses.iter().filter_map(map_address);
    if let Some(primary) = addrs.next() {
        b = b.address(primary);
        let rest: Vec<MAddress> = addrs.collect();
        if !rest.is_empty() {
            b = b.previous_addresses(rest);
        }
    }

    // --- tax_id (default-routed to US SSN unless overridden below) --------
    // The service stores a single free-form `tax_id`; the matcher has no
    // generic tax slot, so it is parked in `us_ssn` by default. A typed
    // TAX/SSN identifier carrying a non-US scheme URI (handled in the
    // identifier loop below) routes to the correct national slot instead.
    if let Some(t) = p.tax_id.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        b = b.us_ssn(t);
    }

    // --- Identifiers → country-specific slots -----------------------------
    // Each `Identifier` is routed by `route_identifier`, which inspects the
    // scheme URI (then the `IdentifierType`) to pick one of the matcher's
    // national-ID builder methods. A later identifier may overwrite the
    // `us_ssn` slot set from `tax_id` above.
    for id in &p.identifiers {
        b = route_identifier(b, id);
    }

    // --- Passports → passport_books ---------------------------------------
    // Only `DocumentType::Passport` documents map across; each becomes one
    // `PassportBook` (country + number + optional dates). Non-passport
    // identity documents (driver's license, national ID, …) have no matcher
    // representation and are skipped.
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

/// Map the service [`Gender`] to the matcher's `Gender`, variant for
/// variant.
///
/// The two enums share the same four-variant shape (`Male` / `Female` /
/// `Other` / `Unknown`); this is a total, lossless mapping. `Unknown`
/// is preserved (not dropped) so the matcher's gender component can treat
/// it as a partial rather than a conflicting match.
fn map_gender(g: Gender) -> MGender {
    match g {
        Gender::Male => MGender::Male,
        Gender::Female => MGender::Female,
        Gender::Other => MGender::Other,
        Gender::Unknown => MGender::Unknown,
    }
}

/// Return the value of the first [`ContactPoint`] in `telecom` whose
/// channel matches `system`, cloned.
///
/// Used to sample one phone / SMS / email out of the heterogeneous
/// telecom vector for the matcher's scalar slots. Returns `None` when no
/// contact point uses the requested channel.
fn first_telecom(telecom: &[ContactPoint], system: ContactPointSystem) -> Option<String> {
    telecom
        .iter()
        .find(|c| matches_system(&c.system, &system))
        .map(|c| c.value.clone())
}

/// Test whether two [`ContactPointSystem`] values name the same channel.
///
/// [`ContactPointSystem`] is not `PartialEq`, so this enumerates the
/// like-for-like pairs explicitly. Any cross-channel pair (and any
/// channel not listed) returns `false`.
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

/// Project a service [`Address`] onto the matcher's `Address`.
///
/// Returns `None` for an address whose every component is absent (so the
/// caller never emits an all-empty matcher address that would falsely
/// "agree" with another empty one). Field renames are the load-bearing
/// part of the mapping:
///
/// | Service field | Matcher field |
/// |---|---|
/// | `line1` | `line1` |
/// | `line2` | `line2` |
/// | `city` | `city` |
/// | `state` | `county` (rename — matcher uses British "county") |
/// | `postal_code` | `postcode` (rename) |
/// | `country` | `country` |
fn map_address(a: &Address) -> Option<MAddress> {
    // Skip wholly-empty addresses: an `Address` with no components present
    // carries no matching signal and must not occupy a matcher slot.
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
        // Field rename: the service's `state` (US-centric region) fills the
        // matcher's `county` (UK-centric region) slot — same role, different name.
        m = m.with_county(v);
    }
    if let Some(v) = a.postal_code.as_deref() {
        // Field rename: `postal_code` → `postcode`.
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

    // UK NI Health & Care number is distinct from the NHS number above; key
    // on an explicit `hc-number` / `health-and-care` fragment so it does not
    // collide with the `nhs-number` fast path.
    if sys.contains("hc-number") || sys.contains("health-and-care") {
        return b.uk_hc_number(val);
    }
    // Scotland CHI. `chi` is specific enough; the `nhi` fast path above does
    // not contain it.
    if sys.contains("chi-number") || sys.contains(":chi") || sys.contains("/chi") {
        return b.uk_chi_number(val);
    }
    if sys.contains("nino") || sys.contains("national-insurance") {
        return b.uk_nino(val);
    }
    // IT codice fiscale — avoid a bare `cf` substring (too collision-prone);
    // require an explicit fragment.
    if sys.contains("codice") || sys.contains("it-cf") || sys.contains(":cf") {
        return b.it_cf(val);
    }
    if sys.contains("egn") {
        return b.bg_egn(val);
    }
    if sys.contains("dni") {
        return b.es_dni(val);
    }
    if sys.contains("oib") {
        return b.hr_oib(val);
    }
    if sys.contains("fnr") || sys.contains("fodselsnummer") {
        return b.no_fnr(val);
    }
    if sys.contains("pesel") {
        return b.pl_pesel(val);
    }
    if sys.contains("cnp") {
        return b.ro_cnp(val);
    }
    if sys.contains("emso") {
        return b.si_emso(val);
    }
    if sys.contains("rrn") {
        return b.cn_rrn(val);
    }

    // Type-based defaults.
    match id.identifier_type {
        IdentifierType::TAX => b.us_ssn(val),
        IdentifierType::SSN => b.us_ssn(val),
        IdentifierType::PPN => b, // passports are handled via IdentityDocument
        IdentifierType::MRN | IdentifierType::DL | IdentifierType::NPI | IdentifierType::Other => b,
    }
}

/// Build a matcher `PassportBook` from a service [`IdentityDocument`].
///
/// Maps `issuing_country` → passport country and `number` → passport
/// number, then layers on the optional `issue_date` / `expiry_date`.
/// Returns `None` when the issuing country is missing/blank, or when
/// `MPassport::new` rejects the `(country, number)` pair (e.g. an empty
/// number) — a passport without a country is not a usable matching key.
///
/// The caller (`to_matcher_person`) only passes `DocumentType::Passport`
/// documents here, so the document-type gate lives at the call site.
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

/// Unit tests for the service→matcher projection.
///
/// These pin the adapter's local field-routing rules in isolation; the
/// end-to-end contract (routing + matcher scoring) is pinned separately
/// by `tests/duplicate_detection.rs`.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{HumanName, Identifier, IdentifierType, Person};
    use chrono::{NaiveDate, Utc};
    use uuid::Uuid;

    /// Build a minimal female `Person` with the given family/given name and
    /// a fixed 1980-05-15 DOB, every other field at its empty/default value.
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
            birth_date: Some(NaiveDate::from_ymd_opt(1980, 5, 15).unwrap()),
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

    /// Family, given, and DOB survive the projection into their matcher
    /// scalar slots unchanged.
    #[test]
    fn round_trip_names_and_dob() {
        let svc = svc_person("Williams", "Alice");
        let m = to_matcher_person(&svc);
        assert_eq!(m.family_name.as_deref(), Some("Williams"));
        assert_eq!(m.given_name.as_deref(), Some("Alice"));
        assert_eq!(
            m.date_of_birth,
            Some(NaiveDate::from_ymd_opt(1980, 5, 15).unwrap())
        );
    }

    /// An identifier carrying an NHS-number FHIR scheme URI routes to the
    /// UK NHS-number matcher slot even when its `IdentifierType` is `Other`
    /// — i.e. the scheme URI, not the type, drives the routing.
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

    /// The free-form `tax_id` field, with no overriding typed identifier,
    /// lands in the matcher's `us_ssn` slot (the default tax routing).
    #[test]
    fn tax_id_defaults_to_us_ssn() {
        let mut svc = svc_person("Smith", "John");
        svc.tax_id = Some("123-45-6789".into());
        let m = to_matcher_person(&svc);
        assert_eq!(m.us_ssn.as_deref(), Some("123-45-6789"));
    }

    /// A `Passport` identity document becomes exactly one matcher
    /// `PassportBook` carrying the same issuing country and number.
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
