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
//! | `identifiers[]` | country-specific slot via `route_identifier` |
//! | `documents[]` of type `Passport` | `passport_books` (one per passport) |
//!
//! Service-only fields (`id`, `active`, `worker_type`, `deceased_datetime`,
//! `managing_organization`, `links`, `created_at`, `marital_status`, `photo`,
//! `multiple_birth`) are dropped — they have no matcher counterpart.

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

/// Maps the service [`Gender`] enum onto the matcher's `Gender` enum
/// one-for-one.
fn map_gender(g: Gender) -> MGender {
    match g {
        Gender::Male => MGender::Male,
        Gender::Female => MGender::Female,
        Gender::Other => MGender::Other,
        Gender::Unknown => MGender::Unknown,
    }
}

/// Returns the value of the first telecom entry whose system matches
/// `system`, or `None`. Used to fill the matcher's single
/// `phone`/`mobile`/`email` slots from the service's telecom vector.
fn first_telecom(telecom: &[ContactPoint], system: ContactPointSystem) -> Option<String> {
    telecom
        .iter()
        .find(|c| matches_system(&c.system, &system))
        .map(|c| c.value.clone())
}

/// Returns `true` when two [`ContactPointSystem`] values are the same variant.
///
/// A dedicated comparison (rather than `PartialEq`) keeps the variant list
/// explicit, so adding a new system forces a deliberate decision here.
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

/// Projects a service [`Address`] onto a matcher `Address`, or `None` if every
/// field is empty (an all-`None` address carries no matching signal).
///
/// Note the field rename: the service's `state` maps to the matcher's
/// `county`, and `postal_code` to `postcode`.
fn map_address(a: &Address) -> Option<MAddress> {
    // Skip addresses where every component is absent.
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
///
/// The system-URI table below recognises a distinctive token per scheme
/// (e.g. `pesel`, `nino`, `codice-fiscale`). Tokens are deliberately chosen
/// not to overlap — `nino` is the UK National Insurance number and never
/// collides with the French `nir`, and short ambiguous abbreviations (`chi`,
/// `hc`) require a longer qualifier (`chi-number`, `hc-number`). An
/// unrecognised URI falls through to the `IdentifierType` match arm.
fn route_identifier(b: MBuilder, id: &Identifier) -> MBuilder {
    // Lower-case the system URI once so every `contains` token below can be
    // written in lower case; trim the value so an all-whitespace identifier is
    // dropped rather than filling a slot with blanks.
    let sys = id.system.to_ascii_lowercase();
    let val = id.value.trim();
    if val.is_empty() {
        return b;
    }

    // Routing precedence: (1) this primary system-URI block, (2)
    // `route_additional_scheme` for the remaining national schemes, (3) the
    // generic `IdentifierType` fallback. The system URI wins over the enum
    // because it is more specific (a typed `Other` identifier with an NHS URI
    // should still land in `uk_nhs_number`). Each `contains` token is chosen to
    // be distinctive enough not to collide with another scheme's token.
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
        // "IHI" is ambiguous between Australia and Ireland; disambiguate by
        // length — the Australian IHI is a 16-digit number, the Irish one is
        // shorter — so a 14+-digit value routes to AU, else IE.
        if val.chars().filter(|c| c.is_ascii_digit()).count() >= 14 {
            return b.au_ihi(val);
        }
        return b.ie_ihi(val);
    }
    // Second pass: the expanded national-scheme table. Returns `matched = true`
    // when it filled a slot, in which case we are done; otherwise fall through
    // to the generic enum mapping below with the builder untouched.
    let (b, matched) = route_additional_scheme(b, &sys, val);
    if matched {
        return b;
    }

    // Fallback: the system URI carried no recognised scheme token, so route by
    // the coarse `IdentifierType` enum instead.
    match id.identifier_type {
        // TAX and SSN both default to the US SSN slot — the most common case
        // for an untyped tax/social identifier in this dataset.
        IdentifierType::TAX | IdentifierType::SSN => b.us_ssn(val),
        // PPN passports flow via IdentityDocument; MRN / DL / NPI / Other
        // have no per-country matcher slot.
        //
        // ODS is a *deliberate, permanent* fall-through (entity task T-7,
        // service spec §6.2): an NHS ODS code identifies an organisation or
        // site, not the worker, so every matcher slot (all person-level
        // national schemes) would be a wrong mapping — an exact-match
        // short-circuit would declare colleagues at the same practice to be
        // the same person. The matcher's `local_id` is never scored, so
        // routing there would be a silent no-op. Pinned by
        // `tests/duplicate_detection.rs`
        // (`ods_organisation_code_falls_through_unmapped`,
        // `shared_ods_code_does_not_make_different_workers_match`).
        IdentifierType::PPN
        | IdentifierType::ODS
        | IdentifierType::MRN
        | IdentifierType::DL
        | IdentifierType::NPI
        | IdentifierType::Other => b,
    }
}

