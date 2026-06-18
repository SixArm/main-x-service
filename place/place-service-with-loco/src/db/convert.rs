//! Date/time conversions across the persistence boundary.
//!
//! Domain models use `chrono` (`DateTime<Utc>`); the `SeaORM` entity models
//! use the `time` crate (`OffsetDateTime`) because `SeaORM` 1.1 is configured
//! with `time` support. These helpers translate between the two
//! representations when reading/writing rows.

use chrono::{DateTime, Utc};
use time::OffsetDateTime;

/// `chrono::DateTime<Utc>` → `time::OffsetDateTime` (UTC).
///
/// # Panics
///
/// Panics if the chrono nanosecond value cannot be represented (only near
/// the chrono timestamp-nanosecond range limits, ~1677–2262), or if it
/// falls outside the representable range of `time::OffsetDateTime`. In
/// practice this cannot happen for real-world audit timestamps.
#[must_use]
pub fn ts_to_offset(ts: DateTime<Utc>) -> OffsetDateTime {
    let nanos = ts
        .timestamp_nanos_opt()
        .expect("chrono DateTime within time::OffsetDateTime range");
    OffsetDateTime::from_unix_timestamp_nanos(i128::from(nanos))
        .expect("chrono DateTime within time::OffsetDateTime range")
}

/// `time::OffsetDateTime` → `chrono::DateTime<Utc>`.
///
/// # Panics
///
/// Panics if the Unix-nanosecond value is outside the representable
/// range of `chrono::DateTime<Utc>`. Unreachable for stored real-world
/// rows.
#[must_use]
pub fn offset_to_ts(odt: OffsetDateTime) -> DateTime<Utc> {
    let nanos = i64::try_from(odt.unix_timestamp_nanos())
        .expect("time::OffsetDateTime within chrono DateTime range");
    DateTime::from_timestamp_nanos(nanos)
}
