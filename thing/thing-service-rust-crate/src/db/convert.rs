//! Date/time conversions across the persistence boundary.
//!
//! Domain models use `chrono` (`DateTime<Utc>`); the SeaORM entity models
//! use the `time` crate (`OffsetDateTime`) because SeaORM 1.1 is configured
//! with `with-time`.

use chrono::{DateTime, Utc};
use time::OffsetDateTime;

/// `chrono::DateTime<Utc>` → `time::OffsetDateTime` (UTC).
///
/// Both types are nanosecond-since-epoch wrappers, so the conversion is
/// a lossless re-wrap of the same instant in UTC.
///
/// # Panics
///
/// Panics if the source instant falls outside the representable range of
/// `time::OffsetDateTime`. In practice this never happens: any real
/// registry timestamp is comfortably within both crates' ranges.
#[must_use]
pub fn ts_to_offset(ts: DateTime<Utc>) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp_nanos(i128::from(ts.timestamp_nanos_opt().unwrap_or(0)))
        .expect("chrono DateTime within time::OffsetDateTime range")
}

/// `time::OffsetDateTime` → `chrono::DateTime<Utc>`.
///
/// The inverse of [`ts_to_offset`]; re-wraps the same nanosecond instant.
///
/// # Panics
///
/// Panics if the source instant falls outside the representable range of
/// `chrono::DateTime<Utc>`. As above, this cannot occur for real persisted
/// rows.
#[must_use]
pub fn offset_to_ts(odt: OffsetDateTime) -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(odt.unix_timestamp(), odt.nanosecond())
        .expect("time::OffsetDateTime within chrono DateTime range")
}
