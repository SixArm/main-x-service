//! Date/time conversions across the persistence boundary.
//!
//! Domain models use `chrono` (`DateTime<Utc>`, `NaiveDate`); the `SeaORM`
//! entity models use the `time` crate (`OffsetDateTime`, `Date`) because
//! `SeaORM` 1.1 is configured with the `with-time` feature. These helpers
//! translate between the two representations when reading/writing rows.

use chrono::{DateTime, Utc};
use time::OffsetDateTime;

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
