//! Date/time conversions across the persistence boundary.
//!
//! Domain models use `chrono` (`DateTime<Utc>`, `NaiveDate`); the SeaORM
//! entity models use the `time` crate (`OffsetDateTime`, `Date`) because
//! SeaORM 1.1 is configured with the `with-time` feature. These helpers
//! translate between the two representations when reading/writing rows.

use chrono::{DateTime, NaiveDate, Utc};
use time::{Date as TimeDate, Month, OffsetDateTime};

/// `chrono::DateTime<Utc>` → `time::OffsetDateTime` (UTC).
pub fn ts_to_offset(ts: DateTime<Utc>) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp_nanos(i128::from(
        ts.timestamp_nanos_opt()
            .expect("chrono DateTime within nanosecond range"),
    ))
    .expect("chrono DateTime within time::OffsetDateTime range")
}

/// `time::OffsetDateTime` → `chrono::DateTime<Utc>`.
pub fn offset_to_ts(odt: OffsetDateTime) -> DateTime<Utc> {
    DateTime::from_timestamp_nanos(
        i64::try_from(odt.unix_timestamp_nanos())
            .expect("time::OffsetDateTime within nanosecond range"),
    )
}

/// `chrono::NaiveDate` → `time::Date`.
pub fn date_to_time(d: NaiveDate) -> TimeDate {
    use chrono::Datelike;
    let month = Month::try_from(d.month() as u8).expect("valid month");
    TimeDate::from_calendar_date(d.year(), month, d.day() as u8).expect("valid date")
}

/// `time::Date` → `chrono::NaiveDate`.
pub fn time_to_date(d: TimeDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(d.year(), u8::from(d.month()) as u32, d.day() as u32)
        .expect("valid date")
}
