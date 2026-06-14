//! Date/time conversions across the persistence boundary.
//!
//! Domain models use `jiff` (`Timestamp`); the SeaORM entity models use
//! the `time` crate (`OffsetDateTime`) because SeaORM 1.1 has no native
//! `jiff` support.

use jiff::Timestamp;
use time::OffsetDateTime;

/// `jiff::Timestamp` → `time::OffsetDateTime` (UTC).
///
/// Both types are nanosecond-since-epoch wrappers, so the conversion is
/// a lossless re-wrap of the same instant in UTC.
///
/// # Panics
///
/// Panics if the source instant falls outside the representable range of
/// `time::OffsetDateTime`. In practice this never happens: any real
/// registry timestamp is comfortably within both crates' ranges.
pub fn ts_to_offset(ts: Timestamp) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp_nanos(ts.as_nanosecond())
        .expect("jiff Timestamp within time::OffsetDateTime range")
}

/// `time::OffsetDateTime` → `jiff::Timestamp`.
///
/// The inverse of [`ts_to_offset`]; re-wraps the same nanosecond instant.
///
/// # Panics
///
/// Panics if the source instant falls outside the representable range of
/// `jiff::Timestamp`. As above, this cannot occur for real persisted rows.
pub fn offset_to_ts(odt: OffsetDateTime) -> Timestamp {
    Timestamp::from_nanosecond(odt.unix_timestamp_nanos())
        .expect("time::OffsetDateTime within jiff Timestamp range")
}
