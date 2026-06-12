//! The core [`Thing`](crate::models::thing::Thing) entity, modeled on
//! [schema.org/Thing](https://schema.org/Thing).
//!
//! A `Thing` is the most general entity in the Main X Index family: a stable
//! identity for any arbitrary discrete object (a book, a paper, a digital
//! asset, a device, a product, …). Its fields mirror the canonical
//! schema.org/Thing properties one-to-one, with a handful of
//! registry-internal fields (`id`, audit timestamps, and the soft-delete
//! pair) added for persistence semantics.
//!
//! A `Thing` is the unit fed to [`compute_match`](crate::matching::scoring::compute_match),
//! [`validate_thing`](crate::validation::validate_thing), and
//! [`mask_thing`](crate::privacy::mask_thing).
//!
//! # Examples
//!
//! ```
//! use thing_service::models::thing::Thing;
//!
//! let mut book = Thing::new("Pride and Prejudice");
//! book.description = Some("A novel of manners by Jane Austen.".into());
//! assert_eq!(book.name, "Pride and Prejudice");
//! assert!(!book.is_deleted);
//! ```

use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::identifier::ThingIdentifier;

/// A canonical record modeled on [schema.org/Thing](https://schema.org/Thing).
///
/// The properties below correspond to schema.org/Thing's canonical
/// properties. Internal fields (`id`, `created_at`, `updated_at`,
/// `is_deleted`, `deleted_at`) are added for registry semantics.
///
/// Only [`name`](Self::name) is required; everything else defaults to empty /
/// `None` via [`Thing::new`]. Construct with `Thing::new` and set the
/// remaining fields directly.
///
/// # Examples
///
/// ```
/// use thing_service::models::thing::Thing;
/// use thing_service::models::identifier::ThingIdentifier;
///
/// let mut t = Thing::new("The Rust Programming Language");
/// t.url = Some("https://doc.rust-lang.org/book/".into());
/// t.identifiers = vec![ThingIdentifier::isbn("9781718500457")];
/// assert_eq!(t.identifiers.len(), 1);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Thing {
    /// Stable system identifier, auto-generated as a UUID v4 by
    /// [`Thing::new`]. Distinct from any external [`identifiers`](Self::identifiers).
    pub id: Uuid,

    /// Primary name — schema.org [`name`](https://schema.org/name).
    /// Required; the only field [`validate_thing`](crate::validation::validate_thing)
    /// rejects when empty.
    pub name: String,
    /// Aliases, prior names, and translations — schema.org
    /// [`alternateName`](https://schema.org/alternateName).
    pub alternate_names: Vec<String>,
    /// Free-text description — schema.org
    /// [`description`](https://schema.org/description).
    pub description: Option<String>,
    /// Short contextual detail that distinguishes this thing from similarly
    /// named ones — schema.org
    /// [`disambiguatingDescription`](https://schema.org/disambiguatingDescription).
    pub disambiguating_description: Option<String>,
    /// URL of a more specific schema.org subtype (e.g.
    /// `https://schema.org/Book`) — schema.org
    /// [`additionalType`](https://schema.org/additionalType).
    pub additional_type: Option<String>,
    /// Canonical URL of the item — schema.org [`url`](https://schema.org/url).
    pub url: Option<String>,
    /// Typed external identifiers (ISBN, DOI, GTIN, …) — schema.org
    /// [`identifier`](https://schema.org/identifier).
    pub identifiers: Vec<ThingIdentifier>,
    /// Image URLs — schema.org [`image`](https://schema.org/image).
    pub images: Vec<String>,
    /// URL of the page for which this is the main entity — schema.org
    /// [`mainEntityOfPage`](https://schema.org/mainEntityOfPage).
    pub main_entity_of_page: Option<String>,
    /// Owning person or organisation — schema.org
    /// [`owner`](https://schema.org/owner). Considered sensitive and masked
    /// by [`mask_thing`](crate::privacy::mask_thing).
    pub owner: Option<String>,
    /// Authoritative external URLs (Wikipedia, Wikidata, …) — schema.org
    /// [`sameAs`](https://schema.org/sameAs). Used as cross-reference
    /// evidence during matching.
    pub same_as: Vec<String>,
    /// URL of a CreativeWork/Event about this item — schema.org
    /// [`subjectOf`](https://schema.org/subjectOf).
    pub subject_of: Option<String>,
    /// Idealised action (URL or descriptor) — schema.org
    /// [`potentialAction`](https://schema.org/potentialAction).
    pub potential_action: Option<String>,

    /// Soft-delete flag. Records are never physically removed; setting this
    /// hides them while preserving the audit trail. See [`Thing::soft_delete`].
    pub is_deleted: bool,
    /// Timestamp the record was soft-deleted, or `None` if still active.
    pub deleted_at: Option<Timestamp>,
    /// Creation timestamp, set once by [`Thing::new`].
    pub created_at: Timestamp,
    /// Last-update timestamp. Equal to [`created_at`](Self::created_at) on a
    /// freshly constructed record.
    pub updated_at: Timestamp,
}

