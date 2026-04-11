use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum IdentifierType {
    GlobalLocationNumber,
    BranchCode,
    Fips,
    Gnis,
    OpenStreetMap,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThingIdentifier {
    pub identifier_type: IdentifierType,
    pub value: String,
}

impl ThingIdentifier {
    pub fn new(identifier_type: IdentifierType, value: &str) -> Self {
        Self {
            identifier_type,
            value: value.to_string(),
        }
    }

    pub fn gln(value: &str) -> Self {
        Self::new(IdentifierType::GlobalLocationNumber, value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identifier_gln() {
        let id = ThingIdentifier::gln("1234567890123");
        assert_eq!(id.identifier_type, IdentifierType::GlobalLocationNumber);
        assert_eq!(id.value, "1234567890123");
    }

    #[test]
    fn test_identifier_custom() {
        let id = ThingIdentifier::new(IdentifierType::Custom("NPI".into()), "ABC123");
        assert_eq!(id.value, "ABC123");
    }

    #[test]
    fn test_identifier_serialization() {
        let id = ThingIdentifier::gln("1234567890123");
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: ThingIdentifier = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }
}
