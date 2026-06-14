//! Date/time conversions across the persistence boundary.
//!
//! Domain models use `chrono` (`DateTime<Utc>`, `NaiveDate`); the SeaORM
//! entity models use the `time` crate (`OffsetDateTime`, `Date`) because
//! SeaORM 1.1 wires its timestamp columns to the `time` crate here. These
//! helpers translate between the two representations when reading/writing
//! rows.
//!
//! All four helpers convert via nanosecond / calendar-component round-trips
//! and assume the values originate from the other library's own valid range,
//! so the conversions are total in practice (see each `# Panics` note for the
//! theoretical out-of-range case).

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use time::{Date as TimeDate, Month, OffsetDateTime};

/// `chrono::DateTime<Utc>` → `time::OffsetDateTime` (UTC), via the nanosecond epoch.
///
/// # Panics
///
/// Panics if the timestamp's nanosecond value falls outside the range
/// representable as `i64` nanoseconds (roughly years 1678–2262) or outside
/// the range representable by [`time::OffsetDateTime`]. Neither happens for
/// values that arose from a real `chrono::DateTime<Utc>` within that range.
pub fn ts_to_offset(ts: DateTime<Utc>) -> OffsetDateTime {
    // Both crates agree on the Unix nanosecond epoch, so this is a lossless
    // numeric hand-off; only an out-of-range value could fail.
    let nanos = ts
        .timestamp_nanos_opt()
        .expect("chrono DateTime within nanosecond range");
    OffsetDateTime::from_unix_timestamp_nanos(i128::from(nanos))
        .expect("DateTime within time::OffsetDateTime range")
}

/// `time::OffsetDateTime` → `chrono::DateTime<Utc>`, via the nanosecond epoch.
///
/// # Panics
///
/// Panics if the offset date-time's nanosecond value falls outside the range
/// representable as `i64` nanoseconds by [`chrono::DateTime`].
pub fn offset_to_ts(odt: OffsetDateTime) -> DateTime<Utc> {
    // Inverse of `ts_to_offset`; `unix_timestamp_nanos` already normalizes the
    // offset to UTC.
    let nanos = i64::try_from(odt.unix_timestamp_nanos())
        .expect("time::OffsetDateTime within chrono nanosecond range");
    DateTime::from_timestamp_nanos(nanos)
}

/// `chrono::NaiveDate` → `time::Date`, rebuilt from year/month/day components.
///
/// # Panics
///
/// Panics if the month or the (year, month, day) triple is rejected by `time`
/// — which cannot happen for a valid `chrono::NaiveDate`, since `chrono` only
/// ever yields legal calendar dates.
pub fn date_to_time(d: NaiveDate) -> TimeDate {
    // `time::Month` is an enum (1..=12); convert the numeric chrono month first.
    let month = Month::try_from(d.month() as u8).expect("valid month");
    TimeDate::from_calendar_date(d.year(), month, d.day() as u8).expect("valid date")
}

/// `time::Date` → `chrono::NaiveDate`, rebuilt from year/month/day components.
///
/// # Panics
///
/// Panics if the (year, month, day) triple is rejected by `chrono` — which
/// cannot happen for a valid `time::Date`.
pub fn time_to_date(d: TimeDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(d.year(), u8::from(d.month()) as u32, d.day() as u32)
        .expect("valid date")
}
