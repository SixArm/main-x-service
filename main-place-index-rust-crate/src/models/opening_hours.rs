use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DayOfWeek {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OpeningHoursSpecification {
    pub day_of_week: DayOfWeek,
    pub opens: String,
    pub closes: String,
}

impl OpeningHoursSpecification {
    pub fn new(day: DayOfWeek, opens: &str, closes: &str) -> Self {
        Self {
            day_of_week: day,
            opens: opens.to_string(),
            closes: closes.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opening_hours() {
        let oh = OpeningHoursSpecification::new(DayOfWeek::Monday, "09:00", "17:00");
        assert_eq!(oh.day_of_week, DayOfWeek::Monday);
        assert_eq!(oh.opens, "09:00");
        assert_eq!(oh.closes, "17:00");
    }

    #[test]
    fn test_opening_hours_serialization() {
        let oh = OpeningHoursSpecification::new(DayOfWeek::Friday, "08:00", "22:00");
        let json = serde_json::to_string(&oh).unwrap();
        let deserialized: OpeningHoursSpecification = serde_json::from_str(&json).unwrap();
        assert_eq!(oh, deserialized);
    }
}
