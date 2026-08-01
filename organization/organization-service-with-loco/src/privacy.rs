//! Privacy: field masking and the GDPR right-of-access export.
//!
//! ## What is sensitive about an organization
//!
//! Most of an organization record is deliberately public — a name, a
//! legal name, a website, a jurisdiction, a registry identifier like an
//! LEI are all published facts, and masking them would defeat the
//! registry. Three things are not:
//!
//! - **`telephone` / `email`** — routinely a named individual's direct
//!   line or inbox, and the spec's declared masking targets (§7 export
//!   sensitivity).
//! - **`street_address`** — for a **sole trader** the registered address
//!   is a home address. There is no `is_sole_trader` flag to key on and
//!   inventing one would be guesswork, so the street line is masked for
//!   every record while locality / region / postal code / country stay
//!   visible: enough to place and disambiguate an organization, not
//!   enough to arrive at someone's door.
//! - **`TaxId` / `Vat` identifier values** — jurisdiction-scoped fiscal
//!   identifiers. For a sole trader these are frequently the person's own
//!   tax number. Registry identifiers (LEI, DUNS, ROR, ISNI, Wikidata, …)
//!   are *not* masked: they identify the entity in public registers, and
//!   masking them would break the very lookups the registry exists for.
//!
//! ## Consent
//!
//! The shared privacy contract's consent model
//! ([`agents/share/privacy.md`]) is about a **data subject** granting a
//! purpose. An organization is not a data subject; the natural persons
//! behind one are, and they are recorded in the person service, which
//! owns their consent. Modelling consent here would create a second,
//! unauthoritative home for it, so this crate deliberately has none.
//!
//! Everything here is pure; the endpoints live in
//! [`crate::controllers::organizations`].
//!
//! [`agents/share/privacy.md`]: ../../../agents/share/privacy.md

use organization_matcher::{IdentifierScheme, Organization};

/// Return a copy of `org` with the sensitive fields masked (see the
/// module docs for what counts as sensitive and why).
///
/// Masking keeps the last four characters of a value visible so an
/// operator can still tell two records apart — a fully redacted phone
/// number makes a duplicate-review screen useless. The street line is
/// the exception: it is dropped rather than partially shown, because a
/// house number's tail is not a disambiguator, it is the address.
#[must_use]
pub fn mask_organization(org: &Organization) -> Organization {
    let mut masked = org.clone();

    if let Some(ref phone) = masked.telephone {
        masked.telephone = Some(mask_value(phone, 4));
    }
    if let Some(ref email) = masked.email {
        masked.email = Some(mask_email(email));
    }
    if let Some(ref mut address) = masked.address {
        address.street_address = None;
    }
    for identifier in &mut masked.identifiers {
        if is_fiscal(&identifier.scheme) {
            identifier.value = mask_value(&identifier.value, 4);
        }
    }

    masked
}

/// Whether a scheme carries a **fiscal** identifier — the kind that, for
/// a sole trader, is often the proprietor's own tax number. Public
/// registry schemes (LEI / DUNS / ROR / …) are deliberately excluded:
/// they name the entity in a public register, and hiding them would
/// break the lookups this registry exists to serve.
fn is_fiscal(scheme: &IdentifierScheme) -> bool {
    matches!(scheme, IdentifierScheme::TaxId | IdentifierScheme::Vat)
}

/// Mask a string, keeping only its last `visible_chars` characters.
///
/// Alphanumerics in the hidden prefix become `*`; punctuation is kept so
/// the shape stays readable (`"+44 20 7946 0958"` →
/// `"+** ** **** 0958"`). Values no longer than `visible_chars` are
/// returned unchanged — there is nothing to hide behind.
///
/// Counts `char`s, not bytes, so a multibyte value is masked correctly
/// and can never be sliced across a UTF-8 boundary (the person service
/// had exactly that panic).
fn mask_value(value: &str, visible_chars: usize) -> String {
    let char_count = value.chars().count();
    if char_count <= visible_chars {
        return value.to_string();
    }
    let hidden = char_count - visible_chars;
    value
        .chars()
        .enumerate()
        .map(|(i, c)| {
            if i < hidden && c.is_alphanumeric() {
                '*'
            } else {
                c
            }
        })
        .collect()
}

/// Mask an email address, keeping the domain and the local part's first
/// character: `"accounts@acme.example"` → `"a*******@acme.example"`.
///
/// The domain stays because it is the organization's own — usually
/// derivable from the (unmasked) `url` anyway, so hiding it buys nothing
/// while making the value useless for recognising a record. What must
/// not survive is the local part, which is frequently a person's name.
/// A value with no `@` is masked as an ordinary string, since it is not
/// an address this function can reason about.
fn mask_email(email: &str) -> String {
    let Some((local, domain)) = email.split_once('@') else {
        return mask_value(email, 4);
    };
    let mut chars = local.chars();
    let masked_local: String = match chars.next() {
        Some(first) => std::iter::once(first)
            .chain(chars.map(|c| if c.is_alphanumeric() { '*' } else { c }))
            .collect(),
        // An empty local part ("@example.com") is malformed; leave it be
        // rather than invent characters.
        None => String::new(),
    };
    format!("{masked_local}@{domain}")
}

