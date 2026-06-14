//! Date/time conversions across the persistence boundary.
//!
//! Domain models use `jiff` (`Timestamp`, `civil::Date`); the SeaORM
//! entity models use the `time` crate (`OffsetDateTime`, `Date`) because
//! SeaORM 1.1 has no native `jiff` support. These helpers translate
//! between the two representations when reading/writing rows.
//!
//! All four helpers convert via nanosecond / calendar-component round-trips
//! and assume the values originate from the other library's own valid range,
//! so the conversions are total in practice (see each `# Panics` note for the
//! theoretical out-of-range case).

use jiff::Timestamp;
use jiff::civil::Date as JiffDate;
use time::{Date as TimeDate, Month, OffsetDateTime};

/// `jiff::Timestamp` → `time::OffsetDateTime` (UTC), via the nanosecond epoch.
///
/// # Panics
///
/// Panics if the timestamp's nanosecond value falls outside the range
/// representable by [`time::OffsetDateTime`]. `jiff`'s supported range is
/// narrower in practice, so this does not panic for values that arose from a
/// real `jiff::Timestamp`.
pub fn ts_to_offset(ts: Timestamp) -> OffsetDateTime {
    // Both crates agree on the Unix nanosecond epoch, so this is a lossless
    // numeric hand-off; only an out-of-range value could fail.
    OffsetDateTime::from_unix_timestamp_nanos(ts.as_nanosecond())
        .expect("jiff Timestamp within time::OffsetDateTime range")
}

/// `time::OffsetDateTime` → `jiff::Timestamp`, via the nanosecond epoch.
///
/// # Panics
///
/// Panics if the offset date-time's nanosecond value falls outside the range
/// representable by [`jiff::Timestamp`].
pub fn offset_to_ts(odt: OffsetDateTime) -> Timestamp {
    // Inverse of `ts_to_offset`; `unix_timestamp_nanos` already normalizes the
    // offset to UTC.
    Timestamp::from_nanosecond(odt.unix_timestamp_nanos())
        .expect("time::OffsetDateTime within jiff Timestamp range")
}

/// `jiff::civil::Date` → `time::Date`, rebuilt from year/month/day components.
///
/// # Panics
///
/// Panics if the month or the (year, month, day) triple is rejected by `time`
/// — which cannot happen for a valid `jiff::civil::Date`, since `jiff` only
/// ever yields legal calendar dates.
pub fn date_to_time(d: JiffDate) -> TimeDate {
    // `time::Month` is an enum (1..=12); convert the numeric jiff month first.
    let month = Month::try_from(d.month() as u8).expect("valid month");
    TimeDate::from_calendar_date(d.year() as i32, month, d.day() as u8).expect("valid date")
}

/// `time::Date` → `jiff::civil::Date`, rebuilt from year/month/day components.
///
/// # Panics
///
/// Panics if the (year, month, day) triple is rejected by `jiff` — which
/// cannot happen for a valid `time::Date`.
pub fn time_to_date(d: TimeDate) -> JiffDate {
    // jiff takes signed component types (i16 year, i8 month/day); the casts are
    // exact for any in-range calendar date.
    JiffDate::new(d.year() as i16, u8::from(d.month()) as i8, d.day() as i8).expect("valid date")
}
