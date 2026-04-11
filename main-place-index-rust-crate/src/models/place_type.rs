use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PlaceType {
    LocalBusiness,
    CivicStructure,
    AdministrativeArea,
    Landform,
    Park,
    Airport,
    Hospital,
    School,
    Library,
    Museum,
    Restaurant,
    Hotel,
    Other(String),
}

impl std::fmt::Display for PlaceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlaceType::Other(s) => write!(f, "{s}"),
            _ => write!(f, "{self:?}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_place_type_display() {
        assert_eq!(PlaceType::Park.to_string(), "Park");
        assert_eq!(PlaceType::Other("Marina".into()).to_string(), "Marina");
    }

    #[test]
    fn test_place_type_equality() {
        assert_eq!(PlaceType::Hospital, PlaceType::Hospital);
        assert_ne!(PlaceType::School, PlaceType::Library);
    }

    #[test]
    fn test_place_type_serialization() {
        let pt = PlaceType::Restaurant;
        let json = serde_json::to_string(&pt).unwrap();
        let deserialized: PlaceType = serde_json::from_str(&json).unwrap();
        assert_eq!(pt, deserialized);
    }

    #[test]
    fn test_place_type_other_serialization() {
        let pt = PlaceType::Other("GolfCourse".into());
        let json = serde_json::to_string(&pt).unwrap();
        let deserialized: PlaceType = serde_json::from_str(&json).unwrap();
        assert_eq!(pt, deserialized);
    }
}