impl Thing {
    /// Constructs a new active `Thing` with the given name.
    ///
    /// Generates a fresh UUID v4 [`id`](Self::id), stamps
    /// [`created_at`](Self::created_at) and [`updated_at`](Self::updated_at)
    /// to "now", and leaves every optional field empty / `None`. The record
    /// is not deleted.
    ///
    /// # Examples
    ///
    /// ```
    /// use thing_service::models::thing::Thing;
    ///
    /// let t = Thing::new("War and Peace");
    /// assert_eq!(t.name, "War and Peace");
    /// assert!(t.description.is_none());
    /// assert!(!t.is_deleted);
    /// ```
    pub fn new(name: &str) -> Self {
        let now = Timestamp::now();
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            alternate_names: Vec::new(),
            description: None,
            disambiguating_description: None,
            additional_type: None,
            url: None,
            identifiers: Vec::new(),
            images: Vec::new(),
            main_entity_of_page: None,
            owner: None,
            same_as: Vec::new(),
            subject_of: None,
            potential_action: None,
            is_deleted: false,
            deleted_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Marks the record as soft-deleted.
    ///
    /// Sets [`is_deleted`](Self::is_deleted) to `true` and records the
    /// deletion time in [`deleted_at`](Self::deleted_at). The record itself
    /// is retained for auditability; callers filter on `is_deleted` to hide
    /// it from normal views.
    ///
    /// # Examples
    ///
    /// ```
    /// use thing_service::models::thing::Thing;
    ///
    /// let mut t = Thing::new("Obsolete Record");
    /// t.soft_delete();
    /// assert!(t.is_deleted);
    /// assert!(t.deleted_at.is_some());
    /// ```
    pub fn soft_delete(&mut self) {
        self.is_deleted = true;
        self.deleted_at = Some(Timestamp::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::identifier::{IdentifierType, ThingIdentifier};

    /// `new` sets the name, generates a non-nil id, and starts undeleted.
    #[test]
    fn test_thing_new() {
        let thing = Thing::new("Pride and Prejudice");
        assert_eq!(thing.name, "Pride and Prejudice");
        assert!(thing.id != Uuid::nil());
        assert!(!thing.is_deleted);
    }

    /// Every optional/collection field defaults to empty / `None`.
    #[test]
    fn test_thing_default_fields() {
        let thing = Thing::new("Test");
        assert!(thing.alternate_names.is_empty());
        assert!(thing.description.is_none());
        assert!(thing.disambiguating_description.is_none());
        assert!(thing.additional_type.is_none());
        assert!(thing.url.is_none());
        assert!(thing.identifiers.is_empty());
        assert!(thing.images.is_empty());
        assert!(thing.main_entity_of_page.is_none());
        assert!(thing.owner.is_none());
        assert!(thing.same_as.is_empty());
        assert!(thing.subject_of.is_none());
        assert!(thing.potential_action.is_none());
        assert!(!thing.is_deleted);
    }

    /// Identifiers can be attached and preserve type and order.
    #[test]
    fn test_thing_with_identifiers() {
        let mut thing = Thing::new("Pride and Prejudice");
        thing.identifiers = vec![
            ThingIdentifier::isbn("9780141439518"),
            ThingIdentifier::new(IdentifierType::Custom("OpenLibrary".into()), "OL1394865W"),
        ];
        assert_eq!(thing.identifiers.len(), 2);
        assert_eq!(thing.identifiers[0].property_id, IdentifierType::Isbn);
    }

    /// URL-valued fields (`url`, `same_as`) round-trip as set.
    #[test]
    fn test_thing_with_urls() {
        let mut thing = Thing::new("Linux Kernel");
        thing.url = Some("https://kernel.org".into());
        thing.same_as = vec![
            "https://en.wikipedia.org/wiki/Linux_kernel".into(),
            "https://www.wikidata.org/wiki/Q14579".into(),
        ];
        assert_eq!(thing.same_as.len(), 2);
    }

    /// A `Thing` survives a JSON serialization round-trip, id included.
    #[test]
    fn test_thing_serialization_roundtrip() {
        let mut thing = Thing::new("Test Thing");
        thing.description = Some("A test".into());
        thing.alternate_names = vec!["Alias One".into(), "Alias Two".into()];
        thing.additional_type = Some("https://schema.org/Book".into());
        let json = serde_json::to_string(&thing).unwrap();
        let deserialized: Thing = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "Test Thing");
        assert_eq!(deserialized.description.as_deref(), Some("A test"));
        assert_eq!(deserialized.alternate_names.len(), 2);
        assert_eq!(
            deserialized.additional_type.as_deref(),
            Some("https://schema.org/Book")
        );
        assert_eq!(deserialized.id, thing.id);
    }

    /// `soft_delete` flips the flag and stamps the deletion time.
    #[test]
    fn test_thing_soft_delete() {
        let mut thing = Thing::new("To Delete");
        assert!(!thing.is_deleted);
        thing.soft_delete();
        assert!(thing.is_deleted);
        assert!(thing.deleted_at.is_some());
    }
}
