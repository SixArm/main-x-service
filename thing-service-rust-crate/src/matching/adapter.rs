//! Adapter from the service's `Thing` domain model to the `thing-matcher`
//! library's `Thing` matching input.
//!
//! The service's `Thing` mirrors `schema.org/Thing` and adds registry fields
//! (`id`, audit timestamps, soft-delete flag). The matcher's `Thing` accepts
//! the same schema.org properties but in a slightly different cardinality —
//! `additional_types: Vec<String>` (the service uses singular
//! `additional_type`), `subject_of: Vec<String>` (service uses singular
//! `subject_of`), and `image: Option<String>` (service uses `images:
//! Vec<String>`). The matcher's `name` is `Option<String>` whereas the
//! service requires `name: String`.
//!
//! Identifier shape also differs: the service uses an `IdentifierType` enum
//! (`Doi`, `Isbn`, `Issn`, `Gtin`, `Sku`, `Mpn`, `SerialNumber`, `Uri`,
//! `Uuid`, `Custom(String)`) plus optional `name` + `url` metadata; the
//! matcher uses an opaque `(property_id: String, value: String)` shape and
//! drops the metadata.
//!
//! [`to_matcher_thing`](crate::matching::adapter::to_matcher_thing) performs
//! the projection so callers can use the canonical algorithm without
//! rewriting their domain model.
//!
//! # Mapping
//!
//! | Service field | Matcher slot |
//! |---|---|
//! | `name` | `name` |
//! | `alternate_names` | `alternate_names` |
//! | `description` | `description` |
//! | `disambiguating_description` | `disambiguating_description` |
//! | `additional_type` (Option) | one entry in `additional_types` |
//! | `url` | `url` |
//! | `identifiers[]` | mapped via [`map_identifier_property`](crate::matching::adapter::map_identifier_property) |
//! | `images[0]` | `image` (first only — matcher takes a single URL) |
//! | `main_entity_of_page` | `main_entity_of_page` |
//! | `owner` | `owner` |
//! | `same_as` | `same_as` |
//! | `subject_of` (Option) | one entry in `subject_of` |
//!
//! Service-only fields (`id`, `is_deleted`, `created_at`, `updated_at`,
//! `potential_action`) are dropped — they have no matcher counterpart.

use thing_matcher::{Identifier as MIdentifier, Thing as MThing};

use crate::models::{
    identifier::{IdentifierType, ThingIdentifier},
    thing::Thing,
};

/// Convert a service `Thing` into a `thing_matcher::Thing` ready for
/// `MatchingEngine::match_things` / `deterministic_match`.
pub fn to_matcher_thing(t: &Thing) -> MThing {
    let mut b = MThing::builder().name(t.name.trim());

    for alt in t
        .alternate_names
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        b = b.add_alternate_name(alt);
    }

    if let Some(d) = t.description.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        b = b.description(d);
    }
    if let Some(d) = t
        .disambiguating_description
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        b = b.disambiguating_description(d);
    }
    if let Some(u) = t.url.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        b = b.url(u);
    }
    if let Some(u) = t
        .main_entity_of_page
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        b = b.main_entity_of_page(u);
    }
    if let Some(o) = t.owner.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        b = b.owner(o);
    }

    // image: matcher takes a single URL; service has Vec — keep the first.
    if let Some(img) = t.images.iter().find(|s| !s.trim().is_empty()) {
        b = b.image(img.trim());
    }

    // additional_type / subject_of: matcher takes Vec; service holds Option.
    if let Some(at) = t.additional_type.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        b = b.add_additional_type(at);
    }
    if let Some(so) = t.subject_of.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        b = b.add_subject_of(so);
    }

    // same_as: vec → vec, filter empties.
    let same_as: Vec<String> = t
        .same_as
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if !same_as.is_empty() {
        b = b.same_as(same_as);
    }

    // Identifiers: service enum → matcher's opaque (property_id, value).
    let ids: Vec<MIdentifier> = t
        .identifiers
        .iter()
        .filter_map(thing_identifier_to_matcher)
        .collect();
    if !ids.is_empty() {
        b = b.identifiers(ids);
    }

    b.build()
}

/// Project a single service
/// [`ThingIdentifier`](crate::models::identifier::ThingIdentifier) onto the
/// matcher's opaque
/// `(property_id, value)` shape, dropping the `name`/`url` metadata. Returns
/// `None` when the matcher rejects the pair (e.g. an empty value).
fn thing_identifier_to_matcher(id: &ThingIdentifier) -> Option<MIdentifier> {
    MIdentifier::new(map_identifier_property(&id.property_id), id.value.trim())
}

/// Map the service's `IdentifierType` enum to the matcher's string-keyed
/// `property_id`. Names follow schema.org's canonical lowercase tokens
/// (`doi`, `isbn`, `issn`, `gtin`, `sku`, `mpn`, `serialNumber`, `uri`,
/// `uuid`); `Custom(s)` passes the carried label through verbatim.
pub fn map_identifier_property(t: &IdentifierType) -> &str {
    match t {
        IdentifierType::Doi => "doi",
        IdentifierType::Isbn => "isbn",
        IdentifierType::Issn => "issn",
        IdentifierType::Gtin => "gtin",
        IdentifierType::Sku => "sku",
        IdentifierType::Mpn => "mpn",
        IdentifierType::SerialNumber => "serialNumber",
        IdentifierType::Uri => "uri",
        IdentifierType::Uuid => "uuid",
        IdentifierType::Custom(s) => s.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Core fields (name, alternate names, additional_type, ISBN) project
    /// across with the expected matcher-side cardinality and tokens.
    #[test]
    fn round_trip_basic() {
        let mut svc = Thing::new("Pride and Prejudice");
        svc.alternate_names = vec!["First Impressions".into()];
        svc.description = Some("A novel.".into());
        svc.additional_type = Some("https://schema.org/Book".into());
        svc.identifiers = vec![ThingIdentifier::isbn("9780141439518")];

        let m = to_matcher_thing(&svc);
        assert_eq!(m.name.as_deref(), Some("Pride and Prejudice"));
        assert_eq!(m.alternate_names.len(), 1);
        assert_eq!(m.additional_types.len(), 1);
        assert_eq!(m.additional_types[0], "https://schema.org/Book");
        assert_eq!(m.identifiers.len(), 1);
        assert_eq!(m.identifiers[0].property_id, "isbn");
        assert_eq!(m.identifiers[0].value, "9780141439518");
    }

    /// The service's `images` vec collapses to the matcher's single `image`.
    #[test]
    fn images_collapse_to_first() {
        let mut svc = Thing::new("Thing");
        svc.images = vec!["https://a.example/1.jpg".into(), "https://b.example/2.jpg".into()];
        let m = to_matcher_thing(&svc);
        assert_eq!(m.image.as_deref(), Some("https://a.example/1.jpg"));
    }

    /// A `Custom(label)` identifier passes its label through as property_id.
    #[test]
    fn custom_identifier_passes_through() {
        let mut svc = Thing::new("Thing");
        svc.identifiers = vec![ThingIdentifier::new(
            IdentifierType::Custom("OpenLibrary".into()),
            "OL1394865W",
        )];
        let m = to_matcher_thing(&svc);
        assert_eq!(m.identifiers[0].property_id, "OpenLibrary");
    }
}
