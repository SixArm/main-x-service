//! Date/time conversions across the persistence boundary.
//!
//! Domain models use `chrono` (`DateTime<Utc>`, `NaiveDate`); the SeaORM
//! entity models use the `time` crate (`OffsetDateTime`, `Date`) because
//! SeaORM 1.1 has no native `chrono` support in this crate's feature set.
//! These helpers translate between the two representations when
//! reading/writing rows.

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use time::{Date as TimeDate, Month, OffsetDateTime};

/// `chrono::DateTime<Utc>` → `time::OffsetDateTime` (UTC).
///
/// Used when writing a domain timestamp into a `SeaORM` row. Both types
/// carry nanosecond UTC instants, so the conversion is lossless.
///
/// # Panics
///
/// Panics if the `chrono` instant falls outside `time::OffsetDateTime`'s
/// representable range. In practice both ranges cover real-world event
/// timestamps, so this cannot fire for stored event data.
#[must_use]
pub fn ts_to_offset(ts: DateTime<Utc>) -> OffsetDateTime {
    let nanos =
        i128::from(ts.timestamp()) * 1_000_000_000 + i128::from(ts.timestamp_subsec_nanos());
    OffsetDateTime::from_unix_timestamp_nanos(nanos)
        .expect("chrono DateTime within time::OffsetDateTime range")
}

/// `time::OffsetDateTime` → `chrono::DateTime<Utc>`.
///
/// Used when reading a `SeaORM` row back into a domain model.
///
/// # Panics
///
/// Panics if the `time` instant falls outside `chrono::DateTime`'s
/// representable range (see [`ts_to_offset`]); unreachable for stored
/// event data.
#[must_use]
pub fn offset_to_ts(odt: OffsetDateTime) -> DateTime<Utc> {
    DateTime::from_timestamp(odt.unix_timestamp(), odt.nanosecond())
        .expect("time::OffsetDateTime within chrono DateTime range")
}

/// `chrono::NaiveDate` → `time::Date`.
///
/// For date-only columns (no time-of-day). The intermediate
/// `time::Month` conversion is why this is not a one-liner: `time`
/// models the month as an enum, `chrono` as a `1..=12` integer.
///
/// # Panics
///
/// Panics if the month is not `1..=12` or the year/day combination is
/// not a real calendar date. A `chrono::NaiveDate` is always valid, so
/// neither `expect` can fire for inputs produced by `chrono`.
#[must_use]
pub fn date_to_time(d: NaiveDate) -> TimeDate {
    let month_num = u8::try_from(d.month()).expect("chrono month in 1..=12");
    let day_num = u8::try_from(d.day()).expect("chrono day in 1..=31");
    let month = Month::try_from(month_num).expect("valid month");
    TimeDate::from_calendar_date(d.year(), month, day_num).expect("valid date")
}

/// `time::Date` → `chrono::NaiveDate`.
///
/// Inverse of [`date_to_time`], used when reading a date column back
/// into a domain model.
///
/// # Panics
///
/// Panics if the components do not form a valid `chrono` date;
/// unreachable for a value produced by `time::Date`.
#[must_use]
pub fn time_to_date(d: TimeDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(d.year(), u32::from(u8::from(d.month())), u32::from(d.day()))
        .expect("valid date")
}
