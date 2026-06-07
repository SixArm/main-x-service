//! Opening hours for a [`Place`](crate::models::place::Place).
//!
//! [`OpeningHoursSpecification`] models
//! [schema.org/OpeningHoursSpecification](https://schema.org/OpeningHoursSpecification):
//! a single day's open/close window. A place's full schedule is a `Vec` of
//! these, typically one entry per [`DayOfWeek`]. Times are stored as plain
//! `HH:MM` strings.
//!
//! # Examples
//!
//! ```
//! use place_service::models::opening_hours::{DayOfWeek, OpeningHoursSpecification};
//!
//! let monday = OpeningHoursSpecification::new(DayOfWeek::Monday, "09:00", "17:00");
//! assert_eq!(monday.opens, "09:00");
//! ```

use serde::{Deserialize, Serialize};

/// A day of the week (Monday through Sunday).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DayOfWeek {
    /// Monday.
    Monday,
    /// Tuesday.
    Tuesday,
    /// Wednesday.
    Wednesday,
    /// Thursday.
    Thursday,
    /// Friday.
    Friday,
    /// Saturday.
    Saturday,
    /// Sunday.
    Sunday,
}

/// One day's opening window: the day plus open and close times.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OpeningHoursSpecification {
    /// The day this window applies to.
    pub day_of_week: DayOfWeek,
    /// Opening time as an `HH:MM` string, e.g. `"09:00"`.
    pub opens: String,
    /// Closing time as an `HH:MM` string, e.g. `"17:00"`.
    pub closes: String,
}

impl OpeningHoursSpecification {
    /// Constructs an opening-hours entry for one day.
    ///
    /// # Examples
    ///
    /// ```
    /// use place_service::models::opening_hours::{DayOfWeek, OpeningHoursSpecification};
    ///
    /// let oh = OpeningHoursSpecification::new(DayOfWeek::Friday, "08:00", "22:00");
    /// assert_eq!(oh.closes, "22:00");
    /// ```
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

    /// `new` stores the day and both times verbatim.
    #[test]
    fn test_opening_hours() {
        let oh = OpeningHoursSpecification::new(DayOfWeek::Monday, "09:00", "17:00");
        assert_eq!(oh.day_of_week, DayOfWeek::Monday);
        assert_eq!(oh.opens, "09:00");
        assert_eq!(oh.closes, "17:00");
    }

    /// An opening-hours entry survives a JSON serialization round-trip.
    #[test]
    fn test_opening_hours_serialization() {
        let oh = OpeningHoursSpecification::new(DayOfWeek::Friday, "08:00", "22:00");
        let json = serde_json::to_string(&oh).unwrap();
        let deserialized: OpeningHoursSpecification = serde_json::from_str(&json).unwrap();
        assert_eq!(oh, deserialized);
    }
}
