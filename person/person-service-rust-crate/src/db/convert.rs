//! Date/time conversions across the persistence boundary.
//!
//! Domain models use `chrono` (`DateTime<Utc>`, `NaiveDate`); the SeaORM
//! entity models use the `time` crate (`OffsetDateTime`, `Date`) because
//! SeaORM 1.1 has no native `chrono` support configured here. These
//! helpers translate between the two representations when reading/writing
//! rows.

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use time::{Date as TimeDate, Month, OffsetDateTime};

/// Convert a `chrono::DateTime<Utc>` to a `time::OffsetDateTime` in UTC.
///
/// Both types are nanosecond-precision instants, so the conversion goes
/// through the Unix-nanosecond representation and stays in UTC; no
/// timezone information is lost because `DateTime<Utc>` is itself a UTC
/// instant.
///
/// # Panics
///
/// Panics if the chrono nanosecond value cannot be represented (only near
/// the chrono timestamp-nanosecond range limits, ~1677–2262), or if it
/// falls outside the representable range of `time::OffsetDateTime`. In
/// practice this cannot happen for real-world birth/death/audit
/// timestamps.
#[must_use]
pub fn ts_to_offset(ts: DateTime<Utc>) -> OffsetDateTime {
    // Round-trip via Unix nanoseconds: the lossless common denominator
    // between the two instant types.
    let nanos = ts
        .timestamp_nanos_opt()
        .expect("chrono DateTime within nanosecond range");
    OffsetDateTime::from_unix_timestamp_nanos(i128::from(nanos))
        .expect("chrono DateTime within time::OffsetDateTime range")
}

/// Convert a `time::OffsetDateTime` to a `chrono::DateTime<Utc>`.
///
/// Inverse of [`ts_to_offset`]. `time::OffsetDateTime` carries an offset,
/// but `unix_timestamp_nanos` normalizes it to a UTC instant first, so
/// the resulting `DateTime<Utc>` is offset-independent.
///
/// # Panics
///
/// Panics if the Unix-nanosecond value is outside the representable
/// range of `chrono::DateTime<Utc>`. Unreachable for stored real-world
/// rows.
#[must_use]
pub fn offset_to_ts(odt: OffsetDateTime) -> DateTime<Utc> {
    let nanos = i64::try_from(odt.unix_timestamp_nanos())
        .expect("time::OffsetDateTime within chrono nanosecond range");
    DateTime::<Utc>::from_timestamp_nanos(nanos)
}

/// Convert a `chrono::NaiveDate` to a `time::Date` (calendar date only).
///
/// Used for `birth_date` and document issue/expiry columns, which are
/// stored as bare SQL `DATE` values. The chrono month is a `1..=12`
/// integer; `time` models the month as a `Month` enum, so it is bridged
/// through `Month::try_from`.
///
/// # Panics
///
/// Panics if the month is not `1..=12` or the year/day combination is
/// not a valid calendar date. A well-formed `NaiveDate` always
/// satisfies both, so this is unreachable for values produced by chrono.
#[must_use]
pub fn date_to_time(d: NaiveDate) -> TimeDate {
    // chrono months are 1-based integers; time wants its Month enum.
    let month_num = u8::try_from(d.month()).expect("month fits in u8");
    let month = Month::try_from(month_num).expect("valid month");
    let day = u8::try_from(d.day()).expect("day fits in u8");
    TimeDate::from_calendar_date(d.year(), month, day).expect("valid date")
}

/// Convert a `time::Date` to a `chrono::NaiveDate` (calendar date only).
///
/// Inverse of [`date_to_time`], used when hydrating domain models from
/// stored `DATE` columns. `u8::from(d.month())` turns the `time` `Month`
/// enum back into its `1..=12` ordinal, which chrono expects as a `u32`.
///
/// # Panics
///
/// Panics if the year/month/day triple is not a valid calendar date. A
/// value read from a valid `time::Date` always is, so this is
/// unreachable for stored rows.
#[must_use]
pub fn time_to_date(d: TimeDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(d.year(), u32::from(u8::from(d.month())), u32::from(d.day()))
        .expect("valid date")
}
