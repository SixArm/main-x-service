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
///
/// Used when writing a domain timestamp into a `SeaORM` row. Both types
/// carry nanosecond UTC instants, so the conversion is lossless.
///
/// # Panics
///
/// Panics if the `jiff` instant falls outside `time::OffsetDateTime`'s
/// representable range. In practice both ranges cover roughly
/// ±200000 years,
/// so any real-world event timestamp is safe.
pub fn ts_to_offset(ts: Timestamp) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp_nanos(ts.as_nanosecond())
        .expect("jiff Timestamp within time::OffsetDateTime range")
}

/// `time::OffsetDateTime` → `jiff::Timestamp`.
///
/// Used when reading a `SeaORM` row back into a domain model.
///
/// # Panics
///
/// Panics if the `time` instant falls outside `jiff::Timestamp`'s
/// representable range (see [`ts_to_offset`]); unreachable for stored
/// event data.
pub fn offset_to_ts(odt: OffsetDateTime) -> Timestamp {
    Timestamp::from_nanosecond(odt.unix_timestamp_nanos())
        .expect("time::OffsetDateTime within jiff Timestamp range")
}

/// `jiff::civil::Date` → `time::Date`.
///
/// For date-only columns (no time-of-day). The intermediate
/// `time::Month` conversion is why this is not a one-liner: `time`
/// models the month as an enum, `jiff` as a `1..=12` integer.
///
/// # Panics
///
/// Panics if the month is not `1..=12` or the year/day combination is
/// not a real calendar date. A `jiff::civil::Date` is always valid, so
/// neither `expect` can fire for inputs produced by `jiff`.
pub fn date_to_time(d: JiffDate) -> TimeDate {
    let month = Month::try_from(d.month() as u8).expect("valid month");
    TimeDate::from_calendar_date(d.year() as i32, month, d.day() as u8).expect("valid date")
}

/// `time::Date` → `jiff::civil::Date`.
///
/// Inverse of [`date_to_time`], used when reading a date column back
/// into a domain model. `jiff` uses narrower integer types (`i16` year,
/// `i8` month/day), hence the casts.
///
/// # Panics
///
/// Panics if the components do not form a valid `jiff` date; unreachable
/// for a value produced by `time::Date`.
pub fn time_to_date(d: TimeDate) -> JiffDate {
    JiffDate::new(d.year() as i16, u8::from(d.month()) as i8, d.day() as i8).expect("valid date")
}
