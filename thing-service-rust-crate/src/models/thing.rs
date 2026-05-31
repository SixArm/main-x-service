use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::identifier::ThingIdentifier;

/// A canonical record modeled on [schema.org/Thing](https://schema.org/Thing).
///
/// The properties below correspond to schema.org/Thing's canonical
/// properties. Internal fields (`id`, `created_at`, `updated_at`,
/// `is_deleted`, `deleted_at`) are added for registry semantics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thing {
    pub id: Uuid,

    pub name: String,
    pub alternate_names: Vec<String>,
    pub description: Option<String>,
    pub disambiguating_description: Option<String>,
    pub additional_type: Option<String>,
    pub url: Option<String>,
    pub identifiers: Vec<ThingIdentifier>,
    pub images: Vec<String>,
    pub main_entity_of_page: Option<String>,
    pub owner: Option<String>,
    pub same_as: Vec<String>,
    pub subject_of: Option<String>,
    pub potential_action: Option<String>,

    pub is_deleted: bool,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Thing {
    pub fn new(name: &str) -> Self {
        let now = Utc::now();
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

    pub fn soft_delete(&mut self) {
        self.is_deleted = true;
        self.deleted_at = Some(Utc::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::identifier::{IdentifierType, ThingIdentifier};

    #[test]
    fn test_thing_new() {
        let thing = Thing::new("Pride and Prejudice");
        assert_eq!(thing.name, "Pride and Prejudice");
        assert!(thing.id != Uuid::nil());
        assert!(!thing.is_deleted);
    }

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
        assert_eq!(deserialized.additional_type.as_deref(), Some("https://schema.org/Book"));
        assert_eq!(deserialized.id, thing.id);
    }

    #[test]
    fn test_thing_soft_delete() {
        let mut thing = Thing::new("To Delete");
        assert!(!thing.is_deleted);
        thing.soft_delete();
        assert!(thing.is_deleted);
        assert!(thing.deleted_at.is_some());
    }
}
