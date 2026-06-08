//! Date/time conversions across the persistence boundary.
//!
//! Domain models use `jiff` (`Timestamp`); the SeaORM entity models use
//! the `time` crate (`OffsetDateTime`) because SeaORM 1.1 has no native
//! `jiff` support.

use jiff::Timestamp;
use time::OffsetDateTime;

/// `jiff::Timestamp` → `time::OffsetDateTime` (UTC).
pub fn ts_to_offset(ts: Timestamp) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp_nanos(ts.as_nanosecond())
        .expect("jiff Timestamp within time::OffsetDateTime range")
}

/// `time::OffsetDateTime` → `jiff::Timestamp`.
pub fn offset_to_ts(odt: OffsetDateTime) -> Timestamp {
    Timestamp::from_nanosecond(odt.unix_timestamp_nanos())
        .expect("time::OffsetDateTime within jiff Timestamp range")
}
