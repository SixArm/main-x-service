use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PostalAddress {
    pub street_address: Option<String>,
    pub address_locality: Option<String>,
    pub address_region: Option<String>,
    pub address_country: Option<String>,
    pub postal_code: Option<String>,
}

impl PostalAddress {
    pub fn new() -> Self {
        Self {
            street_address: None,
            address_locality: None,
            address_region: None,
            address_country: None,
            postal_code: None,
        }
    }
}

impl Default for PostalAddress {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_postal_address_new() {
        let addr = PostalAddress::new();
        assert!(addr.street_address.is_none());
        assert!(addr.address_locality.is_none());
    }

    #[test]
    fn test_postal_address_with_fields() {
        let addr = PostalAddress {
            street_address: Some("123 Main St".into()),
            address_locality: Some("Springfield".into()),
            address_region: Some("IL".into()),
            address_country: Some("US".into()),
            postal_code: Some("62701".into()),
        };
        assert_eq!(addr.street_address.as_deref(), Some("123 Main St"));
        assert_eq!(addr.postal_code.as_deref(), Some("62701"));
    }

    #[test]
    fn test_postal_address_serialization() {
        let addr = PostalAddress {
            street_address: Some("456 Oak Ave".into()),
            address_locality: Some("Portland".into()),
            address_region: Some("OR".into()),
            address_country: Some("US".into()),
            postal_code: Some("97201".into()),
        };
        let json = serde_json::to_string(&addr).unwrap();
        let deserialized: PostalAddress = serde_json::from_str(&json).unwrap();
        assert_eq!(addr, deserialized);
    }

    #[test]
    fn test_postal_address_partial() {
        let addr = PostalAddress {
            street_address: None,
            address_locality: Some("London".into()),
            address_region: None,
            address_country: Some("GB".into()),
            postal_code: None,
        };
        assert!(addr.street_address.is_none());
        assert_eq!(addr.address_country.as_deref(), Some("GB"));
    }
}
