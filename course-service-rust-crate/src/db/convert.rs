//! Date/time conversions across the persistence boundary.
//!
//! Domain models use `jiff` (`Timestamp`, `civil::Date`); the SeaORM
//! entity models use the `time` crate (`OffsetDateTime`, `Date`) because
//! SeaORM 1.1 has no native `jiff` support. These helpers translate
//! between the two representations when reading/writing rows.

use jiff::Timestamp;
use jiff::civil::Date as JiffDate;
use time::{Date as TimeDate, Month, OffsetDateTime};

/// `jiff::Timestamp` → `time::OffsetDateTime` (UTC).
pub fn ts_to_offset(ts: Timestamp) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp_nanos(ts.as_nanosecond())
        .expect("jiff Timestamp within time::OffsetDateTime range")
}

/// `time::OffsetDateTime` → `jiff::Timestamp`.
pub fn offset_to_ts(odt: OffsetDateTime) -> Timestamp {
    Timestamp::from_nanosecond(odt.unix_timestamp_nanos())
        .expect("time::OffsetDateTime within jiff Timestamp range")
}

/// `jiff::civil::Date` → `time::Date`.
pub fn date_to_time(d: JiffDate) -> TimeDate {
    let month = Month::try_from(d.month() as u8).expect("valid month");
    TimeDate::from_calendar_date(d.year() as i32, month, d.day() as u8).expect("valid date")
}

/// `time::Date` → `jiff::civil::Date`.
pub fn time_to_date(d: TimeDate) -> JiffDate {
    JiffDate::new(d.year() as i16, u8::from(d.month()) as i8, d.day() as i8).expect("valid date")
}