/// Route the remaining national schemes the matcher scores deterministically
/// but [`route_identifier`]'s primary block does not cover.
///
/// `sys` is the already-lowercased system URI; `val` is the trimmed value.
/// Returns `(builder, true)` when a scheme token matched (the slot is now
/// filled), or `(builder, false)` — the untouched builder — so the caller can
/// fall through to its `IdentifierType` arm. Each token is distinctive enough
/// not to collide with the primary block (e.g. `nino` is the UK National
/// Insurance number and never overlaps the French `nir`); short ambiguous
/// abbreviations (`chi`, `hc`) require a longer qualifier. Every target slot
/// carries its own weight + breakdown score + deterministic short-circuit in
/// the matcher (spec §12), so wiring them here lets a service-side identifier
/// drive a match instead of silently falling through.
fn route_additional_scheme(b: MBuilder, sys: &str, val: &str) -> (MBuilder, bool) {
    if sys.contains("pesel") {
        return (b.pl_pesel(val), true);
    }
    if sys.contains("nip") {
        return (b.pl_nip(val), true);
    }
    if sys.contains("cnp") {
        return (b.ro_cnp(val), true);
    }
    if sys.contains("nino") || sys.contains("ni-number") || sys.contains("national-insurance") {
        return (b.uk_nino(val), true);
    }
    if sys.contains("chi-number") || sys.contains("community-health-index") {
        return (b.uk_chi_number(val), true);
    }
    if sys.contains("hc-number") || sys.contains("health-and-care-number") {
        return (b.uk_hc_number(val), true);
    }
    if sys.contains("codice-fiscale") || sys.contains("codicefiscale") {
        return (b.it_cf(val), true);
    }
    if sys.contains("dni") {
        return (b.es_dni(val), true);
    }
    if sys.contains("nif") {
        return (b.pt_nif(val), true);
    }
    if sys.contains("hetu") {
        return (b.fi_hetu(val), true);
    }
    if sys.contains("cpr") {
        return (b.dk_cpr(val), true);
    }
    if sys.contains("oib") {
        return (b.hr_oib(val), true);
    }
    if sys.contains("fnr") || sys.contains("fodselsnummer") || sys.contains("fødselsnummer") {
        return (b.no_fnr(val), true);
    }
    if sys.contains("egn") {
        return (b.bg_egn(val), true);
    }
    if sys.contains("emso") {
        return (b.si_emso(val), true);
    }
    if sys.contains("rrn") {
        return (b.cn_rrn(val), true);
    }
    if sys.contains("za-id") || sys.contains("south-africa") {
        return (b.za_id(val), true);
    }
    if sys.contains("rijksregister") || sys.contains("be-nn") {
        return (b.be_nn(val), true);
    }
    (b, false)
}

/// Builds a matcher `PassportBook` from an [`IdentityDocument`], or `None`
/// when the issuing country is missing/blank (the matcher keys passports by
/// country + number, so both are required).
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
    use jiff::Timestamp;
    use uuid::Uuid;

    /// Builds a minimal service worker with the given family/given name.
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

    /// Family and given names survive the projection to the matcher worker.
    #[test]
    fn round_trip_names_and_dob() {
        let svc = svc_worker("Williams", "Alice");
        let m = to_matcher_worker(&svc);
        assert_eq!(m.family_name.as_deref(), Some("Williams"));
        assert_eq!(m.given_name.as_deref(), Some("Alice"));
    }

    /// An NHS system URI routes the identifier into the `uk_nhs_number` slot.
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

    /// Pushes an identifier with the given system URI and returns the routed
    /// matcher worker, so each scheme assertion is a one-liner.
    fn route(system: &str, value: &str) -> MWorker {
        let mut svc = svc_worker("Nowak", "Jan");
        svc.identifiers.push(Identifier::new(
            IdentifierType::Other,
            system.into(),
            value.into(),
        ));
        to_matcher_worker(&svc)
    }

    /// Each newly-wired national scheme routes to its own matcher slot from a
    /// distinctive system-URI token — and nothing else lands in a sibling slot.
    #[test]
    fn routes_additional_national_schemes_by_system_uri() {
        assert_eq!(
            route("urn:gov.pl:pesel", "44051401359").pl_pesel.as_deref(),
            Some("44051401359")
        );
        assert_eq!(
            route("urn:gov.ro:cnp", "1960229052089").ro_cnp.as_deref(),
            Some("1960229052089")
        );
        assert_eq!(
            route("https://fhir.hl7.org.uk/Id/ni-number", "QQ123456C")
                .uk_nino
                .as_deref(),
            Some("QQ123456C")
        );
        assert_eq!(
            route("urn:nhs.scot:chi-number", "0101336489")
                .uk_chi_number
                .as_deref(),
            Some("0101336489")
        );
        assert_eq!(
            route("urn:it:codice-fiscale", "RSSMRA80A01H501U")
                .it_cf
                .as_deref(),
            Some("RSSMRA80A01H501U")
        );
        assert_eq!(
            route("urn:dk:cpr", "0101701234").dk_cpr.as_deref(),
            Some("0101701234")
        );
        assert_eq!(
            route("urn:fi:hetu", "131052-308T").fi_hetu.as_deref(),
            Some("131052-308T")
        );
    }

    /// `nino` (UK National Insurance) must not be swallowed by the earlier
    /// `nir` (FR) check, and vice-versa — the two tokens are kept distinct.
    #[test]
    fn uk_nino_and_fr_nir_do_not_cross_route() {
        let nino = route("urn:uk:nino", "QQ123456C");
        assert_eq!(nino.uk_nino.as_deref(), Some("QQ123456C"));
        assert!(nino.fr_nir.is_none(), "nino must not leak into fr_nir");

        let nir = route("urn:fr:nir", "180057402048077");
        assert_eq!(nir.fr_nir.as_deref(), Some("180057402048077"));
        assert!(nir.uk_nino.is_none(), "nir must not leak into uk_nino");
    }
}