/// Build the GDPR right-of-access payload for one organization.
///
/// An **envelope**, not the bare record: an access request has to be
/// answerable as "here is everything held about this subject, as of
/// then", so the response states what was exported, when, and whether
/// any of it was redacted. `masked` is the honest part — a masked export
/// is a *partial* answer to an access request, and the caller must be
/// able to tell which they received.
#[must_use]
pub fn export_organization(org: &Organization, pid: &str, masked: bool) -> serde_json::Value {
    let record = if masked {
        mask_organization(org)
    } else {
        org.clone()
    };
    serde_json::json!({
        "entity": "organization",
        "pid": pid,
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "masked": masked,
        "record": serde_json::to_value(&record).unwrap_or(serde_json::Value::Null),
        "note": if masked {
            "Partial export: telephone, email, street address, and fiscal \
             identifiers are redacted for this caller."
        } else {
            "Complete export of the stored organization record."
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use organization_matcher::{OrgIdentifier, PostalAddress};

    /// A fully-populated organization to redact.
    fn org() -> Organization {
        Organization {
            legal_name: Some("Acme Incorporated".into()),
            telephone: Some("+44 20 7946 0958".into()),
            email: Some("accounts@acme.example".into()),
            url: Some("https://acme.example".into()),
            jurisdiction: Some("GB".into()),
            identifiers: vec![
                OrgIdentifier {
                    scheme: IdentifierScheme::Lei,
                    value: "5493001KJTIIGC8Y1R12".into(),
                },
                OrgIdentifier {
                    scheme: IdentifierScheme::TaxId,
                    value: "GB123456789".into(),
                },
            ],
            address: Some(PostalAddress {
                street_address: Some("14 Rue Descartes".into()),
                locality: Some("Paris".into()),
                region: Some("Île-de-France".into()),
                postal_code: Some("75005".into()),
                country: Some("FR".into()),
            }),
            ..Organization::new("Acme, Inc.")
        }
    }

    /// The phone number keeps its last four digits and its punctuation.
    #[test]
    fn telephone_is_masked_to_its_tail() {
        assert_eq!(
            mask_organization(&org()).telephone.as_deref(),
            Some("+** ** **** 0958")
        );
    }

    /// The email keeps its domain and the local part's first character.
    #[test]
    fn email_keeps_the_domain_and_one_character() {
        assert_eq!(
            mask_organization(&org()).email.as_deref(),
            Some("a*******@acme.example")
        );
    }

    /// The street line goes; everything else about the address stays, so
    /// the organization is still placeable.
    #[test]
    fn street_address_is_dropped_but_the_locality_remains() {
        let address = mask_organization(&org()).address.expect("address");
        assert_eq!(address.street_address, None);
        assert_eq!(address.locality.as_deref(), Some("Paris"));
        assert_eq!(address.postal_code.as_deref(), Some("75005"));
        assert_eq!(address.country.as_deref(), Some("FR"));
    }

    /// Fiscal identifiers are masked; public registry ones are not —
    /// masking an LEI would break the lookup the registry exists for.
    #[test]
    fn fiscal_identifiers_are_masked_and_registry_ones_are_not() {
        let masked = mask_organization(&org());
        assert_eq!(masked.identifiers[0].value, "5493001KJTIIGC8Y1R12");
        assert_eq!(masked.identifiers[1].value, "*******6789");
    }

    /// The public facts stay public: masking must not damage the fields
    /// that make the record findable.
    #[test]
    fn public_fields_survive_masking() {
        let masked = mask_organization(&org());
        assert_eq!(masked.name, "Acme, Inc.");
        assert_eq!(masked.legal_name.as_deref(), Some("Acme Incorporated"));
        assert_eq!(masked.url.as_deref(), Some("https://acme.example"));
        assert_eq!(masked.jurisdiction.as_deref(), Some("GB"));
    }

    /// Masking an organization with nothing sensitive is a no-op rather
    /// than an error or an invented value.
    #[test]
    fn a_bare_organization_is_unchanged() {
        let bare = Organization::new("Bare Ltd");
        let masked = mask_organization(&bare);
        assert_eq!(masked.name, "Bare Ltd");
        assert_eq!(masked.telephone, None);
        assert_eq!(masked.email, None);
        assert!(masked.address.is_none());
    }

    /// Values no longer than the visible tail are left alone, and
    /// masking counts characters rather than bytes (a byte-indexed
    /// implementation panics here).
    #[test]
    fn mask_value_is_char_safe() {
        assert_eq!(mask_value("123-45-6789", 4), "***-**-6789");
        assert_eq!(mask_value("abc", 4), "abc");
        assert_eq!(mask_value("1é345", 4), "*é345");
        assert_eq!(mask_value("naïve12", 4), "***ve12");
    }

    /// A string that is not an address falls back to ordinary masking
    /// rather than producing something that looks like an address.
    #[test]
    fn mask_email_without_an_at_sign_falls_back() {
        assert_eq!(mask_email("not-an-email"), "***-**-*mail");
        assert_eq!(mask_email("@example.com"), "@example.com");
    }

    /// The export envelope names the subject, says when, and — the part
    /// that matters — says whether what it carries was redacted.
    #[test]
    fn export_envelope_declares_whether_it_is_partial() {
        let full = export_organization(&org(), "pid-1", false);
        assert_eq!(full["entity"], "organization");
        assert_eq!(full["pid"], "pid-1");
        assert_eq!(full["masked"], false);
        assert_eq!(full["record"]["telephone"], "+44 20 7946 0958");
        assert!(full["exported_at"].as_str().is_some_and(|s| !s.is_empty()));

        let partial = export_organization(&org(), "pid-1", true);
        assert_eq!(partial["masked"], true);
        assert_eq!(partial["record"]["telephone"], "+** ** **** 0958");
        assert!(
            partial["note"]
                .as_str()
                .is_some_and(|n| n.contains("Partial")),
            "a masked export must say so"
        );
    }
}
