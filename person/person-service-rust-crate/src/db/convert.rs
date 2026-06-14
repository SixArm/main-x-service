//! Date/time conversions across the persistence boundary.
//!
//! Domain models use `jiff` (`Timestamp`, `civil::Date`); the SeaORM
//! entity models use the `time` crate (`OffsetDateTime`, `Date`) because
//! SeaORM 1.1 has no native `jiff` support. These helpers translate
//! between the two representations when reading/writing rows.

use jiff::Timestamp;
use jiff::civil::Date as JiffDate;
use time::{Date as TimeDate, Month, OffsetDateTime};

/// Convert a `jiff::Timestamp` to a `time::OffsetDateTime` in UTC.
///
/// Both types are nanosecond-precision instants, so the conversion goes
/// through the Unix-nanosecond representation and stays in UTC; no
/// timezone information is lost because `jiff::Timestamp` is itself a
/// UTC instant.
///
/// # Panics
///
/// Panics if the `jiff` nanosecond value falls outside the representable
/// range of `time::OffsetDateTime`. In practice this cannot happen for
/// real-world birth/death/audit timestamps; the wider `jiff` range only
/// matters near the year-9999 boundary.
pub fn ts_to_offset(ts: Timestamp) -> OffsetDateTime {
    // Round-trip via Unix nanoseconds: the lossless common denominator
    // between the two instant types.
    OffsetDateTime::from_unix_timestamp_nanos(ts.as_nanosecond())
        .expect("jiff Timestamp within time::OffsetDateTime range")
}

/// Convert a `time::OffsetDateTime` to a `jiff::Timestamp`.
///
/// Inverse of [`ts_to_offset`]. `time::OffsetDateTime` carries an offset,
/// but `unix_timestamp_nanos` normalizes it to a UTC instant first, so
/// the resulting `jiff::Timestamp` is offset-independent.
///
/// # Panics
///
/// Panics if the Unix-nanosecond value is outside the representable
/// range of `jiff::Timestamp`. Unreachable for stored real-world rows.
pub fn offset_to_ts(odt: OffsetDateTime) -> Timestamp {
    Timestamp::from_nanosecond(odt.unix_timestamp_nanos())
        .expect("time::OffsetDateTime within jiff Timestamp range")
}

/// Convert a `jiff::civil::Date` to a `time::Date` (calendar date only).
///
/// Used for `birth_date` and document issue/expiry columns, which are
/// stored as bare SQL `DATE` values. The `jiff` month is a `1..=12`
/// integer; `time` models the month as a `Month` enum, so it is bridged
/// through `Month::try_from`.
///
/// # Panics
///
/// Panics if the month is not `1..=12` or the year/day combination is
/// not a valid calendar date. A well-formed `jiff::civil::Date` always
/// satisfies both, so this is unreachable for values produced by `jiff`.
pub fn date_to_time(d: JiffDate) -> TimeDate {
    // jiff months are 1-based integers; time wants its Month enum.
    let month = Month::try_from(d.month() as u8).expect("valid month");
    TimeDate::from_calendar_date(d.year() as i32, month, d.day() as u8).expect("valid date")
}

/// Convert a `time::Date` to a `jiff::civil::Date` (calendar date only).
///
/// Inverse of [`date_to_time`], used when hydrating domain models from
/// stored `DATE` columns. `u8::from(d.month())` turns the `time` `Month`
/// enum back into its `1..=12` ordinal, which `jiff` expects as an `i8`.
///
/// # Panics
///
/// Panics if the year/month/day triple is not a valid calendar date. A
/// value read from a valid `time::Date` always is, so this is
/// unreachable for stored rows.
pub fn time_to_date(d: TimeDate) -> JiffDate {
    JiffDate::new(d.year() as i16, u8::from(d.month()) as i8, d.day() as i8).expect("valid date")
}
