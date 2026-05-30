use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AmenityFeature {
    pub name: String,
    pub value: Option<String>,
}

impl AmenityFeature {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            value: None,
        }
    }

    pub fn with_value(name: &str, value: &str) -> Self {
        Self {
            name: name.to_string(),
            value: Some(value.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amenity_new() {
        let a = AmenityFeature::new("WiFi");
        assert_eq!(a.name, "WiFi");
        assert!(a.value.is_none());
    }

    #[test]
    fn test_amenity_with_value() {
        let a = AmenityFeature::with_value("Parking", "100 spaces");
        assert_eq!(a.name, "Parking");
        assert_eq!(a.value.as_deref(), Some("100 spaces"));
    }
}
